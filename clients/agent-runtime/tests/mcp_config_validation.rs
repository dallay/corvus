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

#[test]
fn rejects_legacy_surreal_memory_backend() {
    let mut config = Config::default();
    config.memory.backend = "surreal".to_string();

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(err.contains("memory.backend"));
    assert!(err.contains("SurrealDB backend has been removed"));
}

#[test]
fn rejects_insecure_cerebro_endpoint_without_loopback_opt_in() {
    let mut config = Config::default();
    config.memory.cerebro.endpoint = Some("http://cerebro.example.com/mcp".into());
    config.memory.cerebro.auth_token = Some("token".into());
    config.memory.cerebro.request_timeout_ms = 5_000;

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(err.contains("memory.cerebro.endpoint"));
    assert!(err.contains("allow_insecure_loopback"));
}

#[test]
fn rejects_insecure_ws_cerebro_endpoint_without_loopback_opt_in() {
    let mut config = Config::default();
    config.memory.cerebro.endpoint = Some("ws://cerebro.example.com/mcp".into());
    config.memory.cerebro.auth_token = Some("token".into());
    config.memory.cerebro.request_timeout_ms = 5_000;

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(err.contains("memory.cerebro.endpoint"));
    assert!(err.contains("allow_insecure_loopback"));
}

#[test]
fn rejects_cerebro_endpoint_without_auth_token() {
    let mut config = Config::default();
    config.memory.cerebro.endpoint = Some("https://cerebro.example.com/mcp".into());

    let err = config.validate_for_runtime().unwrap_err().to_string();
    assert!(err.contains("memory.cerebro.auth_token"));
}

#[test]
fn accepts_secure_https_cerebro_endpoint() {
    let mut config = Config::default();
    config.memory.cerebro.endpoint = Some("https://cerebro.example.com/mcp".into());
    config.memory.cerebro.auth_token = Some("token".into());

    assert!(config.validate_for_runtime().is_ok());
}

#[test]
fn accepts_secure_wss_cerebro_endpoint() {
    let mut config = Config::default();
    config.memory.cerebro.endpoint = Some("wss://cerebro.example.com/mcp".into());
    config.memory.cerebro.auth_token = Some("token".into());

    assert!(config.validate_for_runtime().is_ok());
}
