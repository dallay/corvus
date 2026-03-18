#![cfg(feature = "tui")]

use cerebro::tui::event_bus::EventBus;
use cerebro::tui::{start_tui_task, TuiLaunch};
use cerebro::{InMemoryStorage, TuiConfig};
use tokio::sync::watch;

#[tokio::test]
async fn tui_exits_on_shutdown_signal() {
    std::env::set_var("CEREBRO_TUI_HEADLESS", "1");
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

    let _ = tx.send(true);
    handle.join().await.expect("tui join should succeed");
    std::env::remove_var("CEREBRO_TUI_HEADLESS");
}

#[tokio::test]
async fn tui_crash_isolated_from_caller() {
    std::env::set_var("CEREBRO_TUI_HEADLESS", "1");
    std::env::set_var("CEREBRO_TUI_TEST_CRASH", "1");
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

    handle.join().await.expect("tui join should succeed");
    std::env::remove_var("CEREBRO_TUI_HEADLESS");
    std::env::remove_var("CEREBRO_TUI_TEST_CRASH");
}
