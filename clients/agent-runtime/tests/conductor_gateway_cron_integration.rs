use chrono::Utc;
use corvus::conductor::sources::{route_cli_message, CliRouteOutcome};
use corvus::cron::{CronJob, DeliveryConfig, JobType, Schedule, SessionTarget};

#[test]
fn gateway_payload_normalization_produces_composite_task_request() {
    let payload = serde_json::json!({
        "message": "/task ship release notes",
        "channel": "gateway",
        "channel_id": "web-1",
        "sender": "dashboard",
        "session_id": "session-42",
    });

    let request = corvus::gateway::normalize_conductor_task_submit(&payload)
        .expect("expected normalized task request");

    assert_eq!(request.description, "ship release notes");
    assert_eq!(request.domain, corvus::conductor::TaskDomain::Composite);
    assert_eq!(request.tags, vec!["gateway", "task"]);
    assert!(matches!(
        request.origin,
        corvus::conductor::TaskOrigin::Dashboard { session_id } if session_id == "session-42"
    ));
}

#[test]
fn cron_conductor_task_dispatch_builds_request_without_breaking_legacy_jobs() {
    let now = Utc::now();
    let job = CronJob {
        id: "cron-1".to_string(),
        expression: "@every 5m".to_string(),
        schedule: Schedule::Every { every_ms: 300_000 },
        command: "refresh dependencies".to_string(),
        prompt: Some("run security scan".to_string()),
        name: Some("security-scan".to_string()),
        job_type: JobType::ConductorTask,
        session_target: SessionTarget::Isolated,
        model: None,
        enabled: true,
        delivery: DeliveryConfig::default(),
        delete_after_run: false,
        created_at: now,
        next_run: now,
        last_run: None,
        last_status: None,
        last_output: None,
    };

    let request = corvus::cron::scheduler::build_conductor_task_request(&job)
        .expect("should build conductor request");
    assert!(request.description.contains("run security scan"));

    assert_eq!(JobType::parse("shell"), JobType::Shell);
    assert_eq!(JobType::parse("agent"), JobType::Agent);
}

#[test]
fn cli_task_activation_is_explicit_and_non_task_passthrough() {
    let routed = route_cli_message(true, "/task investigate flaky tests");
    assert!(matches!(routed, CliRouteOutcome::Task(_)));

    let passthrough = route_cli_message(true, "plain chat input");
    assert!(matches!(passthrough, CliRouteOutcome::AgentPassthrough));
}

#[test]
fn gateway_non_task_payload_remains_legacy_passthrough_compatible() {
    let payload = serde_json::json!({
        "message": "hello dashboard",
        "channel": "gateway",
        "channel_id": "web-2",
        "sender": "dashboard",
    });

    let request = corvus::gateway::normalize_conductor_task_submit(&payload);
    assert!(request.is_none());
}

#[test]
fn conductor_websocket_route_is_additive_and_does_not_replace_legacy_routes() {
    assert_eq!(
        corvus::gateway::CONDUCTOR_EVENTS_WS_PATH,
        "/api/conductor/events"
    );
    assert_eq!(JobType::parse("shell"), JobType::Shell);
    assert_eq!(JobType::parse("agent"), JobType::Agent);
}
