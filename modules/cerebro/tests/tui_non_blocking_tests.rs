use serde_json::json;
#[cfg(feature = "tui")]
use std::sync::Mutex;
use tokio::time::{timeout, Duration};

mod helpers;

#[cfg(feature = "tui")]
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "tui")]
struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

#[cfg(feature = "tui")]
impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

#[cfg(feature = "tui")]
impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

#[tokio::test]
async fn mcp_path_does_not_block_on_event_bus_backpressure() {
    let mut config = helpers::test_config();
    config.tui.event_buffer = 1;
    let service = helpers::test_service(config);
    let _stream = service.event_bus().subscribe();

    let request = helpers::json_rpc_request("mem_stats", json!({ "input": {} }));
    let result = timeout(
        Duration::from_millis(500),
        service.handle_json_rpc(request, helpers::auth_header()),
    )
    .await;
    assert!(result.is_ok());
}

#[cfg(feature = "tui")]
#[tokio::test]
async fn mcp_path_remains_responsive_with_tui_running() {
    use cerebro::tui::{start_tui_task, TuiLaunch};
    use tokio::sync::watch;

    let _guard = ENV_LOCK.lock().expect("env lock");
    let _headless = EnvVarGuard::set("CEREBRO_TUI_HEADLESS", "1");
    let mut config = helpers::test_config();
    config.tui.enabled = true;
    let tui_config = config.tui.clone();
    let service = helpers::test_service(config);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let launch = start_tui_task(
        tui_config,
        service.storage(),
        service.event_bus(),
        shutdown_rx,
    )
    .await
    .expect("tui start");
    let handle = match launch {
        TuiLaunch::Started(handle) => handle,
        TuiLaunch::Disabled => panic!("tui should be started"),
    };

    let request = helpers::json_rpc_request("mem_stats", json!({ "input": {} }));
    let result = timeout(
        Duration::from_millis(500),
        service.handle_json_rpc(request, helpers::auth_header()),
    )
    .await;
    assert!(result.is_ok());

    shutdown_tx
        .send(true)
        .expect("shutdown send should succeed");
    timeout(Duration::from_secs(1), handle.join())
        .await
        .expect("tui join timeout")
        .expect("tui join should succeed");
}
