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

pub fn cerebro_client(cerebro: &MemoryCerebroConfig) -> anyhow::Result<McpClient> {
    Ok(McpClient::new(build_cerebro_server(cerebro)?))
}

pub fn cerebro_list_tools(cerebro: &MemoryCerebroConfig) -> anyhow::Result<Vec<McpToolManifest>> {
    cerebro_client(cerebro)?.list_tools()
}

pub async fn cerebro_call_tool(
    cerebro: &MemoryCerebroConfig,
    tool_name: &str,
    arguments: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let raw = cerebro_client(cerebro)?.call_tool(tool_name, arguments).await?;
    serde_json::from_str(&raw).or_else(|_| Ok(serde_json::Value::String(raw)))
}

pub fn cerebro_is_configured(cerebro: &MemoryCerebroConfig) -> bool {
    cerebro
        .endpoint
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        && cerebro
            .auth_token
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn configured_cerebro() -> MemoryCerebroConfig {
        MemoryCerebroConfig {
            endpoint: Some("http://127.0.0.1:4040/mcp".into()),
            auth_token: Some("secret-token".into()),
            request_timeout_ms: 5_000,
            allow_insecure_loopback: true,
        }
    }

    #[test]
    fn cerebro_is_configured_requires_endpoint_and_token() {
        let config = configured_cerebro();
        assert!(cerebro_is_configured(&config));

        let mut missing_token = config.clone();
        missing_token.auth_token = None;
        assert!(!cerebro_is_configured(&missing_token));
    }

    #[test]
    fn cerebro_client_uses_http_transport_for_gateway_calls() {
        let client = cerebro_client(&configured_cerebro()).unwrap();
        let tools = client.list_tools().unwrap_err().to_string();
        assert!(tools.contains("MCP HTTP discovery failed") || tools.contains("mcp_transport_error"));
    }
}
