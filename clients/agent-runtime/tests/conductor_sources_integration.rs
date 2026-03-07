use corvus::conductor::sources::{route_channel_message, ChannelRouteOutcome};

#[test]
fn explicit_task_command_routes_to_conductor_request() {
    let routed = route_channel_message(
        true,
        "/task refactor scheduler and add tests",
        "telegram",
        "chan-1",
        "alice",
        Some("thread-9"),
    );

    match routed {
        ChannelRouteOutcome::Task(request) => {
            assert_eq!(request.description, "refactor scheduler and add tests");
            assert_eq!(request.tags, vec!["channel", "task"]);
        }
        ChannelRouteOutcome::ChatPassthrough => panic!("expected task route"),
    }
}

#[test]
fn regular_chat_message_remains_passthrough() {
    let routed = route_channel_message(
        true,
        "hey corvus how are you?",
        "telegram",
        "chan-1",
        "alice",
        None,
    );

    assert!(matches!(routed, ChannelRouteOutcome::ChatPassthrough));
}

#[test]
fn task_routing_is_disabled_when_conductor_is_disabled() {
    let routed = route_channel_message(
        false,
        "/task run nightly audit",
        "telegram",
        "chan-1",
        "alice",
        None,
    );

    assert!(matches!(routed, ChannelRouteOutcome::ChatPassthrough));
}
