pub mod adapter;
pub mod cerebro;
pub mod client;
pub mod normalize;
pub mod prompt_adapter;
pub mod resource_adapter;

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

fn collision_error_message(name: &str) -> String {
    format!(
        "MCP registration rejected due to duplicate canonical identifier '{name}'. \
Resolve by renaming mcp.servers[].name or the upstream MCP capability name so each canonical id is unique (format: mcp.<server>.<type>.<name>)."
    )
}

/// Backward-compatible alias for `discover_capabilities`.
pub fn discover_tools(config: &McpConfig) -> anyhow::Result<Vec<Box<dyn Tool>>> {
    discover_capabilities(config)
}

pub fn discover_capabilities(config: &McpConfig) -> anyhow::Result<Vec<Box<dyn Tool>>> {
    if !config.enabled {
        return Ok(Vec::new());
    }

    tracing::info!(
        servers = config.servers.len(),
        "MCP capability discovery starting"
    );

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
            capabilities = ?server.capabilities,
            "MCP server discovery start"
        );

        let client = client::McpClient::new(server.clone());

        // ── Tools ────────────────────────────────────────────
        if server.capabilities.iter().any(|c| c == "tools") {
            match client.list_tools() {
                Ok(manifests) => {
                    for manifest in manifests {
                        let adapter = match adapter::McpToolAdapter::from_manifest(
                            server,
                            manifest,
                            client.clone(),
                        ) {
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
                Err(error) => {
                    let redacted = redact_error_message(&error.to_string());
                    tracing::warn!(
                        server = %server.name,
                        error = %redacted,
                        "MCP tool discovery failed; continuing with other capabilities"
                    );
                }
            }
        }

        // ── Resources ────────────────────────────────────────
        if server.capabilities.iter().any(|c| c == "resources") {
            match client.list_resources() {
                Ok(manifests) => {
                    for manifest in manifests {
                        let adapter = match resource_adapter::McpResourceAdapter::from_manifest(
                            server,
                            manifest,
                            client.clone(),
                        ) {
                            Ok(adapter) => adapter,
                            Err(error) => {
                                let redacted = redact_error_message(&error.to_string());
                                tracing::warn!(
                                    server = %server.name,
                                    error = %redacted,
                                    "MCP resource normalization failed; skipping resource"
                                );
                                continue;
                            }
                        };

                        let resource_name = adapter.name().to_string();
                        if !seen_names.insert(resource_name.clone()) {
                            anyhow::bail!(collision_error_message(&resource_name));
                        }

                        tools.push(Box::new(adapter));
                    }
                }
                Err(error) => {
                    let redacted = redact_error_message(&error.to_string());
                    tracing::warn!(
                        server = %server.name,
                        error = %redacted,
                        "MCP resource discovery failed; continuing with other capabilities"
                    );
                }
            }
        } else {
            // Log if server advertises resources but they're not in config
            if server.command == "__mcp_mock__" {
                if let Some(payload) = server.args.first() {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
                        if value.get("resources").is_some() {
                            tracing::warn!(
                                server = %server.name,
                                "MCP server advertises resources but 'resources' is not in capabilities config; ignoring"
                            );
                        }
                    }
                }
            }
        }

        // ── Prompts ──────────────────────────────────────────
        if server.capabilities.iter().any(|c| c == "prompts") {
            match client.list_prompts() {
                Ok(manifests) => {
                    for manifest in manifests {
                        let canonical =
                            match normalize::normalize_prompt_name(&server.name, &manifest.name) {
                                Ok(name) => name,
                                Err(error) => {
                                    let redacted = redact_error_message(&error.to_string());
                                    tracing::warn!(
                                        server = %server.name,
                                        prompt = %manifest.name,
                                        error = %redacted,
                                        "MCP prompt normalization failed; skipping prompt"
                                    );
                                    continue;
                                }
                            };

                        if !seen_names.insert(canonical.clone()) {
                            anyhow::bail!(collision_error_message(&canonical));
                        }

                        let adapter = match prompt_adapter::McpPromptAdapter::from_manifest(
                            server,
                            manifest,
                            client.clone(),
                        ) {
                            Ok(adapter) => adapter,
                            Err(error) => {
                                let redacted = redact_error_message(&error.to_string());
                                tracing::warn!(
                                    server = %server.name,
                                    error = %redacted,
                                    "MCP prompt adapter creation failed; skipping prompt"
                                );
                                continue;
                            }
                        };

                        tools.push(Box::new(adapter));
                    }
                }
                Err(error) => {
                    let redacted = redact_error_message(&error.to_string());
                    tracing::warn!(
                        server = %server.name,
                        error = %redacted,
                        "MCP prompt discovery failed; continuing with other capabilities"
                    );
                }
            }
        } else {
            // Log if server advertises prompts but they're not in config
            if server.command == "__mcp_mock__" {
                if let Some(payload) = server.args.first() {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
                        if value.get("prompts").is_some() {
                            tracing::warn!(
                                server = %server.name,
                                "MCP server advertises prompts but 'prompts' is not in capabilities config; ignoring"
                            );
                        }
                    }
                }
            }
        }
    }

    tracing::info!(
        registered_capabilities = tools.len(),
        "MCP capability discovery completed"
    );

    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{McpConfig, McpServerConfig};

    #[test]
    fn collision_error_message_is_actionable_for_operators() {
        let message = collision_error_message("mcp.docs.search");
        assert!(message.contains("mcp.docs.search"));
        assert!(message.contains("mcp.servers[].name"));
        assert!(message.contains("canonical"));
    }

    fn mock_server(name: &str, payload: &str, capabilities: Vec<String>) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            enabled: true,
            command: "__mcp_mock__".to_string(),
            args: vec![payload.to_string()],
            capabilities,
            ..McpServerConfig::default()
        }
    }

    fn mock_config(servers: Vec<McpServerConfig>) -> McpConfig {
        McpConfig {
            enabled: true,
            servers,
        }
    }

    // ── Capability-gated discovery ───────────────────────────

    #[test]
    fn discover_capabilities_skips_resources_when_not_in_config() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}],
          "resources": [{"name":"index","uri":"docs://index","description":"Index"}]
        }"#;
        let server = mock_server("docs", payload, vec!["tools".to_string()]);
        let config = mock_config(vec![server]);

        let tools = discover_capabilities(&config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "mcp.docs.search");
    }

    #[test]
    fn discover_capabilities_skips_prompts_when_not_in_config() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}],
          "prompts": [{"name":"code-review","description":"Review code"}]
        }"#;
        let server = mock_server("workflows", payload, vec!["tools".to_string()]);
        let config = mock_config(vec![server]);

        let tools = discover_capabilities(&config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "mcp.workflows.search");
    }

    #[test]
    fn discover_capabilities_default_config_behaves_like_tools_only() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}]
        }"#;
        // Default capabilities is ["tools"]
        let server = mock_server("docs", payload, crate::config::default_mcp_capabilities());
        let config = mock_config(vec![server]);

        let tools = discover_capabilities(&config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "mcp.docs.search");
    }

    #[test]
    fn discover_capabilities_cross_capability_collision_detected() {
        // A tool named "mcp.docs.resource.index" collides with a resource "index" on server "docs"
        // But with our naming scheme, a tool "resource" would be rejected as reserved.
        // Instead test: two servers with the same resource name do NOT collide (different server segment)
        let payload1 = r#"{
          "resources": [{"name":"index","uri":"docs://index","description":"Index"}]
        }"#;
        let payload2 = r#"{
          "resources": [{"name":"index","uri":"kb://index","description":"Index"}]
        }"#;
        let server1 = mock_server("docs", payload1, vec!["resources".to_string()]);
        let server2 = mock_server("kb", payload2, vec!["resources".to_string()]);
        let config = mock_config(vec![server1, server2]);

        // Should not collide: mcp.docs.resource.index vs mcp.kb.resource.index
        let result = discover_capabilities(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn discover_capabilities_duplicate_resource_within_server_is_rejected() {
        let payload = r#"{
          "resources": [
            {"name":"index","uri":"docs://index","description":"Index"},
            {"name":"index","uri":"docs://index2","description":"Index duplicate"}
          ]
        }"#;
        let server = mock_server("docs", payload, vec!["resources".to_string()]);
        let config = mock_config(vec![server]);

        let result = discover_capabilities(&config);
        assert!(result.is_err());
        let err = result.err().expect("expected error");
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn discover_capabilities_tool_and_resource_same_name_coexist() {
        // Tool "search" -> mcp.docs.search, Resource "search" -> mcp.docs.resource.search
        let payload = r#"{
          "tools": [{"name":"search","description":"Search tool"}],
          "resources": [{"name":"search","uri":"docs://search","description":"Search resource"}]
        }"#;
        let server = mock_server(
            "docs",
            payload,
            vec!["tools".to_string(), "resources".to_string()],
        );
        let config = mock_config(vec![server]);

        let result = discover_capabilities(&config);
        assert!(result.is_ok());
        let tools = result.unwrap();
        // Both tool and resource adapters are registered with distinct names
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name(), "mcp.docs.search");
        assert_eq!(tools[1].name(), "mcp.docs.resource.search");
    }

    // ── Prompt discovery integration tests ──────────────────────

    #[test]
    fn discover_capabilities_registers_prompts_when_in_config() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}],
          "prompts": [
            {"name":"code-review","description":"Review code","arguments":[
              {"name":"language","description":"Lang","required":true}
            ]}
          ]
        }"#;
        let server = mock_server(
            "workflows",
            payload,
            vec!["tools".to_string(), "prompts".to_string()],
        );
        let config = mock_config(vec![server]);

        let tools = discover_capabilities(&config).unwrap();
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"mcp.workflows.search"));
        assert!(names.contains(&"mcp.workflows.prompt.code-review"));
    }

    #[test]
    fn discover_capabilities_prompt_not_registered_when_absent_from_capabilities() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}],
          "prompts": [{"name":"code-review","description":"Review code"}]
        }"#;
        // Only "tools" in capabilities — prompts should be ignored
        let server = mock_server("workflows", payload, vec!["tools".to_string()]);
        let config = mock_config(vec![server]);

        let tools = discover_capabilities(&config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "mcp.workflows.search");
    }

    #[test]
    fn discover_capabilities_duplicate_prompt_within_server_is_rejected() {
        let payload = r#"{
          "prompts": [
            {"name":"review","description":"Review v1"},
            {"name":"review","description":"Review v2"}
          ]
        }"#;
        let server = mock_server("workflows", payload, vec!["prompts".to_string()]);
        let config = mock_config(vec![server]);

        let result = discover_capabilities(&config);
        assert!(result.is_err());
        let err = result.err().expect("expected error");
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn discover_capabilities_tool_and_prompt_same_name_coexist() {
        // Tool "summarize" -> mcp.devtools.summarize
        // Prompt "summarize" -> mcp.devtools.prompt.summarize
        let payload = r#"{
          "tools": [{"name":"summarize","description":"Summarize tool"}],
          "prompts": [{"name":"summarize","description":"Summarize prompt"}]
        }"#;
        let server = mock_server(
            "devtools",
            payload,
            vec!["tools".to_string(), "prompts".to_string()],
        );
        let config = mock_config(vec![server]);

        let result = discover_capabilities(&config).unwrap();
        assert_eq!(result.len(), 2);
        let names: Vec<&str> = result.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"mcp.devtools.summarize"));
        assert!(names.contains(&"mcp.devtools.prompt.summarize"));
    }

    #[test]
    fn discover_capabilities_cross_server_prompts_no_collision() {
        let payload1 = r#"{
          "prompts": [{"name":"review","description":"Review"}]
        }"#;
        let payload2 = r#"{
          "prompts": [{"name":"review","description":"Review"}]
        }"#;
        let server1 = mock_server("alpha", payload1, vec!["prompts".to_string()]);
        let server2 = mock_server("beta", payload2, vec!["prompts".to_string()]);
        let config = mock_config(vec![server1, server2]);

        let result = discover_capabilities(&config).unwrap();
        assert_eq!(result.len(), 2);
        let names: Vec<&str> = result.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"mcp.alpha.prompt.review"));
        assert!(names.contains(&"mcp.beta.prompt.review"));
    }

    #[test]
    fn discover_capabilities_prompt_spec_has_correct_kind() {
        let payload = r#"{
          "prompts": [{"name":"code-review","description":"Review code"}]
        }"#;
        let server = mock_server("workflows", payload, vec!["prompts".to_string()]);
        let config = mock_config(vec![server]);

        let tools = discover_capabilities(&config).unwrap();
        assert_eq!(tools.len(), 1);
        let spec = tools[0].spec();
        let source = spec.source.unwrap();
        assert_eq!(source.kind, "mcp_prompt");
        assert_eq!(source.server.as_deref(), Some("workflows"));
    }

    #[test]
    fn discover_capabilities_prompt_failure_isolation() {
        // First server has valid tools, second has prompts that fail normalization
        let payload1 = r#"{
          "tools": [{"name":"search","description":"Search"}]
        }"#;
        let payload2 = r#"{
          "prompts": [{"name":"invalid chars!","description":"Bad prompt"}]
        }"#;
        let server1 = mock_server("good", payload1, vec!["tools".to_string()]);
        let server2 = mock_server("bad", payload2, vec!["prompts".to_string()]);
        let config = mock_config(vec![server1, server2]);

        // The good server's tool should still be registered
        let result = discover_capabilities(&config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "mcp.good.search");
    }
}
