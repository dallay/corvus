#![cfg(feature = "tui")]

use cerebro::tui::event_bus::EventBus;
use cerebro::tui::{start_tui_task, validate_no_network_listeners, TuiLaunch};
use cerebro::{InMemoryStorage, TuiConfig};
use std::sync::Mutex;
use tokio::sync::watch;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, previous }
    }
}

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
async fn tui_toggle_disabled_skips_start() {
    let config = TuiConfig::default();
    let storage = InMemoryStorage::new();
    let event_bus = EventBus::new(1);
    let (_tx, rx) = watch::channel(false);

    let launch = start_tui_task(config, storage, event_bus, rx)
        .await
        .expect("expected launch result");
    assert!(matches!(launch, TuiLaunch::Disabled));
}

#[tokio::test]
async fn tui_toggle_enabled_starts_headless() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _headless = EnvVarGuard::set("CEREBRO_TUI_HEADLESS", "1");
    let mut config = TuiConfig::default();
    config.enabled = true;
    let storage = InMemoryStorage::new();
    let event_bus = EventBus::new(4);
    let (tx, rx) = watch::channel(false);

    let launch = start_tui_task(config, storage, event_bus, rx)
        .await
        .expect("expected launch result");
    let handle = match launch {
        TuiLaunch::Started(handle) => handle,
        TuiLaunch::Disabled => panic!("expected started"),
    };

    tx.send(true).expect("shutdown send should succeed");
    handle.join().await.expect("tui join should succeed");
}

#[test]
fn tui_validation_rejects_unexpected_listener_env() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _listen = EnvVarGuard::set("CEREBRO_TUI_LISTEN", "1");
    let result = validate_no_network_listeners();
    assert!(result.is_err());
}

#[test]
fn tui_validation_allows_no_listener_env() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _listen = EnvVarGuard::remove("CEREBRO_TUI_LISTEN");
    let _port = EnvVarGuard::remove("CEREBRO_TUI_PORT");
    let _http = EnvVarGuard::remove("CEREBRO_TUI_HTTP");
    let result = validate_no_network_listeners();
    assert!(result.is_ok());
}
