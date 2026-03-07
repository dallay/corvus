use crate::conductor::{PlannedStepForExecution, RiskLevel, StepStatus, TaskDomain, TaskId};
use crate::config::Config;
use crate::memory::Memory;
use crate::providers::Provider;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod browser;
pub mod coding;
pub mod research;
pub mod system;

#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub task_id: TaskId,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Deny,
    Timeout,
    Pending,
}

#[async_trait]
pub trait ApprovalGate: Send + Sync {
    async fn decide(
        &self,
        task_id: &TaskId,
        step_id: &crate::conductor::StepId,
        reason: &str,
        risk: RiskLevel,
    ) -> Result<ApprovalDecision>;
}

#[async_trait]
pub trait SandboxExecutor: Send + Sync {
    async fn run_wrapped(&self, command: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ScopedSandboxExecutor {
    allowed_root: PathBuf,
}

impl ScopedSandboxExecutor {
    pub fn new(allowed_root: PathBuf) -> Self {
        Self { allowed_root }
    }

    fn assert_command_scope(&self, command: &str) -> Result<()> {
        for token in command.split_whitespace().map(strip_wrapping_quotes) {
            if token.is_empty() || token.starts_with('-') || token.contains("://") {
                continue;
            }

            let looks_like_path = token.starts_with('/')
                || token.starts_with("./")
                || token.starts_with("../")
                || token.starts_with('~')
                || token.contains('/');

            if !looks_like_path {
                continue;
            }

            if token.starts_with('~') {
                anyhow::bail!("sandbox path denied: home expansion is not allowed");
            }

            let candidate = if Path::new(token).is_absolute() {
                normalize_path(PathBuf::from(token))
            } else {
                normalize_path(self.allowed_root.join(token))
            };

            if !candidate.starts_with(&self.allowed_root) {
                anyhow::bail!("sandbox path denied: {token}");
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SandboxExecutor for ScopedSandboxExecutor {
    async fn run_wrapped(&self, command: &str) -> Result<()> {
        self.assert_command_scope(command)
    }
}

pub struct PerformerContext {
    pub memory: Option<Arc<dyn Memory>>,
    pub provider: Option<Arc<dyn Provider>>,
    pub sandbox: Arc<dyn SandboxExecutor>,
    pub config: Option<Arc<Config>>,
    pub approval_gate: Arc<dyn ApprovalGate>,
    pub progress_tx: mpsc::Sender<ProgressEvent>,
}

impl PerformerContext {
    pub fn new(sandbox: Arc<dyn SandboxExecutor>, approval_gate: Arc<dyn ApprovalGate>) -> Self {
        let (progress_tx, _progress_rx) = mpsc::channel(64);
        Self {
            memory: None,
            provider: None,
            sandbox,
            config: None,
            approval_gate,
            progress_tx,
        }
    }

    pub fn with_shared(
        memory: Arc<dyn Memory>,
        provider: Arc<dyn Provider>,
        sandbox: Arc<dyn SandboxExecutor>,
        config: Arc<Config>,
        approval_gate: Arc<dyn ApprovalGate>,
        progress_tx: mpsc::Sender<ProgressEvent>,
    ) -> Self {
        Self {
            memory: Some(memory),
            provider: Some(provider),
            sandbox,
            config: Some(config),
            approval_gate,
            progress_tx,
        }
    }
}

#[async_trait]
pub trait Performer: Send + Sync {
    fn domain(&self) -> TaskDomain;
    async fn execute(
        &self,
        step: &PlannedStepForExecution,
        ctx: &PerformerContext,
    ) -> Result<StepStatus>;
}

pub struct InMemoryPerformerRegistry {
    performers: HashMap<TaskDomain, Arc<dyn Performer>>,
}

impl Default for InMemoryPerformerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryPerformerRegistry {
    pub fn new() -> Self {
        let mut performers: HashMap<TaskDomain, Arc<dyn Performer>> = HashMap::new();
        performers.insert(TaskDomain::Coding, Arc::new(coding::CodingPerformer));
        performers.insert(TaskDomain::Research, Arc::new(research::ResearchPerformer));
        performers.insert(TaskDomain::Browser, Arc::new(browser::BrowserPerformer));
        performers.insert(TaskDomain::System, Arc::new(system::SystemPerformer));
        Self { performers }
    }

    pub fn insert(&mut self, performer: Arc<dyn Performer>) {
        self.performers.insert(performer.domain(), performer);
    }

    pub fn get(&self, domain: TaskDomain) -> Option<Arc<dyn Performer>> {
        self.performers.get(&domain).cloned()
    }
}

pub struct PerformerPool {
    registry: InMemoryPerformerRegistry,
}

impl PerformerPool {
    pub fn new(registry: InMemoryPerformerRegistry) -> Self {
        Self { registry }
    }

    pub async fn execute_step(
        &self,
        step: &PlannedStepForExecution,
        context: &PerformerContext,
    ) -> Result<StepStatus> {
        if step.domain == TaskDomain::System {
            if let Err(error) = context.sandbox.run_wrapped(&step.command).await {
                return Ok(StepStatus::Failed {
                    error: format!("sandbox_required:{error}"),
                });
            }
        }

        if step.risk != RiskLevel::Low {
            let approval = context
                .approval_gate
                .decide(&step.task_id, &step.step_id, &step.description, step.risk)
                .await?;

            match approval {
                ApprovalDecision::Allow => {}
                ApprovalDecision::Pending => {
                    return Ok(StepStatus::WaitingForApproval {
                        reason: step.description.clone(),
                        tool_name: step.domain.to_string(),
                    });
                }
                ApprovalDecision::Deny => {
                    return Ok(StepStatus::Failed {
                        error: "approval denied".to_string(),
                    });
                }
                ApprovalDecision::Timeout => {
                    return Ok(StepStatus::Failed {
                        error: "approval timeout".to_string(),
                    });
                }
            }
        }

        let performer = self
            .registry
            .get(step.domain)
            .ok_or_else(|| anyhow::anyhow!("no performer for domain"))?;
        performer.execute(step, context).await
    }
}

impl std::fmt::Display for TaskDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskDomain::Coding => write!(f, "coding"),
            TaskDomain::Research => write!(f, "research"),
            TaskDomain::Browser => write!(f, "browser"),
            TaskDomain::System => write!(f, "system"),
            TaskDomain::Composite => write!(f, "composite"),
        }
    }
}

fn strip_wrapping_quotes(token: &str) -> &str {
    token.trim_matches(|ch| ch == '"' || ch == '\'')
}

fn normalize_path(input: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in input.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}
