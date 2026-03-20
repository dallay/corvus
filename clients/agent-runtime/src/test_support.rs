use crate::config::{Config, McpServerConfig};
use std::collections::BTreeMap;
use tempfile::TempDir;

#[cfg(test)]
const GATEWAY_WEBHOOK_DISPATCHER_ENV_VAR: &str = "CORVUS_GATEWAY_WEBHOOK_DISPATCHER";

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

#[cfg(test)]
fn gateway_webhook_dispatcher_env_mutex() -> &'static tokio::sync::Mutex<()> {
    static GATEWAY_WEBHOOK_DISPATCHER_ENV_MUTEX: std::sync::LazyLock<tokio::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

    &GATEWAY_WEBHOOK_DISPATCHER_ENV_MUTEX
}

#[cfg(test)]
pub(crate) struct GatewayWebhookDispatcherEnvGuard {
    _lock: tokio::sync::MutexGuard<'static, ()>,
    previous: Option<String>,
}

#[cfg(test)]
impl GatewayWebhookDispatcherEnvGuard {
    pub(crate) async fn set(value: &'static str) -> Self {
        let lock = gateway_webhook_dispatcher_env_mutex().lock().await;
        Self::set_with_lock(lock, value)
    }

    pub(crate) fn set_blocking(value: &'static str) -> Self {
        let lock = gateway_webhook_dispatcher_env_mutex().blocking_lock();
        Self::set_with_lock(lock, value)
    }

    fn set_with_lock(lock: tokio::sync::MutexGuard<'static, ()>, value: &'static str) -> Self {
        let previous = std::env::var(GATEWAY_WEBHOOK_DISPATCHER_ENV_VAR).ok();
        std::env::set_var(GATEWAY_WEBHOOK_DISPATCHER_ENV_VAR, value);
        Self {
            _lock: lock,
            previous,
        }
    }
}

#[cfg(test)]
impl Drop for GatewayWebhookDispatcherEnvGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.as_deref() {
            std::env::set_var(GATEWAY_WEBHOOK_DISPATCHER_ENV_VAR, previous);
        } else {
            std::env::remove_var(GATEWAY_WEBHOOK_DISPATCHER_ENV_VAR);
        }
    }
}
