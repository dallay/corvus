use super::traits::{Tool, ToolResult};
use crate::config::MemoryCerebroConfig;
use crate::memory::Memory;
use crate::security::egress::enforce_cerebro_egress;
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
            return match local.forget(key).await {
                Ok(deleted) => {
                    let output = if deleted {
                        format!("Forgot memory: {key}")
                    } else {
                        format!("No memory found with key: {key}")
                    };
                    Ok(ToolResult {
                        success: true,
                        output,
                        error: None,
                        structured: None,
                    })
                }
                Err(error) => Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                    structured: None,
                }),
            };
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
                    error: Some("Cerebro MCP endpoint is required for memory_forget".into()),
                    structured: None,
                });
            }
        };

        if let Err(error) = enforce_cerebro_egress(endpoint, &self.cerebro, ToolOperation::Act) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
                structured: None,
            });
        }

        let recall_adapter =
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
        let recall_payload = json!({
            "input": {
                "query": key,
                "limit": 1,
                "topic_key": key
            }
        });
        let recall = match recall_adapter.execute(recall_payload).await {
            Ok(recall) => recall,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                    structured: None,
                });
            }
        };
        if !recall.success {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: recall.error,
                structured: None,
            });
        }

        let resolved_id = match resolve_memory_id(&recall.output) {
            Ok(value) => value,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                    structured: None,
                });
            }
        };
        let Some(resolved_id) = resolved_id else {
            return Ok(ToolResult {
                success: true,
                output: format!("No memory found with key: {key}"),
                error: None,
                structured: None,
            });
        };
        let payload = json!({
            "input": {
                "memory_id": resolved_id,
                "topic_key": key
            }
        });
        let adapter =
            match cerebro::cerebro_tool_adapter(&self.cerebro, normalize::CEREBRO_TOOL_FORGET) {
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

        let output = match normalize::normalize_legacy_forget_output(&response.output, key) {
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

fn resolve_memory_id(raw_output: &str) -> anyhow::Result<Option<String>> {
    let value: serde_json::Value = serde_json::from_str(raw_output)
        .map_err(|err| anyhow::anyhow!("invalid Cerebro response: {err}"))?;
    let results = value
        .get("results")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Cerebro response missing results"))?;
    let memory_id = results
        .first()
        .and_then(|entry| entry.get("memory_id"))
        .and_then(serde_json::Value::as_str)
        .map(|value| value.to_string());
    Ok(memory_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use axum::routing::post;
    use axum::{Json, Router};
    use tokio::net::TcpListener;

    struct MockMemory {
        deleted: bool,
    }

    #[async_trait]
    impl Memory for MockMemory {
        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: crate::memory::MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
            Ok(vec![])
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<crate::memory::MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&crate::memory::MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
            Ok(vec![])
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(self.deleted)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    fn test_security() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default())
    }

    fn test_cerebro_config(endpoint: String) -> MemoryCerebroConfig {
        MemoryCerebroConfig {
            endpoint: Some(endpoint),
            auth_token: Some("test-token".to_string()),
            request_timeout_ms: 5_000,
            allow_insecure_loopback: true,
        }
    }

    async fn handler_success(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
        let tool = payload
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("unknown");
        let output = match tool {
            "mem_search" => json!({
                "results": [{
                    "memory_id": "mem-1",
                    "summary": "prefers rust",
                    "topic_key": "lang",
                    "score": 0.9
                }]
            }),
            "mem_delete" => json!({ "deleted": true }),
            _ => json!({ "error": "unexpected tool" }),
        };

        Json(json!({
            "jsonrpc": "2.0",
            "id": payload.get("id").cloned().unwrap_or(json!("1")),
            "result": { "output": output }
        }))
    }

    async fn handler_bad_recall(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
        Json(json!({
            "jsonrpc": "2.0",
            "id": payload.get("id").cloned().unwrap_or(json!("1")),
            "result": { "output": "not-json" }
        }))
    }

    async fn start_mock_server_success() -> String {
        let app = Router::new().route("/mcp", post(handler_success));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}/mcp")
    }

    async fn start_mock_server_bad_recall() -> String {
        let app = Router::new().route("/mcp", post(handler_bad_recall));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}/mcp")
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

    #[tokio::test]
    async fn forget_local_success() {
        let memory = Arc::new(MockMemory { deleted: true });
        let tool = MemoryForgetTool::with_local(memory, test_security());
        let result = tool.execute(json!({"key": "temp"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Forgot memory: temp"));
    }

    #[tokio::test]
    async fn forget_missing_endpoint() {
        let tool = MemoryForgetTool::new(MemoryCerebroConfig::default(), test_security());
        let result = tool.execute(json!({"key": "temp"})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Cerebro MCP endpoint"));
    }

    #[tokio::test]
    async fn forget_recall_malformed_response() {
        let endpoint = start_mock_server_bad_recall().await;
        let tool = MemoryForgetTool::new(test_cerebro_config(endpoint), test_security());
        let result = tool.execute(json!({"key": "temp"})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("invalid Cerebro response"));
    }

    #[tokio::test]
    async fn forget_cerebro_success() {
        let endpoint = start_mock_server_success().await;
        let tool = MemoryForgetTool::new(test_cerebro_config(endpoint), test_security());
        let result = tool.execute(json!({"key": "lang"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Forgot memory: lang"));
    }
}
