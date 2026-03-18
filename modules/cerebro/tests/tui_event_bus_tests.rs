use cerebro::tui::event_bus::{EventBus, ToolCallEvent, ToolCallEventKind};

fn sample_event(id: &str) -> ToolCallEvent {
    ToolCallEvent {
        kind: ToolCallEventKind::Started,
        request_id: id.to_string(),
        tool_name: "mem_stats".to_string(),
        timestamp: "now".to_string(),
        duration_ms: None,
        status: Some("started".to_string()),
        redacted_args: None,
        redacted_output: None,
        error: None,
    }
}

#[tokio::test]
async fn lagging_subscriber_records_drops() {
    let bus = EventBus::new(1);
    let mut stream = bus.subscribe();

    bus.publish(sample_event("1"));
    bus.publish(sample_event("2"));
    bus.publish(sample_event("3"));

    let _ = stream.recv().await.expect("expected event");
    assert!(stream.drop_count() >= 2);
}
