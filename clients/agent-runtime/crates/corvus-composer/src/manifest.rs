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

fn default_security_backend() -> String {
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
