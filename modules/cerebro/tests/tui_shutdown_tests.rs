#![cfg(feature = "tui")]

use cerebro::tui::event_bus::EventBus;
use cerebro::tui::{start_tui_task, TuiLaunch};
use cerebro::{InMemoryStorage, TuiConfig};
use tokio::sync::watch;
use tokio::time::{timeout, Duration};

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
async fn tui_exits_on_shutdown_signal() {
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
    timeout(Duration::from_secs(1), handle.join())
        .await
        .expect("tui join timeout")
        .expect("tui join should succeed");
}

#[tokio::test]
async fn tui_crash_isolated_from_caller() {
    let _headless = EnvVarGuard::set("CEREBRO_TUI_HEADLESS", "1");
    let _crash = EnvVarGuard::set("CEREBRO_TUI_TEST_CRASH", "1");
    let mut config = TuiConfig::default();
    config.enabled = true;
    let storage = InMemoryStorage::new();
    let event_bus = EventBus::new(4);
    let (_tx, rx) = watch::channel(false);

    let launch = start_tui_task(config, storage, event_bus, rx)
        .await
        .expect("expected launch result");
    let handle = match launch {
        TuiLaunch::Started(handle) => handle,
        TuiLaunch::Disabled => panic!("expected started"),
    };

    timeout(Duration::from_secs(1), handle.join())
        .await
        .expect("tui join timeout")
        .expect("tui join should succeed");
}
