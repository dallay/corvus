use crate::config::Config;
use crate::memory::{self, Memory};
use crate::observability::{self, Observer};
use crate::providers::{self, Provider, ProviderRuntimeOptions};
use crate::runtime::{self, RuntimeAdapter};
use crate::security::SecurityPolicy;
use crate::tools::{self, Tool};
use std::path::PathBuf;
use std::sync::Arc;

pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4";

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
) -> anyhow::Result<(Arc<dyn Memory>, Arc<dyn Observer>)> {
    let observer: Arc<dyn Observer> =
        Arc::from(observability::create_observer(&config.observability));
    let memory: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    Ok((memory, observer))
}

impl BootstrapContext {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let (memory, observer) = init_memory_and_observer(config)?;
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
    init_memory_and_observer(config)
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
    fn resilient_provider_uses_openrouter_when_default_provider_missing() {
        let mut config = Config::default();
        config.default_provider = None;

        assert_eq!(selected_provider_name(&config), "openrouter");
    }
}
