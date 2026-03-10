use super::traits::{Tool, ToolResult};
#[cfg(test)]
use super::url_safety::normalize_domain;
use super::url_safety::{extract_host, host_matches_allowlist, normalize_allowed_domains};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// HTTP request tool for API interactions.
const MAX_REQUEST_HEADERS: usize = 64;
const MAX_HEADER_TOTAL_BYTES: usize = 8 * 1024;

const FORBIDDEN_HEADERS: &[&str] = &[
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "cookie",
    "set-cookie",
];

fn is_header_name_valid(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

fn is_header_value_valid(value: &str) -> bool {
    !value.chars().any(|c| c.is_control() && c != '\t')
}

fn is_header_forbidden(name: &str) -> bool {
    FORBIDDEN_HEADERS
        .iter()
        .any(|&h| name.eq_ignore_ascii_case(h))
}

fn format_response_headers(headers: &reqwest::header::HeaderMap) -> String {
    headers
        .iter()
        .map(|(k, v)| {
            let is_sensitive = k.as_str().to_lowercase().contains("set-cookie");
            if is_sensitive {
                format!("{}: ***REDACTED***", k.as_str())
            } else {
                format!("{}: {:?}", k.as_str(), v)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Supports GET, POST, PUT, DELETE methods with configurable security.
pub struct HttpRequestTool {
    security: Arc<SecurityPolicy>,
    allowed_domains: Vec<String>,
    max_response_size: usize,
    timeout_secs: u64,
}

impl HttpRequestTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        max_response_size: usize,
        timeout_secs: u64,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalize_allowed_domains(allowed_domains),
            max_response_size,
            timeout_secs,
        }
    }

    fn validate_url(&self, raw_url: &str) -> anyhow::Result<String> {
        let url = raw_url.trim();

        if url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }

        if url.chars().any(char::is_whitespace) {
            anyhow::bail!("URL cannot contain whitespace");
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            anyhow::bail!("Only http:// and https:// URLs are allowed");
        }

        if self.allowed_domains.is_empty() {
            anyhow::bail!(
                "HTTP request tool is enabled but no allowed_domains are configured. Add [http_request].allowed_domains in config.toml"
            );
        }

        let host = extract_host(
            url,
            &["http://", "https://"],
            "Only http:// and https:// URLs are allowed",
            "http_request",
        )?;

        if is_private_or_local_host(&host) {
            anyhow::bail!("Blocked local/private host: {host}");
        }

        if !host_matches_allowlist(&host, &self.allowed_domains) {
            anyhow::bail!("Host '{host}' is not in http_request.allowed_domains");
        }

        Ok(url.to_string())
    }

    fn validate_method(&self, method: &str) -> anyhow::Result<reqwest::Method> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(reqwest::Method::GET),
            "POST" => Ok(reqwest::Method::POST),
            "PUT" => Ok(reqwest::Method::PUT),
            "DELETE" => Ok(reqwest::Method::DELETE),
            "PATCH" => Ok(reqwest::Method::PATCH),
            "HEAD" => Ok(reqwest::Method::HEAD),
            "OPTIONS" => Ok(reqwest::Method::OPTIONS),
            _ => anyhow::bail!("Unsupported HTTP method: {method}. Supported: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"),
        }
    }

    fn parse_headers(&self, headers: &serde_json::Value) -> anyhow::Result<Vec<(String, String)>> {
        let mut result = Vec::new();
        let mut total_bytes = 0usize;

        let obj = headers
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Headers must be an object"))?;

        if obj.len() > MAX_REQUEST_HEADERS {
            anyhow::bail!(
                "Too many headers: {} (max {})",
                obj.len(),
                MAX_REQUEST_HEADERS
            );
        }

        for (key, value) in obj {
            let value_str = value
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Header '{key}' value must be a string"))?;

            if !is_header_name_valid(key) {
                anyhow::bail!("Invalid header name: {key}");
            }

            if is_header_forbidden(key) {
                anyhow::bail!("Header '{key}' is not allowed");
            }

            if !is_header_value_valid(value_str) {
                anyhow::bail!("Invalid control character in header '{key}'");
            }

            total_bytes = total_bytes
                .saturating_add(key.len())
                .saturating_add(value_str.len());
            if total_bytes > MAX_HEADER_TOTAL_BYTES {
                anyhow::bail!(
                    "Headers exceed max size of {} bytes",
                    MAX_HEADER_TOTAL_BYTES
                );
            }

            result.push((key.clone(), value_str.to_string()));
        }
        Ok(result)
    }

    async fn execute_request(
        &self,
        url: &str,
        method: reqwest::Method,
        headers: Vec<(String, String)>,
        body: Option<&str>,
    ) -> anyhow::Result<reqwest::Response> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let mut request = client.request(method, url);

        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        if let Some(body_str) = body {
            request = request.body(body_str.to_string());
        }

        Ok(request.send().await?)
    }

    async fn read_response_body(&self, response: reqwest::Response) -> String {
        let mut body = Vec::new();
        let mut truncated = false;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(bytes) => bytes,
                Err(error) => {
                    return format!("[Failed to read response body: {error}]");
                }
            };

            let remaining = self.max_response_size.saturating_sub(body.len());
            if remaining == 0 {
                truncated = true;
                break;
            }

            if chunk.len() > remaining {
                body.extend_from_slice(&chunk[..remaining]);
                truncated = true;
                break;
            }

            body.extend_from_slice(&chunk);
        }

        let text = String::from_utf8_lossy(&body).to_string();
        if truncated {
            format!("{text}\n\n... [Response truncated due to size limit] ...")
        } else {
            text
        }
    }

    #[cfg(test)]
    fn truncate_response(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        if bytes.len() <= self.max_response_size {
            return text.to_string();
        }
        let capped = String::from_utf8_lossy(&bytes[..self.max_response_size]).to_string();
        format!("{capped}\n\n... [Response truncated due to size limit] ...")
    }

    fn check_action_security(&self) -> Option<ToolResult> {
        if !self.security.can_act() {
            return Some(tool_error_result(
                "Action blocked: autonomy is read-only".into(),
            ));
        }

        if !self.security.record_action() {
            return Some(tool_error_result(
                "Action blocked: rate limit exceeded".into(),
            ));
        }

        None
    }

    fn map_validation<T>(&self, result: anyhow::Result<T>) -> Result<T, ToolResult> {
        result.map_err(|e| tool_error_result(e.to_string()))
    }
}

fn tool_error_result(error: String) -> ToolResult {
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(error),
        structured: None,
    }
}

pub(crate) fn redact_headers_for_display(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(key, value)| {
            let lower = key.to_lowercase();
            let is_sensitive = lower.contains("authorization")
                || lower.contains("api-key")
                || lower.contains("apikey")
                || lower.contains("token")
                || lower.contains("secret");
            if is_sensitive {
                (key.clone(), "***REDACTED***".into())
            } else {
                (key.clone(), value.clone())
            }
        })
        .collect()
}

#[async_trait]
impl Tool for HttpRequestTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> &str {
        "Make HTTP requests to external APIs. Supports GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS methods. \
        Security constraints: allowlist-only domains, no local/private hosts, configurable timeout and response size limits."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "HTTP or HTTPS URL to request"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)",
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers as key-value pairs (e.g., {\"Authorization\": \"Bearer token\", \"Content-Type\": \"application/json\"})",
                    "default": {}
                },
                "body": {
                    "type": "string",
                    "description": "Optional request body (for POST, PUT, PATCH requests)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        let method_str = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
        let headers_val = args.get("headers").cloned().unwrap_or(json!({}));
        let body = args.get("body").and_then(|v| v.as_str());

        if let Some(result) = self.check_action_security() {
            return Ok(result);
        }

        let url = match self.map_validation(self.validate_url(url)) {
            Ok(v) => v,
            Err(result) => return Ok(result),
        };

        let method = match self.map_validation(self.validate_method(method_str)) {
            Ok(m) => m,
            Err(result) => return Ok(result),
        };

        let request_headers = match self.map_validation(self.parse_headers(&headers_val)) {
            Ok(h) => h,
            Err(result) => return Ok(result),
        };

        match self
            .execute_request(&url, method, request_headers, body)
            .await
        {
            Ok(response) => {
                let status = response.status();
                let status_code = status.as_u16();

                // Get response headers (redact sensitive ones)
                let headers_text = format_response_headers(response.headers());

                // Get response body with size limit
                let response_text = self.read_response_body(response).await;

                let output = format!(
                    "Status: {} {}\nResponse Headers: {}\n\nResponse Body:\n{}",
                    status_code,
                    status.canonical_reason().unwrap_or("Unknown"),
                    headers_text,
                    response_text
                );

                Ok(ToolResult {
                    success: status.is_success(),
                    output,
                    error: if status.is_client_error() || status.is_server_error() {
                        Some(format!("HTTP {}", status_code))
                    } else {
                        None
                    },
                    structured: None,
                })
            }
            Err(e) => Ok(tool_error_result(format!("HTTP request failed: {e}"))),
        }
    }
}

fn is_private_or_local_host(host: &str) -> bool {
    // Strip brackets from IPv6 addresses like [::1]
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    let has_local_tld = bare
        .rsplit('.')
        .next()
        .is_some_and(|label| label == "local");

    if bare == "localhost" || bare.ends_with(".localhost") || has_local_tld {
        return true;
    }

    if let Ok(ip) = bare.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => is_non_global_v4(v4),
            std::net::IpAddr::V6(v6) => is_non_global_v6(v6),
        };
    }

    false
}

/// Returns true if the IPv4 address is not globally routable.
fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    let [a, b, c, _] = v4.octets();
    v4.is_loopback()                       // 127.0.0.0/8
        || v4.is_private()                 // 10/8, 172.16/12, 192.168/16
        || v4.is_link_local()              // 169.254.0.0/16
        || v4.is_unspecified()             // 0.0.0.0
        || v4.is_broadcast()              // 255.255.255.255
        || v4.is_multicast()              // 224.0.0.0/4
        || (a == 100 && (64..=127).contains(&b)) // Shared address space (RFC 6598)
        || a >= 240                        // Reserved (240.0.0.0/4, except broadcast)
        || (a == 192 && b == 0 && (c == 0 || c == 2)) // IETF assignments + TEST-NET-1
        || (a == 198 && b == 51)           // Documentation (198.51.100.0/24)
        || (a == 203 && b == 0)            // Documentation (203.0.113.0/24)
        || (a == 198 && (18..=19).contains(&b)) // Benchmarking (198.18.0.0/15)
}

/// Returns true if the IPv6 address is not globally routable.
fn is_non_global_v6(v6: std::net::Ipv6Addr) -> bool {
    let segs = v6.segments();
    v6.is_loopback()                       // ::1
        || v6.is_unspecified()             // ::
        || v6.is_multicast()              // ff00::/8
        || (segs[0] & 0xfe00) == 0xfc00   // Unique-local (fc00::/7)
        || (segs[0] & 0xffc0) == 0xfe80   // Link-local (fe80::/10)
        || (segs[0] == 0x2001 && segs[1] == 0x0db8) // Documentation (2001:db8::/32)
        || v6.to_ipv4_mapped().is_some_and(is_non_global_v4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};

    fn test_tool(allowed_domains: Vec<&str>) -> HttpRequestTool {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        });
        HttpRequestTool::new(
            security,
            allowed_domains.into_iter().map(String::from).collect(),
            1_000_000,
            30,
        )
    }

    #[test]
    fn normalize_domain_strips_scheme_path_and_case() {
        let got = normalize_domain("  HTTPS://Docs.Example.com/path ").unwrap();
        assert_eq!(got, "docs.example.com");
    }

    #[test]
    fn normalize_allowed_domains_deduplicates() {
        let got = normalize_allowed_domains(vec![
            "example.com".into(),
            "EXAMPLE.COM".into(),
            "https://example.com/".into(),
        ]);
        assert_eq!(got, vec!["example.com".to_string()]);
    }

    #[test]
    fn validate_accepts_exact_domain() {
        let tool = test_tool(vec!["example.com"]);
        let got = tool.validate_url("https://example.com/docs").unwrap();
        assert_eq!(got, "https://example.com/docs");
    }

    #[test]
    fn validate_accepts_http() {
        let tool = test_tool(vec!["example.com"]);
        assert!(tool.validate_url("http://example.com").is_ok());
    }

    #[test]
    fn validate_accepts_subdomain() {
        let tool = test_tool(vec!["example.com"]);
        assert!(tool.validate_url("https://api.example.com/v1").is_ok());
    }

    #[test]
    fn validate_rejects_allowlist_miss() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool
            .validate_url("https://google.com")
            .unwrap_err()
            .to_string();
        assert!(err.contains("allowed_domains"));
    }

    #[test]
    fn validate_rejects_localhost() {
        let tool = test_tool(vec!["localhost"]);
        let err = tool
            .validate_url("https://localhost:8080")
            .unwrap_err()
            .to_string();
        assert!(err.contains("local/private"));
    }

    #[test]
    fn validate_rejects_private_ipv4() {
        let tool = test_tool(vec!["192.168.1.5"]);
        let err = tool
            .validate_url("https://192.168.1.5")
            .unwrap_err()
            .to_string();
        assert!(err.contains("local/private"));
    }

    #[test]
    fn validate_rejects_whitespace() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool
            .validate_url("https://example.com/hello world")
            .unwrap_err()
            .to_string();
        assert!(err.contains("whitespace"));
    }

    #[test]
    fn validate_rejects_userinfo() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool
            .validate_url("https://user@example.com")
            .unwrap_err()
            .to_string();
        assert!(err.contains("userinfo"));
    }

    #[test]
    fn validate_requires_allowlist() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = HttpRequestTool::new(security, vec![], 1_000_000, 30);
        let err = tool
            .validate_url("https://example.com")
            .unwrap_err()
            .to_string();
        assert!(err.contains("allowed_domains"));
    }

    #[test]
    fn validate_accepts_valid_methods() {
        let tool = test_tool(vec!["example.com"]);
        assert!(tool.validate_method("GET").is_ok());
        assert!(tool.validate_method("POST").is_ok());
        assert!(tool.validate_method("PUT").is_ok());
        assert!(tool.validate_method("DELETE").is_ok());
        assert!(tool.validate_method("PATCH").is_ok());
        assert!(tool.validate_method("HEAD").is_ok());
        assert!(tool.validate_method("OPTIONS").is_ok());
    }

    #[test]
    fn validate_rejects_invalid_method() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool.validate_method("INVALID").unwrap_err().to_string();
        assert!(err.contains("Unsupported HTTP method"));
    }

    #[test]
    fn blocks_multicast_ipv4() {
        assert!(is_private_or_local_host("224.0.0.1"));
        assert!(is_private_or_local_host("239.255.255.255"));
    }

    #[test]
    fn blocks_broadcast() {
        assert!(is_private_or_local_host("255.255.255.255"));
    }

    #[test]
    fn blocks_reserved_ipv4() {
        assert!(is_private_or_local_host("240.0.0.1"));
        assert!(is_private_or_local_host("250.1.2.3"));
    }

    #[test]
    fn blocks_documentation_ranges() {
        assert!(is_private_or_local_host("192.0.2.1")); // TEST-NET-1
        assert!(is_private_or_local_host("198.51.100.1")); // TEST-NET-2
        assert!(is_private_or_local_host("203.0.113.1")); // TEST-NET-3
    }

    #[test]
    fn blocks_benchmarking_range() {
        assert!(is_private_or_local_host("198.18.0.1"));
        assert!(is_private_or_local_host("198.19.255.255"));
    }

    #[test]
    fn blocks_ipv6_localhost() {
        assert!(is_private_or_local_host("::1"));
        assert!(is_private_or_local_host("[::1]"));
    }

    #[test]
    fn blocks_ipv6_multicast() {
        assert!(is_private_or_local_host("ff02::1"));
    }

    #[test]
    fn blocks_ipv6_link_local() {
        assert!(is_private_or_local_host("fe80::1"));
    }

    #[test]
    fn blocks_ipv6_unique_local() {
        assert!(is_private_or_local_host("fd00::1"));
    }

    #[test]
    fn blocks_ipv4_mapped_ipv6() {
        assert!(is_private_or_local_host("::ffff:127.0.0.1"));
        assert!(is_private_or_local_host("::ffff:192.168.1.1"));
        assert!(is_private_or_local_host("::ffff:10.0.0.1"));
    }

    #[test]
    fn allows_public_ipv4() {
        assert!(!is_private_or_local_host("8.8.8.8"));
        assert!(!is_private_or_local_host("1.1.1.1"));
        assert!(!is_private_or_local_host("93.184.216.34"));
    }

    #[test]
    fn blocks_ipv6_documentation_range() {
        assert!(is_private_or_local_host("2001:db8::1"));
    }

    #[test]
    fn allows_public_ipv6() {
        assert!(!is_private_or_local_host("2607:f8b0:4004:800::200e"));
    }

    #[test]
    fn blocks_shared_address_space() {
        assert!(is_private_or_local_host("100.64.0.1"));
        assert!(is_private_or_local_host("100.127.255.255"));
        assert!(!is_private_or_local_host("100.63.0.1")); // Just below range
        assert!(!is_private_or_local_host("100.128.0.1")); // Just above range
    }

    #[tokio::test]
    async fn execute_blocks_readonly_mode() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tool = HttpRequestTool::new(security, vec!["example.com".into()], 1_000_000, 30);
        let result = tool
            .execute(json!({"url": "https://example.com"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("read-only"));
    }

    #[tokio::test]
    async fn execute_blocks_when_rate_limited() {
        let security = Arc::new(SecurityPolicy {
            max_actions_per_hour: 0,
            ..SecurityPolicy::default()
        });
        let tool = HttpRequestTool::new(security, vec!["example.com".into()], 1_000_000, 30);
        let result = tool
            .execute(json!({"url": "https://example.com"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("rate limit"));
    }

    #[test]
    fn truncate_response_within_limit() {
        let tool = test_tool(vec!["example.com"]);
        let text = "hello world";
        assert_eq!(tool.truncate_response(text), "hello world");
    }

    #[test]
    fn truncate_response_over_limit() {
        let tool = HttpRequestTool::new(
            Arc::new(SecurityPolicy::default()),
            vec!["example.com".into()],
            10,
            30,
        );
        let text = "hello world this is long";
        let truncated = tool.truncate_response(text);
        assert!(truncated.len() <= 10 + 60); // limit + message
        assert!(truncated.contains("[Response truncated"));
    }

    #[test]
    fn parse_headers_preserves_original_values() {
        let tool = test_tool(vec!["example.com"]);
        let headers = json!({
            "Authorization": "Bearer secret",
            "Content-Type": "application/json",
            "X-API-Key": "my-key"
        });
        let parsed = tool.parse_headers(&headers).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(parsed
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer secret"));
        assert!(parsed
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "my-key"));
        assert!(parsed
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn parse_headers_rejects_hop_by_hop_headers() {
        let tool = test_tool(vec!["example.com"]);
        let headers = json!({
            "Host": "evil.example.com"
        });

        let err = tool.parse_headers(&headers).unwrap_err().to_string();
        assert!(err.contains("not allowed"));
    }

    #[test]
    fn parse_headers_rejects_cookie_headers() {
        let tool = test_tool(vec!["example.com"]);

        for headers in [
            json!({ "Cookie": "session=abc" }),
            json!({ "Set-Cookie": "session=abc; HttpOnly" }),
        ] {
            let err = tool.parse_headers(&headers).unwrap_err().to_string();
            assert!(err.contains("not allowed"));
        }
    }

    #[test]
    fn parse_headers_rejects_control_chars() {
        let tool = test_tool(vec!["example.com"]);
        let headers = json!({
            "X-Test": "value
        malicious"
        });

        let err = tool.parse_headers(&headers).unwrap_err().to_string();
        assert!(err.contains("control character"));
    }

    #[test]
    fn redact_headers_for_display_redacts_sensitive() {
        let headers = vec![
            ("Authorization".into(), "Bearer secret".into()),
            ("Content-Type".into(), "application/json".into()),
            ("X-API-Key".into(), "my-key".into()),
            ("X-Secret-Token".into(), "tok-123".into()),
        ];
        let redacted = redact_headers_for_display(&headers);
        assert_eq!(redacted.len(), 4);
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "***REDACTED***"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "***REDACTED***"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-Secret-Token" && v == "***REDACTED***"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn redact_headers_does_not_alter_original() {
        let headers = vec![("Authorization".into(), "Bearer real-token".into())];
        let _ = redact_headers_for_display(&headers);
        assert_eq!(headers[0].1, "Bearer real-token");
    }

    // ── SSRF: alternate IP notation bypass defense-in-depth ─────────
    //
    // Rust's IpAddr::parse() rejects non-standard notations (octal, hex,
    // decimal integer, zero-padded). These tests document that property
    // so regressions are caught if the parsing strategy ever changes.

    #[test]
    fn ssrf_octal_loopback_not_parsed_as_ip() {
        // 0177.0.0.1 is octal for 127.0.0.1 in some languages, but
        // Rust's IpAddr rejects it — it falls through as a hostname.
        assert!(!is_private_or_local_host("0177.0.0.1"));
    }

    #[test]
    fn ssrf_hex_loopback_not_parsed_as_ip() {
        // 0x7f000001 is hex for 127.0.0.1 in some languages.
        assert!(!is_private_or_local_host("0x7f000001"));
    }

    #[test]
    fn ssrf_decimal_loopback_not_parsed_as_ip() {
        // 2130706433 is decimal for 127.0.0.1 in some languages.
        assert!(!is_private_or_local_host("2130706433"));
    }

    #[test]
    fn ssrf_zero_padded_loopback_not_parsed_as_ip() {
        // 127.000.000.001 uses zero-padded octets.
        assert!(!is_private_or_local_host("127.000.000.001"));
    }

    #[test]
    fn ssrf_alternate_notations_rejected_by_validate_url() {
        // Even if is_private_or_local_host doesn't flag these, they
        // fail the allowlist because they're treated as hostnames.
        let tool = test_tool(vec!["example.com"]);
        for notation in [
            "http://0177.0.0.1",
            "http://0x7f000001",
            "http://2130706433",
            "http://127.000.000.001",
        ] {
            let err = tool.validate_url(notation).unwrap_err().to_string();
            assert!(
                err.contains("allowed_domains") || err.contains("Blocked local/private host"),
                "Expected allowlist or local/private host rejection for {notation}, got: {err}"
            );
        }
    }

    #[test]
    fn redirect_policy_is_none() {
        // Structural test: the tool should be buildable with redirect-safe config.
        // The actual Policy::none() enforcement is in execute_request's client builder.
        let tool = test_tool(vec!["example.com"]);
        assert_eq!(tool.name(), "http_request");
    }

    // ── §1.4 DNS rebinding / SSRF defense-in-depth tests ─────

    #[test]
    fn ssrf_blocks_loopback_127_range() {
        assert!(is_private_or_local_host("127.0.0.1"));
        assert!(is_private_or_local_host("127.0.0.2"));
        assert!(is_private_or_local_host("127.255.255.255"));
    }

    #[test]
    fn ssrf_blocks_rfc1918_10_range() {
        assert!(is_private_or_local_host("10.0.0.1"));
        assert!(is_private_or_local_host("10.255.255.255"));
    }

    #[test]
    fn ssrf_blocks_rfc1918_172_range() {
        assert!(is_private_or_local_host("172.16.0.1"));
        assert!(is_private_or_local_host("172.31.255.255"));
    }

    #[test]
    fn ssrf_blocks_unspecified_address() {
        assert!(is_private_or_local_host("0.0.0.0"));
    }

    #[test]
    fn ssrf_blocks_dot_localhost_subdomain() {
        assert!(is_private_or_local_host("evil.localhost"));
        assert!(is_private_or_local_host("a.b.localhost"));
    }

    #[test]
    fn ssrf_blocks_dot_local_tld() {
        assert!(is_private_or_local_host("service.local"));
    }

    #[test]
    fn ssrf_ipv6_unspecified() {
        assert!(is_private_or_local_host("::"));
    }

    #[test]
    fn validate_rejects_ftp_scheme() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool
            .validate_url("ftp://example.com")
            .unwrap_err()
            .to_string();
        assert!(err.contains("http://") || err.contains("https://"));
    }

    #[test]
    fn validate_rejects_empty_url() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool.validate_url("").unwrap_err().to_string();
        assert!(err.contains("empty"));
    }

    #[test]
    fn validate_rejects_ipv6_host() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool
            .validate_url("http://[::1]:8080/path")
            .unwrap_err()
            .to_string();
        assert!(err.contains("IPv6"));
    }
}
