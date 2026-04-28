use crate::tools::traits::ToolSourceMetadata;
use serde::{Deserialize, Serialize};

pub const M2_CAPABILITY_VERSION: &str = "1.0.0";
pub const TOOL_RUNTIME_CONTRACT: &str = "tool-trait-v1";
pub const ENTRYPOINT_AGENT: &str = "agent";
pub const ENTRYPOINT_CHANNELS: &str = "channels";
pub const ENTRYPOINT_GATEWAY: &str = "gateway";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityFamily {
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityKind {
    Executable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CapabilityDependencies {
    pub required: Vec<CapabilityDependency>,
    pub optional: Vec<CapabilityDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDependency {
    pub target: String,
    pub family: Option<String>,
    pub version_constraint: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityLifecycle {
    pub discovery_mode: DiscoveryMode,
    pub activation_mode: ActivationMode,
    pub teardown_mode: Option<TeardownMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoveryMode {
    Static,
    Discovered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivationMode {
    RuntimeWired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeardownMode {
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilitySecurity {
    pub policy_scope: String,
    pub audit_namespace: String,
    pub source_classification: SourceClassification,
    pub risk_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceClassification {
    Native,
    Mcp,
    McpResource,
    McpPrompt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityCompatibility {
    pub runtime_contracts: Vec<String>,
    pub entrypoint_parity_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityMetadata {
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub source: Option<ToolSourceMetadata>,
    pub mcp: Option<McpCapabilityMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCapabilityMetadata {
    pub server: String,
    pub upstream_name: Option<String>,
    pub resource_uri: Option<String>,
    pub mime_type: Option<String>,
    pub prompt_arguments: Vec<PromptArgumentDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptArgumentDescriptor {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub namespace: String,
    pub version: String,
    pub family: CapabilityFamily,
    pub kind: CapabilityKind,
    pub dependencies: CapabilityDependencies,
    pub lifecycle: CapabilityLifecycle,
    pub security: CapabilitySecurity,
    pub compatibility: CapabilityCompatibility,
    pub metadata: CapabilityMetadata,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("capability descriptor missing required field '{field}'{id_suffix}")]
    MissingField {
        field: &'static str,
        id_suffix: String,
    },
    #[error("capability descriptor '{id}' has invalid namespace '{namespace}'")]
    InvalidNamespace { id: String, namespace: String },
    #[error("capability descriptor duplicate id '{id}'")]
    DuplicateId { id: String },
    #[error("capability descriptor '{id}' has invalid kind '{kind}' for M2")]
    InvalidKindForM2 { id: String, kind: String },
    #[error("capability descriptor '{id}' has invalid family '{family}' for M2")]
    InvalidFamilyForM2 { id: String, family: String },
    #[error("capability descriptor '{id}' has invalid metadata: {reason}")]
    InvalidMetadata { id: String, reason: String },
}

impl CapabilityError {
    pub fn missing_field(field: &'static str, id: Option<&str>) -> Self {
        let id_suffix = id.map_or_else(String::new, |value| format!(" for '{value}'"));
        Self::MissingField { field, id_suffix }
    }
}
