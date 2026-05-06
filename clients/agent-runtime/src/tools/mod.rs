pub mod browser;
pub mod browser_open;
pub mod code_search;
pub mod composio;
pub mod cron_add;
pub mod cron_list;
pub mod cron_remove;
pub mod cron_run;
pub mod cron_runs;
pub mod cron_update;
pub mod delegate;
pub mod delegate_cancel;
pub mod delegate_inspect;
pub mod delegate_launch;
pub mod file_read;
pub mod file_write;
pub mod git_operations;
pub mod glob;
pub mod grep;
pub mod hardware_board_info;
pub mod hardware_memory_map;
pub mod hardware_memory_read;
pub mod http_common;
pub mod http_request;
pub mod image_info;
#[cfg(feature = "mcp-runtime")]
pub mod mcp;
pub mod memory_forget;
pub(crate) mod memory_helpers;
pub(crate) mod security_helpers;
pub mod memory_recall;
pub mod memory_store;
#[cfg(feature = "pdf-inspect")]
pub mod pdf_inspect;
pub mod pushover;
pub mod schedule;
pub mod schema;
pub mod screenshot;
pub mod shell;
pub mod task_create;
pub mod task_get;
pub mod task_list;
pub mod task_stop;
pub mod task_update;
pub mod traits;
pub(crate) mod url_safety;
pub mod web_fetch;
pub mod web_search_tool;

pub const PARITY_TOOL_ALIASES: &[(&str, &str)] = &[
    ("Glob", "glob"),
    ("Grep", "grep"),
    ("WebFetch", "web_fetch"),
    ("TaskCreate", "task_create"),
    ("TaskGet", "task_get"),
    ("TaskList", "task_list"),
    ("TaskUpdate", "task_update"),
    ("TaskStop", "task_stop"),
];

pub fn parity_alias_for(canonical_name: &str) -> Option<&'static str> {
    PARITY_TOOL_ALIASES
        .iter()
        .find_map(|(canonical, alias)| (*canonical == canonical_name).then_some(*alias))
}

pub fn canonical_tool_name_for_alias(name: &str) -> &str {
    PARITY_TOOL_ALIASES
        .iter()
        .find_map(|(canonical, alias)| (*alias == name).then_some(*canonical))
        .unwrap_or(name)
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::memory::SqliteMemory;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use crate::tasks::TaskService;
    use std::sync::Arc;
    use tempfile::TempDir;

    pub(crate) fn task_tool_test_context() -> (TempDir, Arc<SecurityPolicy>, Arc<TaskService>) {
        let dir = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: dir.path().to_path_buf(),
            ..SecurityPolicy::default()
        });
        let memory = Arc::new(SqliteMemory::new(dir.path()).unwrap());
        let service = Arc::new(TaskService::new(memory));
        (dir, security, service)
    }
}

pub use browser::{BrowserTool, ComputerUseConfig};
pub use browser_open::BrowserOpenTool;
pub use code_search::CodeSearchTool;
pub use composio::ComposioTool;
pub use cron_add::CronAddTool;
pub use cron_list::CronListTool;
pub use cron_remove::CronRemoveTool;
pub use cron_run::CronRunTool;
pub use cron_runs::CronRunsTool;
pub use cron_update::CronUpdateTool;
pub use delegate::DelegateTool;
pub use delegate_cancel::DelegateCancelTool;
pub use delegate_inspect::DelegateInspectTool;
pub use delegate_launch::DelegateLaunchTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use git_operations::GitOperationsTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use hardware_board_info::HardwareBoardInfoTool;
pub use hardware_memory_map::HardwareMemoryMapTool;
pub use hardware_memory_read::HardwareMemoryReadTool;
pub use http_request::HttpRequestTool;
pub use image_info::ImageInfoTool;
pub use memory_forget::MemoryForgetTool;
pub use memory_recall::MemoryRecallTool;
pub use memory_store::MemoryStoreTool;
#[cfg(feature = "pdf-inspect")]
pub use pdf_inspect::PdfInspectTool;
pub use pushover::PushoverTool;
pub use schedule::ScheduleTool;
#[allow(unused_imports)]
pub use schema::{CleaningStrategy, SchemaCleanr};
pub use screenshot::ScreenshotTool;
pub use shell::ShellTool;
pub use task_create::TaskCreateTool;
pub use task_get::TaskGetTool;
pub use task_list::TaskListTool;
pub use task_stop::TaskStopTool;
pub use task_update::TaskUpdateTool;
pub use traits::Tool;
#[allow(unused_imports)]
pub use traits::{ToolResult, ToolSpec};
pub use web_fetch::WebFetchTool;
pub use web_search_tool::WebSearchTool;

use crate::agent::coordinator::SupervisedOrchestrationService;
use crate::agent::mailbox::{MailboxBackedChildRunner, MailboxWakeupHub, SqliteMailboxStore};
use crate::config::{Config, DelegateAgentConfig};
use crate::memory::Memory;
use crate::runtime::{NativeRuntime, RuntimeAdapter};
use crate::security::SecurityPolicy;
use crate::tasks::TaskService;
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
pub fn default_tools(
    security: Arc<SecurityPolicy>,
    sandbox: Arc<dyn crate::security::Sandbox>,
) -> Vec<Box<dyn Tool>> {
    default_tools_with_runtime(security, Arc::new(NativeRuntime::new()), sandbox)
}

/// Create the default tool registry with explicit runtime adapter.
pub fn default_tools_with_runtime(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    sandbox: Arc<dyn crate::security::Sandbox>,
) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ShellTool::new(security.clone(), runtime, sandbox)),
        Box::new(CodeSearchTool::new(security.clone())),
        Box::new(FileReadTool::new(security.clone())),
        Box::new(FileWriteTool::new(security)),
    ]
}

fn add_browser_tools(
    tools: &mut Vec<Box<dyn Tool>>,
    security: &Arc<SecurityPolicy>,
    browser_config: &crate::config::BrowserConfig,
    sidecar_verification_required: bool,
) {
    if !browser_config.enabled {
        return;
    }

    tools.push(Box::new(BrowserOpenTool::new(
        security.clone(),
        browser_config.allowed_domains.clone(),
    )));
    tools.push(Box::new(
        BrowserTool::new_with_backend(
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
        )
        .with_sidecar_verification_required(sidecar_verification_required),
    ));
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
    tools.push(Box::new(WebFetchTool::new(
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
    workspace_dir: &std::path::Path,
) {
    if agents.is_empty() {
        return;
    }

    let delegate_agents: HashMap<String, DelegateAgentConfig> = agents.clone();
    let delegate_fallback_credential = fallback_api_key.and_then(|value| {
        let trimmed_value = value.trim();
        (!trimmed_value.is_empty()).then(|| trimmed_value.to_owned())
    });

    let service = Arc::new(SupervisedOrchestrationService::new());
    let mailbox_store = match SqliteMailboxStore::from_db_path(SqliteMailboxStore::default_db_path(
        workspace_dir,
    )) {
        Ok(store) => Arc::new(store),
        Err(error) => {
            tracing::warn!(error = %error, "delegate tools disabled: mailbox init failed closed");
            return;
        }
    };
    let wakeups = Arc::new(MailboxWakeupHub::default());
    let delegated_runner: Arc<dyn crate::agent::coordinator::CoordinatorChildRunner> =
        Arc::new(crate::agent::coordinator::DelegatedAgentRunner::new(
            base_config.clone(),
            Arc::new(agents.clone()),
            delegate_fallback_credential.clone(),
        ));
    let mailbox_runner: Arc<dyn crate::agent::coordinator::CoordinatorChildRunner> = Arc::new(
        MailboxBackedChildRunner::new(mailbox_store, delegated_runner, wakeups),
    );

    tools.push(Box::new(DelegateTool::with_supervised_executor(
        delegate_agents,
        delegate_fallback_credential.clone(),
        security.clone(),
        base_config.clone(),
        service.clone(),
        mailbox_runner.clone(),
    )));

    tools.push(Box::new(DelegateLaunchTool::new(
        service.clone(),
        mailbox_runner,
    )));
    tools.push(Box::new(DelegateCancelTool::new(service.clone())));
    tools.push(Box::new(DelegateInspectTool::new(service)));
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
    sandbox: Arc<dyn crate::security::Sandbox>,
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
        sandbox,
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
    sandbox: Arc<dyn crate::security::Sandbox>,
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
        Box::new(ShellTool::new(security.clone(), runtime, sandbox)),
        Box::new(CodeSearchTool::new(security.clone())),
        Box::new(GlobTool::new(security.clone())),
        Box::new(GrepTool::new(security.clone())),
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

        if memory.name() == "sqlite" {
            let task_service = Arc::new(TaskService::new(memory.clone()));
            tools.push(Box::new(TaskCreateTool::new(
                security.clone(),
                task_service.clone(),
            )));
            tools.push(Box::new(TaskGetTool::new(
                security.clone(),
                task_service.clone(),
            )));
            tools.push(Box::new(TaskListTool::new(
                security.clone(),
                task_service.clone(),
            )));
            tools.push(Box::new(TaskUpdateTool::new(
                security.clone(),
                task_service.clone(),
            )));
            tools.push(Box::new(TaskStopTool::new(security.clone(), task_service)));
        }
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

    add_browser_tools(
        &mut tools,
        security,
        browser_config,
        root_config.security.sandbox.require,
    );
    add_http_request_tool(&mut tools, security, http_config);
    add_web_search_tool(&mut tools, security, root_config);

    // Vision tools are always available
    tools.push(Box::new(ScreenshotTool::new(security.clone())));
    tools.push(Box::new(ImageInfoTool::new(security.clone())));

    // PDF inspection (optional feature)
    #[cfg(feature = "pdf-inspect")]
    tools.push(Box::new(PdfInspectTool::new(security.clone())));

    add_composio_tool(&mut tools, security, composio_key, composio_entity_id);

    add_delegate_tool(
        &mut tools,
        security,
        agents,
        fallback_api_key,
        config.clone(),
        workspace_dir,
    );

    #[cfg(feature = "mcp-runtime")]
    extend_with_mcp_tools(&mut tools, root_config);

    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BrowserConfig, Config, DelegateExecutionMode, McpConfig, MemoryConfig};
    use crate::security::{NoopSandbox, Sandbox};
    use crate::test_support::{mock_mcp_server, test_config};
    use tempfile::TempDir;

    fn test_sandbox() -> Arc<dyn Sandbox> {
        Arc::new(NoopSandbox)
    }

    #[test]
    fn default_tools_has_four() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security, test_sandbox());
        assert_eq!(tools.len(), 4);
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
            test_sandbox(),
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
            test_sandbox(),
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
        let tools = default_tools(security, test_sandbox());
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"code_search"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
    }

    #[test]
    fn all_tools_registers_task_tools_only_for_sqlite_memory() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());

        let sqlite_cfg = MemoryConfig {
            backend: "sqlite".into(),
            ..MemoryConfig::default()
        };
        let sqlite_mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&sqlite_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let sqlite_tools = all_tools(
            Arc::new(Config::default()),
            &security,
            test_sandbox(),
            sqlite_mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );
        let sqlite_names: Vec<&str> = sqlite_tools.iter().map(|tool| tool.name()).collect();
        assert!(sqlite_names.contains(&"TaskCreate"));
        assert!(sqlite_names.contains(&"TaskGet"));
        assert!(sqlite_names.contains(&"TaskList"));
        assert!(sqlite_names.contains(&"TaskUpdate"));
        assert!(sqlite_names.contains(&"TaskStop"));

        let markdown_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let markdown_mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&markdown_cfg, tmp.path(), None).unwrap());
        let markdown_tools = all_tools(
            Arc::new(Config::default()),
            &security,
            test_sandbox(),
            markdown_mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
        );
        let markdown_names: Vec<&str> = markdown_tools.iter().map(|tool| tool.name()).collect();
        assert!(!markdown_names.contains(&"TaskCreate"));
        assert!(!markdown_names.contains(&"TaskList"));
    }

    #[test]
    fn default_tools_all_have_descriptions() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security, test_sandbox());
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
        let tools = default_tools(security, test_sandbox());
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
        let tools = default_tools(security, test_sandbox());
        for tool in &tools {
            let spec = tool.spec();
            assert_eq!(spec.name, tool.name());
            assert_eq!(spec.description, tool.description());
            assert!(spec.parameters.is_object());
        }
    }

    #[test]
    fn parity_alias_mapping_round_trips() {
        assert_eq!(parity_alias_for("Glob"), Some("glob"));
        assert_eq!(parity_alias_for("TaskUpdate"), Some("task_update"));
        assert_eq!(canonical_tool_name_for_alias("glob"), "Glob");
        assert_eq!(canonical_tool_name_for_alias("task_update"), "TaskUpdate");
        assert_eq!(
            canonical_tool_name_for_alias("unknown_tool"),
            "unknown_tool"
        );
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
            aliases: vec!["test_alias".into()],
        };
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: ToolSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.description, "A test tool");
        assert_eq!(parsed.aliases, vec!["test_alias"]);
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
            test_sandbox(),
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
            test_sandbox(),
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
            test_sandbox(),
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
            test_sandbox(),
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
            test_sandbox(),
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
