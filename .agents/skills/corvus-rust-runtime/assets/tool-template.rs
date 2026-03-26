use async_trait::async_trait;

pub struct ExampleTool;

#[async_trait]
impl Tool for ExampleTool {
    async fn execute(&self, input: ToolInput) -> Result<ToolResult, ToolError> {
        // validate and sanitize inputs
        // enforce policy/sandbox rules
        // execute minimal behavior
        Err(ToolError::InvalidInput {
            reason: "template implementation required".to_string(),
        })
    }
}
