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
        capabilities: crate::config::default_mcp_capabilities(),
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    }
}

#[cfg(test)]
fn gateway_webhook_dispatcher_env_mutex() -> &'static std::sync::Mutex<()> {
    static GATEWAY_WEBHOOK_DISPATCHER_ENV_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

    &GATEWAY_WEBHOOK_DISPATCHER_ENV_MUTEX
}

#[cfg(test)]
pub(crate) fn acquire_gateway_webhook_dispatcher_lock() -> std::sync::MutexGuard<'static, ()> {
    gateway_webhook_dispatcher_env_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) fn acquire_gateway_webhook_dispatcher_lock_blocking(
) -> std::sync::MutexGuard<'static, ()> {
    gateway_webhook_dispatcher_env_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) struct GatewayWebhookDispatcherEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    previous: Option<String>,
}

#[cfg(test)]
impl GatewayWebhookDispatcherEnvGuard {
    #[allow(clippy::unused_async)]
    pub(crate) async fn set(value: &'static str) -> Self {
        let lock = acquire_gateway_webhook_dispatcher_lock();
        Self::set_with_lock(lock, value)
    }

    pub(crate) fn set_blocking(value: &'static str) -> Self {
        let lock = acquire_gateway_webhook_dispatcher_lock_blocking();
        Self::set_with_lock(lock, value)
    }

    fn set_with_lock(lock: std::sync::MutexGuard<'static, ()>, value: &'static str) -> Self {
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

// ---------------------------------------------------------------------------
// Tracing capture harness — shared across provider and router tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tracing_capture {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tracing::{field::Field, Event, Subscriber};
    use tracing_subscriber::field::Visit;
    use tracing_subscriber::{layer::Context, prelude::*, Layer};

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct CapturedTracingEvent {
        pub fields: BTreeMap<String, String>,
    }

    impl CapturedTracingEvent {
        pub fn field(&self, name: &str) -> Option<&str> {
            self.fields.get(name).map(String::as_str)
        }
    }

    #[derive(Clone, Default)]
    pub struct CaptureLayer {
        events: Arc<parking_lot::Mutex<Vec<CapturedTracingEvent>>>,
    }

    impl CaptureLayer {
        pub fn snapshot(&self) -> Vec<CapturedTracingEvent> {
            self.events.lock().clone()
        }
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = TracingFieldRecorder::default();
            event.record(&mut visitor);
            self.events.lock().push(CapturedTracingEvent {
                fields: visitor.fields,
            });
        }
    }

    #[derive(Default)]
    struct TracingFieldRecorder {
        fields: BTreeMap<String, String>,
    }

    impl TracingFieldRecorder {
        fn insert(&mut self, field: &Field, value: impl ToString) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }

    impl Visit for TracingFieldRecorder {
        fn record_bool(&mut self, field: &Field, value: bool) {
            self.insert(field, value);
        }

        fn record_i64(&mut self, field: &Field, value: i64) {
            self.insert(field, value);
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.insert(field, value);
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.insert(field, value);
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.insert(field, format!("{value:?}"));
        }
    }

    pub fn capture_tracing_events<T>(run: impl FnOnce() -> T) -> (T, Vec<CapturedTracingEvent>) {
        let layer = CaptureLayer::default();
        let subscriber = tracing_subscriber::registry().with(layer.clone());
        let _guard = tracing::subscriber::set_default(subscriber);
        let output = run();
        (output, layer.snapshot())
    }
}
