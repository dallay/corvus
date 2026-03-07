use corvus::conductor::service::{ConductorServiceCore, ReadyStep, SchedulerConfigView};
use corvus::conductor::{RiskLevel, StepId, StepStatus, TaskDomain, TaskId};
use std::time::{Duration, Instant};

fn ready(task: &str, step: &str, domain: TaskDomain) -> ReadyStep {
    ReadyStep {
        task_id: TaskId::new(task).expect("valid task id"),
        step_id: StepId::new(step).expect("valid step id"),
        domain,
        status: StepStatus::Queued,
        enqueued_epoch_ms: 0,
    }
}

#[test]
fn fast_path_planning_stays_within_budget() {
    let planner = corvus::conductor::planner::Planner::new(
        corvus::conductor::planner::PlannerConfigView {
            max_planning_time_ms: 100,
            fast_path_budget_ms: 10,
            prompt_path: None,
        },
        Box::new(corvus::conductor::classifier::RuleBasedClassifier),
        Box::new(corvus::conductor::planner::NoopPlanModel),
    );

    let request = corvus::conductor::TaskRequest {
        description: "fix typo".to_string(),
        origin: corvus::conductor::TaskOrigin::Cli {
            working_dir: "/tmp/repo".to_string(),
        },
        priority: corvus::conductor::TaskPriority::Normal,
        context: None,
        workspace_hint: None,
        timeout_ms: None,
        tags: vec![],
        domain: TaskDomain::Composite,
    };

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let start = Instant::now();
    let _ = runtime.block_on(planner.plan(&request)).expect("plan");
    assert!(start.elapsed() < Duration::from_millis(10));
}

#[test]
fn queue_depth_is_bounded_by_intake_capacity() {
    let mut service = ConductorServiceCore::new(SchedulerConfigView {
        intake_capacity: 2,
        hard_intake_capacity: 3,
        ..SchedulerConfigView::default()
    });

    let _ = service.submit(ready("t1", "s1", TaskDomain::Coding));
    let _ = service.submit(ready("t2", "s2", TaskDomain::Coding));
    let _ = service.submit(ready("t3", "s3", TaskDomain::Coding));

    assert_eq!(service.queue_depth(), 3);

    let saturated = service.submit(ready("t4", "s4", TaskDomain::Coding));
    assert!(matches!(
        saturated,
        corvus::conductor::service::SubmitOutcome::Saturated
    ));
}

#[test]
fn prompt_hot_reload_updates_within_five_seconds() {
    let temp = tempfile::tempdir().expect("tempdir");
    let prompt_path = temp.path().join("CONDUCTOR.md");
    std::fs::write(&prompt_path, "initial prompt").expect("write initial");

    let watcher = corvus::conductor::prompt_watcher::PromptHotReload::new(&prompt_path)
        .expect("watcher init");
    std::fs::write(&prompt_path, "updated prompt").expect("write updated");

    let observed = watcher
        .wait_for_prompt("updated prompt", Duration::from_secs(5))
        .expect("watcher should observe update within 5s");
    assert_eq!(observed.trim(), "updated prompt");
}

#[test]
fn risk_level_is_serialization_stable_for_security_guards() {
    let encoded = serde_json::to_string(&RiskLevel::High).expect("serialize risk");
    assert_eq!(encoded, "\"high\"");
}
