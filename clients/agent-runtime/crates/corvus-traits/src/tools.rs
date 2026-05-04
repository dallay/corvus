use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    /// Optional structured payload for machine-readable consumers (e.g. code-session results).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured: Option<serde_json::Value>,
}

/// Description of a tool for the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub source: Option<ToolSourceMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolSourceMetadata {
    pub kind: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub original_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolDescriptorHint {
    pub mcp: Option<ToolDescriptorMcpHint>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolDescriptorMcpHint {
    pub server: Option<String>,
    pub upstream_name: Option<String>,
    pub resource_uri: Option<String>,
    pub mime_type: Option<String>,
    pub prompt_arguments: Vec<ToolDescriptorMcpPromptArgumentHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptorMcpPromptArgumentHint {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// Core tool trait — implement for any capability.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used in LLM function calling).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// JSON schema for parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with given arguments.
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult>;

    /// Get the full spec for LLM registration.
    fn spec(&self) -> ToolSpec {
        let parameters = self.parameters_schema();
        validate_tool_schema(&parameters).unwrap_or_else(|error| {
            panic!("Invalid tool schema for {}: {error}", self.name());
        });
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters,
            source: None,
            aliases: vec![],
        }
    }

    /// Optional descriptive metadata used by M2 capability registration.
    fn descriptor_hint(&self) -> ToolDescriptorHint {
        ToolDescriptorHint::default()
    }
}

fn validate_tool_schema(schema: &serde_json::Value) -> anyhow::Result<()> {
    let obj = schema
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Schema must be an object"))?;

    if !obj.contains_key("type") {
        anyhow::bail!("Schema missing required 'type' field");
    }

    if let Some(serde_json::Value::String(schema_type)) = obj.get("type") {
        if schema_type == "object" && !obj.contains_key("properties") {
            tracing::warn!("Object schema without 'properties' field may cause issues");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str {
            "dummy_tool"
        }

        fn description(&self) -> &str {
            "A deterministic test tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" }
                }
            })
        }

        async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: args
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                error: None,
                structured: None,
            })
        }
    }

    struct InvalidSchemaTool;

    #[async_trait]
    impl Tool for InvalidSchemaTool {
        fn name(&self) -> &str {
            "invalid_tool"
        }

        fn description(&self) -> &str {
            "Invalid schema test tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "properties": {
                    "value": { "type": "string" }
                }
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: String::new(),
                error: None,
                structured: None,
            })
        }
    }

    #[test]
    fn spec_uses_tool_metadata_and_schema() {
        let tool = DummyTool;
        let spec = tool.spec();

        assert_eq!(spec.name, "dummy_tool");
        assert_eq!(spec.description, "A deterministic test tool");
        assert_eq!(spec.parameters["type"], "object");
        assert_eq!(spec.parameters["properties"]["value"]["type"], "string");
    }

    #[test]
    #[should_panic(expected = "Invalid tool schema for invalid_tool")]
    fn spec_panics_for_invalid_schema() {
        let tool = InvalidSchemaTool;
        let _ = tool.spec();
    }

    #[tokio::test]
    async fn execute_returns_expected_output() {
        let tool = DummyTool;
        let result = tool
            .execute(serde_json::json!({ "value": "hello-tool" }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "hello-tool");
        assert!(result.error.is_none());
    }

    #[test]
    fn tool_result_serialization_roundtrip() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("boom".into()),
            structured: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();

        assert!(!parsed.success);
        assert_eq!(parsed.error.as_deref(), Some("boom"));
    }

    #[test]
    fn tool_result_structured_field_serializes_and_omitted_when_none() {
        let without = ToolResult {
            success: true,
            output: "ok".into(),
            error: None,
            structured: None,
        };
        let json_without = serde_json::to_string(&without).unwrap();
        assert!(
            !json_without.contains("structured"),
            "structured must be omitted when None"
        );

        let with_payload = ToolResult {
            success: true,
            output: "ok".into(),
            error: None,
            structured: Some(serde_json::json!({ "status": "success", "files_changed": 3 })),
        };
        let json_with = serde_json::to_string(&with_payload).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json_with).unwrap();
        let structured = parsed.structured.unwrap();
        assert_eq!(structured["status"], "success");
        assert_eq!(structured["files_changed"], 3);
    }
}
