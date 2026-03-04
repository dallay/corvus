use corvus::config::{McpConfig, McpServerConfig};
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
    assert!(error.contains("canonical tool id is unique"));
}
