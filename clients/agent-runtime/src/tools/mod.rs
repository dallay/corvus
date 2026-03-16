pub mod browser;
pub mod browser_open;
pub mod composio;
pub mod cron_add;
pub mod cron_list;
pub mod cron_remove;
pub mod cron_run;
pub mod cron_runs;
pub mod cron_update;
pub mod delegate;
pub mod file_read;
pub mod file_write;
pub mod git_operations;
pub mod hardware_board_info;
pub mod hardware_memory_map;
pub mod hardware_memory_read;
pub mod http_request;
pub mod image_info;
#[cfg(feature = "mcp-runtime")]
pub mod mcp;
pub mod memory_forget;
pub mod memory_recall;
pub mod memory_store;
pub mod pushover;
pub mod schedule;
pub mod schema;
pub mod screenshot;
pub mod shell;
pub mod traits;
pub(crate) mod url_safety;
pub mod web_search_tool;

pub use browser::{BrowserTool, ComputerUseConfig};
pub use browser_open::BrowserOpenTool;
pub use composio::ComposioTool;
pub use cron_add::CronAddTool;
pub use cron_list::CronListTool;
pub use cron_remove::CronRemoveTool;
pub use cron_run::CronRunTool;
pub use cron_runs::CronRunsTool;
pub use cron_update::CronUpdateTool;
pub use delegate::DelegateTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use git_operations::GitOperationsTool;
pub use hardware_board_info::HardwareBoardInfoTool;
pub use hardware_memory_map::HardwareMemoryMapTool;
pub use hardware_memory_read::HardwareMemoryReadTool;
pub use http_request::HttpRequestTool;
pub use image_info::ImageInfoTool;
pub use memory_forget::MemoryForgetTool;
pub use memory_recall::MemoryRecallTool;
pub use memory_store::MemoryStoreTool;
pub use pushover::PushoverTool;
pub use schedule::ScheduleTool;
#[allow(unused_imports)]
pub use schema::{CleaningStrategy, SchemaCleanr};
pub use screenshot::ScreenshotTool;
pub use shell::ShellTool;
pub use traits::Tool;
#[allow(unused_imports)]
pub use traits::{ToolResult, ToolSpec};
pub use web_search_tool::WebSearchTool;

use crate::config::{Config, DelegateAgentConfig};
use crate::memory::Memory;
use crate::runtime::{NativeRuntime, RuntimeAdapter};
use crate::security::SecurityPolicy;
use std::collections::HashMap;
#[cfg(feature = "mcp-runtime")]
use std::collections::HashSet;
use std::sync::Arc;

pub(crate) fn redact_runtime_error(raw: &str) -> String {
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

/// Create the default tool registry
pub fn default_tools(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    default_tools_with_runtime(security, Arc::new(NativeRuntime::new()))
}

/// Create the default tool registry with explicit runtime adapter.
pub fn default_tools_with_runtime(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ShellTool::new(security.clone(), runtime)),
        Box::new(FileReadTool::new(security.clone())),
        Box::new(FileWriteTool::new(security)),
    ]
}

fn add_browser_tools(
    tools: &mut Vec<Box<dyn Tool>>,
    security: &Arc<SecurityPolicy>,
    browser_config: &crate::config::BrowserConfig,
) {
    if !browser_config.enabled {
        return;
    }

    tools.push(Box::new(BrowserOpenTool::new(
        security.clone(),
        browser_config.allowed_domains.clone(),
    )));
    tools.push(Box::new(BrowserTool::new_with_backend(
        security.clone(),
        browser_config.allowed_domains.clone(),
        browser_config.session_name.clone(),
        browser_config.backend.clone(),
        browser_config.native_headless,
        browser_config.native_webdriver_url.clone(),
        browser_config.native_chrome_path.clone(),
        ComputerUseConfig {
            endpoint: browser_config.computer_use.endpoint.clone(),
            api_key: browser_config.computer_use.api_key.clone(),
            timeout_ms: browser_config.computer_use.timeout_ms,
            allow_remote_endpoint: browser_config.computer_use.allow_remote_endpoint,
            window_allowlist: browser_config.computer_use.window_allowlist.clone(),
            max_coordinate_x: browser_config.computer_use.max_coordinate_x,
            max_coordinate_y: browser_config.computer_use.max_coordinate_y,
        },
    )));
}

fn add_http_request_tool(
    tools: &mut Vec<Box<dyn Tool>>,
    security: &Arc<SecurityPolicy>,
    http_config: &crate::config::HttpRequestConfig,
) {
    if !http_config.enabled {
        return;
    }

    tools.push(Box::new(HttpRequestTool::new(
        security.clone(),
        http_config.allowed_domains.clone(),
        http_config.max_response_size,
        http_config.timeout_secs,
    )));
}

fn add_web_search_tool(
    tools: &mut Vec<Box<dyn Tool>>,
    security: &Arc<SecurityPolicy>,
    root_config: &crate::config::Config,
) {
    if !root_config.web_search.enabled {
        return;
    }

    tools.push(Box::new(WebSearchTool::new(
        security.clone(),
        root_config.web_search.provider.clone(),
        root_config.web_search.brave_api_key.clone(),
        root_config.web_search.max_results,
        root_config.web_search.timeout_secs,
    )));
}

fn add_composio_tool(
    tools: &mut Vec<Box<dyn Tool>>,
    security: &Arc<SecurityPolicy>,
    composio_key: Option<&str>,
    composio_entity_id: Option<&str>,
) {
    let Some(key) = composio_key else {
        return;
    };
    if key.is_empty() {
        return;
    }

    tools.push(Box::new(ComposioTool::new(
        key,
        composio_entity_id,
        security.clone(),
    )));
}

fn add_delegate_tool(
    tools: &mut Vec<Box<dyn Tool>>,
    security: &Arc<SecurityPolicy>,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    base_config: Arc<Config>,
) {
    if agents.is_empty() {
        return;
    }

    let delegate_agents: HashMap<String, DelegateAgentConfig> = agents.clone();
    let delegate_fallback_credential = fallback_api_key.and_then(|value| {
        let trimmed_value = value.trim();
        (!trimmed_value.is_empty()).then(|| trimmed_value.to_owned())
    });
    tools.push(Box::new(DelegateTool::new(
        delegate_agents,
        delegate_fallback_credential,
        security.clone(),
        base_config,
    )));
}

#[cfg(feature = "mcp-runtime")]
fn extend_with_mcp_tools(tools: &mut Vec<Box<dyn Tool>>, root_config: &crate::config::Config) {
    if !root_config.mcp.enabled {
        return;
    }

    match mcp::discover_tools(&root_config.mcp) {
        Ok(mcp_tools) => {
            let mut existing_names: HashSet<String> =
                tools.iter().map(|tool| tool.name().to_string()).collect();
            let mut detected_collision: Option<String> = None;

            for mcp_tool in &mcp_tools {
                let name = mcp_tool.name();
                if !existing_names.insert(name.to_string()) {
                    detected_collision = Some(name.to_string());
                    break;
                }
            }

            if let Some(collision) = detected_collision {
                tracing::warn!(
                    collision = %collision,
                    "MCP registration skipped due to tool-name collision"
                );
            } else {
                tools.extend(mcp_tools);
            }
        }
        Err(error) => {
            let redacted = redact_runtime_error(&error.to_string());
            tracing::warn!(
                error = %redacted,
                "mcp.enabled is true but MCP tool discovery failed"
            );
        }
    }
}

/// Create full tool registry including memory tools and optional Composio
#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub fn all_tools(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    memory: Arc<dyn Memory>,
    composio_key: Option<&str>,
    composio_entity_id: Option<&str>,
    browser_config: &crate::config::BrowserConfig,
    http_config: &crate::config::HttpRequestConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &crate::config::Config,
) -> Vec<Box<dyn Tool>> {
    all_tools_with_runtime(
        config,
        security,
        Arc::new(NativeRuntime::new()),
        memory,
        composio_key,
        composio_entity_id,
        browser_config,
        http_config,
        workspace_dir,
        agents,
        fallback_api_key,
        root_config,
    )
}

/// Create full tool registry including memory tools and optional Composio.
#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub fn all_tools_with_runtime(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    memory: Arc<dyn Memory>,
    composio_key: Option<&str>,
    composio_entity_id: Option<&str>,
    browser_config: &crate::config::BrowserConfig,
    http_config: &crate::config::HttpRequestConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &crate::config::Config,
) -> Vec<Box<dyn Tool>> {
    let local_memory_available = memory.name() != "none";
    let cerebro_configured = crate::memory::cerebro_configured(&root_config.memory);
    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ShellTool::new(security.clone(), runtime)),
        Box::new(FileReadTool::new(security.clone())),
        Box::new(FileWriteTool::new(security.clone())),
        Box::new(CronAddTool::new(config.clone(), security.clone())),
        Box::new(CronListTool::new(config.clone())),
        Box::new(CronRemoveTool::new(config.clone())),
        Box::new(CronUpdateTool::new(config.clone(), security.clone())),
        Box::new(CronRunTool::new(config.clone())),
        Box::new(CronRunsTool::new(config.clone())),
        Box::new(ScheduleTool::new(security.clone(), root_config.clone())),
        Box::new(GitOperationsTool::new(
            security.clone(),
            workspace_dir.to_path_buf(),
        )),
        Box::new(PushoverTool::new(
            security.clone(),
            workspace_dir.to_path_buf(),
        )),
    ];

    if local_memory_available {
        tools.push(Box::new(MemoryStoreTool::with_local(
            memory.clone(),
            security.clone(),
        )));
        tools.push(Box::new(MemoryRecallTool::with_local(
            memory.clone(),
            security.clone(),
        )));
        tools.push(Box::new(MemoryForgetTool::with_local(
            memory.clone(),
            security.clone(),
        )));
    } else if cerebro_configured {
        tools.push(Box::new(MemoryStoreTool::new(
            root_config.memory.cerebro.clone(),
            security.clone(),
        )));
        tools.push(Box::new(MemoryRecallTool::new(
            root_config.memory.cerebro.clone(),
            security.clone(),
        )));
        tools.push(Box::new(MemoryForgetTool::new(
            root_config.memory.cerebro.clone(),
            security.clone(),
        )));
    }

    add_browser_tools(&mut tools, security, browser_config);
    add_http_request_tool(&mut tools, security, http_config);
    add_web_search_tool(&mut tools, security, root_config);

    // Vision tools are always available
    tools.push(Box::new(ScreenshotTool::new(security.clone())));
    tools.push(Box::new(ImageInfoTool::new(security.clone())));

    add_composio_tool(&mut tools, security, composio_key, composio_entity_id);

    add_delegate_tool(
        &mut tools,
        security,
        agents,
        fallback_api_key,
        config.clone(),
    );

    #[cfg(feature = "mcp-runtime")]
    extend_with_mcp_tools(&mut tools, root_config);

    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BrowserConfig, Config, DelegateExecutionMode, McpConfig, MemoryConfig};
    use crate::test_support::{mock_mcp_server, test_config};
    use tempfile::TempDir;

    #[test]
    fn default_tools_has_three() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn all_tools_excludes_browser_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig {
            enabled: false,
            allowed_domains: vec!["example.com".into()],
            session_name: None,
            ..BrowserConfig::default()
        };
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.contains(&"browser_open"));
        assert!(names.contains(&"schedule"));
        assert!(names.contains(&"pushover"));
    }

    #[test]
    fn all_tools_includes_browser_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig {
            enabled: true,
            allowed_domains: vec!["example.com".into()],
            session_name: None,
            ..BrowserConfig::default()
        };
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"browser_open"));
        assert!(names.contains(&"pushover"));
    }

    #[test]
    fn default_tools_names() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
    }

    #[test]
    fn default_tools_all_have_descriptions() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            assert!(
                !tool.description().is_empty(),
                "Tool {} has empty description",
                tool.name()
            );
        }
    }

    #[test]
    fn default_tools_all_have_schemas() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            let schema = tool.parameters_schema();
            assert!(
                schema.is_object(),
                "Tool {} schema is not an object",
                tool.name()
            );
            assert!(
                schema["properties"].is_object(),
                "Tool {} schema has no properties",
                tool.name()
            );
        }
    }

    #[test]
    fn tool_spec_generation() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            let spec = tool.spec();
            assert_eq!(spec.name, tool.name());
            assert_eq!(spec.description, tool.description());
            assert!(spec.parameters.is_object());
        }
    }

    #[test]
    fn tool_result_serde() {
        let result = ToolResult {
            success: true,
            output: "hello".into(),
            error: None,
            structured: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.output, "hello");
        assert!(parsed.error.is_none());
    }

    #[test]
    fn tool_result_with_error_serde() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("boom".into()),
            structured: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(!parsed.success);
        assert_eq!(parsed.error.as_deref(), Some("boom"));
    }

    #[test]
    fn tool_spec_serde() {
        let spec = ToolSpec {
            name: "test".into(),
            description: "A test tool".into(),
            parameters: serde_json::json!({"type": "object"}),
            source: None,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: ToolSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.description, "A test tool");
    }

    #[test]
    fn all_tools_includes_delegate_when_agents_configured() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let mut agents = HashMap::new();
        agents.insert(
            "researcher".to_string(),
            DelegateAgentConfig {
                provider: "ollama".to_string(),
                model: "llama3".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &agents,
            Some("delegate-test-credential"),
            &cfg,
        );
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"delegate"));
    }

    #[test]
    fn all_tools_excludes_delegate_when_no_agents() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.contains(&"delegate"));
    }

    #[cfg(feature = "mcp-runtime")]
    #[test]
    fn all_tools_registers_mcp_tools_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let mut cfg = test_config(&tmp);
        cfg.mcp = McpConfig {
            enabled: true,
            servers: vec![mock_mcp_server("docs", "search")],
        };

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"mcp.docs.search"));
    }

    #[test]
    fn all_tools_skips_disabled_mcp_servers() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let mut cfg = test_config(&tmp);
        let mut server = mock_mcp_server("docs", "search");
        server.enabled = false;
        cfg.mcp = McpConfig {
            enabled: true,
            servers: vec![server],
        };

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.iter().any(|name| name.starts_with("mcp.")));
    }

    #[test]
    fn all_tools_fails_closed_on_mcp_name_collisions() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let mut cfg = test_config(&tmp);
        cfg.mcp = McpConfig {
            enabled: true,
            servers: vec![
                mock_mcp_server("docs", "search"),
                mock_mcp_server("docs", "search"),
            ],
        };

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.iter().any(|name| name.starts_with("mcp.")));
    }
}
