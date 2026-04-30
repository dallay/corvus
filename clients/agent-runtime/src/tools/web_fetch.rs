use super::http_common::{
    execute_get, normalized_allowed_domains, read_response_body_limited, validate_outbound_url,
};
use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
struct FetchedResponse {
    status_code: u16,
    status_text: String,
    final_url: String,
    content_type: String,
    body: Vec<u8>,
    truncated: bool,
}

#[async_trait]
trait WebFetchTransport: Send + Sync {
    async fn get(
        &self,
        url: &str,
        timeout_secs: u64,
        max_response_size: usize,
    ) -> anyhow::Result<FetchedResponse>;
}

struct ReqwestWebFetchTransport;

#[async_trait]
impl WebFetchTransport for ReqwestWebFetchTransport {
    async fn get(
        &self,
        url: &str,
        timeout_secs: u64,
        max_response_size: usize,
    ) -> anyhow::Result<FetchedResponse> {
        let response = execute_get(url, timeout_secs).await?;
        let status = response.status();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let (body, truncated) = read_response_body_limited(response, max_response_size).await?;

        Ok(FetchedResponse {
            status_code: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("Unknown").to_string(),
            final_url,
            content_type,
            body,
            truncated,
        })
    }
}

pub struct WebFetchTool {
    security: Arc<SecurityPolicy>,
    allowed_domains: Vec<String>,
    max_response_size: usize,
    timeout_secs: u64,
    transport: Arc<dyn WebFetchTransport>,
}

impl WebFetchTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        max_response_size: usize,
        timeout_secs: u64,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalized_allowed_domains(allowed_domains),
            max_response_size,
            timeout_secs,
            transport: Arc::new(ReqwestWebFetchTransport),
        }
    }

    #[cfg(test)]
    fn new_with_transport(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        max_response_size: usize,
        timeout_secs: u64,
        transport: Arc<dyn WebFetchTransport>,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalized_allowed_domains(allowed_domains),
            max_response_size,
            timeout_secs,
            transport,
        }
    }
}

#[derive(Debug, Clone)]
struct WebFetchRequest {
    url: String,
    prompt: String,
}

impl WebFetchRequest {
    fn from_args(args: &Value) -> Result<Self, String> {
        let object = args
            .as_object()
            .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;
        if let Some(unexpected) = object
            .keys()
            .find(|key| !matches!(key.as_str(), "url" | "prompt"))
        {
            return Err(format!("Unknown parameter: {unexpected}"));
        }

        let url = object
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing required parameter: url".to_string())?
            .to_string();
        let prompt = object
            .get("prompt")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing required parameter: prompt".to_string())?
            .trim()
            .to_string();
        if prompt.is_empty() {
            return Err("Prompt must not be empty".to_string());
        }

        Ok(Self { url, prompt })
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        "Read-only fetch-and-extract tool for allowlisted web content backed by Corvus http_request policy checks."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": { "type": "string" },
                "prompt": { "type": "string" }
            },
            "required": ["url", "prompt"]
        })
    }

    fn spec(&self) -> super::traits::ToolSpec {
        super::traits::ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
            source: None,
            aliases: super::parity_alias_for(self.name())
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let request = match WebFetchRequest::from_args(&args) {
            Ok(request) => request,
            Err(error) => return Ok(tool_error(error)),
        };

        if self.security.is_rate_limited() {
            return Ok(tool_error(
                "Rate limit exceeded: too many actions in the last hour".to_string(),
            ));
        }

        let url = match validate_outbound_url(&request.url, &self.allowed_domains, "http_request") {
            Ok(url) => url,
            Err(error) => return Ok(tool_error(error.to_string())),
        };

        if !self.security.record_action() {
            return Ok(tool_error(
                "Rate limit exceeded: action budget exhausted".to_string(),
            ));
        }

        let start = Instant::now();
        let response = match self
            .transport
            .get(&url, self.timeout_secs, self.max_response_size)
            .await
        {
            Ok(response) => response,
            Err(error) => return Ok(tool_error(format!("HTTP request failed: {error}"))),
        };

        let result =
            match extract_response_text(&response.content_type, &response.body, response.truncated)
            {
                Ok(result) => result,
                Err(error) => return Ok(tool_error(error)),
            };
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let status_success = (200..300).contains(&response.status_code);
        let status_error = response.status_code >= 400;

        Ok(ToolResult {
            success: status_success,
            output: result.clone(),
            error: if status_error {
                Some(format!("HTTP {}", response.status_code))
            } else {
                None
            },
            structured: Some(json!({
                "bytes": response.body.len(),
                "code": response.status_code,
                "codeText": response.status_text,
                "result": result,
                "durationMs": duration_ms,
                "url": response.final_url,
            })),
        })
    }
}

fn extract_response_text(
    content_type: &str,
    body: &[u8],
    truncated: bool,
) -> Result<String, String> {
    let normalized = content_type.to_ascii_lowercase();
    let text = if normalized.contains("text/html") {
        html_to_text(&String::from_utf8_lossy(body))
    } else if normalized.starts_with("text/")
        || normalized.contains("application/json")
        || normalized.contains("+json")
        || normalized.contains("application/xml")
        || normalized.is_empty()
    {
        String::from_utf8_lossy(body).to_string()
    } else {
        return Err(format!(
            "WebFetch only supports textual responses in this slice (got {content_type})"
        ));
    };

    if truncated {
        Ok(format!(
            "{text}\n\n... [Response truncated due to size limit] ..."
        ))
    } else {
        Ok(text)
    }
}

fn html_to_text(html: &str) -> String {
    let script_re = Regex::new(r"(?is)<(script|style)[^>]*>.*?</(script|style)>").ok();
    let block_re =
        Regex::new(r"(?is)</?(p|div|section|article|main|li|ul|ol|h[1-6]|br)[^>]*>").ok();
    let tag_re = Regex::new(r"(?is)<[^>]+>").ok();

    let without_scripts = script_re
        .as_ref()
        .map(|re| re.replace_all(html, " ").into_owned())
        .unwrap_or_else(|| html.to_string());
    let with_blocks = block_re
        .as_ref()
        .map(|re| re.replace_all(&without_scripts, "\n").into_owned())
        .unwrap_or(without_scripts);
    let without_tags = tag_re
        .as_ref()
        .map(|re| re.replace_all(&with_blocks, " ").into_owned())
        .unwrap_or(with_blocks);

    decode_html_entities(&without_tags)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn tool_error(error: String) -> ToolResult {
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(error.clone()),
        structured: Some(json!({
            "error": {
                "message": error,
            }
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use tempfile::TempDir;

    struct FakeWebFetchTransport {
        response: FetchedResponse,
    }

    impl FakeWebFetchTransport {
        fn success(url: &str, content_type: &str, body: &[u8]) -> Arc<Self> {
            Arc::new(Self {
                response: FetchedResponse {
                    status_code: 200,
                    status_text: "OK".to_string(),
                    final_url: url.to_string(),
                    content_type: content_type.to_string(),
                    body: body.to_vec(),
                    truncated: false,
                },
            })
        }
    }

    #[async_trait]
    impl WebFetchTransport for FakeWebFetchTransport {
        async fn get(
            &self,
            _url: &str,
            _timeout_secs: u64,
            _max_response_size: usize,
        ) -> anyhow::Result<FetchedResponse> {
            Ok(self.response.clone())
        }
    }

    fn test_tool() -> WebFetchTool {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        WebFetchTool::new(security, vec!["example.com".into()], 1024, 5)
    }

    #[test]
    fn web_fetch_spec_exposes_snake_case_alias() {
        let tool = test_tool();
        let spec = tool.spec();
        assert_eq!(spec.name, "WebFetch");
        assert_eq!(spec.aliases, vec!["web_fetch"]);
    }

    fn test_tool_with_transport(transport: Arc<dyn WebFetchTransport>) -> WebFetchTool {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        WebFetchTool::new_with_transport(security, vec!["example.com".into()], 1024, 5, transport)
    }

    #[test]
    fn web_fetch_name_and_schema() {
        let tool = test_tool();
        assert_eq!(tool.name(), "WebFetch");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["url"].is_object());
        assert!(schema["properties"]["prompt"].is_object());
    }

    #[test]
    fn html_extraction_returns_textual_content() {
        let result = extract_response_text(
            "text/html; charset=utf-8",
            b"<html><body><main><h1>Hello</h1><p>World &amp; friends</p></main></body></html>",
            false,
        )
        .unwrap();
        assert!(result.contains("Hello"));
        assert!(result.contains("World & friends"));
    }

    #[test]
    fn binary_response_is_rejected() {
        let err =
            extract_response_text("application/octet-stream", &[0, 1, 2, 3], false).unwrap_err();
        assert!(err.contains("textual responses"));
    }

    #[tokio::test]
    async fn web_fetch_rejects_private_network_target() {
        let tool = test_tool();
        let result = tool
            .execute(json!({
                "url": "http://127.0.0.1:8080/admin",
                "prompt": "Summarize"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap_or_default().contains("local/private"));
    }

    #[tokio::test]
    async fn web_fetch_rejects_unsupported_scheme() {
        let tool = test_tool();
        let result = tool
            .execute(json!({
                "url": "file:///etc/passwd",
                "prompt": "Summarize"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .unwrap_or_default()
            .contains("http:// and https://"));
    }

    #[tokio::test]
    async fn web_fetch_requires_prompt() {
        let _dir = TempDir::new().unwrap();
        let tool = test_tool();
        let result = tool
            .execute(json!({"url": "https://example.com"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap_or_default().contains("prompt"));
    }

    #[tokio::test]
    async fn web_fetch_successful_html_response_returns_extracted_text() {
        let tool = test_tool_with_transport(FakeWebFetchTransport::success(
            "https://example.com/docs",
            "text/html; charset=utf-8",
            b"<html><body><main><h1>Docs</h1><p>Hello &amp; welcome</p></main></body></html>",
        ));

        let result = tool
            .execute(json!({
                "url": "https://example.com/docs",
                "prompt": "Summarize this page"
            }))
            .await
            .unwrap();

        assert!(result.success, "unexpected error: {:?}", result.error);
        assert!(result.output.contains("Docs"));
        assert!(result.output.contains("Hello & welcome"));

        let structured = result.structured.unwrap();
        assert_eq!(structured["code"], 200);
        assert_eq!(structured["url"], "https://example.com/docs");
    }
}
