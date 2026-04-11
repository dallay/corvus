//! Corvus Composer - Agent Manifest and Composer
//!
//! Provides the Agent Manifest TOML schema and `AgentComposer` for
//! composing agents from manifests using the existing runtime registries.

use anyhow::{Context, Result};

use serde::{Deserialize, Serialize};

// =============================================================================
// Agent Manifest Schema (TOML)
// =============================================================================

/// Agent Manifest - defines agent composition via TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    /// Manifest version (required)
    pub version: String,

    /// Agent name (required)
    pub name: String,

    /// Agent description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Provider configuration
    pub providers: ProviderSection,

    /// Channel configuration
    pub channels: ChannelSection,

    /// Tools configuration  
    pub tools: ToolsSection,

    /// Memory configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemorySection>,

    /// Observer configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observer: Option<ObserverSection>,

    /// Security configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySection>,
}

/// Provider section - at least one must be enabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSection {
    /// List of provider names (required, at least one)
    pub providers: Vec<String>,

    /// Default provider (required, must be in providers list)
    pub default: String,

    /// Model to use
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Temperature setting
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// Channel section - at least one must be enabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSection {
    /// List of channel names (required, at least one)
    pub channels: Vec<String>,

    /// Default channel
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Tools section configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsSection {
    /// List of tool names (required)
    pub tools: Vec<String>,

    /// Tool mode: "allow" or "deny"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Memory section configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    /// Memory backend: "sqlite", "none", etc.
    pub backend: String,

    /// Additional config
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<toml::Value>,
}

/// Observer section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverSection {
    /// Observer name
    pub name: String,

    /// Config
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<toml::Value>,
}

/// Security section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySection {
    /// Sandbox backend: "wasmi", "landlock", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,

    /// Tool restrictions (subset of enabled tools)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_restrictions: Option<Vec<String>>,
}

// =============================================================================
// Validation Errors
// =============================================================================

/// Validation errors for manifests and capability reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    /// No providers configured
    NoProviders,
    /// No channels configured
    NoChannels,
    /// Default provider not in enabled list
    DefaultProviderDisabled { name: String },
    /// Unknown capability referenced
    UnknownCapability { name: String, kind: String },
    /// Invalid memory backend
    InvalidMemoryBackend { backend: String },
    /// Invalid sandbox backend
    InvalidSandboxBackend { sandbox: String },
    /// Tool restrictions must be subset of enabled tools
    ToolRestrictionsNotSubset {
        restrictions: Vec<String>,
        enabled: Vec<String>,
    },
    /// Inline secrets not allowed
    InlineSecret { field: String },
    /// No tools configured
    NoTools,
    /// No default channel specified
    NoDefaultChannel,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::NoProviders => write!(f, "at least one provider must be configured"),
            ValidationError::NoChannels => write!(f, "at least one channel must be configured"),
            ValidationError::DefaultProviderDisabled { name } => {
                write!(f, "default provider '{}' must be enabled", name)
            }
            ValidationError::UnknownCapability { name, kind } => {
                write!(f, "unknown {} capability '{}'", kind, name)
            }
            ValidationError::InvalidMemoryBackend { backend } => {
                write!(f, "invalid memory backend '{}'", backend)
            }
            ValidationError::InvalidSandboxBackend { sandbox } => {
                write!(f, "invalid sandbox backend '{}'", sandbox)
            }
            ValidationError::ToolRestrictionsNotSubset {
                restrictions,
                enabled,
            } => {
                write!(
                    f,
                    "tool restrictions {:?} must be subset of enabled tools {:?}",
                    restrictions, enabled
                )
            }
            ValidationError::InlineSecret { field } => {
                write!(
                    f,
                    "inline secret not allowed in '{}'; use environment variable references",
                    field
                )
            }
            ValidationError::NoTools => write!(f, "at least one tool must be configured"),
            ValidationError::NoDefaultChannel => write!(
                f,
                "default channel must be specified when multiple channels are configured"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

// =============================================================================
// Capability Report
// =============================================================================

/// Report of required capabilities for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityReport {
    /// Required provider names
    pub providers: Vec<String>,
    /// Required channel names
    pub channels: Vec<String>,
    /// Required tool names
    pub tools: Vec<String>,
    /// Required memory backend (if specified)
    pub memory_backend: Option<String>,
    /// Required observer (if specified)
    pub observer: Option<String>,
    /// Required sandbox (if specified)
    pub sandbox: Option<String>,
}

// =============================================================================
// Known Capabilities (from registries)
// =============================================================================

/// Known provider names (registered in the runtime)
pub const KNOWN_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "openrouter",
    "google",
    "gemini",
    "azure",
    "cerebro",
    "ollama",
    "xai",
];

/// Known channel names
pub const KNOWN_CHANNELS: &[&str] = &["telegram", "discord", "slack", "webhook", "stdio"];

/// Known tool names
pub const KNOWN_TOOLS: &[&str] = &[
    "shell",
    "file_read",
    "file_write",
    "browser",
    "http_request",
    "memory_recall",
    "memory_store",
    "memory_forget",
    "web_search_tool",
    "code_search",
    "git_operations",
    "image_info",
    "screenshot",
    "browser_open",
    "delegate",
    "composio",
    "cron_add",
    "cron_list",
    "cron_remove",
    "cron_run",
    "cron_runs",
    "cron_update",
    "pushover",
    "schedule",
    "hardware_board_info",
    "hardware_memory_map",
    "hardware_memory_read",
];

/// Known memory backends
pub const KNOWN_MEMORY_BACKENDS: &[&str] = &["sqlite", "none"];

/// Known sandbox backends  
pub const KNOWN_SANDBOX_BACKENDS: &[&str] = &["wasmi", "landlock", "bubblewrap", "none"];

// =============================================================================
// Agent Composer
// =============================================================================

/// Agent Composer - parses and validates manifests, resolves components from registries
pub struct AgentComposer {
    manifest: AgentManifest,
    reports: CapabilityReport,
}

/// Reasons why a validation might warn (non-fatal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

impl AgentComposer {
    /// Create a new composer from a manifest
    pub fn from_manifest(manifest: AgentManifest) -> Result<Self> {
        let composer = Self {
            manifest: manifest.clone(),
            reports: CapabilityReport {
                providers: manifest.providers.providers.clone(),
                channels: manifest.channels.channels.clone(),
                tools: manifest.tools.tools.clone(),
                memory_backend: manifest.memory.as_ref().map(|m| m.backend.clone()),
                observer: manifest.observer.as_ref().map(|o| o.name.clone()),
                sandbox: manifest.security.as_ref().and_then(|s| s.sandbox.clone()),
            },
        };

        // Validate and return
        composer.validate()?;
        Ok(composer)
    }

    /// Parse a manifest from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let manifest: AgentManifest =
            toml::from_str(toml_str).context("failed to parse manifest TOML")?;
        Self::from_manifest(manifest)
    }

    /// Parse a manifest from a file path
    pub fn from_path(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest from {}", path.display()))?;
        Self::from_toml(&content)
    }

    /// Validate the manifest per PRD requirements (R1-R7)
    pub fn validate(&self) -> Result<(), ValidationError> {
        // R1: At least one provider must be enabled
        if self.manifest.providers.providers.is_empty() {
            return Err(ValidationError::NoProviders);
        }

        // R2: At least one channel must be enabled
        if self.manifest.channels.channels.is_empty() {
            return Err(ValidationError::NoChannels);
        }

        // R3: Default provider must be enabled
        let default = &self.manifest.providers.default;
        if !self.manifest.providers.providers.contains(default) {
            return Err(ValidationError::DefaultProviderDisabled {
                name: default.clone(),
            });
        }

        // R4: Enabled capabilities must be known
        for provider in &self.manifest.providers.providers {
            if !KNOWN_PROVIDERS.contains(&provider.as_str()) {
                return Err(ValidationError::UnknownCapability {
                    name: provider.clone(),
                    kind: "provider".to_string(),
                });
            }
        }

        for channel in &self.manifest.channels.channels {
            if !KNOWN_CHANNELS.contains(&channel.as_str()) {
                return Err(ValidationError::UnknownCapability {
                    name: channel.clone(),
                    kind: "channel".to_string(),
                });
            }
        }

        for tool in &self.manifest.tools.tools {
            if !KNOWN_TOOLS.contains(&tool.as_str()) {
                return Err(ValidationError::UnknownCapability {
                    name: tool.clone(),
                    kind: "tool".to_string(),
                });
            }
        }

        // R5: Memory backend must be valid
        if let Some(memory) = &self.manifest.memory {
            if !KNOWN_MEMORY_BACKENDS.contains(&memory.backend.as_str()) {
                return Err(ValidationError::InvalidMemoryBackend {
                    backend: memory.backend.clone(),
                });
            }
        }

        // R6: Security sandbox must be valid
        if let Some(security) = &self.manifest.security {
            if let Some(sandbox) = &security.sandbox {
                if !sandbox.is_empty() && !KNOWN_SANDBOX_BACKENDS.contains(&sandbox.as_str()) {
                    return Err(ValidationError::InvalidSandboxBackend {
                        sandbox: sandbox.clone(),
                    });
                }
            }

            // R7: Tool restrictions must be subset of enabled tools
            if let Some(restrictions) = &security.tool_restrictions {
                if !restrictions.is_empty() {
                    let enabled: Vec<&str> = self
                        .manifest
                        .tools
                        .tools
                        .iter()
                        .map(|s| s.as_str())
                        .collect();
                    for r in restrictions {
                        if !enabled.contains(&r.as_str()) {
                            return Err(ValidationError::ToolRestrictionsNotSubset {
                                restrictions: restrictions.clone(),
                                enabled: self.manifest.tools.tools.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Note: We cannot check inline secrets until we have access to resolve
        // environment variables at runtime - the manifest parsing itself won't contain
        // resolved secrets, they'll be resolved during composer execution

        Ok(())
    }

    /// Validate with warnings (non-fatal issues that don't block composition)
    pub fn validate_with_warnings(&self) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();

        // Check for potentially missing tools
        if self.manifest.tools.tools.is_empty() {
            warnings.push(ValidationWarning {
                field: "tools.tools".to_string(),
                message: "no tools configured - agent will have limited capabilities".to_string(),
            });
        }

        // Check for no memory
        if self.manifest.memory.is_none() {
            warnings.push(ValidationWarning {
                field: "memory".to_string(),
                message: "no memory configured - agent will not persist conversation history"
                    .to_string(),
            });
        }

        // Check for multiple channels without default
        if self.manifest.channels.channels.len() > 1 && self.manifest.channels.default.is_none() {
            warnings.push(ValidationWarning {
                field: "channels.default".to_string(),
                message: "multiple channels configured but no default set".to_string(),
            });
        }

        warnings
    }

    /// Get the required capabilities report
    pub fn required_capabilities(&self) -> &CapabilityReport {
        &self.reports
    }

    /// Get the manifest
    pub fn manifest(&self) -> &AgentManifest {
        &self.manifest
    }

    /// Build a composed agent using the existing AgentBuilder
    ///
    /// Note: This produces a configured AgentBuilder that can produce an Agent.
    /// The actual construction would need access to the component registries
    /// (providers, channels, tools, memory, observer, security) which are
    /// typically available via the runtime's bootstrap process.
    pub fn into_agent_builder<B: AgentBuilderTrait>(self) -> Result<B> {
        // This is a placeholder - the actual implementation would
        // resolve components from registries and build using AgentBuilder
        Err(anyhow::anyhow!(
            "registry resolution not implemented in corvus-composer; use bootstrap module"
        ))
    }
}

/// Placeholder trait for AgentBuilder integration
///
/// In practice, this would integrate with the existing AgentBuilder from corvus
/// via the main corvus crate.
pub trait AgentBuilderTrait {
    fn new() -> Self;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["anthropic"]
default = "anthropic"

[channels]
channels = ["telegram"]

[tools]
tools = ["shell", "file_read"]
"#;
        let composer = AgentComposer::from_toml(toml).unwrap();
        assert_eq!(composer.manifest().name, "test-agent");
    }

    #[test]
    fn validate_requires_providers() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = []
default = ""

[channels]
channels = ["telegram"]

[tools]
tools = ["shell"]
"#;
        let result = AgentComposer::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn validate_requires_channels() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["anthropic"]
default = "anthropic"

[channels]
channels = []

[tools]
tools = ["shell"]
"#;
        let result = AgentComposer::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn validate_requires_default_in_providers() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["anthropic"]
default = "openai"

[channels]
channels = ["telegram"]

[tools]
tools = ["shell"]
"#;
        let result = AgentComposer::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn validate_unknown_provider() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["unknown-provider"]
default = "unknown-provider"

[channels]
channels = ["telegram"]

[tools]
tools = ["shell"]
"#;
        let result = AgentComposer::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn validate_tool_restrictions_subset() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["anthropic"]
default = "anthropic"

[channels]
channels = ["telegram"]

[tools]
tools = ["shell", "file_read"]

[security]
tool_restrictions = ["shell", "unknown_tool"]
"#;
        let result = AgentComposer::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn required_capabilities() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["anthropic", "openrouter"]
default = "anthropic"

[channels]
channels = ["telegram", "discord"]

[tools]
tools = ["shell", "file_read"]

[memory]
backend = "sqlite"

[observer]
name = "prometheus"

[security]
sandbox = "wasmi"
"#;
        let composer = AgentComposer::from_toml(toml).unwrap();
        let report = composer.required_capabilities();

        assert_eq!(report.providers, vec!["anthropic", "openrouter"]);
        assert_eq!(report.channels, vec!["telegram", "discord"]);
        assert_eq!(report.tools, vec!["shell", "file_read"]);
        assert_eq!(report.memory_backend, Some("sqlite".to_string()));
        assert_eq!(report.observer, Some("prometheus".to_string()));
        assert_eq!(report.sandbox, Some("wasmi".to_string()));
    }

    #[test]
    fn warnings_for_empty_tools() {
        let toml = r#"
version = "1.0"
name = "test-agent"

[providers]
providers = ["anthropic"]
default = "anthropic"

[channels]
channels = ["telegram"]

[tools]
tools = []
"#;
        let composer = AgentComposer::from_toml(toml).unwrap();
        let warnings = composer.validate_with_warnings();

        assert!(warnings.iter().any(|w| w.field == "tools.tools"));
    }
}
