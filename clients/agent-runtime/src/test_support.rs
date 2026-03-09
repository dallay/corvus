use crate::config::{Config, McpServerConfig};
use std::collections::BTreeMap;
use tempfile::TempDir;

pub(crate) fn test_config(tmp: &TempDir) -> Config {
    Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    }
}

pub(crate) fn mock_mcp_server(name: &str, tool_name: &str) -> McpServerConfig {
    McpServerConfig {
        name: name.to_string(),
        enabled: true,
        command: "__mcp_mock__".to_string(),
        args: vec![format!(
            r#"{{"tools":[{{"name":"{tool_name}","description":"Mock tool","parameters":{{"type":"object"}}}}]}}"#
        )],
        env: BTreeMap::new(),
        startup_timeout_ms: 100,
        call_timeout_ms: 500,
        output_limit_bytes: 1024,
    }
}
