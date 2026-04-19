use crate::memory::{TaskPriority, TaskRecord, TaskStatus};

pub const DEFAULT_TASK_LIST_LIMIT: u32 = 50;
pub const MAX_TASK_LIST_LIMIT: u32 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCreateRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<TaskPriority>,
    pub session_id: Option<String>,
    pub caller_scope_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListRequest {
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub caller_scope_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskUpdateRequest {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<TaskPriority>,
    pub status: Option<TaskStatus>,
    pub session_id: Option<String>,
    pub caller_scope_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskRecord>,
    pub applied_limit: u32,
    pub applied_offset: u32,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskServiceErrorKind {
    Validation,
    NotFound,
    UnsupportedBackend,
    InvalidState,
    StorageFailure,
}

impl TaskServiceErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::NotFound => "not_found",
            Self::UnsupportedBackend => "unsupported_backend",
            Self::InvalidState => "invalid_state",
            Self::StorageFailure => "storage_failure",
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct TaskServiceError {
    pub kind: TaskServiceErrorKind,
    pub message: String,
}

impl TaskServiceError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            kind: TaskServiceErrorKind::Validation,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: TaskServiceErrorKind::NotFound,
            message: message.into(),
        }
    }

    pub fn unsupported_backend(message: impl Into<String>) -> Self {
        Self {
            kind: TaskServiceErrorKind::UnsupportedBackend,
            message: message.into(),
        }
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self {
            kind: TaskServiceErrorKind::InvalidState,
            message: message.into(),
        }
    }

    pub fn storage_failure(message: impl Into<String>) -> Self {
        Self {
            kind: TaskServiceErrorKind::StorageFailure,
            message: message.into(),
        }
    }
}
