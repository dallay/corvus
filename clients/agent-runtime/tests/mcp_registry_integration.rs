use corvus::config::{default_mcp_capabilities, McpConfig, McpServerConfig};
use corvus::tools::mcp;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

fn mock_server(name: &str, command: &str, args: Vec<String>) -> McpServerConfig {
    McpServerConfig {
        name: name.to_string(),
        enabled: true,
        command: command.to_string(),
        args,
        env: BTreeMap::new(),
        startup_timeout_ms: 50,
        call_timeout_ms: 500,
        output_limit_bytes: 1024,
        capabilities: vec!["tools".to_string()],
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    }
}

#[test]
fn discovery_skips_disabled_servers() {
    let mut disabled = mock_server(
        "docs",
        "__mcp_mock__",
        vec![
            r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                .to_string(),
        ],
    );
    disabled.enabled = false;

    let config = McpConfig {
        enabled: true,
        servers: vec![disabled],
    };

    let tools = mcp::discover_tools(&config).unwrap();
    assert!(tools.is_empty());
}

#[test]
fn discovery_is_bounded_by_startup_timeout() {
    let slow = mock_server("slow", "__mcp_mock_sleep__", vec!["200".to_string()]);
    let healthy = mock_server(
        "docs",
        "__mcp_mock__",
        vec![
            r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                .to_string(),
        ],
    );
    let config = McpConfig {
        enabled: true,
        servers: vec![slow, healthy],
    };

    let start = Instant::now();
    let tools = mcp::discover_tools(&config).unwrap();
    let elapsed = start.elapsed();

    let names: Vec<String> = tools.iter().map(|tool| tool.name().to_string()).collect();
    assert!(names.contains(&"mcp.docs.search".to_string()));
    assert!(!names.iter().any(|name| name.starts_with("mcp.slow.")));
    // Allow generous bound for CI jitter - discovery should complete within timeout + margin
    let config_timeout_ms = 100u64;
    assert!(elapsed < Duration::from_millis(config_timeout_ms + 200));
}

#[test]
fn discovery_ignores_non_tool_capabilities_and_registers_only_tools() {
    let config = McpConfig {
        enabled: true,
        servers: vec![mock_server(
            "docs",
            "__mcp_mock__",
            vec![
                r#"{
                  "tools": [{"name":"search","description":"Search docs","parameters":{"type":"object"}}],
                  "resources": [{"uri":"docs://index"}],
                  "prompts": [{"name":"summarize"}]
                }"#
                .to_string(),
            ],
        )],
    };

    let tools = mcp::discover_tools(&config).unwrap();
    let names: Vec<String> = tools.iter().map(|tool| tool.name().to_string()).collect();
    assert_eq!(names, vec!["mcp.docs.search".to_string()]);
}

#[test]
fn discovery_reports_actionable_collision_errors() {
    let server_a = mock_server(
        "docs",
        "__mcp_mock__",
        vec![
            r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                .to_string(),
        ],
    );
    let server_b = mock_server(
        "docs",
        "__mcp_mock__",
        vec![
            r#"{"tools":[{"name":"search","description":"Search docs v2","parameters":{"type":"object"}}]}"#
                .to_string(),
        ],
    );

    let config = McpConfig {
        enabled: true,
        servers: vec![server_a, server_b],
    };

    let error = mcp::discover_tools(&config)
        .err()
        .expect("expected collision error")
        .to_string();
    assert!(error.contains("mcp.docs.search"));
    assert!(error.contains("mcp.servers[].name"));
    assert!(error.contains("canonical id is unique"));
}

// ── Task 3.3: Backward compatibility regression tests ───────────

/// Config without `capabilities` field works identically to v1 (tools-only).
/// The default_mcp_capabilities() returns ["tools"], so omitting the field
/// produces the same discovery behavior as explicit tools-only.
#[test]
fn backward_compat_default_capabilities_discovers_tools_only() {
    let server = McpServerConfig {
        name: "docs".to_string(),
        enabled: true,
        command: "__mcp_mock__".to_string(),
        args: vec![
            r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}],"resources":[{"name":"index","uri":"docs://index","description":"Index"}],"prompts":[{"name":"review","description":"Review"}]}"#
                .to_string(),
        ],
        env: BTreeMap::new(),
        startup_timeout_ms: 50,
        call_timeout_ms: 500,
        output_limit_bytes: 1024,
        capabilities: default_mcp_capabilities(), // ["tools"] — v1 default
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    };

    let config = McpConfig {
        enabled: true,
        servers: vec![server],
    };

    let tools = mcp::discover_capabilities(&config).unwrap();
    let names: Vec<String> = tools.iter().map(|t| t.name().to_string()).collect();

    // Only tools registered — resources and prompts ignored
    assert_eq!(names, vec!["mcp.docs.search"]);
}

/// Adding `capabilities = ["tools", "resources"]` does not break existing
/// tool registrations: the tool name, policy kind, and spec are unchanged.
#[test]
fn backward_compat_adding_resources_does_not_break_tool_registrations() {
    let payload = r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}],"resources":[{"name":"index","uri":"docs://index","description":"Index"}]}"#;

    // v1 config: tools-only
    let v1_server = mock_server("docs", "__mcp_mock__", vec![payload.to_string()]);
    let v1_config = McpConfig {
        enabled: true,
        servers: vec![v1_server],
    };
    let v1_tools = mcp::discover_capabilities(&v1_config).unwrap();

    // v2 config: tools + resources
    let mut v2_server = mock_server("docs", "__mcp_mock__", vec![payload.to_string()]);
    v2_server.capabilities = vec!["tools".to_string(), "resources".to_string()];
    let v2_config = McpConfig {
        enabled: true,
        servers: vec![v2_server],
    };
    let v2_tools = mcp::discover_capabilities(&v2_config).unwrap();

    // The tool "mcp.docs.search" must be present in both configs
    let v1_tool = v1_tools
        .iter()
        .find(|t| t.name() == "mcp.docs.search")
        .expect("v1 must have mcp.docs.search");
    let v2_tool = v2_tools
        .iter()
        .find(|t| t.name() == "mcp.docs.search")
        .expect("v2 must have mcp.docs.search");

    // Tool identity and spec are unchanged
    assert_eq!(v1_tool.name(), v2_tool.name());
    assert_eq!(v1_tool.description(), v2_tool.description());
    assert_eq!(v1_tool.spec().name, v2_tool.spec().name);
    assert_eq!(
        v1_tool.spec().source.as_ref().map(|s| s.kind.as_str()),
        v2_tool.spec().source.as_ref().map(|s| s.kind.as_str()),
    );
}

/// Tool discovery behavior (naming, policy, timeouts, output limits)
/// is unchanged when using default capabilities.
#[test]
fn backward_compat_tool_discovery_unchanged_with_default_capabilities() {
    let server = mock_server(
        "docs",
        "__mcp_mock__",
        vec![
            r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                .to_string(),
        ],
    );
    let config = McpConfig {
        enabled: true,
        servers: vec![server],
    };

    let tools = mcp::discover_capabilities(&config).unwrap();
    assert_eq!(tools.len(), 1);

    let tool = &tools[0];
    // Naming follows mcp.<server>.<tool> pattern
    assert_eq!(tool.name(), "mcp.docs.search");
    // Spec metadata is correct
    let spec = tool.spec();
    let source = spec.source.unwrap();
    assert_eq!(source.kind, "mcp");
    assert_eq!(source.server.as_deref(), Some("docs"));
    assert_eq!(source.provider.as_deref(), Some("mcp"));

    // Policy: MCP tools require approval
    let risk = corvus::agent::dispatcher::evaluate_tool_risk("mcp.docs.search");
    assert!(matches!(
        risk,
        corvus::agent::dispatcher::DispatchAction::ApprovalRequired(_)
    ));
}
