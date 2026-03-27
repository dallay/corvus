use super::adapter::McpToolAdapter;
use super::client::{McpClient, McpToolManifest};
use crate::config::{McpServerConfig, MemoryCerebroConfig};
use serde_json::json;
use std::collections::BTreeMap;

const CEREBRO_SERVER_NAME: &str = "cerebro";
const CEREBRO_HTTP_COMMAND: &str = "__mcp_cerebro_http__";

pub fn cerebro_tool_adapter(
    cerebro: &MemoryCerebroConfig,
    tool_name: &str,
) -> anyhow::Result<McpToolAdapter> {
    let server = build_cerebro_server(cerebro)?;
    let client = McpClient::new(server.clone());
    let manifest = McpToolManifest {
        name: tool_name.to_string(),
        description: format!("Cerebro MCP tool '{tool_name}'"),
        parameters: json!({"type": "object"}),
    };
    McpToolAdapter::from_manifest(&server, manifest, client)
}

fn build_cerebro_server(cerebro: &MemoryCerebroConfig) -> anyhow::Result<McpServerConfig> {
    let endpoint = cerebro
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("memory.cerebro.endpoint must be configured"))?;
    let token = cerebro
        .auth_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("memory.cerebro.auth_token must be configured"))?;
    let mut env = BTreeMap::new();
    env.insert("MCP_AUTH_TOKEN".to_string(), token.to_string());

    Ok(McpServerConfig {
        name: CEREBRO_SERVER_NAME.to_string(),
        enabled: true,
        command: CEREBRO_HTTP_COMMAND.to_string(),
        args: vec![endpoint.to_string()],
        env,
        startup_timeout_ms: 5_000,
        call_timeout_ms: cerebro.request_timeout_ms,
        output_limit_bytes: 64 * 1024,
        capabilities: crate::config::default_mcp_capabilities(),
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    })
}
