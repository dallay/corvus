use async_trait::async_trait;
use corvus::conductor::performers::{
    ApprovalDecision, ApprovalGate, InMemoryPerformerRegistry, PerformerContext, PerformerPool,
    SandboxExecutor, ScopedSandboxExecutor,
};
use corvus::conductor::{
    PlannedStepForExecution, RiskLevel, StepId, StepStatus, TaskDomain, TaskId,
};
use std::sync::Arc;

struct MockSandbox {
    allow: bool,
}

#[async_trait]
impl SandboxExecutor for MockSandbox {
    async fn run_wrapped(&self, _command: &str) -> anyhow::Result<()> {
        if self.allow {
            Ok(())
        } else {
            anyhow::bail!("sandbox unavailable")
        }
    }
}

struct StaticApprovalGate {
    decision: ApprovalDecision,
}

#[async_trait]
impl ApprovalGate for StaticApprovalGate {
    async fn decide(
        &self,
        _task_id: &TaskId,
        _step_id: &StepId,
        _reason: &str,
        _risk: RiskLevel,
    ) -> anyhow::Result<ApprovalDecision> {
        Ok(self.decision)
    }
}

fn step(domain: TaskDomain, command: &str, risk: RiskLevel) -> PlannedStepForExecution {
    PlannedStepForExecution {
        task_id: TaskId::new("task_sec").expect("valid task id"),
        step_id: StepId::new("step_sec").expect("valid step id"),
        domain,
        description: "secure step".to_string(),
        command: command.to_string(),
        risk,
    }
}

#[tokio::test]
async fn unsandboxed_system_execution_is_blocked() {
    let registry = InMemoryPerformerRegistry::default();
    let pool = PerformerPool::new(registry);
    let context = PerformerContext::new(
        Arc::new(MockSandbox { allow: false }),
        Arc::new(StaticApprovalGate {
            decision: ApprovalDecision::Allow,
        }),
    );

    let status = pool
        .execute_step(
            &step(TaskDomain::System, "rm -rf /tmp/x", RiskLevel::Low),
            &context,
        )
        .await
        .expect("execution result");

    assert!(matches!(status, StepStatus::Failed { .. }));
}

#[tokio::test]
async fn risky_actions_enter_waiting_for_approval() {
    let registry = InMemoryPerformerRegistry::default();
    let pool = PerformerPool::new(registry);
    let context = PerformerContext::new(
        Arc::new(MockSandbox { allow: true }),
        Arc::new(StaticApprovalGate {
            decision: ApprovalDecision::Pending,
        }),
    );

    let status = pool
        .execute_step(
            &step(TaskDomain::Coding, "apply migration", RiskLevel::High),
            &context,
        )
        .await
        .expect("execution result");

    assert!(matches!(status, StepStatus::WaitingForApproval { .. }));
}

#[tokio::test]
async fn deny_or_timeout_fails_closed() {
    let registry = InMemoryPerformerRegistry::default();
    let pool = PerformerPool::new(registry);

    let deny_context = PerformerContext::new(
        Arc::new(MockSandbox { allow: true }),
        Arc::new(StaticApprovalGate {
            decision: ApprovalDecision::Deny,
        }),
    );
    let denied = pool
        .execute_step(
            &step(TaskDomain::Coding, "drop table", RiskLevel::High),
            &deny_context,
        )
        .await
        .expect("deny result");
    assert!(matches!(denied, StepStatus::Failed { .. }));

    let timeout_context = PerformerContext::new(
        Arc::new(MockSandbox { allow: true }),
        Arc::new(StaticApprovalGate {
            decision: ApprovalDecision::Timeout,
        }),
    );
    let timed_out = pool
        .execute_step(
            &step(
                TaskDomain::Research,
                "request escalation",
                RiskLevel::Medium,
            ),
            &timeout_context,
        )
        .await
        .expect("timeout result");
    assert!(matches!(timed_out, StepStatus::Failed { .. }));
}

#[tokio::test]
async fn least_privilege_sandbox_scope_allows_workspace_relative_paths_only() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sandbox = ScopedSandboxExecutor::new(tmp.path().to_path_buf());
    let registry = InMemoryPerformerRegistry::default();
    let pool = PerformerPool::new(registry);
    let context = PerformerContext::new(
        Arc::new(sandbox),
        Arc::new(StaticApprovalGate {
            decision: ApprovalDecision::Allow,
        }),
    );

    let status = pool
        .execute_step(
            &step(
                TaskDomain::System,
                "cat ./workspace_file.txt",
                RiskLevel::Low,
            ),
            &context,
        )
        .await
        .expect("execution result");

    assert!(matches!(status, StepStatus::Completed));
}

#[tokio::test]
async fn least_privilege_sandbox_scope_blocks_escape_paths() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sandbox = ScopedSandboxExecutor::new(tmp.path().to_path_buf());
    let registry = InMemoryPerformerRegistry::default();
    let pool = PerformerPool::new(registry);
    let context = PerformerContext::new(
        Arc::new(sandbox),
        Arc::new(StaticApprovalGate {
            decision: ApprovalDecision::Allow,
        }),
    );

    let status = pool
        .execute_step(
            &step(TaskDomain::System, "cat /etc/passwd", RiskLevel::Low),
            &context,
        )
        .await
        .expect("execution result");

    assert!(matches!(status, StepStatus::Failed { .. }));
}
