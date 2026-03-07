pub mod classifier;
pub mod config;
pub mod events;
pub mod performers;
pub mod planner;
pub mod prompt_watcher;
pub mod service;
pub mod sources;
pub mod task_store;
pub mod types;
pub mod workspace;

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use tokio::time::Duration;

use crate::conductor::classifier::{
    ChainedClassifier, Confidence, RuleBasedClassifier, StaticLlmClassifier,
};
use crate::conductor::performers::{
    ApprovalDecision, ApprovalGate, InMemoryPerformerRegistry, PerformerContext, PerformerPool,
    ScopedSandboxExecutor,
};
use crate::conductor::planner::{PlanModel, PlannedStep, Planner, PlannerConfigView, TaskPlan};
use crate::conductor::service::{ConductorRuntime, NoopRuntimeUpdateSink};
use crate::conductor::task_store::{InMemoryTransitionLog, TaskStore};
use crate::config::Config;

#[allow(unused_imports)]
pub use types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSubmitOutcome {
    Submitted,
    RuntimeInactive,
}

static ACTIVE_RUNTIME: LazyLock<RwLock<Option<Arc<ConductorRuntime>>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn activate_runtime(config: &Config) -> Result<()> {
    let runtime = build_runtime(config)?;
    let mut guard = ACTIVE_RUNTIME
        .write()
        .map_err(|_| anyhow::anyhow!("active runtime lock poisoned"))?;
    *guard = Some(runtime);
    Ok(())
}

pub fn runtime_is_active() -> bool {
    ACTIVE_RUNTIME
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().cloned())
        .is_some()
}

pub fn submit_task(request: TaskRequest) -> Result<RuntimeSubmitOutcome> {
    let runtime = ACTIVE_RUNTIME
        .read()
        .map_err(|_| anyhow::anyhow!("active runtime lock poisoned"))?
        .as_ref()
        .cloned();

    let Some(runtime) = runtime else {
        return Ok(RuntimeSubmitOutcome::RuntimeInactive);
    };

    tokio::spawn(async move {
        match runtime.submit_and_run(request).await {
            Ok(report) => {
                tracing::debug!(
                    task_id = report.task_id.as_str(),
                    "conductor runtime task finished",
                );
            }
            Err(error) => {
                tracing::error!(error = %error, "conductor runtime task execution failed");
            }
        }
    });

    Ok(RuntimeSubmitOutcome::Submitted)
}

pub async fn run_supervised_worker(config: Config) -> Result<()> {
    activate_runtime(&config)?;
    crate::health::mark_component_ok("conductor");

    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    loop {
        heartbeat.tick().await;
        crate::health::mark_component_ok("conductor");
    }
}

struct LocalPlanModel;

#[async_trait]
impl PlanModel for LocalPlanModel {
    async fn decompose(&self, request: &TaskRequest, _prompt: &str) -> Result<TaskPlan> {
        let domain = infer_domain(&request.description, request.domain);
        Ok(TaskPlan {
            steps: vec![PlannedStep {
                id: StepId::new("step_runtime_1")?,
                domain,
                description: request.description.clone(),
                depends_on: vec![],
            }],
        })
    }
}

#[derive(Default)]
struct DenyByDefaultApprovalGate;

#[async_trait]
impl ApprovalGate for DenyByDefaultApprovalGate {
    async fn decide(
        &self,
        _task_id: &TaskId,
        _step_id: &StepId,
        _reason: &str,
        _risk: RiskLevel,
    ) -> Result<ApprovalDecision> {
        Ok(ApprovalDecision::Deny)
    }
}

fn build_runtime(config: &Config) -> Result<Arc<ConductorRuntime>> {
    let prompt_path = {
        let candidate = config.workspace_dir.join("CONDUCTOR.md");
        candidate.exists().then_some(candidate)
    };

    let planner = Arc::new(Planner::new(
        PlannerConfigView {
            max_planning_time_ms: config.conductor.planner.max_planning_time_ms,
            fast_path_budget_ms: 10,
            prompt_path,
        },
        Box::new(ChainedClassifier::new(
            RuleBasedClassifier,
            StaticLlmClassifier::new(TaskDomain::Coding, Confidence::High),
        )),
        Box::new(LocalPlanModel),
    ));

    let store = Arc::new(TaskStore::new(Box::<InMemoryTransitionLog>::default()));
    let pool = Arc::new(PerformerPool::new(InMemoryPerformerRegistry::new()));

    let workspace_root = if config.workspace_dir.is_absolute() {
        config.workspace_dir.clone()
    } else {
        absolute_workspace_root(&config.workspace_dir)
    };
    let sandbox = Arc::new(ScopedSandboxExecutor::new(workspace_root));
    let approval_gate = Arc::new(DenyByDefaultApprovalGate);
    let context = Arc::new(PerformerContext::new(sandbox, approval_gate));

    Ok(Arc::new(ConductorRuntime::new(
        planner,
        store,
        pool,
        context,
        Duration::from_millis(config.conductor.stall_timeout_ms.max(1)),
        Arc::new(NoopRuntimeUpdateSink),
    )))
}

fn infer_domain(description: &str, requested: TaskDomain) -> TaskDomain {
    if requested != TaskDomain::Composite {
        return requested;
    }
    let normalized = description.to_ascii_lowercase();
    if normalized.contains("browser") || normalized.contains("screenshot") {
        TaskDomain::Browser
    } else if normalized.contains("research") || normalized.contains("analyze") {
        TaskDomain::Research
    } else if normalized.contains("deploy") || normalized.contains("restart") {
        TaskDomain::System
    } else {
        TaskDomain::Coding
    }
}

fn absolute_workspace_root(workspace_dir: &Path) -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(workspace_dir)
}
