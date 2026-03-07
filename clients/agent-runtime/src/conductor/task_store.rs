use crate::conductor::{StepId, StepStatus, TaskDomain, TaskId, TaskStatus};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct StepRecord {
    pub id: StepId,
    pub domain: TaskDomain,
    pub depends_on: Vec<StepId>,
    pub status: StepStatus,
}

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: TaskId,
    pub description: String,
    pub status: TaskStatus,
    pub steps: HashMap<StepId, StepRecord>,
}

impl TaskRecord {
    pub fn step_status(&self, step_id: &str) -> Option<&StepStatus> {
        self.steps
            .iter()
            .find_map(|(id, step)| (id.as_str() == step_id).then_some(&step.status))
    }
}

#[derive(Default)]
pub struct TaskRecordBuilder {
    id: Option<TaskId>,
    description: Option<String>,
    steps: Vec<(String, TaskDomain, Vec<String>)>,
}

impl TaskRecordBuilder {
    pub fn new(id: TaskId, description: impl Into<String>) -> Self {
        Self {
            id: Some(id),
            description: Some(description.into()),
            steps: Vec::new(),
        }
    }

    pub fn step(mut self, step_id: &str, domain: TaskDomain, depends_on: Vec<&str>) -> Self {
        self.steps.push((
            step_id.to_string(),
            domain,
            depends_on.into_iter().map(ToString::to_string).collect(),
        ));
        self
    }

    pub fn build(self) -> TaskRecord {
        let mut steps = HashMap::new();
        for (step_id, domain, depends_on) in self.steps {
            let id = StepId::new(step_id).expect("step id should be valid");
            let deps = depends_on
                .into_iter()
                .map(|dep| StepId::new(dep).expect("dependency step id should be valid"))
                .collect();
            steps.insert(
                id.clone(),
                StepRecord {
                    id,
                    domain,
                    depends_on: deps,
                    status: StepStatus::Queued,
                },
            );
        }

        TaskRecord {
            id: self.id.expect("task id should be present"),
            description: self.description.expect("description should be present"),
            status: TaskStatus::Active,
            steps,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransitionRecord {
    pub task_id: String,
    pub step_id: String,
    pub status: StepStatus,
}

pub trait TransitionLog: Send + Sync {
    fn persist_step_transition(&self, record: &TransitionRecord) -> Result<()>;
}

#[derive(Default)]
pub struct InMemoryTransitionLog {
    pub transitions: Mutex<Vec<TransitionRecord>>,
}

impl TransitionLog for InMemoryTransitionLog {
    fn persist_step_transition(&self, record: &TransitionRecord) -> Result<()> {
        self.transitions
            .lock()
            .expect("transition log mutex poisoned")
            .push(record.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct FailingTransitionLog;

impl TransitionLog for FailingTransitionLog {
    fn persist_step_transition(&self, _record: &TransitionRecord) -> Result<()> {
        anyhow::bail!("persistence failure");
    }
}

pub struct SqliteTransitionLog {
    conn: Mutex<Connection>,
}

impl SqliteTransitionLog {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("failed opening sqlite db: {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable sqlite wal mode")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS conductor_transitions (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              task_id TEXT NOT NULL,
              step_id TEXT NOT NULL,
              status_json TEXT NOT NULL
            )",
            [],
        )
        .context("failed to initialize conductor_transitions table")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn journal_mode(&self) -> Result<String> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .context("failed querying sqlite journal_mode")?;
        Ok(mode)
    }
}

impl TransitionLog for SqliteTransitionLog {
    fn persist_step_transition(&self, record: &TransitionRecord) -> Result<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let status_json = serde_json::to_string(&record.status)
            .context("failed to serialize step status for persistence")?;
        conn.execute(
            "INSERT INTO conductor_transitions (task_id, step_id, status_json) VALUES (?1, ?2, ?3)",
            params![record.task_id, record.step_id, status_json],
        )
        .context("failed persisting transition to sqlite")?;
        Ok(())
    }
}

pub struct TaskStore {
    log: Box<dyn TransitionLog>,
    tasks: Mutex<HashMap<TaskId, TaskRecord>>,
}

impl TaskStore {
    pub fn new(log: Box<dyn TransitionLog>) -> Self {
        Self {
            log,
            tasks: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert_task(&self, task: TaskRecord) -> Result<()> {
        let mut tasks = self.tasks.lock().expect("task store mutex poisoned");
        tasks.insert(task.id.clone(), task);
        Ok(())
    }

    pub fn task_snapshot(&self, task_id: &TaskId) -> Result<Option<TaskRecord>> {
        let tasks = self.tasks.lock().expect("task store mutex poisoned");
        Ok(tasks.get(task_id).cloned())
    }

    pub fn set_task_status(&self, task_id: &TaskId, status: TaskStatus) -> Result<()> {
        let mut tasks = self.tasks.lock().expect("task store mutex poisoned");
        let task = tasks
            .get_mut(task_id)
            .with_context(|| format!("task not found: {}", task_id.as_str()))?;
        task.status = status;
        Ok(())
    }

    pub fn dependencies_completed(&self, task_id: &TaskId, depends_on: &[StepId]) -> Result<bool> {
        let tasks = self.tasks.lock().expect("task store mutex poisoned");
        let task = tasks
            .get(task_id)
            .with_context(|| format!("task not found: {}", task_id.as_str()))?;
        Ok(depends_on.iter().all(|dep_id| {
            task.steps
                .get(dep_id)
                .is_some_and(|step| matches!(step.status, StepStatus::Completed))
        }))
    }

    pub fn transition_step(
        &self,
        task_id: &TaskId,
        step_id: &StepId,
        status: StepStatus,
    ) -> Result<()> {
        let mut tasks = self.tasks.lock().expect("task store mutex poisoned");
        let task = tasks
            .get_mut(task_id)
            .with_context(|| format!("task not found: {}", task_id.as_str()))?;
        let step = task
            .steps
            .get_mut(step_id)
            .with_context(|| format!("step not found: {}", step_id.as_str()))?;

        if is_terminal(&step.status) {
            anyhow::bail!("step is terminal and immutable");
        }

        let transition = TransitionRecord {
            task_id: task_id.as_str().to_string(),
            step_id: step_id.as_str().to_string(),
            status: status.clone(),
        };

        self.log
            .persist_step_transition(&transition)
            .context("persistence failure while writing transition")?;

        step.status = status;
        Ok(())
    }

    pub fn reconcile_restart(&self, task_id: &TaskId) -> Result<()> {
        let mut tasks = self.tasks.lock().expect("task store mutex poisoned");
        let task = tasks
            .get_mut(task_id)
            .with_context(|| format!("task not found: {}", task_id.as_str()))?;

        for step in task.steps.values_mut() {
            if matches!(step.status, StepStatus::Running | StepStatus::Scheduled) {
                step.status = StepStatus::Queued;
            }
        }
        Ok(())
    }

    pub fn propagate_dependency_failure(
        &self,
        task_id: &TaskId,
        failed_step_id: &StepId,
        reason: &str,
    ) -> Result<()> {
        let mut tasks = self.tasks.lock().expect("task store mutex poisoned");
        let task = tasks
            .get_mut(task_id)
            .with_context(|| format!("task not found: {}", task_id.as_str()))?;

        let mut cancelled = vec![failed_step_id.clone()];
        let mut changed = true;
        while changed {
            changed = false;
            let mut ids: Vec<StepId> = task.steps.keys().cloned().collect();
            ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
            for step_id in ids {
                if cancelled.iter().any(|id| id == &step_id) {
                    continue;
                }
                let should_cancel = task
                    .steps
                    .get(&step_id)
                    .is_some_and(|step| step.depends_on.iter().any(|dep| cancelled.contains(dep)));

                if should_cancel {
                    if let Some(step) = task.steps.get_mut(&step_id) {
                        if !is_terminal(&step.status) {
                            let failed_dep = step
                                .depends_on
                                .iter()
                                .find(|dep| cancelled.contains(dep))
                                .expect("failed dependency should exist")
                                .as_str()
                                .to_string();
                            step.status = StepStatus::Cancelled {
                                reason: dependency_cancel_reason(&failed_dep),
                            };
                            cancelled.push(step_id.clone());
                            changed = true;
                        }
                    }
                }
            }
        }

        task.status = TaskStatus::Failed {
            error: format!("dependency failed: {reason}"),
        };
        Ok(())
    }
}

fn is_terminal(status: &StepStatus) -> bool {
    matches!(
        status,
        StepStatus::Completed | StepStatus::Failed { .. } | StepStatus::Cancelled { .. }
    )
}

fn dependency_cancel_reason(step_id: &str) -> String {
    format!("dependency_failed:{step_id}")
}
