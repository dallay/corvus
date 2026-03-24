use super::memory_helpers::{err_result, extract_trimmed_str, validated_endpoint};
use super::traits::{Tool, ToolResult};
use crate::config::MemoryCerebroConfig;
use crate::memory::{Memory, MemoryCategory};
use crate::security::egress::enforce_cerebro_egress;
use crate::security::policy::ToolOperation;
use crate::security::SecurityPolicy;
use crate::tools::mcp::{cerebro, normalize};
use async_trait::async_trait;
use regex::RegexSet;
use serde_json::json;
use std::sync::Arc;
use std::sync::OnceLock;

const SENSITIVE_DATA_ERROR: &str = "Sensitive data is not allowed to be stored in Cerebro. Remove secrets, credentials, PII, or ephemeral context before calling memory_store.";

fn contains_sensitive_data(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let normalized: String = lower.chars().filter(|ch| !ch.is_whitespace()).collect();

    if contains_sensitive_labels(&normalized) {
        return true;
    }

    if looks_like_api_key(&lower) {
        return true;
    }

    if sensitive_regex_set().is_match(content) {
        return true;
    }

    false
}

fn contains_sensitive_labels(normalized: &str) -> bool {
    const LABELS: [&str; 16] = [
        "password:",
        "password=",
        "passwd:",
        "passwd=",
        "passphrase:",
        "passphrase=",
        "apikey:",
        "apikey=",
        "api_key:",
        "api_key=",
        "token:",
        "token=",
        "accesskey:",
        "accesskey=",
        "access_key:",
        "access_key=",
    ];

    LABELS.iter().any(|label| normalized.contains(label))
}

fn looks_like_api_key(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let mut idx = 0;
    while idx + 3 < bytes.len() {
        if bytes[idx] == b's' && bytes[idx + 1] == b'k' && bytes[idx + 2] == b'-' {
            let mut count = 0;
            for &byte in &bytes[idx + 3..] {
                if byte.is_ascii_alphanumeric() {
                    count += 1;
                    if count >= 20 {
                        return true;
                    }
                } else {
                    break;
                }
            }
        }
        idx += 1;
    }

    false
}

fn sensitive_regex_set() -> &'static RegexSet {
    static REGEXES: OnceLock<RegexSet> = OnceLock::new();
    REGEXES.get_or_init(|| {
        RegexSet::new([
            r"(?i)\bpassword\s*[:=]",
            r"(?i)\bpassphrase\s*[:=]",
            r"(?i)\bapi[_-]?key\s*[:=]",
            r"(?i)\baccess[_-]?key\s*[:=]",
            r"(?i)\bsecret\s*[:=]",
            r"(?i)\btoken\s*[:=]",
            r"(?i)authorization\s*:\s*bearer\s+[A-Za-z0-9\-_.=]+",
            r"(?i)\bbearer\s+[A-Za-z0-9\-_.=]+",
            r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
            r"\bAKIA[0-9A-Z]{16}\b",
            r"\bASIA[0-9A-Z]{16}\b",
            r"\bghp_[A-Za-z0-9]{20,}\b",
            r"\bxox[baprs]-[A-Za-z0-9-]+\b",
            r"(?i)-----BEGIN (OPENSSH|RSA|EC|PGP|PRIVATE) KEY-----",
            r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b",
            r"\b\+?\d[\d\s().-]{7,}\d\b",
            r#"(?i)"(password|passphrase|api[_-]?key|access[_-]?key|secret|token)"\s*:\s*"[^"]+""#,
        ])
        .expect("invalid sensitive data regex set")
    })
}

/// Parse a category string into `MemoryCategory`.
/// Returns `Err` for `None` or whitespace-only input.
fn parse_category(cat: Option<&str>) -> Result<MemoryCategory, String> {
    let s = cat
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "Missing or empty 'category' parameter".to_string())?;
    match s.to_ascii_lowercase().as_str() {
        "core" => Ok(MemoryCategory::Core),
        "daily" => Ok(MemoryCategory::Daily),
        "conversation" => Ok(MemoryCategory::Conversation),
        _ => Ok(MemoryCategory::Custom(s.to_string())),
    }
}

/// Store via local memory backend.
async fn store_local(
    local: &Arc<dyn Memory>,
    key: &str,
    content: &str,
    category: MemoryCategory,
) -> anyhow::Result<ToolResult> {
    match local.store(key, content, category, None).await {
        Ok(()) => Ok(ToolResult {
            success: true,
            output: format!("Stored memory: {key}"),
            error: None,
            structured: None,
        }),
        Err(error) => Ok(err_result(&error.to_string())),
    }
}

/// Store via Cerebro MCP.
async fn store_via_cerebro(
    cerebro: &MemoryCerebroConfig,
    key: &str,
    content: &str,
    category: &MemoryCategory,
) -> anyhow::Result<ToolResult> {
    let adapter = match cerebro::cerebro_tool_adapter(cerebro, normalize::CEREBRO_TOOL_STORE) {
        Ok(a) => a,
        Err(e) => return Ok(err_result(&format!("Cerebro store adapter error: {e}"))),
    };
    let payload = json!({
        "input": {
            "scope": "shared",
            "topic_key": key,
            "observation": {
                "content": content,
                "metadata": {
                    "legacy_category": category.to_string()
                }
            }
        }
    });
    let response = match adapter.execute(payload).await {
        Ok(r) => r,
        Err(e) => return Ok(err_result(&e.to_string())),
    };
    if response.success {
        return Ok(ToolResult {
            success: true,
            output: format!("Stored memory: {key}"),
            error: None,
            structured: None,
        });
    }
    Ok(ToolResult {
        success: false,
        output: String::new(),
        error: response.error,
        structured: None,
    })
}

/// Let the agent store memories — its own brain writes
pub struct MemoryStoreTool {
    cerebro: MemoryCerebroConfig,
    security: Arc<SecurityPolicy>,
    local: Option<Arc<dyn Memory>>,
}

impl MemoryStoreTool {
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
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store a fact, preference, or note in long-term memory. Use category 'core' for permanent facts, 'daily' for session notes, 'conversation' for chat context, or a custom category name."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Unique key for this memory (e.g. 'user_lang', 'project_stack')"
                },
                "content": {
                    "type": "string",
                    "description": "The information to remember"
                },
                "category": {
                    "type": "string",
                    "description": "Memory category: 'core' (permanent), 'daily' (session), 'conversation' (chat), or a custom category name. Defaults to 'core'."
                }
            },
            "required": ["key", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = match extract_trimmed_str(&args, "key") {
            Some(v) => v,
            None => return Ok(err_result("Missing or empty 'key' parameter")),
        };

        if contains_sensitive_data(key) {
            return Ok(err_result(SENSITIVE_DATA_ERROR));
        }

        let content = match extract_trimmed_str(&args, "content") {
            Some(v) => v,
            None => return Ok(err_result("Missing or empty 'content' parameter")),
        };

        let category = match parse_category(args.get("category").and_then(|v| v.as_str())) {
            Ok(c) => c,
            Err(e) => return Ok(err_result(&e)),
        };

        if contains_sensitive_data(content) {
            return Ok(err_result(SENSITIVE_DATA_ERROR));
        }

        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Act, "memory_store")
        {
            return Ok(err_result(&error));
        }

        if let Some(local) = &self.local {
            return store_local(local, key, content, category).await;
        }

        let endpoint = match validated_endpoint(&self.cerebro, "memory_store") {
            Ok(ep) => ep,
            Err(r) => return Ok(r),
        };

        if let Err(error) = enforce_cerebro_egress(endpoint, &self.cerebro, ToolOperation::Act) {
            return Ok(err_result(&error.to_string()));
        }

        store_via_cerebro(&self.cerebro, key, content, &category).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use axum::extract::State;
    use axum::routing::post;
    use axum::{Json, Router};
    use std::sync::Mutex;
    use tokio::net::TcpListener;

    fn test_security() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default())
    }

    fn test_cerebro_config() -> MemoryCerebroConfig {
        MemoryCerebroConfig {
            endpoint: Some("http://127.0.0.1:3001/mcp".to_string()),
            auth_token: Some("test-token".to_string()),
            request_timeout_ms: 5_000,
            allow_insecure_loopback: true,
        }
    }

    async fn mock_handler(
        State(calls): State<Arc<Mutex<Vec<String>>>>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let tool = payload
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or("unknown")
            .to_string();
        calls.lock().unwrap().push(tool);

        Json(json!({
            "jsonrpc": "2.0",
            "id": payload.get("id").cloned().unwrap_or(json!("1")),
            "result": { "output": { "memory_id": "topic-key", "status": "saved" } }
        }))
    }

    async fn start_mock_server() -> (String, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/mcp", post(mock_handler))
            .with_state(calls.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{addr}/mcp"), calls)
    }

    #[test]
    fn name_and_schema() {
        let tool = MemoryStoreTool::new(MemoryCerebroConfig::default(), test_security());
        assert_eq!(tool.name(), "memory_store");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["key"].is_object());
        assert!(schema["properties"]["content"].is_object());
    }

    #[tokio::test]
    async fn store_missing_key() {
        let tool = MemoryStoreTool::new(MemoryCerebroConfig::default(), test_security());
        let result = tool.execute(json!({"content": "no key"})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Missing or empty 'key'"));
    }

    #[tokio::test]
    async fn store_missing_content() {
        let tool = MemoryStoreTool::new(MemoryCerebroConfig::default(), test_security());
        let result = tool.execute(json!({"key": "no_content"})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Missing or empty 'content'"));
    }

    #[tokio::test]
    async fn store_rejects_insecure_non_loopback_endpoint() {
        let cerebro = MemoryCerebroConfig {
            endpoint: Some("http://cerebro.example.com/mcp".to_string()),
            auth_token: Some("token".to_string()),
            request_timeout_ms: 5_000,
            allow_insecure_loopback: true,
        };
        let tool = MemoryStoreTool::new(cerebro, test_security());
        let result = tool
            .execute(json!({"key": "lang", "content": "Prefers Rust"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap_or_default().contains("loopback"));
    }

    #[tokio::test]
    async fn store_blocked_in_readonly_mode() {
        let readonly = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tool = MemoryStoreTool::new(test_cerebro_config(), readonly);
        let result = tool
            .execute(json!({"key": "lang", "content": "Prefers Rust"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("read-only mode"));
    }

    #[tokio::test]
    async fn store_blocked_when_rate_limited() {
        let limited = Arc::new(SecurityPolicy {
            max_actions_per_hour: 0,
            ..SecurityPolicy::default()
        });
        let tool = MemoryStoreTool::new(test_cerebro_config(), limited);
        let result = tool
            .execute(json!({"key": "lang", "content": "Prefers Rust"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Rate limit exceeded"));
    }

    #[tokio::test]
    async fn store_blocks_api_key_pattern() {
        let tool = MemoryStoreTool::new(MemoryCerebroConfig::default(), test_security());
        let token = format!("{}{}{}", "sk-", "1234567890abcdef", "1234567890");
        let result = tool
            .execute(json!({"key": "token", "content": format!("api_key: {token}")}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("Sensitive data is not allowed"));
    }

    #[tokio::test]
    async fn store_blocks_password_label() {
        let tool = MemoryStoreTool::new(MemoryCerebroConfig::default(), test_security());
        let result = tool
            .execute(json!({"key": "login", "content": "password: hunter2"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("Sensitive data is not allowed"));
    }

    #[tokio::test]
    async fn store_allows_non_sensitive_preference() {
        let (endpoint, calls) = start_mock_server().await;
        let cerebro = MemoryCerebroConfig {
            endpoint: Some(endpoint),
            auth_token: Some("token".into()),
            request_timeout_ms: 5_000,
            allow_insecure_loopback: true,
        };
        let tool = MemoryStoreTool::new(cerebro, test_security());
        let result = tool
            .execute(json!({"key": "lang", "content": "Prefers Rust"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(calls.lock().unwrap().as_slice(), ["mem_save"]);
    }
}
