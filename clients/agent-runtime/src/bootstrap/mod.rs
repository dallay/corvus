use crate::config::Config;
use crate::memory::{self, Memory};
use crate::observability::{self, Observer};
use crate::providers::{self, Provider, ProviderRuntimeOptions};
use crate::runtime::{self, RuntimeAdapter};
use crate::security::SecurityPolicy;
use crate::tools::{self, Tool};
use anyhow::bail;
use std::path::PathBuf;
use std::sync::Arc;

pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentProfile {
    Full,
    Code,
    Lite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolCapability {
    Lite,
    Code,
    FullOnly,
    Mcp,
}

const LITE_TOOL_ALLOWLIST: &[&str] = &["shell", "file_read", "file_write"];

const CODE_TOOL_ALLOWLIST: &[&str] = &[
    "browser",
    "browser_open",
    "delegate",
    "file_read",
    "file_write",
    "git_operations",
    "http_request",
    "image_info",
    "memory_forget",
    "memory_recall",
    "memory_store",
    "screenshot",
    "shell",
    "web_search_tool",
];

const FULL_ONLY_TOOL_ALLOWLIST: &[&str] = &[
    "composio",
    "cron_add",
    "cron_list",
    "cron_remove",
    "cron_run",
    "cron_runs",
    "cron_update",
    "hardware_board_info",
    "hardware_memory_map",
    "hardware_memory_read",
    "pushover",
    "schedule",
];

fn classify_tool_capability(tool_name: &str) -> Option<ToolCapability> {
    if tool_name.starts_with("mcp.") {
        return Some(ToolCapability::Mcp);
    }

    if LITE_TOOL_ALLOWLIST.contains(&tool_name) {
        return Some(ToolCapability::Lite);
    }

    if CODE_TOOL_ALLOWLIST.contains(&tool_name) {
        return Some(ToolCapability::Code);
    }

    if FULL_ONLY_TOOL_ALLOWLIST.contains(&tool_name) {
        return Some(ToolCapability::FullOnly);
    }

    None
}

impl AgentProfile {
    fn from_config(config: &Config) -> anyhow::Result<Self> {
        Self::from_raw(config.agent.profile.as_str())
    }

    fn from_raw(raw: &str) -> anyhow::Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "full" => Ok(Self::Full),
            "code" => Ok(Self::Code),
            "lite" => Ok(Self::Lite),
            other => {
                bail!("unsupported agent.profile '{other}'; supported values are: full, code, lite")
            }
        }
    }

    fn compose_memory_config(
        self,
        config: &crate::config::MemoryConfig,
    ) -> crate::config::MemoryConfig {
        let mut composed = config.clone();
        if matches!(self, Self::Lite) {
            composed.backend = "none".into();
        }
        composed
    }

    fn allows_tool(self, tool_name: &str) -> bool {
        match self {
            Self::Full => true,
            Self::Code => matches!(
                classify_tool_capability(tool_name),
                Some(ToolCapability::Lite | ToolCapability::Code | ToolCapability::Mcp)
            ),
            Self::Lite => {
                matches!(
                    classify_tool_capability(tool_name),
                    Some(ToolCapability::Lite)
                )
            }
        }
    }
}

pub struct BootstrapContext {
    pub observer: Arc<dyn Observer>,
    pub runtime: Arc<dyn RuntimeAdapter>,
    pub security: Arc<SecurityPolicy>,
    pub memory: Arc<dyn Memory>,
    pub tools: Vec<Box<dyn Tool>>,
}

fn selected_provider_name(config: &Config) -> &str {
    config.default_provider.as_deref().unwrap_or("openrouter")
}

fn provider_runtime_options(config: &Config) -> ProviderRuntimeOptions {
    ProviderRuntimeOptions {
        auth_profile_override: None,
        corvus_dir: config.config_path.parent().map(PathBuf::from),
        secrets_encrypt: config.secrets.encrypt,
    }
}

fn init_memory_and_observer(
    config: &Config,
    profile: AgentProfile,
) -> anyhow::Result<(Arc<dyn Memory>, Arc<dyn Observer>)> {
    init_memory_and_observer_with_overrides(config, profile, None, None)
}

fn init_memory_and_observer_with_overrides(
    config: &Config,
    profile: AgentProfile,
    memory_override: Option<Arc<dyn Memory>>,
    observer_override: Option<Arc<dyn Observer>>,
) -> anyhow::Result<(Arc<dyn Memory>, Arc<dyn Observer>)> {
    let observer: Arc<dyn Observer> = observer_override
        .unwrap_or_else(|| Arc::from(observability::create_observer(&config.observability)));
    let memory: Arc<dyn Memory> = if let Some(memory) = memory_override {
        memory
    } else {
        let memory_config = profile.compose_memory_config(&config.memory);
        Arc::from(memory::create_memory(
            &memory_config,
            &config.workspace_dir,
            config.api_key.as_deref(),
        )?)
    };

    Ok((memory, observer))
}

impl BootstrapContext {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        Self::from_config_with_profile(config, config.agent.profile.as_str())
    }

    pub(crate) fn from_config_with_profile(config: &Config, profile: &str) -> anyhow::Result<Self> {
        let mut effective_config = config.clone();
        effective_config.agent.profile = profile.to_string();

        Self::from_effective_config(&effective_config)
    }

    pub(crate) fn for_gateway(
        config: &Config,
        memory: Arc<dyn Memory>,
        observer: Arc<dyn Observer>,
    ) -> anyhow::Result<Self> {
        Self::from_effective_config_with_overrides(config, Some(memory), Some(observer))
    }

    fn from_effective_config(config: &Config) -> anyhow::Result<Self> {
        Self::from_effective_config_with_overrides(config, None, None)
    }

    fn from_effective_config_with_overrides(
        config: &Config,
        memory_override: Option<Arc<dyn Memory>>,
        observer_override: Option<Arc<dyn Observer>>,
    ) -> anyhow::Result<Self> {
        let profile = AgentProfile::from_config(config)?;
        let (memory, observer) = init_memory_and_observer_with_overrides(
            config,
            profile,
            memory_override,
            observer_override,
        )?;
        let runtime: Arc<dyn RuntimeAdapter> = Arc::from(runtime::create_runtime(&config.runtime)?);
        let security = Arc::new(SecurityPolicy::from_config(
            &config.autonomy,
            &config.workspace_dir,
        ));

        let (composio_key, composio_entity_id) = if config.composio.enabled {
            (
                config.composio.api_key.as_deref(),
                Some(config.composio.entity_id.as_str()),
            )
        } else {
            (None, None)
        };

        let tools = tools::all_tools_with_runtime(
            Arc::new(config.clone()),
            &security,
            Arc::clone(&runtime),
            Arc::clone(&memory),
            composio_key,
            composio_entity_id,
            &config.browser,
            &config.http_request,
            &config.workspace_dir,
            &config.agents,
            config.api_key.as_deref(),
            config,
        );
        let tools = tools
            .into_iter()
            .filter(|tool| profile.allows_tool(tool.name()))
            .collect();

        Ok(Self {
            observer,
            runtime,
            security,
            memory,
            tools,
        })
    }
}

pub fn create_resilient_provider(config: &Config) -> anyhow::Result<Arc<dyn Provider>> {
    let provider_name = selected_provider_name(config);

    Ok(Arc::from(
        providers::create_resilient_provider_with_options(
            provider_name,
            config.api_key.as_deref(),
            config.api_url.as_deref(),
            &config.reliability,
            &provider_runtime_options(config),
        )?,
    ))
}

pub fn create_routed_provider(
    config: &Config,
    default_model: &str,
) -> anyhow::Result<Box<dyn Provider>> {
    let provider_name = selected_provider_name(config);

    providers::create_routed_provider(
        provider_name,
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &config.model_routes,
        default_model,
        &provider_runtime_options(config),
    )
}

pub fn create_memory_and_observer(
    config: &Config,
) -> anyhow::Result<(Arc<dyn Memory>, Arc<dyn Observer>)> {
    let profile = AgentProfile::from_config(config)?;
    init_memory_and_observer(config, profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DelegateAgentConfig, DelegateExecutionMode};
    use crate::test_support::{mock_mcp_server, test_config};
    use std::collections::HashSet;

    struct BootstrapMatrixCase {
        name: &'static str,
        profile: &'static str,
        memory_backend: &'static str,
        enable_mcp: bool,
        expect_memory_name: &'static str,
        expected_present: &'static [&'static str],
        expected_absent: &'static [&'static str],
    }

    #[test]
    fn bootstrap_context_builds_core_components() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = test_config(&tmp);

        let ctx = BootstrapContext::from_config(&config).unwrap();

        assert!(!ctx.tools.is_empty());
        assert!(!ctx.memory.name().is_empty());
        assert!(!ctx.runtime.name().is_empty());
    }

    #[test]
    fn bootstrap_code_profile_excludes_non_coding_tools() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.agent.profile = "code".into();

        let ctx = BootstrapContext::from_config(&config).unwrap();
        let names: Vec<&str> = ctx.tools.iter().map(|tool| tool.name()).collect();

        assert!(names.contains(&"shell"));
        assert!(names.contains(&"git_operations"));
        assert!(!names.contains(&"pushover"));
        assert!(!names.contains(&"cron_add"));
    }

    #[test]
    fn bootstrap_lite_profile_uses_minimal_tools_and_none_memory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.agent.profile = "lite".into();

        let ctx = BootstrapContext::from_config(&config).unwrap();
        let names: Vec<&str> = ctx.tools.iter().map(|tool| tool.name()).collect();
        let actual: HashSet<&str> = names.iter().copied().collect();
        let expected: HashSet<&str> = ["shell", "file_read", "file_write"].into_iter().collect();

        assert_eq!(ctx.memory.name(), "none");
        assert_eq!(actual, expected);
    }

    #[test]
    fn bootstrap_rejects_unknown_profile() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.agent.profile = "unknown".into();

        let error = BootstrapContext::from_config(&config).err().unwrap();
        assert!(error.to_string().contains("unsupported agent.profile"));
    }

    #[test]
    fn code_and_lite_profiles_explicitly_classify_registered_tools() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.browser.enabled = true;
        config.http_request.enabled = true;
        config.web_search.enabled = true;
        config.agents.insert(
            "delegate-worker".into(),
            DelegateAgentConfig {
                provider: "openrouter".into(),
                model: "test-model".into(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                execution_mode: DelegateExecutionMode::default(),
                max_iterations: None,
                timeout_ms: None,
            },
        );

        let security = Arc::new(SecurityPolicy::from_config(
            &config.autonomy,
            &config.workspace_dir,
        ));
        let runtime: Arc<dyn RuntimeAdapter> =
            Arc::from(runtime::create_runtime(&config.runtime).unwrap());
        let memory_cfg = crate::config::MemoryConfig {
            backend: "none".into(),
            ..crate::config::MemoryConfig::default()
        };
        let memory: Arc<dyn Memory> = Arc::from(
            memory::create_memory(
                &memory_cfg,
                &config.workspace_dir,
                config.api_key.as_deref(),
            )
            .unwrap(),
        );

        let tools = tools::all_tools_with_runtime(
            Arc::new(config.clone()),
            &security,
            runtime,
            memory,
            Some("composio-test-key"),
            Some("composio-test-entity"),
            &config.browser,
            &config.http_request,
            &config.workspace_dir,
            &config.agents,
            config.api_key.as_deref(),
            &config,
        );

        let unclassified: Vec<String> = tools
            .iter()
            .map(|tool| tool.name())
            .filter(|tool_name| classify_tool_capability(tool_name).is_none())
            .map(ToString::to_string)
            .collect();

        assert!(
            unclassified.is_empty(),
            "found unclassified tools; assign each tool to lite/code/full: {}",
            unclassified.join(", ")
        );
    }

    #[test]
    fn create_memory_and_observer_respects_lite_profile_memory_backend() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.agent.profile = "lite".into();

        let (memory, _observer) = create_memory_and_observer(&config).unwrap();

        assert_eq!(memory.name(), "none");
    }

    #[test]
    fn resilient_provider_uses_openrouter_when_default_provider_missing() {
        let mut config = Config::default();
        config.default_provider = None;

        assert_eq!(selected_provider_name(&config), "openrouter");
    }

    #[test]
    fn bootstrap_feature_flag_matrix_reports_expected_assembly() {
        let expected_mcp_tool = if cfg!(feature = "mcp-runtime") {
            Some("mcp.docs.search")
        } else {
            None
        };
        let cases = [
            BootstrapMatrixCase {
                name: "baseline-full",
                profile: "full",
                memory_backend: "sqlite",
                enable_mcp: false,
                expect_memory_name: "sqlite",
                expected_present: &["shell", "git_operations", "memory_store", "schedule"],
                expected_absent: &["mcp.docs.search"],
            },
            BootstrapMatrixCase {
                name: "full-with-mcp",
                profile: "full",
                memory_backend: "sqlite",
                enable_mcp: true,
                expect_memory_name: "sqlite",
                expected_present: &["shell", "git_operations"],
                expected_absent: &[],
            },
            BootstrapMatrixCase {
                name: "code-with-mcp",
                profile: "code",
                memory_backend: "sqlite",
                enable_mcp: true,
                expect_memory_name: "sqlite",
                expected_present: &["shell", "git_operations"],
                expected_absent: &["schedule", "pushover", "cron_add"],
            },
            BootstrapMatrixCase {
                name: "lite-overrides-memory-backend",
                profile: "lite",
                memory_backend: "markdown",
                enable_mcp: true,
                expect_memory_name: "none",
                expected_present: &["shell", "file_read", "file_write"],
                expected_absent: &[
                    "git_operations",
                    "memory_store",
                    "schedule",
                    "mcp.docs.search",
                ],
            },
        ];

        for case in cases {
            let tmp = tempfile::TempDir::new().unwrap();
            let mut config = test_config(&tmp);
            config.agent.profile = case.profile.into();
            config.memory.backend = case.memory_backend.into();

            if case.enable_mcp {
                config.mcp.enabled = true;
                config.mcp.servers = vec![mock_mcp_server("docs", "search")];
            }

            let ctx = BootstrapContext::from_config(&config)
                .unwrap_or_else(|error| panic!("matrix case '{}' failed: {error:#}", case.name));
            let names: HashSet<&str> = ctx.tools.iter().map(|tool| tool.name()).collect();

            assert_eq!(
                ctx.memory.name(),
                case.expect_memory_name,
                "matrix case '{}' produced unexpected memory backend",
                case.name
            );
            assert!(
                !ctx.runtime.name().is_empty(),
                "matrix case '{}' produced empty runtime",
                case.name
            );

            for tool_name in case.expected_present {
                assert!(
                    names.contains(tool_name),
                    "matrix case '{}' expected tool '{}' but saw {:?}",
                    case.name,
                    tool_name,
                    names
                );
            }

            for tool_name in case.expected_absent {
                assert!(
                    !names.contains(tool_name),
                    "matrix case '{}' unexpectedly included tool '{}' with {:?}",
                    case.name,
                    tool_name,
                    names
                );
            }

            match expected_mcp_tool {
                Some(tool_name) if case.enable_mcp && case.profile != "lite" => assert!(
                    names.contains(tool_name),
                    "matrix case '{}' expected MCP tool '{}' when feature enabled",
                    case.name,
                    tool_name
                ),
                Some(tool_name) => assert!(
                    !names.contains(tool_name),
                    "matrix case '{}' should not expose MCP tool '{}'",
                    case.name,
                    tool_name
                ),
                None => assert!(
                    !names.iter().any(|name| name.starts_with("mcp.")),
                    "matrix case '{}' should not expose MCP tools without feature",
                    case.name
                ),
            }
        }
    }

    #[test]
    fn gateway_bootstrap_reuses_canonical_mcp_tool_registry() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.mcp.enabled = true;
        config.mcp.servers = vec![mock_mcp_server("docs", "search")];

        let (memory, observer) = create_memory_and_observer(&config).unwrap();
        let ctx = BootstrapContext::for_gateway(&config, memory, observer).unwrap();
        let names: HashSet<&str> = ctx.tools.iter().map(|tool| tool.name()).collect();

        if cfg!(feature = "mcp-runtime") {
            assert!(names.contains("mcp.docs.search"));
        } else {
            assert!(!names.iter().any(|name| name.starts_with("mcp.")));
        }
    }
}
