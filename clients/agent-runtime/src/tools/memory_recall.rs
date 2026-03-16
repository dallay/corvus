use super::traits::{Tool, ToolResult};
use crate::config::MemoryCerebroConfig;
use crate::memory::{Memory, MemoryEntry};
use crate::tools::mcp::{cerebro, normalize};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Let the agent search its own memory
pub struct MemoryRecallTool {
    cerebro: MemoryCerebroConfig,
    local: Option<Arc<dyn Memory>>,
}

impl MemoryRecallTool {
    pub fn new(cerebro: MemoryCerebroConfig) -> Self {
        Self {
            cerebro,
            local: None,
        }
    }

    pub fn with_local(memory: Arc<dyn Memory>) -> Self {
        Self {
            cerebro: MemoryCerebroConfig::default(),
            local: Some(memory),
        }
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
                    "minLength": 1,
                    "description": "Keywords or phrase to search for in memory"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "description": "Max results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = match args
            .get("query")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => value,
            None => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing or empty 'query' parameter".into()),
                    structured: None,
                });
            }
        };

        let limit = match args.get("limit") {
            None => 5,
            Some(value) => match value.as_i64() {
                Some(parsed) if (1..=100).contains(&parsed) => {
                    match usize::try_from(parsed) {
                        Ok(value) => value,
                        Err(_) => {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some("'limit' must be a positive integer".into()),
                                structured: None,
                            });
                        }
                    }
                }
                Some(_) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("'limit' must be between 1 and 100".into()),
                        structured: None,
                    });
                }
                None => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("'limit' must be an integer".into()),
                        structured: None,
                    });
                }
            },
        };

        if let Some(local) = &self.local {
            let entries = local.recall(query, limit, None).await?;
            let output = format_local_recall_output(entries);
            return Ok(ToolResult {
                success: true,
                output,
                error: None,
                structured: None,
            });
        }

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

fn format_local_recall_output(entries: Vec<MemoryEntry>) -> String {
    if entries.is_empty() {
        return "No memories found matching that query.".to_string();
    }
    let mut output = format!("Found {} memories:\n", entries.len());
    for entry in entries {
        let _ = std::fmt::Write::write_fmt(
            &mut output,
            format_args!("- [local] {}: {}\n", entry.key, entry.content),
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recall_missing_query() {
        let tool = MemoryRecallTool::new(MemoryCerebroConfig::default());
        let result = tool.execute(json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Missing or empty 'query'"));
    }

    #[test]
    fn name_and_schema() {
        let tool = MemoryRecallTool::new(MemoryCerebroConfig::default());
        assert_eq!(tool.name(), "memory_recall");
        assert!(tool.parameters_schema()["properties"]["query"].is_object());
    }
}
