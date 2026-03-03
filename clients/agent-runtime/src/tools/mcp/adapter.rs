use super::client::{McpClient, McpToolManifest};
use super::normalize;
use crate::config::McpServerConfig;
use crate::tools::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;

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
        let mut truncated = output;
        truncated.truncate(max_body);
        truncated.push_str(&marker);
        truncated
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

        match self.client.call_tool(&self.original_name, args).await {
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
