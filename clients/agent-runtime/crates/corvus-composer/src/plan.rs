use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectedCapability {
    pub key: String,
    pub config: Option<toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeSettings {
    pub profile: Option<String>,
    pub max_tool_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySettings {
    pub auto_save: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentitySettings {
    pub format: Option<String>,
    pub aieos_path: Option<String>,
    pub aieos_inline: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentMetadata {
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComposedRuntimePlan {
    pub agent: AgentMetadata,
    pub provider: SelectedCapability,
    pub channels: Vec<SelectedCapability>,
    pub default_channel: Option<String>,
    pub tools: Vec<SelectedCapability>,
    pub memory: SelectedCapability,
    pub memory_settings: MemorySettings,
    pub observers: Vec<SelectedCapability>,
    pub security: SelectedCapability,
    pub tool_restrictions: Vec<String>,
    pub runtime: RuntimeSettings,
    pub identity: IdentitySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityReport {
    pub providers: Vec<String>,
    pub channels: Vec<String>,
    pub tools: Vec<String>,
    pub memory_backend: Option<String>,
    pub observers: Vec<String>,
    pub sandbox: Option<String>,
}

impl From<&ComposedRuntimePlan> for CapabilityReport {
    fn from(plan: &ComposedRuntimePlan) -> Self {
        Self {
            providers: vec![plan.provider.key.clone()],
            channels: plan.channels.iter().map(|item| item.key.clone()).collect(),
            tools: plan.tools.iter().map(|item| item.key.clone()).collect(),
            memory_backend: Some(plan.memory.key.clone()),
            observers: plan.observers.iter().map(|item| item.key.clone()).collect(),
            sandbox: Some(plan.security.key.clone()),
        }
    }
}

pub type CapabilityConfigMap = BTreeMap<String, toml::Value>;
