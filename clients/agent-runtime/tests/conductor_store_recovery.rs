use corvus::conductor::task_store::{InMemoryTransitionLog, TaskRecordBuilder, TaskStore};
use corvus::conductor::{StepStatus, TaskDomain, TaskId, TaskStatus};

fn build_store() -> TaskStore {
    TaskStore::new(Box::new(InMemoryTransitionLog::default()))
}

#[test]
fn transition_is_atomic_and_rolls_back_on_persistence_failure() {
    let store = TaskStore::new(Box::new(
        corvus::conductor::task_store::FailingTransitionLog,
    ));
    let task = TaskRecordBuilder::new(TaskId::new("task_atomic").expect("valid"), "compile module")
        .step("step_a", TaskDomain::Coding, vec![])
        .build();
    store.insert_task(task).expect("insert task");

    let error = store
        .transition_step(
            &TaskId::new("task_atomic").expect("valid"),
            &corvus::conductor::StepId::new("step_a").expect("valid"),
            StepStatus::Running,
        )
        .expect_err("persistence failure must bubble up");
    assert!(error.to_string().contains("persistence"));

    let snapshot = store
        .task_snapshot(&TaskId::new("task_atomic").expect("valid"))
        .expect("snapshot")
        .expect("task exists");
    assert_eq!(snapshot.step_status("step_a"), Some(&StepStatus::Queued));
}

#[test]
fn reconcile_restart_requeues_running_and_scheduled_steps() {
    let store = build_store();
    let task = TaskRecordBuilder::new(
        TaskId::new("task_recovery").expect("valid"),
        "recover interrupted work",
    )
    .step("step_running", TaskDomain::Coding, vec![])
    .step("step_scheduled", TaskDomain::Research, vec![])
    .build();
    store.insert_task(task).expect("insert task");

    let task_id = TaskId::new("task_recovery").expect("valid");
    store
        .transition_step(
            &task_id,
            &corvus::conductor::StepId::new("step_running").expect("valid"),
            StepStatus::Running,
        )
        .expect("running transition");
    store
        .transition_step(
            &task_id,
            &corvus::conductor::StepId::new("step_scheduled").expect("valid"),
            StepStatus::Scheduled,
        )
        .expect("scheduled transition");

    store
        .reconcile_restart(&task_id)
        .expect("reconcile restart");

    let snapshot = store
        .task_snapshot(&task_id)
        .expect("snapshot")
        .expect("task exists");
    assert_eq!(
        snapshot.step_status("step_running"),
        Some(&StepStatus::Queued)
    );
    assert_eq!(
        snapshot.step_status("step_scheduled"),
        Some(&StepStatus::Queued)
    );
}

#[test]
fn terminal_states_are_immutable() {
    let store = build_store();
    let task_id = TaskId::new("task_terminal").expect("valid");
    let task = TaskRecordBuilder::new(task_id.clone(), "immutable terminal")
        .step("step_done", TaskDomain::Browser, vec![])
        .build();
    store.insert_task(task).expect("insert task");

    store
        .transition_step(
            &task_id,
            &corvus::conductor::StepId::new("step_done").expect("valid"),
            StepStatus::Completed,
        )
        .expect("complete transition");

    let error = store
        .transition_step(
            &task_id,
            &corvus::conductor::StepId::new("step_done").expect("valid"),
            StepStatus::Running,
        )
        .expect_err("terminal state must be immutable");
    assert!(error.to_string().contains("terminal"));
}

#[test]
fn dependency_failure_propagation_is_deterministic() {
    let store = build_store();
    let task_id = TaskId::new("task_dep_fail").expect("valid");
    let task = TaskRecordBuilder::new(task_id.clone(), "dependency propagation")
        .step("a", TaskDomain::Coding, vec![])
        .step("b", TaskDomain::Research, vec!["a"])
        .step("c", TaskDomain::System, vec!["b"])
        .build();
    store.insert_task(task).expect("insert task");

    store
        .transition_step(
            &task_id,
            &corvus::conductor::StepId::new("a").expect("valid"),
            StepStatus::Failed {
                error: "compile error".to_string(),
            },
        )
        .expect("mark failed");

    store
        .propagate_dependency_failure(
            &task_id,
            &corvus::conductor::StepId::new("a").expect("valid"),
            "compile error",
        )
        .expect("propagate dependency failure");

    let snapshot = store
        .task_snapshot(&task_id)
        .expect("snapshot")
        .expect("task exists");
    assert_eq!(
        snapshot.step_status("b"),
        Some(&StepStatus::Cancelled {
            reason: "dependency_failed:a".to_string(),
        }),
    );
    assert_eq!(
        snapshot.step_status("c"),
        Some(&StepStatus::Cancelled {
            reason: "dependency_failed:b".to_string(),
        }),
    );
    assert_eq!(
        snapshot.status,
        TaskStatus::Failed {
            error: "dependency failed: compile error".to_string(),
        },
    );
}

#[test]
fn sqlite_transition_log_uses_wal_mode() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = tmp_dir.path().join("conductor.db");
    let sqlite_log = corvus::conductor::task_store::SqliteTransitionLog::open(&db_path)
        .expect("open sqlite transition log");

    let mode = sqlite_log.journal_mode().expect("journal mode");
    assert_eq!(mode.to_ascii_lowercase(), "wal");
}
