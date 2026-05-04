use super::client::{McpClient, McpToolManifest};
use super::normalize;
use crate::config::McpServerConfig;
use crate::tools::traits::{Tool, ToolDescriptorHint, ToolDescriptorMcpHint, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde_json::Map;

#[derive(Clone)]
pub struct McpToolAdapter {
    name: String,
    description: String,
    parameters: serde_json::Value,
    original_name: String,
    server_name: String,
    call_timeout_ms: u64,
    output_limit_bytes: usize,
    client: McpClient,
}

impl McpToolAdapter {
    pub fn from_manifest(
        server: &McpServerConfig,
        manifest: McpToolManifest,
        client: McpClient,
    ) -> anyhow::Result<Self> {
        let canonical_name = normalize::normalize_tool_name(&server.name, &manifest.name)?;
        Ok(Self {
            name: canonical_name,
            description: manifest.description,
            parameters: manifest.parameters,
            original_name: manifest.name,
            server_name: server.name.clone(),
            call_timeout_ms: server.call_timeout_ms,
            output_limit_bytes: server.output_limit_bytes,
            client,
        })
    }

    fn enforce_output_limit(&self, output: String) -> String {
        if output.len() <= self.output_limit_bytes {
            return output;
        }

        let marker = format!(
            "\n[output_limit_enforced limit_bytes={} original_bytes={}]",
            self.output_limit_bytes,
            output.len()
        );
        if self.output_limit_bytes <= marker.len() {
            let mut end = self.output_limit_bytes;
            while end > 0 && !marker.is_char_boundary(end) {
                end -= 1;
            }
            return marker[..end].to_string();
        }

        let max_body = self.output_limit_bytes.saturating_sub(marker.len());
        let mut end = max_body.min(output.len());
        while end > 0 && !output.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = if end == 0 {
            String::new()
        } else {
            output[..end].to_string()
        };

        format!("{truncated}{marker}")
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        tracing::debug!(
            server = %self.server_name,
            tool = %self.original_name,
            timeout_ms = self.call_timeout_ms,
            output_limit_bytes = self.output_limit_bytes,
            "MCP adapter execute"
        );

        // Validate and sanitize input args
        let validated_args = match &args {
            serde_json::Value::Object(_) => args,
            serde_json::Value::Null => serde_json::Value::Object(Map::new()),
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("MCP tool arguments must be a JSON object".to_string()),
                    structured: None,
                });
            }
        };

        // Check output limit bounds to prevent abuse from dynamically loaded in-memory configs
        // Even though schema validation catches file-based configs, in-memory instances could bypass it.
        if self.output_limit_bytes > 10 * 1024 * 1024 {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("output_limit_bytes exceeds maximum allowed (10MB)".to_string()),
                structured: None,
            });
        }

        match self
            .client
            .call_tool(&self.original_name, validated_args)
            .await
        {
            Ok(output) => Ok(ToolResult {
                success: true,
                output: self.enforce_output_limit(output),
                error: None,
                structured: None,
            }),
            Err(error) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
                structured: None,
            }),
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
            source: Some(normalize::source_metadata(
                &self.server_name,
                &self.original_name,
            )),
            aliases: vec![],
        }
    }

    fn descriptor_hint(&self) -> ToolDescriptorHint {
        ToolDescriptorHint {
            mcp: Some(ToolDescriptorMcpHint {
                server: Some(self.server_name.clone()),
                upstream_name: Some(self.original_name.clone()),
                resource_uri: None,
                mime_type: None,
                prompt_arguments: vec![],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> McpClient {
        let server = crate::config::McpServerConfig {
            name: "test".into(),
            command: "__mcp_mock__".into(),
            ..crate::config::McpServerConfig::default()
        };
        McpClient::new(server)
    }

    #[test]
    fn mcp_adapter_enforces_output_limit() {
        let adapter = McpToolAdapter {
            name: "test".into(),
            description: "test".into(),
            parameters: serde_json::Value::Null,
            original_name: "test".into(),
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 100, // Very small limit for testing
            client: test_client(),
        };

        let large_output = "A".repeat(200);
        let enforced = adapter.enforce_output_limit(large_output.clone());

        // Ensure it's truncated and exactly matches limit
        assert_eq!(enforced.len(), 100);
        assert!(enforced.ends_with("[output_limit_enforced limit_bytes=100 original_bytes=200]"));

        let exact_output = "A".repeat(100);
        let unenforced_exact = adapter.enforce_output_limit(exact_output.clone());
        assert_eq!(exact_output, unenforced_exact);

        let over_limit_output = "A".repeat(101);
        let enforced_over = adapter.enforce_output_limit(over_limit_output.clone());
        assert_eq!(enforced_over.len(), 100);
        assert!(
            enforced_over.ends_with("[output_limit_enforced limit_bytes=100 original_bytes=101]")
        );

        let small_output = "A".repeat(50);
        let unenforced = adapter.enforce_output_limit(small_output.clone());
        assert_eq!(small_output, unenforced);
    }

    #[test]
    fn mcp_adapter_enforces_marker_truncation() {
        let adapter = McpToolAdapter {
            name: "test".into(),
            description: "test".into(),
            parameters: serde_json::Value::Null,
            original_name: "test".into(),
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 10,
            client: test_client(),
        };

        let large_output = "A".repeat(200);
        let enforced = adapter.enforce_output_limit(large_output);

        // Assert length is EXACTLY 10 and that it actually truncated the marker itself
        assert_eq!(enforced.len(), 10);
        assert!(enforced.starts_with("\n[output_l"));
    }

    #[test]
    fn mcp_adapter_enforces_multibyte_truncation() {
        let adapter = McpToolAdapter {
            name: "test".into(),
            description: "test".into(),
            parameters: serde_json::Value::Null,
            original_name: "test".into(),
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 50,
            client: test_client(),
        };

        // 'é' is 2 bytes in UTF-8. 100 characters = 200 bytes.
        let multibyte_output = "é".repeat(100);
        let enforced = adapter.enforce_output_limit(multibyte_output);

        // Ensure strictly bounded and ends cleanly on a valid char boundary without panicking
        assert!(enforced.len() <= 50);
        assert!(enforced.is_char_boundary(enforced.len()));
        assert!(std::str::from_utf8(enforced.as_bytes()).is_ok());
    }

    #[tokio::test]
    async fn mcp_adapter_blocks_massive_limit_configuration() {
        let adapter = McpToolAdapter {
            name: "test".into(),
            description: "test".into(),
            parameters: serde_json::Value::Null,
            original_name: "test".into(),
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 20 * 1024 * 1024, // 20MB, exceeds the hardcoded 10MB limit in execute
            client: test_client(),
        };

        let result = adapter.execute(serde_json::json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("exceeds maximum allowed"));
    }
}
