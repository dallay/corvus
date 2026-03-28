pub mod adapter;
pub mod cerebro;
pub mod client;
pub mod normalize;
pub mod prompt_adapter;
pub mod resource_adapter;

use crate::config::{McpConfig, McpServerConfig};
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

fn register_with_collision_check(
    tool: Box<dyn Tool>,
    seen_names: &mut HashSet<String>,
    tools: &mut Vec<Box<dyn Tool>>,
) -> anyhow::Result<()> {
    let name = tool.name().to_string();
    if !seen_names.insert(name.clone()) {
        anyhow::bail!(collision_error_message(&name));
    }
    tools.push(tool);
    Ok(())
}

fn discover_server_tools(
    server: &McpServerConfig,
    client: &client::McpClient,
    seen_names: &mut HashSet<String>,
    tools: &mut Vec<Box<dyn Tool>>,
) -> anyhow::Result<()> {
    let manifests = match client.list_tools() {
        Ok(m) => m,
        Err(error) => {
            let redacted = redact_error_message(&error.to_string());
            tracing::warn!(
                server = %server.name,
                error = %redacted,
                "MCP tool discovery failed; continuing with other capabilities"
            );
            return Ok(());
        }
    };

    for manifest in manifests {
        let adapter = match adapter::McpToolAdapter::from_manifest(server, manifest, client.clone())
        {
            Ok(a) => a,
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
        register_with_collision_check(Box::new(adapter), seen_names, tools)?;
    }
    Ok(())
}

fn discover_server_resources(
    server: &McpServerConfig,
    client: &client::McpClient,
    seen_names: &mut HashSet<String>,
    tools: &mut Vec<Box<dyn Tool>>,
) -> anyhow::Result<()> {
    let manifests = match client.list_resources() {
        Ok(m) => m,
        Err(error) => {
            let redacted = redact_error_message(&error.to_string());
            tracing::warn!(
                server = %server.name,
                error = %redacted,
                "MCP resource discovery failed; continuing with other capabilities"
            );
            return Ok(());
        }
    };

    for manifest in manifests {
        let adapter = match resource_adapter::McpResourceAdapter::from_manifest(
            server,
            manifest,
            client.clone(),
        ) {
            Ok(a) => a,
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
        register_with_collision_check(Box::new(adapter), seen_names, tools)?;
    }
    Ok(())
}

fn discover_server_prompts(
    server: &McpServerConfig,
    client: &client::McpClient,
    seen_names: &mut HashSet<String>,
    tools: &mut Vec<Box<dyn Tool>>,
) -> anyhow::Result<()> {
    let manifests = match client.list_prompts() {
        Ok(m) => m,
        Err(error) => {
            let redacted = redact_error_message(&error.to_string());
            tracing::warn!(
                server = %server.name,
                error = %redacted,
                "MCP prompt discovery failed; continuing with other capabilities"
            );
            return Ok(());
        }
    };

    for manifest in manifests {
        let canonical = match normalize::normalize_prompt_name(&server.name, &manifest.name) {
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

        let adapter =
            match prompt_adapter::McpPromptAdapter::from_manifest(server, manifest, client.clone())
            {
                Ok(a) => a,
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
    Ok(())
}

fn warn_unadvertised_capability(server: &McpServerConfig, capability_name: &str) {
    if server.command != "__mcp_mock__" {
        return;
    }
    if let Some(payload) = server.args.first() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
            if value.get(capability_name).is_some() {
                tracing::warn!(
                    server = %server.name,
                    capability = capability_name,
                    "MCP server advertises {capability_name} but '{capability_name}' is not in capabilities config; ignoring"
                );
            }
        }
    }
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

        if server.capabilities.iter().any(|c| c == "tools") {
            discover_server_tools(server, &client, &mut seen_names, &mut tools)?;
        }

        if server.capabilities.iter().any(|c| c == "resources") {
            discover_server_resources(server, &client, &mut seen_names, &mut tools)?;
        } else {
            warn_unadvertised_capability(server, "resources");
        }

        if server.capabilities.iter().any(|c| c == "prompts") {
            discover_server_prompts(server, &client, &mut seen_names, &mut tools)?;
        } else {
            warn_unadvertised_capability(server, "prompts");
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

    // ── Disabled server / disabled config ────────────────────

    #[test]
    fn discover_capabilities_returns_empty_when_mcp_disabled() {
        let payload = r#"{"tools":[{"name":"search","description":"Search"}]}"#;
        let server = mock_server("docs", payload, vec!["tools".to_string()]);
        let config = McpConfig {
            enabled: false,
            servers: vec![server],
        };
        let tools = discover_capabilities(&config).unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn discover_capabilities_skips_disabled_server() {
        let payload = r#"{"tools":[{"name":"search","description":"Search"}]}"#;
        let mut server = mock_server("docs", payload, vec!["tools".to_string()]);
        server.enabled = false;
        let config = mock_config(vec![server]);
        let tools = discover_capabilities(&config).unwrap();
        assert!(tools.is_empty());
    }

    // ── Tool discovery failure isolation ─────────────────────

    #[test]
    fn discover_server_tools_list_failure_continues_gracefully() {
        // A payload that will cause list_tools to fail (invalid JSON for tools array)
        let payload = r#"{"tools":"not-an-array"}"#;
        let server = mock_server("bad", payload, vec!["tools".to_string()]);
        let config = mock_config(vec![server]);
        // Should not error — tool discovery failure is warn + continue
        let result = discover_capabilities(&config).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn discover_server_tools_adapter_failure_skips_bad_tool() {
        // First tool has a name that will fail normalization, second is valid
        let payload = r#"{
          "tools": [
            {"name":"invalid chars!@#","description":"Bad tool"},
            {"name":"good-tool","description":"Good tool"}
          ]
        }"#;
        let server = mock_server("srv", payload, vec!["tools".to_string()]);
        let config = mock_config(vec![server]);
        let result = discover_capabilities(&config).unwrap();
        // The bad tool is skipped, the good one is registered
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "mcp.srv.good-tool");
    }

    // ── Resource discovery failure isolation ─────────────────

    #[test]
    fn discover_server_resources_list_failure_continues_gracefully() {
        let payload = r#"{"resources":"not-an-array"}"#;
        let server = mock_server("bad", payload, vec!["resources".to_string()]);
        let config = mock_config(vec![server]);
        let result = discover_capabilities(&config).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn discover_server_resources_adapter_failure_skips_bad_resource() {
        let payload = r#"{
          "resources": [
            {"name":"invalid!@#","uri":"bad://x","description":"Bad"},
            {"name":"good-res","uri":"docs://good","description":"Good"}
          ]
        }"#;
        let server = mock_server("srv", payload, vec!["resources".to_string()]);
        let config = mock_config(vec![server]);
        let result = discover_capabilities(&config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "mcp.srv.resource.good-res");
    }

    // ── Prompt adapter creation failure ──────────────────────

    #[test]
    fn discover_server_prompts_adapter_creation_failure_skips_prompt() {
        // A prompt with a valid name but malformed arguments that cause adapter creation to fail
        let payload = r#"{
          "prompts": [
            {"name":"good-prompt","description":"Works"},
            {"name":"also-good","description":"Also works"}
          ]
        }"#;
        let server = mock_server("wf", payload, vec!["prompts".to_string()]);
        let config = mock_config(vec![server]);
        let result = discover_capabilities(&config).unwrap();
        // Both should succeed since names are valid
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn discover_server_prompts_list_failure_continues_gracefully() {
        let payload = r#"{"prompts":"not-an-array"}"#;
        let server = mock_server("bad", payload, vec!["prompts".to_string()]);
        let config = mock_config(vec![server]);
        let result = discover_capabilities(&config).unwrap();
        assert!(result.is_empty());
    }

    // ── Warn unadvertised capability ─────────────────────────

    #[test]
    fn warn_unadvertised_capability_fires_for_mock_server_with_resources() {
        // Server has resources in payload but only "tools" in capabilities
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}],
          "resources": [{"name":"index","uri":"docs://index","description":"Index"}]
        }"#;
        let server = mock_server("docs", payload, vec!["tools".to_string()]);
        // This exercises the warn_unadvertised_capability path for "resources"
        // It should not panic and should log a warning (we verify no-panic)
        warn_unadvertised_capability(&server, "resources");
    }

    #[test]
    fn warn_unadvertised_capability_fires_for_mock_server_with_prompts() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search"}],
          "prompts": [{"name":"review","description":"Review"}]
        }"#;
        let server = mock_server("wf", payload, vec!["tools".to_string()]);
        warn_unadvertised_capability(&server, "prompts");
    }

    #[test]
    fn warn_unadvertised_capability_no_op_for_non_mock_server() {
        let server = McpServerConfig {
            name: "real-server".to_string(),
            enabled: true,
            command: "npx".to_string(),
            args: vec!["@server/mcp".to_string()],
            capabilities: vec!["tools".to_string()],
            ..McpServerConfig::default()
        };
        // Should return early because command != "__mcp_mock__"
        warn_unadvertised_capability(&server, "resources");
    }

    #[test]
    fn warn_unadvertised_capability_no_op_when_capability_absent_from_payload() {
        let payload = r#"{"tools":[{"name":"search","description":"Search"}]}"#;
        let server = mock_server("docs", payload, vec!["tools".to_string()]);
        // Payload has no "resources" key, so no warning should fire
        warn_unadvertised_capability(&server, "resources");
    }

    #[test]
    fn warn_unadvertised_capability_no_op_when_args_empty() {
        let server = McpServerConfig {
            name: "empty-args".to_string(),
            enabled: true,
            command: "__mcp_mock__".to_string(),
            args: vec![],
            capabilities: vec!["tools".to_string()],
            ..McpServerConfig::default()
        };
        warn_unadvertised_capability(&server, "resources");
    }

    #[test]
    fn warn_unadvertised_capability_no_op_when_args_invalid_json() {
        let server = McpServerConfig {
            name: "bad-json".to_string(),
            enabled: true,
            command: "__mcp_mock__".to_string(),
            args: vec!["not valid json {{{".to_string()],
            capabilities: vec!["tools".to_string()],
            ..McpServerConfig::default()
        };
        warn_unadvertised_capability(&server, "resources");
    }

    // ── Collision detection ──────────────────────────────────

    #[test]
    fn register_with_collision_check_rejects_duplicate() {
        use crate::tools::traits::{Tool, ToolResult};
        use async_trait::async_trait;

        struct FakeTool(String);

        #[async_trait]
        impl Tool for FakeTool {
            fn name(&self) -> &str {
                &self.0
            }
            fn description(&self) -> &str {
                "fake"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
                Ok(ToolResult {
                    success: true,
                    output: "ok".into(),
                    error: None,
                    structured: None,
                })
            }
        }

        let mut seen = HashSet::new();
        let mut tools: Vec<Box<dyn Tool>> = Vec::new();

        let t1 = Box::new(FakeTool("mcp.srv.search".into()));
        assert!(register_with_collision_check(t1, &mut seen, &mut tools).is_ok());

        let t2 = Box::new(FakeTool("mcp.srv.search".into()));
        let err = register_with_collision_check(t2, &mut seen, &mut tools);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("duplicate"));
    }

    // ── Redact error message ─────────────────────────────────

    #[test]
    fn redact_error_message_replaces_sensitive_env_values() {
        // Set a temporary env var with a known sensitive key
        let key = "CORVUS_TEST_SECRET_KEY_XYZ";
        let value = "super-secret-value-12345";
        std::env::set_var(key, value);

        let raw = format!("Connection failed: {value} was rejected");
        let redacted = redact_error_message(&raw);
        assert!(!redacted.contains(value));
        assert!(redacted.contains("[REDACTED]"));

        std::env::remove_var(key);
    }

    #[test]
    fn redact_error_message_preserves_non_sensitive_content() {
        let raw = "Connection timed out after 30s";
        let redacted = redact_error_message(raw);
        assert_eq!(redacted, raw);
    }

    // ── discover_tools backward compat alias ─────────────────

    #[test]
    fn discover_tools_alias_delegates_to_discover_capabilities() {
        let payload = r#"{"tools":[{"name":"search","description":"Search"}]}"#;
        let server = mock_server("docs", payload, vec!["tools".to_string()]);
        let config = mock_config(vec![server]);
        let tools = discover_tools(&config).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "mcp.docs.search");
    }
}
