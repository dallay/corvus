use cerebro::tui::event_bus::ToolCallEventKind;
use serde_json::json;

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

    let started = stream.recv().await.expect("started event");
    assert!(matches!(started.kind, ToolCallEventKind::Started));
    let finished = stream.recv().await.expect("finished event");
    assert!(matches!(finished.kind, ToolCallEventKind::Finished));
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

    let started = stream.recv().await.expect("started event");
    assert!(matches!(started.kind, ToolCallEventKind::Started));
    let failed = stream.recv().await.expect("failed event");
    assert!(matches!(failed.kind, ToolCallEventKind::Failed));
}
