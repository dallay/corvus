use super::traits::{Tool, ToolResult};
use crate::security::{SecurityPolicy, ToolOperation};
use crate::tasks::{TaskService, TaskServiceErrorKind};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct TaskGetTool {
    security: Arc<SecurityPolicy>,
    service: Arc<TaskService>,
}

impl TaskGetTool {
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
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }

    fn description(&self) -> &str {
        "Fetch a persistent runtime task by UUID."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        })
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
        if let Some(unexpected) = object.keys().find(|key| key.as_str() != "id") {
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

        match self.service.get_task(id, None).await {
            Ok(task) => Ok(ToolResult {
                success: true,
                output: format!("Fetched task {}", task.id),
                error: None,
                structured: Some(json!({ "task": task })),
            }),
            Err(error) => Ok(tool_error(error.kind, error.message)),
        }
    }
}
