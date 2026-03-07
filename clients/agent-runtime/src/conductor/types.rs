use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_identifier("task_id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(String);

impl StepId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_identifier("step_id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_identifier(name: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("{name} must not be empty");
    }
    if value.len() > 128 {
        anyhow::bail!("{name} exceeds max length of 128 characters");
    }
    if !value
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
    {
        anyhow::bail!("{name} must use [a-zA-Z0-9_-] only");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDomain {
    Coding,
    Research,
    Browser,
    System,
    Composite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOrigin {
    Chat {
        channel_name: String,
        channel_id: String,
        sender: String,
        thread_id: Option<String>,
    },
    Cli {
        working_dir: String,
    },
    Dashboard {
        session_id: String,
    },
    Cron {
        job_id: String,
        schedule_name: String,
    },
    Internal {
        component: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRequest {
    pub description: String,
    pub origin: TaskOrigin,
    pub priority: TaskPriority,
    pub context: Option<String>,
    pub workspace_hint: Option<String>,
    pub timeout_ms: Option<u64>,
    pub tags: Vec<String>,
    pub domain: TaskDomain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Received,
    Planning,
    Active,
    Completed,
    Failed { error: String },
    Cancelled { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Queued,
    WaitingForDependency {
        blocked_by: Vec<StepId>,
    },
    Scheduled,
    Running,
    RetryQueued {
        attempt: u32,
        retry_after_epoch_ms: u64,
    },
    WaitingForApproval {
        reason: String,
        tool_name: String,
    },
    Completed,
    Failed {
        error: String,
    },
    Cancelled {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConductorCommandEnvelope {
    Submit { request: TaskRequest },
    Cancel { task_id: TaskId },
    Status { task_id: TaskId },
    StepStatus { task_id: TaskId, step_id: StepId },
    Nudge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConductorEventEnvelope {
    TaskAccepted {
        task_id: TaskId,
    },
    TaskStateChanged {
        task_id: TaskId,
        status: TaskStatus,
    },
    StepStateChanged {
        task_id: TaskId,
        step_id: StepId,
        status: StepStatus,
    },
    HealthTick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedStepForExecution {
    pub task_id: TaskId,
    pub step_id: StepId,
    pub domain: TaskDomain,
    pub description: String,
    pub command: String,
    pub risk: RiskLevel,
}
