use corvus::conductor::service::{
    ConductorServiceCore, ReadyStep, SchedulerConfigView, SubmitOutcome,
};
use corvus::conductor::{StepId, StepStatus, TaskDomain, TaskId};

fn step(task: &str, step: &str, domain: TaskDomain, status: StepStatus) -> ReadyStep {
    ReadyStep {
        task_id: TaskId::new(task).expect("valid task id"),
        step_id: StepId::new(step).expect("valid step id"),
        domain,
        status,
        enqueued_epoch_ms: 100,
    }
}

#[test]
fn enforces_global_and_per_domain_caps() {
    let mut service = ConductorServiceCore::new(SchedulerConfigView {
        global_max: 2,
        coding_max: 1,
        research_max: 1,
        browser_max: 1,
        system_max: 1,
        intake_capacity: 16,
        hard_intake_capacity: 32,
    });

    service.enqueue(step("t1", "c1", TaskDomain::Coding, StepStatus::Queued));
    service.enqueue(step("t2", "c2", TaskDomain::Coding, StepStatus::Queued));
    service.enqueue(step("t3", "r1", TaskDomain::Research, StepStatus::Queued));

    let dispatch = service.mini_tick(1_000);

    assert_eq!(dispatch.dispatched.len(), 2);
    assert_eq!(
        dispatch
            .dispatched
            .iter()
            .filter(|s| s.domain == TaskDomain::Coding)
            .count(),
        1,
    );
    assert_eq!(
        dispatch
            .dispatched
            .iter()
            .filter(|s| s.domain == TaskDomain::Research)
            .count(),
        1,
    );
}

#[test]
fn fair_queue_order_across_mixed_domains() {
    let mut service = ConductorServiceCore::new(SchedulerConfigView {
        global_max: 3,
        coding_max: 2,
        research_max: 2,
        browser_max: 1,
        system_max: 1,
        intake_capacity: 16,
        hard_intake_capacity: 32,
    });

    service.enqueue(step(
        "t1",
        "coding_old",
        TaskDomain::Coding,
        StepStatus::Queued,
    ));
    service.enqueue(step(
        "t2",
        "coding_new",
        TaskDomain::Coding,
        StepStatus::Queued,
    ));
    service.enqueue(step(
        "t3",
        "research_old",
        TaskDomain::Research,
        StepStatus::Queued,
    ));

    let dispatch = service.mini_tick(1_000);
    let ids: Vec<String> = dispatch
        .dispatched
        .iter()
        .map(|ready| ready.step_id.as_str().to_string())
        .collect();

    assert_eq!(ids, vec!["coding_old", "research_old", "coding_new"]);
}

#[test]
fn retry_backoff_blocks_until_elapsed() {
    let mut service = ConductorServiceCore::new(SchedulerConfigView::default());
    service.enqueue(step(
        "t_retry",
        "retry_step",
        TaskDomain::Coding,
        StepStatus::RetryQueued {
            attempt: 2,
            retry_after_epoch_ms: 10_000,
        },
    ));

    let before = service.mini_tick(9_000);
    assert!(before.dispatched.is_empty());

    let after = service.mini_tick(10_000);
    assert_eq!(after.dispatched.len(), 1);
    assert_eq!(after.dispatched[0].step_id.as_str(), "retry_step");
}

#[test]
fn intake_backpressure_is_bounded_and_explicit() {
    let mut service = ConductorServiceCore::new(SchedulerConfigView {
        intake_capacity: 2,
        hard_intake_capacity: 3,
        ..SchedulerConfigView::default()
    });

    assert!(matches!(
        service.submit(step("t1", "s1", TaskDomain::Coding, StepStatus::Queued)),
        SubmitOutcome::Queued
    ));
    assert!(matches!(
        service.submit(step("t2", "s2", TaskDomain::Coding, StepStatus::Queued)),
        SubmitOutcome::Queued
    ));
    assert!(matches!(
        service.submit(step("t3", "s3", TaskDomain::Coding, StepStatus::Queued)),
        SubmitOutcome::QueuedWithBackpressure
    ));
    assert_eq!(service.queue_depth(), 3);

    assert!(matches!(
        service.submit(step("t4", "s4", TaskDomain::Coding, StepStatus::Queued)),
        SubmitOutcome::Saturated
    ));
    assert_eq!(service.queue_depth(), 3);
}
