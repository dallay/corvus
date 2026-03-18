use serde_json::json;
use tokio::time::{timeout, Duration};

mod helpers;

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

    std::env::set_var("CEREBRO_TUI_HEADLESS", "1");
    let mut config = helpers::test_config();
    config.tui.enabled = true;
    let tui_config = config.tui.clone();
    let service = helpers::test_service(config);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let launch = start_tui_task(tui_config, service.storage(), service.event_bus(), shutdown_rx)
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

    let _ = shutdown_tx.send(true);
    handle.join().await.expect("tui join");
    std::env::remove_var("CEREBRO_TUI_HEADLESS");
}
