use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub version: String,
    pub agent: AgentSection,
    pub providers: ProviderFamilySection,
    pub channels: ChannelFamilySection,
    #[serde(default)]
    pub tools: ToolFamilySection,
    pub memory: MemorySection,
    #[serde(default)]
    pub observability: ObservabilitySection,
    #[serde(default)]
    pub security: SecuritySection,
    #[serde(default)]
    pub runtime: RuntimeSection,
    #[serde(default)]
    pub identity: IdentitySection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderFamilySection {
    pub enabled: Vec<String>,
    pub default: String,
    #[serde(default)]
    pub config: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelFamilySection {
    pub enabled: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolFamilySection {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub config: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    pub backend: String,
    #[serde(default)]
    pub config: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub auto_save: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservabilitySection {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub config: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecuritySection {
    #[serde(default = "default_security_backend")]
    pub backend: String,
    #[serde(default)]
    pub config: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub require: Option<bool>,
    #[serde(default)]
    pub tool_restrictions: Vec<String>,
}

pub(crate) fn default_security_backend() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeSection {
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub max_tool_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentitySection {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub aieos_path: Option<String>,
    #[serde(default)]
    pub aieos_inline: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // --- AgentManifest parsing ---

    #[test]
    fn parse_minimal_manifest_succeeds() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.version, "1");
        assert_eq!(manifest.agent.name, "minimal");
    }

    #[test]
    fn parse_agent_optional_fields_default_to_none() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert!(manifest.agent.description.is_none());
        assert!(manifest.agent.model.is_none());
        assert!(manifest.agent.temperature.is_none());
        assert!(manifest.agent.profile.is_none());
    }

    #[test]
    fn parse_provider_family_section_populated() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.providers.enabled, vec!["anthropic"]);
        assert_eq!(manifest.providers.default, "anthropic");
        assert!(manifest.providers.config.is_empty());
    }

    #[test]
    fn parse_channel_family_section_populated() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.channels.enabled, vec!["stdio"]);
        assert!(manifest.channels.default.is_none());
    }

    #[test]
    fn parse_memory_section_populated() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.memory.backend, "none");
        assert!(manifest.memory.config.is_empty());
        assert!(manifest.memory.auto_save.is_none());
    }

    // --- SecuritySection default ---

    #[test]
    fn security_section_default_backend_is_auto() {
        // When [security] is not present at all, backend must default to "auto"
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.security.backend, "auto");
    }

    #[test]
    fn security_section_explicit_backend_overrides_default() {
        let toml_str = format!(
            "{}\n[security]\nbackend = \"none\"\n",
            minimal_toml()
        );
        let manifest: AgentManifest = toml::from_str(&toml_str).unwrap();
        assert_eq!(manifest.security.backend, "none");
    }

    #[test]
    fn security_section_tool_restrictions_default_empty() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert!(manifest.security.tool_restrictions.is_empty());
    }

    // --- ToolFamilySection defaults ---

    #[test]
    fn tool_family_section_defaults_to_empty_enabled_list() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert!(manifest.tools.enabled.is_empty());
    }

    #[test]
    fn tool_family_section_derive_default_works() {
        let tools = ToolFamilySection::default();
        assert!(tools.enabled.is_empty());
        assert!(tools.config.is_empty());
    }

    // --- ObservabilitySection defaults ---

    #[test]
    fn observability_section_defaults_to_empty_enabled_list() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert!(manifest.observability.enabled.is_empty());
    }

    #[test]
    fn observability_section_derive_default_works() {
        let obs = ObservabilitySection::default();
        assert!(obs.enabled.is_empty());
        assert!(obs.config.is_empty());
    }

    // --- RuntimeSection defaults ---

    #[test]
    fn runtime_section_defaults_are_none() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert!(manifest.runtime.profile.is_none());
        assert!(manifest.runtime.max_tool_iterations.is_none());
    }

    // --- IdentitySection defaults ---

    #[test]
    fn identity_section_defaults_are_none() {
        let manifest: AgentManifest = toml::from_str(minimal_toml()).unwrap();
        assert!(manifest.identity.format.is_none());
        assert!(manifest.identity.aieos_path.is_none());
        assert!(manifest.identity.aieos_inline.is_none());
    }

    // --- Config BTreeMap parsing ---

    #[test]
    fn provider_config_map_is_parsed_correctly() {
        let toml_str = r#"
version = "1"
[agent]
name = "config-test"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[providers.config.anthropic]
api_url = "https://example.test"
[channels]
enabled = ["stdio"]
[memory]
backend = "none"
"#;
        let manifest: AgentManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.providers.config.contains_key("anthropic"));
    }

    // Boundary: agent with all optional fields set
    #[test]
    fn parse_full_agent_section() {
        let toml_str = r#"
version = "1"
[agent]
name = "full-agent"
description = "A complete agent"
model = "anthropic/claude-3"
temperature = 0.7
profile = "code"
[providers]
enabled = ["anthropic"]
default = "anthropic"
[channels]
enabled = ["stdio"]
[memory]
backend = "sqlite"
"#;
        let manifest: AgentManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.agent.description.as_deref(), Some("A complete agent"));
        assert_eq!(manifest.agent.model.as_deref(), Some("anthropic/claude-3"));
        assert_eq!(manifest.agent.temperature, Some(0.7));
        assert_eq!(manifest.agent.profile.as_deref(), Some("code"));
    }
}