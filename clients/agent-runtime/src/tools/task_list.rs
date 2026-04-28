use super::traits::{Tool, ToolResult};
use crate::security::{SecurityPolicy, ToolOperation};
use crate::tasks::{TaskListRequest, TaskService, TaskServiceErrorKind};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

pub struct TaskListTool {
    security: Arc<SecurityPolicy>,
    service: Arc<TaskService>,
}

impl TaskListTool {
    pub fn new(security: Arc<SecurityPolicy>, service: Arc<TaskService>) -> Self {
        Self { security, service }
    }
}

fn parse_optional_positive_u32(value: Option<&Value>, name: &str) -> Result<Option<u32>, String> {
    match value {
        None => Ok(None),
        Some(value) => {
            let number = value
                .as_u64()
                .ok_or_else(|| format!("{name} must be a positive integer"))?;
            if number == 0 {
                return Err(format!("{name} must be a positive integer"));
            }
            u32::try_from(number)
                .map(Some)
                .map_err(|_| format!("{name} is too large"))
        }
    }
}

fn parse_optional_non_negative_u32(
    value: Option<&Value>,
    name: &str,
) -> Result<Option<u32>, String> {
    match value {
        None => Ok(None),
        Some(value) => {
            let number = value
                .as_u64()
                .ok_or_else(|| format!("{name} must be a non-negative integer"))?;
            u32::try_from(number)
                .map(Some)
                .map_err(|_| format!("{name} is too large"))
        }
    }
}

fn tool_error(kind: TaskServiceErrorKind, error: impl Into<String>) -> ToolResult {
    let error = error.into();
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(error.clone()),
        structured: Some(json!({ "error": { "message": error, "kind": kind.as_str() } })),
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }

    fn description(&self) -> &str {
        "List persistent runtime tasks with basic filters and pagination."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"] },
                "priority": { "type": "string", "enum": ["low", "medium", "high"] },
                "session_id": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1 },
                "offset": { "type": "integer", "minimum": 0 }
            }
        })
    }

    fn spec(&self) -> super::traits::ToolSpec {
        super::traits::ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
            source: None,
            aliases: vec!["task_list".to_string()],
        }
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Read, self.name())
        {
            return Ok(tool_error(TaskServiceErrorKind::Validation, error));
        }
        let object = match args.as_object() {
            Some(object) => object,
            None => {
                return Ok(tool_error(
                    TaskServiceErrorKind::Validation,
                    "Tool arguments must be a JSON object",
                ))
            }
        };
        if let Some(unexpected) = object.keys().find(|key| {
            !matches!(
                key.as_str(),
                "status" | "priority" | "session_id" | "limit" | "offset"
            )
        }) {
            return Ok(tool_error(
                TaskServiceErrorKind::Validation,
                format!("Unknown parameter: {unexpected}"),
            ));
        }

        let status = match object
            .get("status")
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "status must be a string".to_string())
                    .and_then(|raw| {
                        crate::memory::TaskStatus::from_str(raw).map_err(|_| {
                            "status must be one of: pending, in_progress, completed, cancelled"
                                .to_string()
                        })
                    })
            })
            .transpose()
        {
            Ok(status) => status,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };
        let priority = match object
            .get("priority")
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "priority must be a string".to_string())
                    .and_then(|raw| {
                        crate::memory::TaskPriority::from_str(raw)
                            .map_err(|_| "priority must be one of: low, medium, high".to_string())
                    })
            })
            .transpose()
        {
            Ok(priority) => priority,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };
        let session_id = match object
            .get("session_id")
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "session_id must be a string".to_string())
                    .map(ToString::to_string)
            })
            .transpose()
        {
            Ok(session_id) => session_id,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };
        let limit = match parse_optional_positive_u32(object.get("limit"), "limit") {
            Ok(limit) => limit,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };
        let offset = match parse_optional_non_negative_u32(object.get("offset"), "offset") {
            Ok(offset) => offset,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };

        match self
            .service
            .list_tasks(TaskListRequest {
                status,
                priority,
                session_id,
                limit,
                offset,
                caller_scope_key: None,
            })
            .await
        {
            Ok(page) => Ok(ToolResult {
                success: true,
                output: format!("Listed {} task(s)", page.tasks.len()),
                error: None,
                structured: Some(json!({
                    "tasks": page.tasks,
                    "applied_limit": page.applied_limit,
                    "applied_offset": page.applied_offset,
                    "has_more": page.has_more,
                })),
            }),
            Err(error) => Ok(tool_error(error.kind, error.message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::test_helpers::task_tool_test_context;

    #[test]
    fn task_list_spec_exposes_snake_case_alias() {
        let (_dir, security, service) = task_tool_test_context();
        let tool = TaskListTool::new(security, service);
        let spec = tool.spec();
        assert_eq!(spec.name, "TaskList");
        assert_eq!(spec.aliases, vec!["task_list"]);
    }
}
