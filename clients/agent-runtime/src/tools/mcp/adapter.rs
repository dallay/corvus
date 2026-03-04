use super::client::{McpClient, McpToolManifest};
use super::normalize;
use crate::config::McpServerConfig;
use crate::tools::traits::{Tool, ToolResult, ToolSpec};
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
        let bytes = output.as_bytes();
        if bytes.len() <= self.output_limit_bytes {
            return output;
        }

        let marker = format!(
            "\n[output_limit_enforced limit_bytes={} original_bytes={}]",
            self.output_limit_bytes,
            bytes.len()
        );
        let max_body = self.output_limit_bytes.saturating_sub(marker.len());

        // Find a valid UTF-8 char boundary <= max_body
        let truncated = if max_body >= bytes.len() {
            output.clone()
        } else {
            output
                .chars()
                .take_while(|_| true)
                .scan(0usize, |acc, c| {
                    let next = *acc + c.len_utf8();
                    if next <= max_body {
                        Some(next)
                    } else {
                        None
                    }
                })
                .last()
                .map(|end| output[..end].to_string())
                .unwrap_or_else(|| output[..max_body].to_string())
        };

        format!("{}{}", truncated, marker)
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
                });
            }
        };

        // Check output limit bounds to prevent abuse
        if self.output_limit_bytes > 10 * 1024 * 1024 {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("output_limit_bytes exceeds maximum allowed (10MB)".to_string()),
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
            }),
            Err(error) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
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
        }
    }
}
