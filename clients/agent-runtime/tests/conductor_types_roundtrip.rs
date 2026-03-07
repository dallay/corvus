use corvus::conductor::{
    ConductorCommandEnvelope, ConductorEventEnvelope, StepId, StepStatus, TaskDomain, TaskId,
    TaskOrigin, TaskPriority, TaskRequest, TaskStatus,
};

#[test]
fn task_status_encoding_is_stable() {
    let planning = serde_json::to_string(&TaskStatus::Planning).expect("serialize planning");
    assert_eq!(planning, "\"planning\"");

    let failed = serde_json::to_string(&TaskStatus::Failed {
        error: "planner timeout".to_string(),
    })
    .expect("serialize failed");
    assert_eq!(failed, r#"{"failed":{"error":"planner timeout"}}"#);

    let decoded: TaskStatus = serde_json::from_str(&failed).expect("deserialize failed");
    assert_eq!(
        decoded,
        TaskStatus::Failed {
            error: "planner timeout".to_string(),
        },
    );
}

#[test]
fn step_status_encoding_is_stable() {
    let waiting = StepStatus::WaitingForApproval {
        reason: "destructive command".to_string(),
        tool_name: "shell".to_string(),
    };

    let json = serde_json::to_string(&waiting).expect("serialize waiting");
    assert_eq!(
        json,
        r#"{"waiting_for_approval":{"reason":"destructive command","tool_name":"shell"}}"#,
    );

    let decoded: StepStatus = serde_json::from_str(&json).expect("deserialize waiting");
    assert_eq!(decoded, waiting);
}

#[test]
fn request_and_event_roundtrip() {
    let request = TaskRequest {
        description: "Run cargo test for mission module".to_string(),
        origin: TaskOrigin::Cli {
            working_dir: "/tmp/project".to_string(),
        },
        priority: TaskPriority::High,
        context: Some("Check regressions before release".to_string()),
        workspace_hint: None,
        timeout_ms: Some(45_000),
        tags: vec!["ci".to_string(), "regression".to_string()],
        domain: TaskDomain::Coding,
    };

    let cmd = ConductorCommandEnvelope::Submit {
        request: request.clone(),
    };
    let encoded_cmd = serde_json::to_vec(&cmd).expect("serialize command");
    let decoded_cmd: ConductorCommandEnvelope =
        serde_json::from_slice(&encoded_cmd).expect("deserialize command");
    assert_eq!(decoded_cmd, cmd);

    let event = ConductorEventEnvelope::StepStateChanged {
        task_id: TaskId::new("task-123").expect("valid task id"),
        step_id: StepId::new("step-1").expect("valid step id"),
        status: StepStatus::Running,
    };
    let encoded_event = serde_json::to_vec(&event).expect("serialize event");
    let decoded_event: ConductorEventEnvelope =
        serde_json::from_slice(&encoded_event).expect("deserialize event");
    assert_eq!(decoded_event, event);
}
