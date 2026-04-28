use super::traits::{Tool, ToolResult};
use crate::security::{SecurityPolicy, ToolOperation};
use crate::tasks::{TaskService, TaskServiceErrorKind, TaskUpdateRequest};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

pub struct TaskUpdateTool {
    security: Arc<SecurityPolicy>,
    service: Arc<TaskService>,
}

impl TaskUpdateTool {
    pub fn new(security: Arc<SecurityPolicy>, service: Arc<TaskService>) -> Self {
        Self { security, service }
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
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }

    fn description(&self) -> &str {
        "Update mutable fields on a persistent runtime task."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "priority": { "type": "string", "enum": ["low", "medium", "high"] },
                "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"] }
            },
            "required": ["id"]
        })
    }

    fn spec(&self) -> super::traits::ToolSpec {
        super::traits::ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
            source: None,
            aliases: vec!["task_update".to_string()],
        }
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Act, self.name())
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
                "id" | "title" | "description" | "priority" | "status"
            )
        }) {
            return Ok(tool_error(
                TaskServiceErrorKind::Validation,
                format!("Unknown parameter: {unexpected}"),
            ));
        }

        let Some(id) = object.get("id").and_then(Value::as_str) else {
            return Ok(tool_error(
                TaskServiceErrorKind::Validation,
                "Missing required parameter: id",
            ));
        };
        let title = match object
            .get("title")
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "title must be a string".to_string())
                    .map(ToString::to_string)
            })
            .transpose()
        {
            Ok(title) => title,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };
        let description = match object
            .get("description")
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "description must be a string".to_string())
                    .map(ToString::to_string)
            })
            .transpose()
        {
            Ok(description) => description,
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
        match self
            .service
            .update_task(TaskUpdateRequest {
                id: id.to_string(),
                title,
                description,
                priority,
                status,
                session_id: None,
                caller_scope_key: None,
            })
            .await
        {
            Ok(task) => Ok(ToolResult {
                success: true,
                output: format!("Updated task {}", task.id),
                error: None,
                structured: Some(json!({ "task": task })),
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
    fn task_update_spec_exposes_snake_case_alias() {
        let (_dir, security, service) = task_tool_test_context();
        let tool = TaskUpdateTool::new(security, service);
        let spec = tool.spec();
        assert_eq!(spec.name, "TaskUpdate");
        assert_eq!(spec.aliases, vec!["task_update"]);
    }
}
