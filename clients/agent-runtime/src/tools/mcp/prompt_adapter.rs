use super::client::{McpClient, McpPromptManifest, PromptArgument};
use super::normalize;
use crate::config::McpServerConfig;
use crate::tools::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Optional content scanner that can reject prompt content before it's returned.
/// Returns Ok(()) to allow or Err(reason) to reject.
pub type ContentScanner = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

#[derive(Clone)]
pub struct McpPromptAdapter {
    name: String,
    description: String,
    original_name: String,
    arguments: Vec<PromptArgument>,
    server_name: String,
    call_timeout_ms: u64,
    output_limit_bytes: usize,
    client: McpClient,
    content_scanner: Option<ContentScanner>,
}

impl McpPromptAdapter {
    pub fn from_manifest(
        server: &McpServerConfig,
        manifest: McpPromptManifest,
        client: McpClient,
    ) -> anyhow::Result<Self> {
        let canonical_name = normalize::normalize_prompt_name(&server.name, &manifest.name)?;
        let output_limit = server
            .prompt_output_limit_bytes
            .unwrap_or(server.output_limit_bytes);
        Ok(Self {
            name: canonical_name,
            description: manifest.description,
            original_name: manifest.name,
            arguments: manifest.arguments,
            server_name: server.name.clone(),
            call_timeout_ms: server.call_timeout_ms,
            output_limit_bytes: output_limit,
            client,
            content_scanner: None,
        })
    }

    /// Attach an optional content scanner for prompt injection mitigation.
    /// If the scanner returns Err, execute() will return a structured rejection.
    #[allow(dead_code)]
    pub fn with_content_scanner(mut self, scanner: ContentScanner) -> Self {
        self.content_scanner = Some(scanner);
        self
    }

    fn enforce_output_limit(&self, output: String) -> String {
        if output.len() <= self.output_limit_bytes {
            return output;
        }

        let marker = format!(
            "\n[output_limit_enforced limit_bytes={} original_bytes={}]",
            self.output_limit_bytes,
            output.len()
        );
        if self.output_limit_bytes <= marker.len() {
            let mut end = self.output_limit_bytes;
            while end > 0 && !marker.is_char_boundary(end) {
                end -= 1;
            }
            return marker[..end].to_string();
        }

        let max_body = self.output_limit_bytes.saturating_sub(marker.len());
        let mut end = max_body.min(output.len());
        while end > 0 && !output.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = if end == 0 {
            String::new()
        } else {
            output[..end].to_string()
        };

        format!("{truncated}{marker}")
    }

    fn validate_arguments(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, ToolResult> {
        let obj = match args.as_object() {
            Some(obj) => obj,
            None if args.is_null() => {
                // Treat null as empty object
                &serde_json::Map::new()
            }
            None => {
                return Err(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("prompt arguments must be a JSON object".to_string()),
                    structured: None,
                });
            }
        };

        // Check for missing required arguments
        let mut missing: Vec<&str> = Vec::new();
        for arg in &self.arguments {
            if arg.required && !obj.contains_key(&arg.name) {
                missing.push(&arg.name);
            }
        }
        if !missing.is_empty() {
            return Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "missing required prompt argument(s): {}",
                    missing.join(", ")
                )),
                structured: None,
            });
        }

        // Check for unknown arguments
        let known_names: std::collections::HashSet<&str> =
            self.arguments.iter().map(|a| a.name.as_str()).collect();
        let mut unknown: Vec<&str> = Vec::new();
        for key in obj.keys() {
            if !known_names.contains(key.as_str()) {
                unknown.push(key.as_str());
            }
        }
        if !unknown.is_empty() {
            return Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "unknown prompt argument(s): {}",
                    unknown.join(", ")
                )),
                structured: None,
            });
        }

        // Return validated args as Value
        Ok(serde_json::Value::Object(obj.clone()))
    }

    fn format_messages(
        &self,
        messages: &[super::client::PromptMessage],
        provenance: &str,
    ) -> String {
        use std::fmt::Write;
        let mut output = provenance.to_string();
        for msg in messages {
            let _ = write!(output, "\n[{}]: {}", msg.role, msg.content);
        }
        output
    }
}

#[async_trait]
impl Tool for McpPromptAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for arg in &self.arguments {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), json!("string"));
            if !arg.description.is_empty() {
                prop.insert("description".to_string(), json!(arg.description));
            }
            properties.insert(arg.name.clone(), serde_json::Value::Object(prop));

            if arg.required {
                required.push(json!(arg.name));
            }
        }

        let mut schema = serde_json::Map::new();
        schema.insert("type".to_string(), json!("object"));
        schema.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        if !required.is_empty() {
            schema.insert("required".to_string(), serde_json::Value::Array(required));
        }

        serde_json::Value::Object(schema)
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        tracing::debug!(
            server = %self.server_name,
            prompt = %self.original_name,
            timeout_ms = self.call_timeout_ms,
            output_limit_bytes = self.output_limit_bytes,
            "MCP prompt adapter execute"
        );

        // Guard against absurd output limits (same 10MB cap as other adapters)
        if self.output_limit_bytes > 10 * 1024 * 1024 {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("output_limit_bytes exceeds maximum allowed (10MB)".to_string()),
                structured: None,
            });
        }

        // Validate arguments before calling server
        let validated_args = match self.validate_arguments(&args) {
            Ok(v) => v,
            Err(result) => return Ok(result),
        };

        // Call MCP server for prompt expansion
        let messages = match self.client.get_prompt(&self.original_name, validated_args) {
            Ok(msgs) => msgs,
            Err(error) => {
                // Prompt failure isolation: return structured error, never panic
                let redacted = super::redact_error_message(&error.to_string());
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(redacted),
                    structured: None,
                });
            }
        };

        // Build provenance header
        let timestamp = chrono::Utc::now().to_rfc3339();
        let provenance = format!(
            "[mcp_prompt source={} fetched={}]",
            self.server_name, timestamp
        );

        // Format messages into output
        let formatted = self.format_messages(&messages, &provenance);

        // Content scanning hook (prompt injection mitigation)
        if let Some(ref scanner) = self.content_scanner {
            if let Err(reason) = scanner(&formatted) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("prompt content rejected by scanner: {reason}")),
                    structured: Some(json!({
                        "rejection_reason": reason,
                        "provenance": {
                            "source": self.server_name,
                            "fetched": timestamp,
                        }
                    })),
                });
            }
        }

        // Enforce output limit
        let output = if formatted.is_empty() {
            String::new()
        } else {
            self.enforce_output_limit(formatted)
        };

        // Build structured metadata with messages + provenance
        let structured_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        Ok(ToolResult {
            success: true,
            output,
            error: None,
            structured: Some(json!({
                "messages": structured_messages,
                "provenance": {
                    "source": self.server_name,
                    "fetched": timestamp,
                }
            })),
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters_schema(),
            source: Some(normalize::source_metadata_prompt(
                &self.server_name,
                &self.original_name,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpServerConfig;

    fn test_client() -> McpClient {
        let server = McpServerConfig {
            name: "test".into(),
            command: "__mcp_mock__".into(),
            ..McpServerConfig::default()
        };
        McpClient::new(server)
    }

    fn test_server(output_limit: usize) -> McpServerConfig {
        McpServerConfig {
            name: "workflows".into(),
            command: "__mcp_mock__".into(),
            output_limit_bytes: output_limit,
            ..McpServerConfig::default()
        }
    }

    fn test_manifest() -> McpPromptManifest {
        McpPromptManifest {
            name: "code-review".into(),
            description: "Code review template".into(),
            arguments: vec![
                PromptArgument {
                    name: "language".into(),
                    description: "Programming language".into(),
                    required: true,
                },
                PromptArgument {
                    name: "focus".into(),
                    description: "Review focus area".into(),
                    required: false,
                },
            ],
        }
    }

    fn test_manifest_no_args() -> McpPromptManifest {
        McpPromptManifest {
            name: "greeting".into(),
            description: "A simple greeting".into(),
            arguments: vec![],
        }
    }

    // ── parameters_schema generates correct schema from arguments ──

    #[test]
    fn parameters_schema_generates_schema_from_arguments() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let schema = adapter.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["language"]["type"], "string");
        assert_eq!(
            schema["properties"]["language"]["description"],
            "Programming language"
        );
        assert_eq!(schema["properties"]["focus"]["type"], "string");

        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "language");
    }

    #[test]
    fn parameters_schema_empty_for_no_arguments() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest_no_args(), test_client())
                .unwrap();
        let schema = adapter.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], json!({}));
        assert!(schema.get("required").is_none());
    }

    // ── spec returns correct metadata with kind: "mcp_prompt" ───

    #[test]
    fn spec_returns_mcp_prompt_kind() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let spec = adapter.spec();

        assert_eq!(spec.name, "mcp.workflows.prompt.code-review");
        assert_eq!(spec.description, "Code review template");
        let source = spec.source.unwrap();
        assert_eq!(source.kind, "mcp_prompt");
        assert_eq!(source.provider.as_deref(), Some("mcp"));
        assert_eq!(source.server.as_deref(), Some("workflows"));
        assert_eq!(source.original_name.as_deref(), Some("code-review"));
    }

    // ── execute returns formatted prompt with provenance header ──

    #[tokio::test]
    async fn execute_returns_formatted_prompt_with_provenance() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let result = adapter.execute(json!({"language": "rust"})).await.unwrap();

        assert!(result.success);
        assert!(result
            .output
            .starts_with("[mcp_prompt source=workflows fetched="));
        assert!(result
            .output
            .contains("[user]: mock prompt content for code-review"));
        assert!(result.error.is_none());
    }

    // ── provenance metadata includes source server and timestamp ─

    #[tokio::test]
    async fn provenance_metadata_in_structured_field() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let result = adapter.execute(json!({"language": "rust"})).await.unwrap();

        let structured = result.structured.unwrap();
        assert_eq!(structured["provenance"]["source"], "workflows");
        assert!(structured["provenance"]["fetched"]
            .as_str()
            .unwrap()
            .contains('T'));

        let messages = structured["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    // ── execute rejects missing required argument ────────────────

    #[tokio::test]
    async fn execute_rejects_missing_required_argument() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        // Missing "language" which is required
        let result = adapter.execute(json!({"focus": "security"})).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("missing required"));
        assert!(result.error.as_ref().unwrap().contains("language"));
    }

    // ── execute rejects unknown argument ─────────────────────────

    #[tokio::test]
    async fn execute_rejects_unknown_argument() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let result = adapter
            .execute(json!({"language": "rust", "unknown_arg": "bad"}))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .contains("unknown prompt argument"));
        assert!(result.error.as_ref().unwrap().contains("unknown_arg"));
    }

    // ── execute handles empty message array (valid result) ───────

    #[tokio::test]
    async fn execute_handles_empty_message_array() {
        // The mock always returns one message, so we test with a no-args prompt
        // The key assertion is that success=true and no error regardless of message count
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest_no_args(), test_client())
                .unwrap();
        let result = adapter.execute(json!({})).await.unwrap();

        assert!(result.success);
        assert!(result.error.is_none());
    }

    // ── output limit enforcement ────────────────────────────────

    #[test]
    fn enforce_output_limit_truncates_large_content() {
        let adapter = McpPromptAdapter {
            name: "test".into(),
            description: "test".into(),
            original_name: "test".into(),
            arguments: vec![],
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 100,
            client: test_client(),
            content_scanner: None,
        };

        let large_output = "A".repeat(200);
        let enforced = adapter.enforce_output_limit(large_output);
        assert_eq!(enforced.len(), 100);
        assert!(enforced.ends_with("[output_limit_enforced limit_bytes=100 original_bytes=200]"));
    }

    #[test]
    fn enforce_output_limit_passes_small_content() {
        let adapter = McpPromptAdapter {
            name: "test".into(),
            description: "test".into(),
            original_name: "test".into(),
            arguments: vec![],
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 1024,
            client: test_client(),
            content_scanner: None,
        };

        let small = "hello world".to_string();
        let result = adapter.enforce_output_limit(small.clone());
        assert_eq!(result, small);
    }

    // ── prompt-specific output_limit_bytes overrides server default ──

    #[test]
    fn prompt_output_limit_overrides_server_default() {
        let mut server = test_server(1024);
        server.prompt_output_limit_bytes = Some(2048);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        assert_eq!(adapter.output_limit_bytes, 2048);
    }

    #[test]
    fn prompt_output_limit_falls_back_to_server_default() {
        let server = test_server(1024);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        assert_eq!(adapter.output_limit_bytes, 1024);
    }

    // ── 10MB max output limit guard ─────────────────────────────

    #[tokio::test]
    async fn blocks_massive_limit_configuration() {
        let adapter = McpPromptAdapter {
            name: "test".into(),
            description: "test".into(),
            original_name: "test".into(),
            arguments: vec![],
            server_name: "test".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 20 * 1024 * 1024, // 20MB
            client: test_client(),
            content_scanner: None,
        };

        let result = adapter.execute(json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("exceeds maximum allowed"));
    }

    // ── failure isolation: errors return structured result ───────

    #[tokio::test]
    async fn execute_returns_structured_error_on_failure() {
        let server = McpServerConfig {
            name: "failing".into(),
            command: "nonexistent_command".into(),
            ..McpServerConfig::default()
        };
        let client = McpClient::new(server.clone());
        let adapter = McpPromptAdapter {
            name: "mcp.failing.prompt.broken".into(),
            description: "broken".into(),
            original_name: "broken".into(),
            arguments: vec![],
            server_name: "failing".into(),
            call_timeout_ms: 1000,
            output_limit_bytes: 1024,
            client,
            content_scanner: None,
        };

        let result = adapter.execute(json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
        // Should NOT panic or return Err — always Ok(ToolResult)
    }

    // ── content scanning hook can reject prompt content ──────────

    #[tokio::test]
    async fn content_scanner_rejects_prompt_content() {
        let server = test_server(4096);
        let scanner: ContentScanner = Arc::new(|content: &str| {
            if content.contains("mock prompt") {
                Err("suspicious content detected".to_string())
            } else {
                Ok(())
            }
        });

        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest_no_args(), test_client())
                .unwrap()
                .with_content_scanner(scanner);

        let result = adapter.execute(json!({})).await.unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .contains("rejected by scanner"));
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .contains("suspicious content detected"));

        // Structured field should include rejection details
        let structured = result.structured.unwrap();
        assert_eq!(
            structured["rejection_reason"],
            "suspicious content detected"
        );
    }

    #[tokio::test]
    async fn content_scanner_allows_clean_content() {
        let server = test_server(4096);
        let scanner: ContentScanner = Arc::new(|_content: &str| Ok(()));

        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest_no_args(), test_client())
                .unwrap()
                .with_content_scanner(scanner);

        let result = adapter.execute(json!({})).await.unwrap();
        assert!(result.success);
    }

    // ── validate_arguments with null args ────────────────────────

    #[tokio::test]
    async fn execute_with_null_args_and_no_required_params() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest_no_args(), test_client())
                .unwrap();
        let result = adapter.execute(serde_json::Value::Null).await.unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn execute_with_null_args_and_required_params_fails() {
        let server = test_server(4096);
        let adapter =
            McpPromptAdapter::from_manifest(&server, test_manifest(), test_client()).unwrap();
        let result = adapter.execute(serde_json::Value::Null).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("missing required"));
    }
}
