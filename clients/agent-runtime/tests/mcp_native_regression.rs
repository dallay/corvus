use corvus::config::{BrowserConfig, Config, McpConfig, McpServerConfig, MemoryConfig};
use corvus::memory::{self, Memory};
use corvus::security::SecurityPolicy;
use corvus::tools;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

fn build_tools(
    root_config: &Config,
    workspace: &std::path::Path,
) -> Vec<Box<dyn corvus::tools::Tool>> {
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig {
        backend: "markdown".into(),
        ..MemoryConfig::default()
    };
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(&mem_cfg, workspace, None).unwrap());

    tools::all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        None,
        None,
        &BrowserConfig::default(),
        &corvus::config::HttpRequestConfig::default(),
        workspace,
        &HashMap::new(),
        None,
        root_config,
    )
}

fn mock_server() -> McpServerConfig {
    McpServerConfig {
        name: "docs".to_string(),
        enabled: true,
        command: "__mcp_mock__".to_string(),
        args: vec![
            r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                .to_string(),
        ],
        env: BTreeMap::new(),
        startup_timeout_ms: 50,
        call_timeout_ms: 50,
        output_limit_bytes: 512,
    }
}

#[test]
fn native_tool_registry_is_stable_with_mcp_enabled_or_disabled() {
    let tmp = tempfile::TempDir::new().unwrap();
    let disabled = Config {
        workspace_dir: tmp.path().to_path_buf(),
        config_path: tmp.path().join("config-disabled.toml"),
        mcp: McpConfig {
            enabled: false,
            servers: vec![mock_server()],
        },
        ..Config::default()
    };

    let mut enabled = disabled.clone();
    enabled.config_path = tmp.path().join("config-enabled.toml");
    enabled.mcp.enabled = true;

    let disabled_tools = build_tools(&disabled, tmp.path());
    let enabled_tools = build_tools(&enabled, tmp.path());

    let native_disabled: HashSet<String> = disabled_tools
        .iter()
        .map(|tool| tool.name().to_string())
        .filter(|name| !name.starts_with("mcp."))
        .collect();
    let native_enabled: HashSet<String> = enabled_tools
        .iter()
        .map(|tool| tool.name().to_string())
        .filter(|name| !name.starts_with("mcp."))
        .collect();

    assert_eq!(native_disabled, native_enabled);
    assert!(native_enabled.contains("shell"));
    assert!(native_enabled.contains("file_read"));
    assert!(native_enabled.contains("file_write"));
}

#[test]
fn native_tools_remain_available_when_mcp_discovery_fails() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = Config {
        workspace_dir: tmp.path().to_path_buf(),
        config_path: tmp.path().join("config.toml"),
        mcp: McpConfig {
            enabled: true,
            servers: vec![McpServerConfig {
                name: "broken".to_string(),
                enabled: true,
                command: "__mcp_mock_sleep__".to_string(),
                args: vec!["200".to_string()],
                env: BTreeMap::new(),
                startup_timeout_ms: 50,
                call_timeout_ms: 50,
                output_limit_bytes: 512,
            }],
        },
        ..Config::default()
    };

    let tools = build_tools(&config, tmp.path());
    let names: HashSet<String> = tools.iter().map(|tool| tool.name().to_string()).collect();
    assert!(names.contains("shell"));
    assert!(names.contains("file_read"));
    assert!(names.contains("file_write"));
    assert!(!names.iter().any(|name| name.starts_with("mcp.")));
}
