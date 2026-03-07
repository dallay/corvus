use async_trait::async_trait;
use corvus::conductor::classifier::{
    ChainedClassifier, Confidence, RuleBasedClassifier, StaticLlmClassifier,
};
use corvus::conductor::performers::{
    ApprovalDecision, ApprovalGate, InMemoryPerformerRegistry, Performer, PerformerContext,
    PerformerPool, SandboxExecutor,
};
use corvus::conductor::planner::{PlanModel, PlannedStep, Planner, PlannerConfigView, TaskPlan};
use corvus::conductor::service::{ConductorRuntime, RuntimeTerminal, RuntimeUpdateSink};
use corvus::conductor::task_store::{InMemoryTransitionLog, TaskStore};
use corvus::conductor::{
    StepId, StepStatus, TaskDomain, TaskId, TaskOrigin, TaskPriority, TaskRequest,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

struct AllowSandbox;

#[async_trait]
impl SandboxExecutor for AllowSandbox {
    async fn run_wrapped(&self, _command: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

struct AlwaysAllowApproval;

#[async_trait]
impl ApprovalGate for AlwaysAllowApproval {
    async fn decide(
        &self,
        _task_id: &TaskId,
        _step_id: &StepId,
        _reason: &str,
        _risk: corvus::conductor::RiskLevel,
    ) -> anyhow::Result<ApprovalDecision> {
        Ok(ApprovalDecision::Allow)
    }
}

#[derive(Default)]
struct CaptureSink {
    planning_failures: Mutex<Vec<String>>,
    step_updates: Mutex<Vec<(String, String)>>,
}

impl RuntimeUpdateSink for CaptureSink {
    fn planning_failed(&self, task_id: &TaskId, error: &str) {
        self.planning_failures
            .lock()
            .expect("planning_failures mutex poisoned")
            .push(format!("{}:{error}", task_id.as_str()));
    }

    fn step_progress(
        &self,
        task_id: &TaskId,
        step_id: &StepId,
        status: &StepStatus,
        _remaining_steps: usize,
    ) {
        self.step_updates
            .lock()
            .expect("step_updates mutex poisoned")
            .push((
                format!("{}:{}", task_id.as_str(), step_id.as_str()),
                format!("{status:?}"),
            ));
    }
}

#[derive(Clone, Copy)]
enum ModelMode {
    Success,
    PlannerFailure,
    PanicPerformer,
    TimeoutPerformer,
}

struct ModePlanModel {
    mode: ModelMode,
}

#[async_trait]
impl PlanModel for ModePlanModel {
    async fn decompose(&self, request: &TaskRequest, _prompt: &str) -> anyhow::Result<TaskPlan> {
        match self.mode {
            ModelMode::PlannerFailure => anyhow::bail!("planner decomposition failed"),
            ModelMode::Success | ModelMode::PanicPerformer | ModelMode::TimeoutPerformer => {
                let first = PlannedStep {
                    id: StepId::new("step_1")?,
                    domain: TaskDomain::Coding,
                    description: request.description.clone(),
                    depends_on: vec![],
                };
                let second = PlannedStep {
                    id: StepId::new("step_2")?,
                    domain: TaskDomain::Research,
                    description: "finalize".to_string(),
                    depends_on: vec![StepId::new("step_1")?],
                };
                Ok(TaskPlan {
                    steps: vec![first, second],
                })
            }
        }
    }
}

struct TestCodingPerformer;

#[async_trait]
impl Performer for TestCodingPerformer {
    fn domain(&self) -> TaskDomain {
        TaskDomain::Coding
    }

    async fn execute(
        &self,
        step: &corvus::conductor::PlannedStepForExecution,
        _ctx: &PerformerContext,
    ) -> anyhow::Result<StepStatus> {
        if step.description.contains("panic") {
            panic!("panic from performer");
        }
        if step.description.contains("timeout") {
            tokio::time::sleep(Duration::from_millis(250)).await;
            return Ok(StepStatus::Completed);
        }
        Ok(StepStatus::Completed)
    }
}

fn request(text: &str) -> TaskRequest {
    TaskRequest {
        description: text.to_string(),
        origin: TaskOrigin::Cli {
            working_dir: ".".to_string(),
        },
        priority: TaskPriority::Normal,
        context: None,
        workspace_hint: None,
        timeout_ms: None,
        tags: vec![],
        domain: TaskDomain::Composite,
    }
}

fn runtime(mode: ModelMode) -> (ConductorRuntime, Arc<TaskStore>, Arc<CaptureSink>) {
    let planner = Arc::new(Planner::new(
        PlannerConfigView::default(),
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Composite, Confidence::Low),
        )),
        Box::new(ModePlanModel { mode }),
    ));
    let store = Arc::new(TaskStore::new(Box::<InMemoryTransitionLog>::default()));

    let mut registry = InMemoryPerformerRegistry::new();
    registry.insert(Arc::new(TestCodingPerformer));
    let pool = Arc::new(PerformerPool::new(registry));
    let context = Arc::new(PerformerContext::new(
        Arc::new(AllowSandbox),
        Arc::new(AlwaysAllowApproval),
    ));
    let sink = Arc::new(CaptureSink::default());
    (
        ConductorRuntime::new(
            planner,
            Arc::clone(&store),
            pool,
            context,
            Duration::from_millis(100),
            sink.clone(),
        ),
        store,
        sink,
    )
}

#[tokio::test]
async fn submission_planning_dispatch_and_terminal_complete_end_to_end() {
    let (runtime, store, sink) = runtime(ModelMode::Success);

    let report = runtime
        .submit_and_run(request("finish migration safely"))
        .await
        .expect("runtime execution should succeed");

    assert!(matches!(report.terminal, RuntimeTerminal::Completed));

    let snapshot = store
        .task_snapshot(&report.task_id)
        .expect("task snapshot should be available")
        .expect("task should be persisted");
    assert!(matches!(
        snapshot.status,
        corvus::conductor::TaskStatus::Completed
    ));
    assert!(snapshot
        .steps
        .values()
        .all(|step| matches!(step.status, StepStatus::Completed)));

    let updates = sink
        .step_updates
        .lock()
        .expect("step updates mutex poisoned");
    assert_eq!(updates.len(), 2);
}

#[tokio::test]
async fn planning_failure_is_terminal_and_notifies_originating_sink() {
    let (runtime, store, sink) = runtime(ModelMode::PlannerFailure);

    let report = runtime
        .submit_and_run(request("this should fail planning"))
        .await
        .expect("runtime should return terminal result");

    match report.terminal {
        RuntimeTerminal::Failed { error } => {
            assert!(error.contains("planner decomposition failed"));
        }
        RuntimeTerminal::Completed => panic!("expected planning failure"),
    }

    let planning_failures = sink
        .planning_failures
        .lock()
        .expect("planning failures mutex poisoned");
    assert_eq!(planning_failures.len(), 1);
    let snapshot = store
        .task_snapshot(&report.task_id)
        .expect("snapshot query should not fail");
    assert!(snapshot.is_none());
}

#[tokio::test]
async fn timeout_and_panic_are_isolated_without_crashing_runtime() {
    let (timeout_runtime, _, _) = runtime(ModelMode::TimeoutPerformer);
    let timed_out = timeout_runtime
        .submit_and_run(request("timeout test"))
        .await
        .expect("timeout run should complete with terminal state");
    assert!(matches!(timed_out.terminal, RuntimeTerminal::Failed { .. }));

    let (panic_runtime, _, _) = runtime(ModelMode::PanicPerformer);
    let panic_result = panic_runtime
        .submit_and_run(request("panic test"))
        .await
        .expect("panic run should still return terminal state");
    assert!(matches!(
        panic_result.terminal,
        RuntimeTerminal::Failed { .. }
    ));

    let recovered = panic_runtime
        .submit_and_run(request("safe follow-up"))
        .await
        .expect("runtime should continue after panic isolation");
    assert!(matches!(recovered.terminal, RuntimeTerminal::Completed));
}
