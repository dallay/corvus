use crate::bootstrap::BootstrapContext;
use crate::capabilities::build_registry_from_tools;
use crate::config::{Config, SandboxBackend};
use crate::cost::CostTracker;
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::Provider;
use crate::runtime::{self, RuntimeAdapter};
use crate::security::SecurityPolicy;
use crate::tools::Tool;
use anyhow::{anyhow, Context, Result};
use corvus_composer::ComposedRuntimePlan;
use std::sync::Arc;

pub fn bootstrap_from_plan(
    base_config: &Config,
    plan: &ComposedRuntimePlan,
) -> Result<(BootstrapContext, Box<dyn Provider>)> {
    let config = config_from_plan(base_config, plan)?;

    build_bootstrap_and_provider(&config, plan)
}

fn config_from_plan(base_config: &Config, plan: &ComposedRuntimePlan) -> Result<Config> {
    let mut config = base_config.clone();
    config.default_provider = Some(plan.provider.key.clone());
    if let Some(model) = &plan.agent.model {
        config.default_model = Some(model.clone());
    }
    if let Some(temperature) = plan.agent.temperature {
        config.default_temperature = temperature;
    }
    if let Some(profile) = &plan.runtime.profile {
        config.agent.profile = profile.clone();
    }
    if let Some(max_tool_iterations) = plan.runtime.max_tool_iterations {
        config.agent.max_tool_iterations = max_tool_iterations;
    }
    // First check memory_settings.auto_save (from resolved plan), fallback to config
    if let Some(auto_save) = plan.memory_settings.auto_save {
        config.memory.auto_save = auto_save;
    } else if let Some(auto_save) = plan
        .memory
        .config
        .as_ref()
        .and_then(|value| value.get("auto_save"))
        .and_then(toml::Value::as_bool)
    {
        config.memory.auto_save = auto_save;
    }
    config.memory.backend = plan.memory.key.clone();

    // Validate and apply observers - only single observer supported for now
    if plan.observers.len() > 1 {
        return Err(anyhow!(
            "multiple observers not supported: found {} ({:?}), expected 1",
            plan.observers.len(),
            plan.observers
                .iter()
                .map(|o| o.key.clone())
                .collect::<Vec<_>>()
        ));
    }
    if let Some(observer) = plan.observers.first() {
        config.observability.backend = observer.key.clone();
        if let Some(observer_config) = &observer.config {
            if let Some(endpoint) = observer_config
                .get("otel_endpoint")
                .and_then(toml::Value::as_str)
            {
                config.observability.otel_endpoint = Some(endpoint.to_string());
            }
            if let Some(service_name) = observer_config
                .get("otel_service_name")
                .and_then(toml::Value::as_str)
            {
                config.observability.otel_service_name = Some(service_name.to_string());
            }
        }
    }

    // Validate and apply security backend - fail on unknown keys instead of silently defaulting
    config.security.sandbox.backend = match plan.security.key.as_str() {
        "landlock" => SandboxBackend::Landlock,
        "firejail" => SandboxBackend::Firejail,
        "bubblewrap" => SandboxBackend::Bubblewrap,
        "docker" => SandboxBackend::Docker,
        "none" => SandboxBackend::None,
        unknown => {
            return Err(anyhow!(
                "unknown security backend: '{}', valid options are: landlock, firejail, bubblewrap, docker, none",
                unknown
            ))
        }
    };
    if let Some(require) = plan
        .security
        .config
        .as_ref()
        .and_then(|value| value.get("require"))
        .and_then(toml::Value::as_bool)
    {
        config.security.sandbox.require = require;
    }
    if let Some(format) = &plan.identity.format {
        config.identity.format = format.clone();
    }
    config.identity.aieos_path = plan.identity.aieos_path.clone();
    config.identity.aieos_inline = plan.identity.aieos_inline.clone();

    Ok(config)
}

fn build_bootstrap_and_provider(
    config: &Config,
    plan: &ComposedRuntimePlan,
) -> Result<(BootstrapContext, Box<dyn Provider>)> {
    let sandbox = crate::security::create_sandbox(&config.security)?;
    let observer: Arc<dyn Observer> =
        Arc::from(crate::observability::create_observer(&config.observability));
    let memory: Arc<dyn Memory> = Arc::from(crate::memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    let runtime: Arc<dyn RuntimeAdapter> = Arc::from(runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
        config.agent.execution_mode,
    ));
    let (composio_key, composio_entity_id) = if config.composio.enabled {
        (
            config.composio.api_key.as_deref(),
            Some(config.composio.entity_id.as_str()),
        )
    } else {
        (None, None)
    };
    let all_tools = crate::tools::all_tools_with_runtime(
        Arc::new(config.clone()),
        &security,
        Arc::clone(&runtime),
        sandbox,
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
    let selected_tool_keys: Vec<String> = plan.tools.iter().map(|tool| tool.key.clone()).collect();
    let tools: Vec<Box<dyn Tool>> = all_tools
        .into_iter()
        .filter(|tool| {
            selected_tool_keys
                .iter()
                .any(|selected| selected == tool.name())
        })
        .collect();
    if tools.len() != selected_tool_keys.len() {
        let actual: Vec<&str> = tools.iter().map(|tool| tool.name()).collect();
        return Err(anyhow!("manifest-selected tools could not all be materialized: requested {:?}, materialized {:?}", selected_tool_keys, actual));
    }

    let capability_registry = build_registry_from_tools(&tools)?;
    let cost_tracker = if config.cost.enabled {
        match CostTracker::new(config.cost.clone(), &config.workspace_dir) {
            Ok(tracker) => Some(Arc::new(tracker)),
            Err(error) => {
                tracing::warn!("Failed to initialize cost tracker: {error}");
                None
            }
        }
    } else {
        None
    };

    let provider_api_url = plan
        .provider
        .config
        .as_ref()
        .and_then(|value| value.get("api_url"))
        .and_then(toml::Value::as_str)
        .or(config.api_url.as_deref());
    let provider_api_key = plan
        .provider
        .config
        .as_ref()
        .and_then(|value| value.get("api_key"))
        .and_then(toml::Value::as_str)
        .or(config.api_key.as_deref());
    let provider = crate::providers::create_provider_with_url(
        &plan.provider.key,
        provider_api_key,
        provider_api_url,
    )
    .with_context(|| format!("failed to create composed provider '{}'", plan.provider.key))?;

    Ok((
        BootstrapContext {
            observer,
            runtime,
            security,
            memory,
            tools,
            capability_registry,
            cost_tracker,
        },
        provider,
    ))
}

pub fn agent_from_plan(
    base_config: &Config,
    plan: &ComposedRuntimePlan,
) -> Result<crate::agent::Agent> {
    let config = config_from_plan(base_config, plan)?;
    let (bootstrap, provider) = build_bootstrap_and_provider(&config, plan)?;
    crate::agent::Agent::from_bootstrap_with_provider(&config, bootstrap, provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::BootstrapContext;
    use crate::test_support::test_config;
    use corvus_composer::AgentComposer;

    fn lite_manifest() -> &'static str {
        r#"
version = "1"

[agent]
name = "lite-agent"
model = "anthropic/claude-sonnet-4"
profile = "lite"

[providers]
enabled = ["anthropic"]
default = "anthropic"

[channels]
enabled = ["stdio", "telegram"]
default = "stdio"

[tools]
enabled = ["shell", "file_read", "file_write"]

[memory]
backend = "none"

[observability]
enabled = ["none"]

[security]
backend = "none"
"#
    }

    #[test]
    fn composed_bootstrap_materializes_selected_components_only() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let config = test_config(&tempdir);
        let manifest = r#"
version = "1"

[agent]
name = "boot-agent"
model = "anthropic/claude-sonnet-4"
profile = "code"

[providers]
enabled = ["anthropic"]
default = "anthropic"

[channels]
enabled = ["stdio"]

[tools]
enabled = ["shell", "file_read"]

[memory]
backend = "markdown"

[observability]
enabled = ["none"]

[security]
backend = "none"
"#;
        let composer = AgentComposer::from_toml(manifest).unwrap();
        let (bootstrap, _provider) = bootstrap_from_plan(&config, composer.resolve_plan()).unwrap();

        assert_eq!(bootstrap.memory.name(), "markdown");
        let tool_names: Vec<&str> = bootstrap.tools.iter().map(|tool| tool.name()).collect();
        assert_eq!(tool_names, vec!["shell", "file_read"]);
    }

    #[test]
    fn composed_plan_preserves_channel_selection_for_future_runtime_wiring() {
        let composer = AgentComposer::from_toml(lite_manifest()).unwrap();
        let channel_keys: Vec<&str> = composer
            .resolve_plan()
            .channels
            .iter()
            .map(|channel| channel.key.as_str())
            .collect();

        assert_eq!(channel_keys, vec!["stdio", "telegram"]);
        assert_eq!(
            composer.resolve_plan().default_channel.as_deref(),
            Some("stdio")
        );
    }

    #[test]
    fn composed_bootstrap_matches_full_runtime_for_lite_profile_path() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&tempdir);
        config.agent.profile = "lite".into();
        config.default_provider = Some("anthropic".into());
        config.observability.backend = "none".into();
        config.memory.backend = "none".into();

        let composer = AgentComposer::from_toml(lite_manifest()).unwrap();
        let (composed_bootstrap, _provider) =
            bootstrap_from_plan(&config, composer.resolve_plan()).unwrap();
        let full_bootstrap = BootstrapContext::from_config(&config).unwrap();

        let composed_tool_names: Vec<&str> = composed_bootstrap
            .tools
            .iter()
            .map(|tool| tool.name())
            .collect();
        let full_tool_names: Vec<&str> = full_bootstrap
            .tools
            .iter()
            .map(|tool| tool.name())
            .collect();

        assert_eq!(
            composed_bootstrap.memory.name(),
            full_bootstrap.memory.name()
        );
        assert_eq!(
            composed_bootstrap.observer.name(),
            full_bootstrap.observer.name()
        );
        assert_eq!(composed_tool_names, full_tool_names);
    }
}
