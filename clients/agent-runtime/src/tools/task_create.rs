use super::traits::{Tool, ToolResult};
use crate::security::{SecurityPolicy, ToolOperation};
use crate::tasks::{TaskCreateRequest, TaskService, TaskServiceErrorKind};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

pub struct TaskCreateTool {
    security: Arc<SecurityPolicy>,
    service: Arc<TaskService>,
}

impl TaskCreateTool {
    pub fn new(security: Arc<SecurityPolicy>, service: Arc<TaskService>) -> Self {
        Self { security, service }
    }
}

fn parse_request(args: &Value) -> Result<TaskCreateRequest, String> {
    let object = args
        .as_object()
        .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;
    if let Some(unexpected) = object.keys().find(|key| {
        !matches!(
            key.as_str(),
            "title" | "description" | "priority" | "session_id"
        )
    }) {
        return Err(format!("Unknown parameter: {unexpected}"));
    }

    let title = object
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing required parameter: title".to_string())?
        .to_string();
    let description = object
        .get("description")
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "description must be a string".to_string())
                .map(ToString::to_string)
        })
        .transpose()?;
    let priority = object
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
        .transpose()?;
    let session_id = object
        .get("session_id")
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "session_id must be a string".to_string())
                .map(ToString::to_string)
        })
        .transpose()?;

    Ok(TaskCreateRequest {
        title,
        description,
        priority,
        session_id,
        caller_scope_key: None,
    })
}

fn tool_error(kind: TaskServiceErrorKind, error: impl Into<String>) -> ToolResult {
    let error = error.into();
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(error.clone()),
        structured: Some(json!({
            "error": {
                "message": error,
                "kind": kind.as_str(),
            }
        })),
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }

    fn description(&self) -> &str {
        "Create a persistent runtime task with optional session linkage."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "title": { "type": "string" },
                "description": { "type": "string" },
                "priority": { "type": "string", "enum": ["low", "medium", "high"] },
                "session_id": { "type": "string" }
            },
            "required": ["title"]
        })
    }

    fn spec(&self) -> super::traits::ToolSpec {
        super::traits::ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
            source: None,
            aliases: vec!["task_create".to_string()],
        }
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Act, self.name())
        {
            return Ok(tool_error(TaskServiceErrorKind::Validation, error));
        }

        let request = match parse_request(&args) {
            Ok(request) => request,
            Err(error) => return Ok(tool_error(TaskServiceErrorKind::Validation, error)),
        };

        match self.service.create_task(request).await {
            Ok(task) => Ok(ToolResult {
                success: true,
                output: format!("Created task {}", task.id),
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
    fn task_create_spec_exposes_snake_case_alias() {
        let (_dir, security, service) = task_tool_test_context();
        let tool = TaskCreateTool::new(security, service);
        let spec = tool.spec();
        assert_eq!(spec.name, "TaskCreate");
        assert_eq!(spec.aliases, vec!["task_create"]);
    }
}
