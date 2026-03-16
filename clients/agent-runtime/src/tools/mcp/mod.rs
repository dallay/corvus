pub mod adapter;
pub mod cerebro;
pub mod client;
pub mod normalize;

use crate::config::McpConfig;
use crate::tools::Tool;
use std::collections::HashSet;

fn redact_error_message(raw: &str) -> String {
    let mut sanitized = raw.to_string();
    for (key, value) in std::env::vars() {
        let upper = key.to_ascii_uppercase();
        let sensitive = upper.contains("TOKEN")
            || upper.contains("SECRET")
            || upper.contains("PASSWORD")
            || upper.contains("API_KEY")
            || upper.contains("AUTH");
        if sensitive && !value.is_empty() {
            sanitized = sanitized.replace(&value, "[REDACTED]");
        }
    }
    sanitized
}

fn collision_error_message(tool_name: &str) -> String {
    format!(
        "MCP registration rejected due to duplicate canonical tool identifier '{tool_name}'. \
Resolve by renaming mcp.servers[].name or the upstream MCP tool name so each canonical tool id is unique (format: mcp.<server>.<tool>)."
    )
}

pub fn discover_tools(config: &McpConfig) -> anyhow::Result<Vec<Box<dyn Tool>>> {
    if !config.enabled {
        return Ok(Vec::new());
    }

    tracing::info!(servers = config.servers.len(), "MCP discovery starting");

    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    for server in &config.servers {
        if !server.enabled {
            tracing::debug!(server = %server.name, "MCP server disabled; skipping");
            continue;
        }

        tracing::debug!(
            server = %server.name,
            startup_timeout_ms = server.startup_timeout_ms,
            call_timeout_ms = server.call_timeout_ms,
            output_limit_bytes = server.output_limit_bytes,
            "MCP server discovery start"
        );

        let client = client::McpClient::new(server.clone());
        let manifests = match client.list_tools() {
            Ok(manifests) => manifests,
            Err(error) => {
                let redacted = redact_error_message(&error.to_string());
                tracing::warn!(
                    server = %server.name,
                    error = %redacted,
                    "MCP server discovery failed; skipping server"
                );
                continue;
            }
        };

        for manifest in manifests {
            let adapter =
                match adapter::McpToolAdapter::from_manifest(server, manifest, client.clone()) {
                    Ok(adapter) => adapter,
                    Err(error) => {
                        let redacted = redact_error_message(&error.to_string());
                        tracing::warn!(
                            server = %server.name,
                            error = %redacted,
                            "MCP tool normalization failed; skipping tool"
                        );
                        continue;
                    }
                };

            let tool_name = adapter.name().to_string();
            if !seen_names.insert(tool_name.clone()) {
                anyhow::bail!(collision_error_message(&tool_name));
            }

            tools.push(Box::new(adapter));
        }
    }

    tracing::info!(registered_tools = tools.len(), "MCP discovery completed");

    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_error_message_is_actionable_for_operators() {
        let message = collision_error_message("mcp.docs.search");
        assert!(message.contains("mcp.docs.search"));
        assert!(message.contains("mcp.servers[].name"));
        assert!(message.contains("canonical"));
    }
}
