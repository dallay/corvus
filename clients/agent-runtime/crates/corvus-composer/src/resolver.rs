use crate::manifest::AgentManifest;
use crate::plan::{
    AgentMetadata, CapabilityReport, ComposedRuntimePlan, IdentitySettings, MemorySettings,
    RuntimeSettings, SelectedCapability,
};
use crate::registry_snapshot::{CapabilityFamily, CapabilityStatus, RegistrySnapshot};
use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    UnsupportedVersion {
        version: String,
    },
    MissingRequiredFamily {
        family: &'static str,
    },
    DefaultProviderDisabled {
        name: String,
    },
    DefaultChannelDisabled {
        name: String,
    },
    UnknownCapability {
        family: &'static str,
        name: String,
    },
    FamilyMismatch {
        expected_family: &'static str,
        actual_family: &'static str,
        name: String,
    },
    UncompiledCapability {
        family: &'static str,
        name: String,
    },
    PlatformUnavailableCapability {
        family: &'static str,
        name: String,
    },
    InvalidCapabilityConfig {
        family: &'static str,
        name: String,
        reason: String,
    },
    ToolRestrictionsNotSubset {
        restrictions: Vec<String>,
        enabled: Vec<String>,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported manifest version '{version}'")
            }
            Self::MissingRequiredFamily { family } => {
                write!(f, "manifest must select at least one {family}")
            }
            Self::DefaultProviderDisabled { name } => {
                write!(f, "default provider '{name}' must be enabled")
            }
            Self::DefaultChannelDisabled { name } => {
                write!(f, "default channel '{name}' must be enabled")
            }
            Self::UnknownCapability { family, name } => {
                write!(f, "unknown {family} capability '{name}'")
            }
            Self::FamilyMismatch {
                expected_family,
                actual_family,
                name,
            } => write!(
                f,
                "capability '{name}' belongs to family '{actual_family}', not '{expected_family}'"
            ),
            Self::UncompiledCapability { family, name } => write!(
                f,
                "{family} capability '{name}' is known but not compiled into this runtime artifact"
            ),
            Self::PlatformUnavailableCapability { family, name } => write!(
                f,
                "{family} capability '{name}' is unavailable on this platform"
            ),
            Self::InvalidCapabilityConfig {
                family,
                name,
                reason,
            } => write!(f, "invalid {family} configuration for '{name}': {reason}"),
            Self::ToolRestrictionsNotSubset {
                restrictions,
                enabled,
            } => write!(
                f,
                "tool restrictions {:?} must be subset of enabled tools {:?}",
                restrictions, enabled
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn parse_manifest(toml_str: &str) -> Result<AgentManifest> {
    toml::from_str(toml_str).context("failed to parse manifest TOML")
}

pub fn resolve_manifest(
    manifest: &AgentManifest,
    snapshot: &RegistrySnapshot,
) -> Result<ComposedRuntimePlan, ValidationError> {
    validate_version(&manifest.version)?;
    ensure_non_empty(CapabilityFamily::Provider, &manifest.providers.enabled)?;
    ensure_non_empty(CapabilityFamily::Channel, &manifest.channels.enabled)?;

    // Canonicalize and validate default provider against registry
    let _default_provider = snapshot
        .find_in_family(CapabilityFamily::Provider, &manifest.providers.default)
        .map(|r| r.key)
        .ok_or_else(|| ValidationError::UnknownCapability {
            family: CapabilityFamily::Provider.as_str(),
            name: manifest.providers.default.clone(),
        })?;
    if !manifest
        .providers
        .enabled
        .iter()
        .any(|item| item.eq_ignore_ascii_case(&manifest.providers.default))
    {
        return Err(ValidationError::DefaultProviderDisabled {
            name: manifest.providers.default.clone(),
        });
    }

    // Canonicalize and validate default channel against registry
    if let Some(default_channel) = &manifest.channels.default {
        let _canonical_channel = snapshot
            .find_in_family(CapabilityFamily::Channel, default_channel)
            .map(|r| r.key)
            .ok_or_else(|| ValidationError::UnknownCapability {
                family: CapabilityFamily::Channel.as_str(),
                name: default_channel.clone(),
            })?;
        if !manifest
            .channels
            .enabled
            .iter()
            .any(|item| item.eq_ignore_ascii_case(default_channel))
        {
            return Err(ValidationError::DefaultChannelDisabled {
                name: default_channel.clone(),
            });
        }
    }

    validate_selected_config_keys(
        CapabilityFamily::Provider,
        &manifest.providers.enabled,
        manifest.providers.config.keys().map(String::as_str),
    )?;
    validate_selected_config_keys(
        CapabilityFamily::Channel,
        &manifest.channels.enabled,
        manifest.channels.config.keys().map(String::as_str),
    )?;
    validate_selected_config_keys(
        CapabilityFamily::Tool,
        &manifest.tools.enabled,
        manifest.tools.config.keys().map(String::as_str),
    )?;
    validate_selected_config_keys(
        CapabilityFamily::Memory,
        std::slice::from_ref(&manifest.memory.backend),
        manifest.memory.config.keys().map(String::as_str),
    )?;
    validate_selected_config_keys(
        CapabilityFamily::Observer,
        &manifest.observability.enabled,
        manifest.observability.config.keys().map(String::as_str),
    )?;
    validate_selected_config_keys(
        CapabilityFamily::Security,
        std::slice::from_ref(&manifest.security.backend),
        manifest.security.config.keys().map(String::as_str),
    )?;

    let provider = resolve_one(
        CapabilityFamily::Provider,
        &manifest.providers.default,
        snapshot,
        manifest.providers.config.get(&manifest.providers.default),
    )?;
    let channels = resolve_many(
        CapabilityFamily::Channel,
        &manifest.channels.enabled,
        snapshot,
        &manifest.channels.config,
    )?;
    let tools = resolve_many(
        CapabilityFamily::Tool,
        &manifest.tools.enabled,
        snapshot,
        &manifest.tools.config,
    )?;
    let memory = resolve_one(
        CapabilityFamily::Memory,
        &manifest.memory.backend,
        snapshot,
        manifest.memory.config.get(&manifest.memory.backend),
    )?;
    let observers = resolve_many(
        CapabilityFamily::Observer,
        &manifest.observability.enabled,
        snapshot,
        &manifest.observability.config,
    )?;
    let security = resolve_one(
        CapabilityFamily::Security,
        &manifest.security.backend,
        snapshot,
        manifest.security.config.get(&manifest.security.backend),
    )?;

    if !manifest.security.tool_restrictions.is_empty() {
        let enabled_tools: Vec<String> = tools.iter().map(|item| item.key.clone()).collect();
        if !manifest
            .security
            .tool_restrictions
            .iter()
            .all(|tool| enabled_tools.iter().any(|enabled| enabled == tool))
        {
            return Err(ValidationError::ToolRestrictionsNotSubset {
                restrictions: manifest.security.tool_restrictions.clone(),
                enabled: enabled_tools,
            });
        }
    }

    Ok(ComposedRuntimePlan {
        agent: AgentMetadata {
            name: manifest.agent.name.clone(),
            description: manifest.agent.description.clone(),
            model: manifest.agent.model.clone(),
            temperature: manifest.agent.temperature,
        },
        provider,
        channels,
        default_channel: manifest
            .channels
            .default
            .clone()
            .or_else(|| manifest.channels.enabled.first().cloned()),
        tools,
        memory,
        memory_settings: MemorySettings {
            auto_save: manifest.memory.auto_save,
        },
        observers,
        security,
        tool_restrictions: manifest.security.tool_restrictions.clone(),
        runtime: RuntimeSettings {
            profile: manifest
                .runtime
                .profile
                .clone()
                .or_else(|| manifest.agent.profile.clone()),
            max_tool_iterations: manifest.runtime.max_tool_iterations,
        },
        identity: IdentitySettings {
            format: manifest.identity.format.clone(),
            aieos_path: manifest.identity.aieos_path.clone(),
            aieos_inline: manifest.identity.aieos_inline.clone(),
        },
    })
}

pub fn required_capabilities(plan: &ComposedRuntimePlan) -> CapabilityReport {
    CapabilityReport::from(plan)
}

fn validate_version(version: &str) -> Result<(), ValidationError> {
    let trimmed = version.trim();
    if trimmed == "1" || trimmed == "1.0" || trimmed.eq_ignore_ascii_case("v1") {
        Ok(())
    } else {
        Err(ValidationError::UnsupportedVersion {
            version: version.to_string(),
        })
    }
}

fn ensure_non_empty(family: CapabilityFamily, items: &[String]) -> Result<(), ValidationError> {
    if items.is_empty() {
        Err(ValidationError::MissingRequiredFamily {
            family: family.as_str(),
        })
    } else {
        Ok(())
    }
}

fn validate_selected_config_keys<'a>(
    family: CapabilityFamily,
    selected: &[String],
    configured_keys: impl Iterator<Item = &'a str>,
) -> Result<(), ValidationError> {
    for key in configured_keys {
        if !selected.iter().any(|selected_key| selected_key == key) {
            return Err(ValidationError::InvalidCapabilityConfig {
                family: family.as_str(),
                name: key.to_string(),
                reason: "configuration exists for an unselected capability".to_string(),
            });
        }
    }
    Ok(())
}

fn resolve_many(
    family: CapabilityFamily,
    requested: &[String],
    snapshot: &RegistrySnapshot,
    config: &std::collections::BTreeMap<String, toml::Value>,
) -> Result<Vec<SelectedCapability>, ValidationError> {
    requested
        .iter()
        .map(|name| resolve_one(family, name, snapshot, config.get(name)))
        .collect()
}

fn resolve_one(
    family: CapabilityFamily,
    requested: &str,
    snapshot: &RegistrySnapshot,
    config: Option<&toml::Value>,
) -> Result<SelectedCapability, ValidationError> {
    let Some(record) = snapshot.find_in_family(family, requested) else {
        if let Some(other_family) = snapshot.find_in_other_family(family, requested) {
            return Err(ValidationError::FamilyMismatch {
                expected_family: family.as_str(),
                actual_family: other_family.family.as_str(),
                name: requested.to_string(),
            });
        }
        return Err(ValidationError::UnknownCapability {
            family: family.as_str(),
            name: requested.to_string(),
        });
    };

    match record.status {
        CapabilityStatus::Constructible => Ok(SelectedCapability {
            key: record.key.to_string(),
            config: config.cloned(),
        }),
        CapabilityStatus::Uncompiled => Err(ValidationError::UncompiledCapability {
            family: family.as_str(),
            name: record.key.to_string(),
        }),
        CapabilityStatus::PlatformUnavailable => {
            Err(ValidationError::PlatformUnavailableCapability {
                family: family.as_str(),
                name: record.key.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_snapshot::RegistrySnapshot;

    // Helper: build a minimal manifest TOML string and return the parsed AgentManifest.
    fn parse(toml_str: &str) -> crate::manifest::AgentManifest {
        parse_manifest(toml_str).expect("test manifest must parse")
    }

    fn snapshot() -> RegistrySnapshot {
        RegistrySnapshot::collect()
    }

    fn minimal_toml() -> &'static str {
        r#"
version = "1"
[agent]
name = "minimal"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
"#
    }

    // --- validate_version ---

    #[test]
    fn validate_version_accepts_1() {
        assert!(validate_version("1").is_ok());
    }

    #[test]
    fn validate_version_accepts_1_dot_0() {
        assert!(validate_version("1.0").is_ok());
    }

    #[test]
    fn validate_version_accepts_v1_case_insensitive() {
        assert!(validate_version("v1").is_ok());
        assert!(validate_version("V1").is_ok());
    }

    #[test]
    fn validate_version_accepts_version_with_surrounding_whitespace() {
        assert!(validate_version("  1  ").is_ok());
        assert!(validate_version("\t1.0\n").is_ok());
    }

    #[test]
    fn validate_version_rejects_unsupported_versions() {
        for bad in &["2", "3.0", "0", "1.1", "v2", ""] {
            let result = validate_version(bad);
            assert!(
                result.is_err(),
                "expected error for version '{bad}', got Ok"
            );
            match result.unwrap_err() {
                ValidationError::UnsupportedVersion { version } => {
                    assert_eq!(
                        version, *bad,
                        "error should preserve original version string"
                    )
                }
                other => panic!("expected UnsupportedVersion, got {other:?}"),
            }
        }
    }

    // --- ensure_non_empty ---

    #[test]
    fn ensure_non_empty_passes_when_list_has_items() {
        let items = vec!["a".to_string()];
        assert!(ensure_non_empty(CapabilityFamily::Provider, &items).is_ok());
    }

    #[test]
    fn ensure_non_empty_fails_for_empty_list() {
        let items: Vec<String> = vec![];
        let result = ensure_non_empty(CapabilityFamily::Channel, &items);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::MissingRequiredFamily { family: "channel" }
        ));
    }

    #[test]
    fn ensure_non_empty_error_includes_family_name() {
        let items: Vec<String> = vec![];
        let err = ensure_non_empty(CapabilityFamily::Tool, &items).unwrap_err();
        assert_eq!(err.to_string(), "manifest must select at least one tool");
    }

    // --- validate_selected_config_keys ---

    #[test]
    fn validate_selected_config_keys_passes_when_all_keys_are_selected() {
        let selected = vec!["shell".to_string(), "file_read".to_string()];
        let configured_keys = ["shell", "file_read"];
        let result = validate_selected_config_keys(
            CapabilityFamily::Tool,
            &selected,
            configured_keys.iter().copied(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_selected_config_keys_passes_with_empty_config() {
        let selected = vec!["shell".to_string()];
        let configured_keys: &[&str] = &[];
        let result = validate_selected_config_keys(
            CapabilityFamily::Tool,
            &selected,
            configured_keys.iter().copied(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_selected_config_keys_rejects_unselected_config_key() {
        let selected = vec!["shell".to_string()];
        let configured_keys = ["browser"]; // not in selected list
        let result = validate_selected_config_keys(
            CapabilityFamily::Tool,
            &selected,
            configured_keys.iter().copied(),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ValidationError::InvalidCapabilityConfig { ref name, .. } if name == "browser"
        ));
        assert!(err
            .to_string()
            .contains("configuration exists for an unselected capability"));
    }

    // --- ValidationError Display ---

    #[test]
    fn validation_error_display_unsupported_version() {
        let err = ValidationError::UnsupportedVersion {
            version: "99".to_string(),
        };
        assert_eq!(err.to_string(), "unsupported manifest version '99'");
    }

    #[test]
    fn validation_error_display_missing_required_family() {
        let err = ValidationError::MissingRequiredFamily { family: "provider" };
        assert_eq!(
            err.to_string(),
            "manifest must select at least one provider"
        );
    }

    #[test]
    fn validation_error_display_default_provider_disabled() {
        let err = ValidationError::DefaultProviderDisabled {
            name: "openai".to_string(),
        };
        assert_eq!(err.to_string(), "default provider 'openai' must be enabled");
    }

    #[test]
    fn validation_error_display_default_channel_disabled() {
        let err = ValidationError::DefaultChannelDisabled {
            name: "telegram".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "default channel 'telegram' must be enabled"
        );
    }

    #[test]
    fn validation_error_display_unknown_capability() {
        let err = ValidationError::UnknownCapability {
            family: "provider",
            name: "unknown-thing".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "unknown provider capability 'unknown-thing'"
        );
    }

    #[test]
    fn validation_error_display_family_mismatch() {
        let err = ValidationError::FamilyMismatch {
            expected_family: "provider",
            actual_family: "tool",
            name: "shell".to_string(),
        };
        assert!(err.to_string().contains("belongs to family 'tool'"));
        assert!(err.to_string().contains("not 'provider'"));
    }

    #[test]
    fn validation_error_display_uncompiled_capability() {
        let err = ValidationError::UncompiledCapability {
            family: "channel",
            name: "webhook".to_string(),
        };
        assert!(err.to_string().contains("known but not compiled"));
    }

    #[test]
    fn validation_error_display_platform_unavailable() {
        let err = ValidationError::PlatformUnavailableCapability {
            family: "channel",
            name: "imessage".to_string(),
        };
        assert!(err.to_string().contains("unavailable on this platform"));
    }

    #[test]
    fn validation_error_display_tool_restrictions_not_subset() {
        let err = ValidationError::ToolRestrictionsNotSubset {
            restrictions: vec!["unknown_tool".to_string()],
            enabled: vec!["shell".to_string()],
        };
        assert!(err.to_string().contains("must be subset of enabled tools"));
    }

    // --- resolve_manifest ---

    #[test]
    fn resolve_manifest_succeeds_for_minimal_valid_manifest() {
        let manifest = parse(minimal_toml());
        let snap = snapshot();
        let result = resolve_manifest(&manifest, &snap);
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }

    #[test]
    fn resolve_manifest_populates_agent_metadata() {
        let manifest = parse(minimal_toml());
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        assert_eq!(plan.agent.name, "minimal");
    }

    #[test]
    fn resolve_manifest_default_channel_falls_back_to_first_enabled() {
        // [channels] has no 'default' field; should fall back to first enabled.
        let manifest = parse(minimal_toml());
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        assert_eq!(plan.default_channel.as_deref(), Some("stdio"));
    }

    #[test]
    fn resolve_manifest_explicit_default_channel_is_preserved() {
        let toml_str = r#"
version = "1"
[agent]
name = "multi-chan"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio", "telegram"]
default = "telegram"
[memory]
backend = "none"
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        assert_eq!(plan.default_channel.as_deref(), Some("telegram"));
    }

    #[test]
    fn resolve_manifest_runtime_profile_falls_back_to_agent_profile() {
        let toml_str = r#"
version = "1"
[agent]
name = "profile-agent"
profile = "code"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        // runtime.profile should fall back to agent.profile when not explicitly set
        assert_eq!(plan.runtime.profile.as_deref(), Some("code"));
    }

    #[test]
    fn resolve_manifest_runtime_profile_section_overrides_agent_profile() {
        let toml_str = r#"
version = "1"
[agent]
name = "profile-override"
profile = "lite"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
[runtime]
profile = "full"
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        assert_eq!(plan.runtime.profile.as_deref(), Some("full"));
    }

    #[test]
    fn resolve_manifest_tool_restrictions_must_be_subset_of_enabled_tools() {
        let toml_str = r#"
version = "1"
[agent]
name = "restricted"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
[tools]
enabled = ["shell"]
[security]
backend = "none"
tool_restrictions = ["shell", "file_read"]
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let result = resolve_manifest(&manifest, &snap);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::ToolRestrictionsNotSubset { .. }
        ));
    }

    #[test]
    fn resolve_manifest_valid_tool_restrictions_subset_passes() {
        let toml_str = r#"
version = "1"
[agent]
name = "ok-restricted"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
[tools]
enabled = ["shell", "file_read"]
[security]
backend = "none"
tool_restrictions = ["shell"]
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let result = resolve_manifest(&manifest, &snap);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.tool_restrictions, vec!["shell"]);
    }

    #[test]
    fn resolve_manifest_default_channel_not_in_enabled_fails() {
        let toml_str = r#"
version = "1"
[agent]
name = "bad-default"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
default = "telegram"
[memory]
backend = "none"
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let result = resolve_manifest(&manifest, &snap);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::DefaultChannelDisabled { name } if name == "telegram"
        ));
    }

    #[test]
    fn resolve_manifest_memory_settings_auto_save_propagated() {
        let toml_str = r#"
version = "1"
[agent]
name = "memory-agent"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "sqlite"
auto_save = true
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        assert_eq!(plan.memory_settings.auto_save, Some(true));
    }

    #[test]
    fn resolve_manifest_identity_fields_propagated() {
        let toml_str = r#"
version = "1"
[agent]
name = "identity-agent"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
[identity]
format = "openclaw"
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let plan = resolve_manifest(&manifest, &snap).unwrap();
        assert_eq!(plan.identity.format.as_deref(), Some("openclaw"));
    }

    // Regression: unsupported version in manifest must be caught before registry lookup.
    #[test]
    fn resolve_manifest_rejects_unsupported_manifest_version() {
        let toml_str = r#"
version = "99"
[agent]
name = "future"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
"#;
        let manifest = parse(toml_str);
        let snap = snapshot();
        let result = resolve_manifest(&manifest, &snap);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::UnsupportedVersion { .. }
        ));
    }

    // --- parse_manifest ---

    #[test]
    fn parse_manifest_returns_error_for_invalid_toml() {
        let result = parse_manifest("this is not valid toml !!!");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("failed to parse manifest TOML"));
    }

    #[test]
    fn parse_manifest_returns_error_for_missing_required_fields() {
        // Missing [providers], [channels], [memory]
        let result = parse_manifest("version = \"1\"\n[agent]\nname = \"x\"\n");
        assert!(result.is_err());
    }
}
