use super::model::{
    TaskCreateRequest, TaskListRequest, TaskListResponse, TaskServiceError, TaskUpdateRequest,
    DEFAULT_TASK_LIST_LIMIT, MAX_TASK_LIST_LIMIT,
};
use crate::memory::{
    is_task_unsupported_error, Memory, TaskCreateInput, TaskListQuery, TaskPatch, TaskPriority,
    TaskRecord, TaskStatus,
};
use crate::session_commands::types::sanitize_storage_error;
use std::sync::Arc;
use uuid::Uuid;

pub struct TaskService {
    memory: Arc<dyn Memory>,
}

impl TaskService {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }

    pub async fn create_task(
        &self,
        request: TaskCreateRequest,
    ) -> Result<TaskRecord, TaskServiceError> {
        self.ensure_supported_backend()?;
        let title = Self::validate_title(&request.title)?;
        let description = request.description.unwrap_or_default();
        let priority = request.priority.unwrap_or(TaskPriority::Medium);
        let session_id =
            Self::normalize_optional_string(request.session_id.as_deref(), "session_id")?;
        if let Some(session_id) = session_id.as_deref() {
            self.ensure_session_visible(session_id, request.caller_scope_key.as_deref())
                .await?;
        }

        let now = chrono::Utc::now().to_rfc3339();
        self.memory
            .create_task(TaskCreateInput {
                id: Uuid::new_v4().to_string(),
                title,
                description,
                status: TaskStatus::Pending,
                priority,
                session_id,
                created_at: now.clone(),
                updated_at: now,
            })
            .await
            .map_err(Self::map_storage_error)
    }

    pub async fn list_tasks(
        &self,
        request: TaskListRequest,
    ) -> Result<TaskListResponse, TaskServiceError> {
        self.ensure_supported_backend()?;
        let session_id =
            Self::normalize_optional_string(request.session_id.as_deref(), "session_id")?;
        if let Some(session_id) = session_id.as_deref() {
            self.ensure_session_visible(session_id, request.caller_scope_key.as_deref())
                .await?;
        }

        let applied_limit = request
            .limit
            .unwrap_or(DEFAULT_TASK_LIST_LIMIT)
            .clamp(1, MAX_TASK_LIST_LIMIT);
        let applied_offset = request.offset.unwrap_or(0);

        let page = self
            .memory
            .list_tasks(TaskListQuery {
                session_id,
                status: request.status,
                priority: request.priority,
                limit: applied_limit,
                offset: applied_offset,
            })
            .await
            .map_err(Self::map_storage_error)?;

        Ok(TaskListResponse {
            tasks: page.tasks,
            applied_limit,
            applied_offset,
            has_more: page.has_more,
        })
    }

    pub async fn update_task(
        &self,
        request: TaskUpdateRequest,
    ) -> Result<TaskRecord, TaskServiceError> {
        self.ensure_supported_backend()?;
        Self::validate_uuid(&request.id)?;
        if request.session_id.is_some() {
            return Err(TaskServiceError::validation(
                "TaskUpdate does not allow editing session_id",
            ));
        }
        if request.title.is_none()
            && request.description.is_none()
            && request.priority.is_none()
            && request.status.is_none()
        {
            return Err(TaskServiceError::validation(
                "TaskUpdate requires at least one mutable field",
            ));
        }

        let existing = self
            .get_task(&request.id, request.caller_scope_key.as_deref())
            .await?;
        Self::ensure_mutable_state(existing.status)?;

        let title = request
            .title
            .as_deref()
            .map(Self::validate_title)
            .transpose()?;

        if request.status == Some(TaskStatus::Cancelled) {
            return Err(TaskServiceError::validation(
                "TaskUpdate cannot set status=cancelled; use TaskStop",
            ));
        }

        if let Some(next_status) = request.status {
            Self::validate_status_transition(existing.status, next_status)?;
        }

        self.memory
            .update_task(TaskPatch {
                id: request.id,
                title,
                description: request.description,
                status: request.status,
                priority: request.priority,
            })
            .await
            .map_err(Self::map_storage_error)?
            .ok_or_else(|| TaskServiceError::not_found("task not found"))
    }

    pub async fn stop_task(
        &self,
        id: &str,
        caller_scope_key: Option<&str>,
    ) -> Result<TaskRecord, TaskServiceError> {
        self.ensure_supported_backend()?;
        Self::validate_uuid(id)?;
        let existing = self.get_task(id, caller_scope_key).await?;
        match existing.status {
            TaskStatus::Pending | TaskStatus::InProgress => {}
            TaskStatus::Completed => {
                return Err(TaskServiceError::invalid_state(
                    "TaskStop cannot cancel a completed task",
                ));
            }
            TaskStatus::Cancelled => {
                return Err(TaskServiceError::invalid_state(
                    "TaskStop cannot cancel an already cancelled task",
                ));
            }
        }

        self.memory
            .update_task(TaskPatch {
                id: id.to_string(),
                title: None,
                description: None,
                status: Some(TaskStatus::Cancelled),
                priority: None,
            })
            .await
            .map_err(Self::map_storage_error)?
            .ok_or_else(|| TaskServiceError::not_found("task not found"))
    }

    fn validate_uuid(id: &str) -> Result<(), TaskServiceError> {
        Uuid::parse_str(id)
            .map(|_| ())
            .map_err(|_| TaskServiceError::validation("id must be a valid UUID"))
    }

    fn validate_title(title: &str) -> Result<String, TaskServiceError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(TaskServiceError::validation("title must not be empty"));
        }
        Ok(trimmed.to_string())
    }

    fn normalize_optional_string(
        value: Option<&str>,
        field_name: &str,
    ) -> Result<Option<String>, TaskServiceError> {
        match value {
            Some(value) if value.trim().is_empty() => Err(TaskServiceError::validation(format!(
                "{field_name} must not be empty"
            ))),
            Some(value) => Ok(Some(value.trim().to_string())),
            None => Ok(None),
        }
    }

    async fn ensure_session_visible(
        &self,
        session_id: &str,
        caller_scope_key: Option<&str>,
    ) -> Result<(), TaskServiceError> {
        let Some(caller_scope_key) = caller_scope_key.filter(|value| !value.trim().is_empty())
        else {
            return Err(TaskServiceError::validation(
                "permission denied: caller scope unavailable",
            ));
        };

        let session = self
            .memory
            .get_session_for_scope(session_id, caller_scope_key)
            .await
            .map_err(Self::map_storage_error)?;
        if session.is_none() {
            return Err(TaskServiceError::validation(
                "permission denied: session_id is not available for this caller",
            ));
        }
        Ok(())
    }

    async fn enforce_task_visibility(
        &self,
        task: &TaskRecord,
        caller_scope_key: Option<&str>,
    ) -> Result<(), TaskServiceError> {
        if let Some(session_id) = task.session_id.as_deref() {
            self.ensure_session_visible(session_id, caller_scope_key)
                .await?;
        }

        Ok(())
    }

    pub async fn get_task(
        &self,
        id: &str,
        caller_scope_key: Option<&str>,
    ) -> Result<TaskRecord, TaskServiceError> {
        self.ensure_supported_backend()?;
        Self::validate_uuid(id)?;
        let task = self
            .memory
            .get_task(id)
            .await
            .map_err(Self::map_storage_error)?
            .ok_or_else(|| TaskServiceError::not_found("task not found"))?;
        self.enforce_task_visibility(&task, caller_scope_key)
            .await?;
        Ok(task)
    }

    fn ensure_supported_backend(&self) -> Result<(), TaskServiceError> {
        if self.memory.name() == "sqlite" {
            return Ok(());
        }

        Err(TaskServiceError::unsupported_backend(
            "persistent task tools require sqlite memory backend",
        ))
    }

    fn ensure_mutable_state(status: TaskStatus) -> Result<(), TaskServiceError> {
        match status {
            TaskStatus::Pending | TaskStatus::InProgress => Ok(()),
            TaskStatus::Completed => Err(TaskServiceError::invalid_state(
                "completed tasks are terminal in this slice",
            )),
            TaskStatus::Cancelled => Err(TaskServiceError::invalid_state(
                "cancelled tasks are terminal in this slice",
            )),
        }
    }

    fn validate_status_transition(
        current: TaskStatus,
        next: TaskStatus,
    ) -> Result<(), TaskServiceError> {
        if current == next {
            return Ok(());
        }

        let allowed = matches!(
            (current, next),
            (
                TaskStatus::Pending,
                TaskStatus::InProgress | TaskStatus::Completed
            ) | (TaskStatus::InProgress, TaskStatus::Completed)
        );

        if allowed {
            Ok(())
        } else {
            Err(TaskServiceError::invalid_state(format!(
                "invalid task status transition: {} -> {}",
                current.as_str(),
                next.as_str()
            )))
        }
    }

    fn map_storage_error(error: anyhow::Error) -> TaskServiceError {
        if is_task_unsupported_error(&error) {
            return TaskServiceError::unsupported_backend(
                "persistent task tools require sqlite memory backend",
            );
        }

        TaskServiceError::storage_failure(sanitize_storage_error(&error))
    }
}
