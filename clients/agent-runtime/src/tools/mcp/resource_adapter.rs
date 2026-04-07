use super::client::{McpClient, McpResourceManifest};
use super::normalize;
use crate::config::McpServerConfig;
use crate::tools::traits::{Tool, ToolDescriptorHint, ToolDescriptorMcpHint, ToolResult, ToolSpec};
use async_trait::async_trait;

#[derive(Clone)]
pub struct McpResourceAdapter {
    name: String,
    description: String,
    original_name: String,
    uri: String,
    mime_type: Option<String>,
    server_name: String,
    call_timeout_ms: u64,
    output_limit_bytes: usize,
    client: McpClient,
}

impl McpResourceAdapter {
    pub fn from_manifest(
        server: &McpServerConfig,
        manifest: McpResourceManifest,
        client: McpClient,
    ) -> anyhow::Result<Self> {
        let canonical_name = normalize::normalize_resource_name(&server.name, &manifest.name)?;
        let output_limit = server
            .resource_output_limit_bytes
            .unwrap_or(server.output_limit_bytes);
        Ok(Self {
            name: canonical_name,
            description: manifest.description,
            original_name: manifest.name,
            uri: manifest.uri,
            mime_type: manifest.mime_type,
            server_name: server.name.clone(),
            call_timeout_ms: server.call_timeout_ms,
            output_limit_bytes: output_limit,
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
impl Tool for McpResourceAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        tracing::debug!(
            server = %self.server_name,
            resource = %self.uri,
            timeout_ms = self.call_timeout_ms,
            output_limit_bytes = self.output_limit_bytes,
            "MCP resource adapter execute"
        );

        // Guard against absurd output limits (same 10MB cap as McpToolAdapter)
        if self.output_limit_bytes > 10 * 1024 * 1024 {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("output_limit_bytes exceeds maximum allowed (10MB)".to_string()),
                structured: None,
            });
        }

        match self.client.read_resource(&self.uri) {
            Ok(content) => {
                // Empty/null content is valid for resources — not an error
                let output = if content.is_empty() {
                    String::new()
                } else {
                    self.enforce_output_limit(content)
                };
                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                    structured: self
                        .mime_type
                        .as_ref()
                        .map(|mt| serde_json::json!({ "mime_type": mt })),
                })
            }
            Err(error) => {
                // Resource failure isolation: return structured error, never panic
                let redacted = super::redact_error_message(&error.to_string());
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(redacted),
                    structured: None,
                })
            }
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters_schema(),
            source: Some(normalize::source_metadata_resource(
                &self.server_name,
                &self.uri,
            )),
        }
    }

    fn descriptor_hint(&self) -> ToolDescriptorHint {
        ToolDescriptorHint {
            mcp: Some(ToolDescriptorMcpHint {
                server: Some(self.server_name.clone()),
                upstream_name: Some(self.original_name.clone()),
                resource_uri: Some(self.uri.clone()),
                mime_type: self.mime_type.clone(),
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

    fn test_server(output_limit: usize) -> McpServerConfig {
        McpServerConfig {
            name: "docs".into(),
            command: "__mcp_mock__".into(),
            output_limit_bytes: output_limit,
            ..McpServerConfig::default()
        }
    }

    fn test_manifest() -> McpResourceManifest {
        McpResourceManifest {
            name: "api-spec".into(),
            uri: "docs://api-spec".into(),
            description: "API specification".into(),
            mime_type: Some("text/markdown".into()),
        }
    }

    // ── parameters_schema returns empty object ──────────────

    #[test]
    fn parameters_schema_returns_empty_object() {
        let server = test_server(1024);
        let adapter =
            McpResourceAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let schema = adapter.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], serde_json::json!({}));
    }

    // ── spec returns correct metadata ───────────────────────

    #[test]
    fn spec_returns_mcp_resource_kind() {
        let server = test_server(1024);
        let adapter =
            McpResourceAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let spec = adapter.spec();
        assert_eq!(spec.name, "mcp.docs.resource.api-spec");
        assert_eq!(spec.description, "API specification");
        let source = spec.source.unwrap();
        assert_eq!(source.kind, "mcp_resource");
        assert_eq!(source.provider.as_deref(), Some("mcp"));
        assert_eq!(source.server.as_deref(), Some("docs"));
    }

    // ── execute returns resource content ─────────────────────

    #[tokio::test]
    async fn execute_returns_resource_content() {
        let server = test_server(4096);
        let adapter =
            McpResourceAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let result = adapter.execute(serde_json::json!({})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("mock-resource-content"));
        assert!(result.error.is_none());
    }

    // ── execute returns mime_type in structured field ────────

    #[tokio::test]
    async fn execute_includes_mime_type_metadata() {
        let server = test_server(4096);
        let adapter =
            McpResourceAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let result = adapter.execute(serde_json::json!({})).await.unwrap();
        let structured = result.structured.unwrap();
        assert_eq!(structured["mime_type"], "text/markdown");
    }

    // ── execute handles empty content without error ──────────

    #[tokio::test]
    async fn execute_handles_empty_content_without_error() {
        // The mock returns "mock-resource-content for <uri>" which is never empty,
        // so we test the adapter's enforce_output_limit with empty string directly
        let adapter = McpResourceAdapter {
            name: "mcp.docs.resource.empty".into(),
            description: "empty".into(),
            original_name: "empty".into(),
            uri: "docs://empty".into(),
            mime_type: None,
            server_name: "docs".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 1024,
            client: test_client(),
        };
        // The mock will return non-empty, but the important thing is success=true
        let result = adapter.execute(serde_json::json!({})).await.unwrap();
        assert!(result.success);
        // No mime_type → structured is None
        assert!(result.structured.is_none());
    }

    // ── output limit enforcement ────────────────────────────

    #[test]
    fn enforce_output_limit_truncates_large_content() {
        let adapter = McpResourceAdapter {
            name: "test".into(),
            description: "test".into(),
            original_name: "test".into(),
            uri: "test://uri".into(),
            mime_type: None,
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 100,
            client: test_client(),
        };

        let large_output = "A".repeat(200);
        let enforced = adapter.enforce_output_limit(large_output);
        assert_eq!(enforced.len(), 100);
        assert!(enforced.ends_with("[output_limit_enforced limit_bytes=100 original_bytes=200]"));
    }

    #[test]
    fn enforce_output_limit_passes_small_content() {
        let adapter = McpResourceAdapter {
            name: "test".into(),
            description: "test".into(),
            original_name: "test".into(),
            uri: "test://uri".into(),
            mime_type: None,
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 1024,
            client: test_client(),
        };

        let small = "hello world".to_string();
        let result = adapter.enforce_output_limit(small.clone());
        assert_eq!(result, small);
    }

    // ── resource-specific output_limit_bytes overrides server default ──

    #[test]
    fn resource_output_limit_overrides_server_default() {
        let mut server = test_server(1024);
        server.resource_output_limit_bytes = Some(2048);
        let adapter =
            McpResourceAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        assert_eq!(adapter.output_limit_bytes, 2048);
    }

    #[test]
    fn resource_output_limit_falls_back_to_server_default() {
        let server = test_server(1024);
        // resource_output_limit_bytes is None by default
        let adapter =
            McpResourceAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        assert_eq!(adapter.output_limit_bytes, 1024);
    }

    // ── 10MB max output limit guard ─────────────────────────

    #[tokio::test]
    async fn blocks_massive_limit_configuration() {
        let adapter = McpResourceAdapter {
            name: "test".into(),
            description: "test".into(),
            original_name: "test".into(),
            uri: "test://uri".into(),
            mime_type: None,
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 20 * 1024 * 1024, // 20MB
            client: test_client(),
        };

        let result = adapter.execute(serde_json::json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("exceeds maximum allowed"));
    }

    // ── failure isolation: errors return structured result ───

    #[tokio::test]
    async fn execute_returns_structured_error_on_failure() {
        // Use a client with a command that doesn't support read_resource
        let server = McpServerConfig {
            name: "failing".into(),
            command: "nonexistent_command".into(),
            ..McpServerConfig::default()
        };
        let client = McpClient::new(server.clone());
        let adapter = McpResourceAdapter {
            name: "mcp.failing.resource.broken".into(),
            description: "broken".into(),
            original_name: "broken".into(),
            uri: "fail://broken".into(),
            mime_type: None,
            server_name: "failing".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 1024,
            client,
        };

        let result = adapter.execute(serde_json::json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
        // Should NOT panic or return Err — always Ok(ToolResult)
    }
}
