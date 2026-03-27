use corvus::config::{BrowserConfig, Config, McpConfig, McpServerConfig, MemoryConfig};
use corvus::memory::{self, Memory};
use corvus::security::SecurityPolicy;
use corvus::tools::{self, Tool};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

fn server(name: &str, command: &str, args: Vec<String>) -> McpServerConfig {
    McpServerConfig {
        name: name.to_string(),
        enabled: true,
        command: command.to_string(),
        args,
        env: BTreeMap::new(),
        startup_timeout_ms: 50,
        call_timeout_ms: 50,
        output_limit_bytes: 512,
        capabilities: vec!["tools".to_string()],
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    }
}

fn build_tools(root_config: &Config, workspace: &std::path::Path) -> Vec<Box<dyn Tool>> {
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

#[tokio::test]
async fn runtime_registers_and_invokes_mcp_tool_when_enabled() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = Config {
        workspace_dir: tmp.path().to_path_buf(),
        config_path: tmp.path().join("config.toml"),
        mcp: McpConfig {
            enabled: true,
            servers: vec![server(
                "docs",
                "__mcp_mock__",
                vec![
                    r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                        .to_string(),
                ],
            )],
        },
        ..Config::default()
    };

    let tools = build_tools(&cfg, tmp.path());
    let mcp = tools
        .iter()
        .find(|tool| tool.name() == "mcp.docs.search")
        .expect("mcp tool should be registered");
    let result = mcp
        .execute(serde_json::json!({"query": "rust"}))
        .await
        .unwrap();
    assert!(result.success);
    assert_eq!(result.output, "mock-ok");
}

#[tokio::test]
async fn runtime_isolates_failing_server_and_keeps_healthy_server() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = Config {
        workspace_dir: tmp.path().to_path_buf(),
        config_path: tmp.path().join("config.toml"),
        mcp: McpConfig {
            enabled: true,
            servers: vec![
                server("slow", "__mcp_mock_sleep__", vec!["200".to_string()]),
                server(
                    "docs",
                    "__mcp_mock__",
                    vec![
                        r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                            .to_string(),
                    ],
                ),
            ],
        },
        ..Config::default()
    };

    let tools = build_tools(&cfg, tmp.path());
    let names: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
    assert!(names.contains(&"mcp.docs.search"));
    assert!(!names.iter().any(|name| name.starts_with("mcp.slow.")));
}

#[tokio::test]
async fn runtime_skips_mcp_registration_when_disabled() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = Config {
        workspace_dir: tmp.path().to_path_buf(),
        config_path: tmp.path().join("config.toml"),
        mcp: McpConfig {
            enabled: false,
            servers: vec![server(
                "docs",
                "__mcp_mock__",
                vec![
                    r#"{"tools":[{"name":"search","description":"Search docs","parameters":{"type":"object"}}]}"#
                        .to_string(),
                ],
            )],
        },
        ..Config::default()
    };

    let tools = build_tools(&cfg, tmp.path());
    let names: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
    assert!(!names.iter().any(|name| name.starts_with("mcp.")));
}
