use crate::manifest::AgentManifest;
use crate::plan::{
    AgentMetadata, CapabilityReport, ComposedRuntimePlan, IdentitySettings, RuntimeSettings,
    SelectedCapability,
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
    if !manifest
        .providers
        .enabled
        .iter()
        .any(|item| item == &manifest.providers.default)
    {
        return Err(ValidationError::DefaultProviderDisabled {
            name: manifest.providers.default.clone(),
        });
    }
    if let Some(default_channel) = &manifest.channels.default {
        if !manifest
            .channels
            .enabled
            .iter()
            .any(|item| item == default_channel)
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
