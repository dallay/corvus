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
            Self::Code => {
                if tool_name.starts_with("mcp.") {
                    return true;
                }

                !matches!(
                    tool_name,
                    "cron_add"
                        | "cron_list"
                        | "cron_remove"
                        | "cron_run"
                        | "cron_runs"
                        | "cron_update"
                        | "schedule"
                        | "pushover"
                        | "composio"
                        | "hardware_board_info"
                        | "hardware_memory_map"
                        | "hardware_memory_read"
                )
            }
            Self::Lite => matches!(tool_name, "shell" | "file_read" | "file_write"),
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
    let observer: Arc<dyn Observer> =
        Arc::from(observability::create_observer(&config.observability));
    let memory_config = profile.compose_memory_config(&config.memory);
    let memory: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &memory_config,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    Ok((memory, observer))
}

impl BootstrapContext {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let profile = AgentProfile::from_config(config)?;
        let (memory, observer) = init_memory_and_observer(config, profile)?;
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

    #[test]
    fn bootstrap_context_builds_core_components() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");

        let ctx = BootstrapContext::from_config(&config).unwrap();

        assert!(!ctx.tools.is_empty());
        assert!(!ctx.memory.name().is_empty());
        assert!(!ctx.runtime.name().is_empty());
    }

    #[test]
    fn bootstrap_code_profile_excludes_non_coding_tools() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
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
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
        config.agent.profile = "lite".into();

        let ctx = BootstrapContext::from_config(&config).unwrap();
        let names: Vec<&str> = ctx.tools.iter().map(|tool| tool.name()).collect();

        assert_eq!(ctx.memory.name(), "none");
        assert_eq!(names, vec!["shell", "file_read", "file_write"]);
    }

    #[test]
    fn bootstrap_rejects_unknown_profile() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
        config.agent.profile = "unknown".into();

        let error = BootstrapContext::from_config(&config).err().unwrap();
        assert!(error.to_string().contains("unsupported agent.profile"));
    }

    #[test]
    fn create_memory_and_observer_respects_lite_profile_memory_backend() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.toml");
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
}
