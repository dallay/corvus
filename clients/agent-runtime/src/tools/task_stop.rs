use super::traits::{Tool, ToolResult};
use crate::security::{SecurityPolicy, ToolOperation};
use crate::tasks::{TaskService, TaskServiceErrorKind};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct TaskStopTool {
    security: Arc<SecurityPolicy>,
    service: Arc<TaskService>,
}

impl TaskStopTool {
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
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "TaskStop"
    }

    fn description(&self) -> &str {
        "Cancel a persistent runtime task."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        })
    }

    fn spec(&self) -> super::traits::ToolSpec {
        super::traits::ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
            source: None,
            aliases: vec!["task_stop".to_string()],
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

        match self.service.stop_task(id, None).await {
            Ok(task) => Ok(ToolResult {
                success: true,
                output: format!("Cancelled task {}", task.id),
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
    use crate::memory::SqliteMemory;
    use crate::security::AutonomyLevel;
    use crate::tasks::TaskService;
    use tempfile::TempDir;

    #[test]
    fn task_stop_spec_exposes_snake_case_alias() {
        let dir = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: dir.path().to_path_buf(),
            ..SecurityPolicy::default()
        });
        let memory = Arc::new(SqliteMemory::new(dir.path()).unwrap());
        let service = Arc::new(TaskService::new(memory));
        let tool = TaskStopTool::new(security, service);
        let spec = tool.spec();
        assert_eq!(spec.name, "TaskStop");
        assert_eq!(spec.aliases, vec!["task_stop"]);
    }
}
