use super::traits::{Tool, ToolResult};
use crate::config::MemoryCerebroConfig;
use crate::memory::Memory;
use crate::security::policy::ToolOperation;
use crate::security::SecurityPolicy;
use crate::tools::mcp::{cerebro, normalize};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Let the agent forget/delete a memory entry
pub struct MemoryForgetTool {
    cerebro: MemoryCerebroConfig,
    security: Arc<SecurityPolicy>,
    local: Option<Arc<dyn Memory>>,
}

impl MemoryForgetTool {
    pub fn new(cerebro: MemoryCerebroConfig, security: Arc<SecurityPolicy>) -> Self {
        Self {
            cerebro,
            security,
            local: None,
        }
    }

    pub fn with_local(memory: Arc<dyn Memory>, security: Arc<SecurityPolicy>) -> Self {
        Self {
            cerebro: MemoryCerebroConfig::default(),
            security,
            local: Some(memory),
        }
    }
}

#[async_trait]
impl Tool for MemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Remove a memory by key. Use to delete outdated facts or sensitive data. Returns whether the memory was found and removed."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "minLength": 1,
                    "description": "The key of the memory to forget"
                }
            },
            "required": ["key"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = match args
            .get("key")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => value,
            None => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing or empty 'key' parameter".into()),
                    structured: None,
                });
            }
        };

        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Act, "memory_forget")
        {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
                structured: None,
            });
        }

        if let Some(local) = &self.local {
            let deleted = local.forget(key).await?;
            let output = if deleted {
                format!("Forgot memory: {key}")
            } else {
                format!("No memory found with key: {key}")
            };
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
                error: Some("Cerebro MCP endpoint is required for memory_forget".into()),
                structured: None,
            });
        }

        let recall_adapter =
            cerebro::cerebro_tool_adapter(&self.cerebro, normalize::CEREBRO_TOOL_RECALL)?;
        let recall_payload = json!({
            "input": {
                "query": key,
                "limit": 1,
                "topic_key": key
            }
        });
        let recall = recall_adapter.execute(recall_payload).await?;
        if !recall.success {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: recall.error,
                structured: None,
            });
        }

        let resolved_id = resolve_memory_id(&recall.output)?;
        let payload = json!({
            "input": {
                "memory_id": resolved_id,
                "topic_key": key
            }
        });
        let adapter =
            cerebro::cerebro_tool_adapter(&self.cerebro, normalize::CEREBRO_TOOL_FORGET)?;
        let response = adapter.execute(payload).await?;
        if !response.success {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: response.error,
                structured: None,
            });
        }

        let output = normalize::normalize_legacy_forget_output(&response.output, key)?;
        Ok(ToolResult {
            success: true,
            output,
            error: None,
            structured: None,
        })
    }
}

fn resolve_memory_id(raw_output: &str) -> anyhow::Result<Option<String>> {
    let value: serde_json::Value = serde_json::from_str(raw_output)
        .map_err(|err| anyhow::anyhow!("invalid Cerebro response: {err}"))?;
    let memory_id = value
        .get("results")
        .and_then(serde_json::Value::as_array)
        .and_then(|results| results.first())
        .and_then(|entry| entry.get("memory_id"))
        .and_then(serde_json::Value::as_str)
        .map(|value| value.to_string());
    Ok(memory_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};

    fn test_security() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default())
    }

    #[test]
    fn name_and_schema() {
        let tool = MemoryForgetTool::new(MemoryCerebroConfig::default(), test_security());
        assert_eq!(tool.name(), "memory_forget");
        assert!(tool.parameters_schema()["properties"]["key"].is_object());
    }

    #[tokio::test]
    async fn forget_missing_key() {
        let tool = MemoryForgetTool::new(MemoryCerebroConfig::default(), test_security());
        let result = tool.execute(json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Missing or empty 'key'"));
    }

    #[tokio::test]
    async fn forget_blocked_in_readonly_mode() {
        let readonly = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tool = MemoryForgetTool::new(MemoryCerebroConfig::default(), readonly);
        let result = tool.execute(json!({"key": "temp"})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("read-only mode"));
    }

    #[tokio::test]
    async fn forget_blocked_when_rate_limited() {
        let limited = Arc::new(SecurityPolicy {
            max_actions_per_hour: 0,
            ..SecurityPolicy::default()
        });
        let tool = MemoryForgetTool::new(MemoryCerebroConfig::default(), limited);
        let result = tool.execute(json!({"key": "temp"})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Rate limit exceeded"));
    }
}
