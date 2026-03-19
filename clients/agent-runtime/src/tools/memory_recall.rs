use super::traits::{Tool, ToolResult};
use crate::config::MemoryCerebroConfig;
use crate::memory::{Memory, MemoryEntry};
use crate::security::egress::enforce_cerebro_egress;
use crate::security::policy::ToolOperation;
use crate::security::SecurityPolicy;
use crate::tools::mcp::{cerebro, normalize};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Let the agent search its own memory
pub struct MemoryRecallTool {
    cerebro: MemoryCerebroConfig,
    local: Option<Arc<dyn Memory>>,
    security: Arc<SecurityPolicy>,
}

impl MemoryRecallTool {
    pub fn new(cerebro: MemoryCerebroConfig, security: Arc<SecurityPolicy>) -> Self {
        Self {
            cerebro,
            local: None,
            security,
        }
    }

    pub fn with_local(memory: Arc<dyn Memory>, security: Arc<SecurityPolicy>) -> Self {
        Self {
            cerebro: MemoryCerebroConfig::default(),
            local: Some(memory),
            security,
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
                Some(parsed) if (1..=100).contains(&parsed) => match usize::try_from(parsed) {
                    Ok(value) => value,
                    Err(_) => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("'limit' must be a positive integer".into()),
                            structured: None,
                        });
                    }
                },
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

        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Read, "memory_recall")
        {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
                structured: None,
            });
        }

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

        let endpoint = match endpoint {
            Some(value) => value,
            None => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Cerebro MCP endpoint is required for memory_recall".into()),
                    structured: None,
                });
            }
        };

        if let Err(error) = enforce_cerebro_egress(endpoint, &self.cerebro, ToolOperation::Read) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
                structured: None,
            });
        }

        let adapter =
            match cerebro::cerebro_tool_adapter(&self.cerebro, normalize::CEREBRO_TOOL_RECALL) {
                Ok(adapter) => adapter,
                Err(error) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(error.to_string()),
                        structured: None,
                    });
                }
            };
        let payload = json!({
            "input": {
                "query": query,
                "limit": limit
            }
        });
        let response = match adapter.execute(payload).await {
            Ok(response) => response,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                    structured: None,
                });
            }
        };
        if !response.success {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: response.error,
                structured: None,
            });
        }

        let output = match normalize::normalize_legacy_recall_output(&response.output) {
            Ok(output) => output,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                    structured: None,
                });
            }
        };
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
    use crate::security::SecurityPolicy;
    #[tokio::test]
    async fn recall_missing_query() {
        let tool = MemoryRecallTool::new(
            MemoryCerebroConfig::default(),
            Arc::new(SecurityPolicy::default()),
        );
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
        let tool = MemoryRecallTool::new(
            MemoryCerebroConfig::default(),
            Arc::new(SecurityPolicy::default()),
        );
        assert_eq!(tool.name(), "memory_recall");
        assert!(tool.parameters_schema()["properties"]["query"].is_object());
    }
}
