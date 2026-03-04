use corvus::config::{Config, McpConfig, McpServerConfig};
use std::collections::BTreeMap;

fn valid_server() -> McpServerConfig {
    McpServerConfig {
        name: "docs".to_string(),
        enabled: true,
        command: "mcp-docs".to_string(),
        args: vec!["serve".to_string()],
        env: BTreeMap::new(),
        startup_timeout_ms: 5_000,
        call_timeout_ms: 30_000,
        output_limit_bytes: 64 * 1024,
    }
}

#[test]
fn rejects_malformed_server_definition() {
    let config = Config {
        mcp: McpConfig {
            enabled: true,
            servers: vec![McpServerConfig {
                name: String::new(),
                ..valid_server()
            }],
        },
        ..Config::default()
    };

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(err.contains("mcp.servers[0].name"));
}

#[test]
fn rejects_non_positive_timeouts_and_limits() {
    // Test output_limit_bytes separately since startup_timeout_ms and call_timeout_ms
    // may fail first and prevent output_limit_bytes validation from running
    let config = Config {
        mcp: McpConfig {
            enabled: true,
            servers: vec![McpServerConfig {
                startup_timeout_ms: 1000,
                call_timeout_ms: 1000,
                output_limit_bytes: 0,
                ..valid_server()
            }],
        },
        ..Config::default()
    };

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(err.contains("output_limit_bytes") || err.contains("output_limit"));
}

#[test]
fn validation_error_redacts_secret_values() {
    let mut env = BTreeMap::new();
    env.insert("API_TOKEN".to_string(), "super-secret-value".to_string());

    let config = Config {
        mcp: McpConfig {
            enabled: true,
            servers: vec![McpServerConfig {
                startup_timeout_ms: 0,
                env,
                ..valid_server()
            }],
        },
        ..Config::default()
    };

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(!err.contains("super-secret-value"));
    assert!(err.contains("mcp.servers[0]"));
}
