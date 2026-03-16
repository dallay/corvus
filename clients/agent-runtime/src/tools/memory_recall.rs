use super::traits::{Tool, ToolResult};
use crate::config::MemoryCerebroConfig;
use crate::tools::mcp::{cerebro, normalize};
use async_trait::async_trait;
use serde_json::json;

/// Let the agent search its own memory
pub struct MemoryRecallTool {
    cerebro: MemoryCerebroConfig,
}

impl MemoryRecallTool {
    pub fn new(cerebro: MemoryCerebroConfig) -> Self {
        Self { cerebro }
    }
}

#[async_trait]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Search long-term memory for relevant facts, preferences, or context. Returns scored results ranked by relevance."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keywords or phrase to search for in memory"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map_or(5, |v| v as usize);

        let endpoint = self
            .cerebro
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if endpoint.is_none() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Cerebro MCP endpoint is required for memory_recall".into()),
                structured: None,
            });
        }

        let adapter = cerebro::cerebro_tool_adapter(&self.cerebro, normalize::CEREBRO_TOOL_RECALL)?;
        let payload = json!({
            "input": {
                "query": query,
                "limit": limit
            }
        });
        let response = adapter.execute(payload).await?;
        if !response.success {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: response.error,
                structured: None,
            });
        }

        let output = normalize::normalize_legacy_recall_output(&response.output)?;
        Ok(ToolResult {
            success: true,
            output,
            error: None,
            structured: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recall_missing_query() {
        let tool = MemoryRecallTool::new(MemoryCerebroConfig::default());
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn name_and_schema() {
        let tool = MemoryRecallTool::new(MemoryCerebroConfig::default());
        assert_eq!(tool.name(), "memory_recall");
        assert!(tool.parameters_schema()["properties"]["query"].is_object());
    }
}
