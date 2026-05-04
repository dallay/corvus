use cerebro::tui::event_bus::ToolCallEventKind;
use serde_json::json;
use tokio::time::{timeout, Duration};

mod helpers;

#[tokio::test]
async fn emits_started_and_finished_events() {
    let config = helpers::test_config();
    let service = helpers::test_service(config);
    let mut stream = service.event_bus().subscribe();

    let request = helpers::json_rpc_request("mem_stats", json!({ "input": {} }));
    let _response = service
        .handle_json_rpc(request, helpers::auth_header())
        .await;

    let started = timeout(Duration::from_secs(1), stream.recv())
        .await
        .expect("started event timeout")
        .expect("started event");
    assert!(matches!(started.kind, ToolCallEventKind::Started));
    let finished = timeout(Duration::from_secs(1), stream.recv())
        .await
        .expect("finished event timeout")
        .expect("finished event");
    assert!(matches!(finished.kind, ToolCallEventKind::Finished));
    assert_eq!(started.request_id, "1");
    assert_eq!(finished.request_id, "1");
    assert_eq!(started.tool_name, "mem_stats");
    assert_eq!(finished.tool_name, "mem_stats");
    assert_eq!(finished.status.as_deref(), Some("ok"));
    assert!(finished.duration_ms.is_some());
}

#[tokio::test]
async fn emits_failed_event_on_error() {
    let config = helpers::test_config();
    let service = helpers::test_service(config);
    let mut stream = service.event_bus().subscribe();

    let request = helpers::json_rpc_request(
        "mem_get_observation",
        json!({
            "input": { "memory_id": "" }
        }),
    );
    let _response = service
        .handle_json_rpc(request, helpers::auth_header())
        .await;

    let started = timeout(Duration::from_secs(1), stream.recv())
        .await
        .expect("started event timeout")
        .expect("started event");
    assert!(matches!(started.kind, ToolCallEventKind::Started));
    let failed = timeout(Duration::from_secs(1), stream.recv())
        .await
        .expect("failed event timeout")
        .expect("failed event");
    assert!(matches!(failed.kind, ToolCallEventKind::Failed));
}
