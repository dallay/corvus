use crate::config::McpServerConfig;
use anyhow::Context;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct McpToolManifest {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct McpResourceManifest {
    pub name: String,
    pub uri: String,
    pub description: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpPromptManifest {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct McpClient {
    server: McpServerConfig,
}

impl McpClient {
    pub fn new(server: McpServerConfig) -> Self {
        Self { server }
    }

    pub fn list_tools(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        match self.server.command.as_str() {
            "__mcp_mock__" => self.list_tools_from_mock_payload(),
            "__mcp_mock_sleep__" => self.list_tools_from_mock_sleep(),
            "__mcp_cerebro_http__" => self.list_tools_http(),
            _ => self.list_tools_from_command(),
        }
    }

    pub fn list_resources(&self) -> anyhow::Result<Vec<McpResourceManifest>> {
        match self.server.command.as_str() {
            "__mcp_mock__" => self.list_resources_from_mock_payload(),
            _ => {
                tracing::debug!(server = %self.server.name, "list_resources not yet implemented for live servers");
                Ok(Vec::new())
            }
        }
    }

    pub fn read_resource(&self, uri: &str) -> anyhow::Result<String> {
        tracing::debug!(
            server = %self.server.name,
            uri = %uri,
            "MCP resource read started"
        );
        match self.server.command.as_str() {
            "__mcp_mock__" => Ok(format!("mock-resource-content for {uri}")),
            _ => {
                anyhow::bail!("read_resource not yet implemented for live servers")
            }
        }
    }

    pub fn list_prompts(&self) -> anyhow::Result<Vec<McpPromptManifest>> {
        match self.server.command.as_str() {
            "__mcp_mock__" => self.list_prompts_from_mock_payload(),
            _ => {
                tracing::debug!(server = %self.server.name, "list_prompts not yet implemented for live servers");
                Ok(Vec::new())
            }
        }
    }

    pub fn get_prompt(
        &self,
        name: &str,
        _arguments: serde_json::Value,
    ) -> anyhow::Result<Vec<PromptMessage>> {
        tracing::debug!(
            server = %self.server.name,
            prompt = %name,
            "MCP prompt get started"
        );
        match self.server.command.as_str() {
            "__mcp_mock__" => Ok(vec![PromptMessage {
                role: "user".to_string(),
                content: format!("mock prompt content for {name}"),
            }]),
            _ => {
                anyhow::bail!("get_prompt not yet implemented for live servers")
            }
        }
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<String> {
        tracing::debug!(
            server = %self.server.name,
            tool = %name,
            timeout_ms = self.server.call_timeout_ms,
            output_limit = self.server.output_limit_bytes,
            "MCP call started"
        );

        match self.server.command.as_str() {
            "__mcp_mock_sleep__" => self.call_tool_mock_sleep(name, &arguments),
            "__mcp_mock_output__" => self.call_tool_mock_output(&arguments),
            "__mcp_mock_error__" => self.call_tool_mock_error(name, &arguments),
            "__mcp_cerebro_http__" => self.call_tool_http(name, &arguments).await,
            "__mcp_mock__" => Ok("mock-ok".to_string()),
            _ => self.call_tool_from_command(name, arguments).await,
        }
    }

    fn call_tool_mock_sleep(
        &self,
        name: &str,
        _arguments: &serde_json::Value,
    ) -> anyhow::Result<String> {
        let delay_ms = self
            .server
            .args
            .first()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        if delay_ms > self.server.call_timeout_ms {
            let reason = format!(
                "mcp call timeout after {}ms for '{}'",
                self.server.call_timeout_ms, name
            );
            tracing::warn!(server = %self.server.name, tool = %name, "{reason}");
            anyhow::bail!(
                "{}",
                json!({
                    "code": "mcp_timeout",
                    "server": self.server.name,
                    "tool": name,
                    "reason": reason,
                })
            );
        }

        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }

        Ok("mock-sleep-ok".to_string())
    }

    fn call_tool_mock_output(&self, _arguments: &serde_json::Value) -> anyhow::Result<String> {
        let output_len = self
            .server
            .args
            .first()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        Ok("x".repeat(output_len))
    }

    fn call_tool_mock_error(
        &self,
        name: &str,
        _arguments: &serde_json::Value,
    ) -> anyhow::Result<String> {
        let reason = format!(
            "mock MCP transport failure for '{}:{}'",
            self.server.name, name
        );
        tracing::warn!(server = %self.server.name, tool = %name, "{reason}");
        anyhow::bail!(
            "{}",
            json!({
                "code": "mcp_transport_error",
                "server": self.server.name,
                "tool": name,
                "reason": reason,
            })
        )
    }

    async fn call_tool_from_command(
        &self,
        name: &str,
        _arguments: serde_json::Value,
    ) -> anyhow::Result<String> {
        use tokio::process::Command as TokioCommand;

        let mut command = TokioCommand::new(&self.server.command);
        command
            .args(&self.server.args)
            .envs(self.server.env.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = command
            .spawn()
            .with_context(|| format!("failed to start MCP server '{}'", self.server.name))?;

        let timeout = Duration::from_millis(self.server.call_timeout_ms);
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(result) => result.context("failed to await MCP process output")?,
            Err(_) => {
                let reason = format!(
                    "mcp call timeout after {}ms for '{}'",
                    self.server.call_timeout_ms, name
                );
                anyhow::bail!(
                    "{}",
                    json!({
                        "code": "mcp_timeout",
                        "server": self.server.name,
                        "tool": name,
                        "reason": reason,
                    })
                );
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let redacted = redact_diagnostic(
                stderr.as_ref(),
                self.server
                    .env
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str())),
            );
            anyhow::bail!(
                "{}",
                json!({
                    "code": "mcp_transport_error",
                    "server": self.server.name,
                    "tool": name,
                    "reason": redacted,
                })
            );
        }

        let stdout = String::from_utf8(output.stdout).context("MCP call output was not UTF-8")?;
        Ok(stdout)
    }

    async fn call_tool_http(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> anyhow::Result<String> {
        let endpoint = self
            .server
            .args
            .first()
            .context("MCP HTTP endpoint missing in server args")?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(self.server.call_timeout_ms))
            .build()
            .context("failed to build MCP HTTP client")?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments,
            }
        });

        let mut req = client.post(endpoint).json(&request);
        if let Some(token) = self.server.env.get("MCP_AUTH_TOKEN") {
            if !token.trim().is_empty() {
                req = req.bearer_auth(token.trim());
            }
        }

        let response = req.send().await.context("MCP HTTP call failed")?;
        let status = response.status();
        let limit = self.server.output_limit_bytes as usize;

        if let Some(content_length) = response.content_length() {
            let content_length = usize::try_from(content_length).map_err(|_| {
                anyhow::anyhow!("MCP HTTP response exceeded output_limit_bytes ({})", limit)
            })?;
            if content_length > limit {
                anyhow::bail!(
                    "MCP HTTP response exceeded output_limit_bytes ({} > {})",
                    content_length,
                    limit
                );
            }
        }

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("MCP HTTP response stream failed")?;
            if body.len().saturating_add(chunk.len()) > limit {
                anyhow::bail!("MCP HTTP response exceeded output_limit_bytes ({})", limit);
            }
            body.extend_from_slice(&chunk);
        }

        if !status.is_success() {
            let details = serde_json::from_slice::<serde_json::Value>(&body).unwrap_or_else(|_| {
                json!({
                    "raw": String::from_utf8_lossy(&body).to_string(),
                })
            });
            anyhow::bail!(
                "{}",
                json!({
                    "code": "mcp_transport_error",
                    "server": self.server.name,
                    "tool": name,
                    "reason": format!("HTTP {}", status.as_u16()),
                    "details": details,
                })
            );
        }

        let payload: serde_json::Value =
            serde_json::from_slice(&body).context("MCP HTTP response was not valid JSON")?;

        if let Some(error) = payload.get("error") {
            anyhow::bail!(
                "{}",
                json!({
                    "code": "mcp_error",
                    "server": self.server.name,
                    "tool": name,
                    "error": error,
                })
            );
        }

        let output = payload
            .get("result")
            .and_then(|value| value.get("output"))
            .ok_or_else(|| anyhow::anyhow!("MCP HTTP response missing result.output"))?;

        if let Some(value) = output.as_str() {
            Ok(value.to_string())
        } else {
            Ok(output.to_string())
        }
    }

    fn list_tools_from_mock_payload(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        let payload = self
            .server
            .args
            .first()
            .context("mock MCP server requires one JSON payload argument")?;
        parse_tool_manifest_payload(payload)
    }

    fn list_resources_from_mock_payload(&self) -> anyhow::Result<Vec<McpResourceManifest>> {
        let payload = self
            .server
            .args
            .first()
            .context("mock MCP server requires one JSON payload argument")?;
        parse_resource_manifest_payload(payload)
    }

    fn list_prompts_from_mock_payload(&self) -> anyhow::Result<Vec<McpPromptManifest>> {
        let payload = self
            .server
            .args
            .first()
            .context("mock MCP server requires one JSON payload argument")?;
        parse_prompt_manifest_payload(payload)
    }

    fn list_tools_from_mock_sleep(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        let delay_ms = self
            .server
            .args
            .first()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);

        if delay_ms > self.server.startup_timeout_ms {
            thread::sleep(Duration::from_millis(self.server.startup_timeout_ms + 5));
            anyhow::bail!(
                "MCP startup discovery timed out after {} ms",
                self.server.startup_timeout_ms
            );
        }

        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }

        Ok(Vec::new())
    }

    fn list_tools_from_command(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        let mut child = Command::new(&self.server.command)
            .args(&self.server.args)
            .envs(self.server.env.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // inherit stderr so we don't block the pipe and capture diagnostic logs
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start MCP server '{}' for discovery",
                    self.server.name
                )
            })?;

        let mut stdout = child.stdout.take().context("child process has no stdout")?;
        let output_limit = self.server.output_limit_bytes;

        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let mut buffer = Vec::new();
            // Read up to limit + 1 bytes to detect truncation without unbounded allocation
            use std::io::Read;
            let result = stdout
                .by_ref()
                .take((output_limit as u64) + 1)
                .read_to_end(&mut buffer);
            let _ = tx.send((buffer, result));
        });

        let timeout = Duration::from_millis(self.server.startup_timeout_ms);
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }

            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!(
                    "MCP startup discovery timed out after {} ms",
                    self.server.startup_timeout_ms
                );
            }

            thread::sleep(Duration::from_millis(10));
        };

        if !status.success() {
            anyhow::bail!(
                "MCP server '{}' exited during discovery with status {}",
                self.server.name,
                status
            );
        }

        let (stdout_bytes, read_result) = rx
            .recv_timeout(Duration::from_secs(2))
            .context("failed to consume stdout from reader thread")?;

        read_result.context("failed to read MCP discovery output")?;

        if stdout_bytes.len() > output_limit {
            anyhow::bail!(
                "MCP discovery output exceeded output_limit_bytes ({})",
                output_limit
            );
        }

        let stdout_str =
            String::from_utf8(stdout_bytes).context("MCP discovery output was not UTF-8")?;
        parse_tool_manifest_payload(&stdout_str)
    }

    fn list_tools_http(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        if tokio::runtime::Handle::try_current().is_ok() {
            let this = self.clone();
            return std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .context("failed to create runtime for MCP HTTP discovery")?;
                runtime.block_on(this.list_tools_http_async())
            })
            .join()
            .map_err(|_| anyhow::anyhow!("MCP HTTP discovery worker thread panicked"))?;
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to create runtime for MCP HTTP discovery")?;
        runtime.block_on(self.list_tools_http_async())
    }

    async fn list_tools_http_async(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        let endpoint = self
            .server
            .args
            .first()
            .context("MCP HTTP endpoint missing in server args")?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(self.server.call_timeout_ms))
            .build()
            .context("failed to build MCP HTTP client")?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": "tools/list",
            "params": {}
        });

        let mut req = client.post(endpoint).json(&request);
        if let Some(token) = self.server.env.get("MCP_AUTH_TOKEN") {
            if !token.trim().is_empty() {
                req = req.bearer_auth(token.trim());
            }
        }

        let response = req.send().await.context("MCP HTTP discovery failed")?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read MCP HTTP discovery body")?;
        let redacted = redact_diagnostic(
            &body,
            self.server.env.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        );

        if !status.is_success() {
            anyhow::bail!(
                "{}",
                json!({
                    "code": "mcp_transport_error",
                    "server": self.server.name,
                    "reason": format!("HTTP {}", status.as_u16()),
                    "details": redacted,
                })
            );
        }

        let payload: serde_json::Value =
            serde_json::from_str(&body).context("MCP HTTP discovery response was not valid JSON")?;

        if let Some(error) = payload.get("error") {
            let safe_error = redact_diagnostic(
                &error.to_string(),
                self.server.env.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            );
            anyhow::bail!(
                "{}",
                json!({
                    "code": "mcp_error",
                    "server": self.server.name,
                    "reason": safe_error,
                })
            );
        }

        let tools = payload
            .get("result")
            .and_then(|value| value.get("tools"))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MCP HTTP discovery response missing result.tools"))?;

        parse_tool_manifest_payload(&json!({ "tools": tools }).to_string())
    }
}

pub(crate) fn redact_diagnostic<'a>(
    input: &str,
    extra_env: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> String {
    let mut sanitized = input.to_string();

    // Redact from process environment variables
    for (key, value) in std::env::vars() {
        let upper = key.to_ascii_uppercase();
        let looks_sensitive = upper.contains("TOKEN")
            || upper.contains("SECRET")
            || upper.contains("PASSWORD")
            || upper.contains("API_KEY")
            || upper.contains("AUTH");
        if looks_sensitive && !value.is_empty() {
            sanitized = sanitized.replace(&value, "[REDACTED]");
        }
    }

    // Redact from extra server env values
    for (_, value) in extra_env {
        if !value.is_empty() {
            sanitized = sanitized.replace(value, "[REDACTED]");
        }
    }

    sanitized
}

#[derive(Debug, Deserialize)]
struct ToolWire {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ResourceWire {
    #[serde(default)]
    name: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PromptWire {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    arguments: Vec<PromptArgument>,
}

fn parse_tool_manifest_payload(payload: &str) -> anyhow::Result<Vec<McpToolManifest>> {
    let value: serde_json::Value =
        serde_json::from_str(payload).context("invalid JSON tool manifest payload")?;

    let parsed: Vec<ToolWire> = if value.is_array() {
        serde_json::from_value(value).context("failed to parse tool list payload")?
    } else if let Some(obj) = value.as_object() {
        let has_resources = obj.contains_key("resources");
        let has_prompts = obj.contains_key("prompts");

        match obj.get("tools") {
            Some(tools) => serde_json::from_value(tools.clone())
                .context("failed to parse tool envelope payload")?,
            None if has_resources || has_prompts => Vec::new(),
            None => anyhow::bail!("failed to parse tool envelope payload: missing 'tools' field"),
        }
    } else {
        anyhow::bail!("failed to parse tool manifest payload: expected JSON array or object");
    };

    let manifests = parsed
        .into_iter()
        .map(|tool| McpToolManifest {
            name: tool.name,
            description: tool.description,
            parameters: if tool.parameters.is_null() {
                serde_json::json!({"type": "object"})
            } else {
                tool.parameters
            },
        })
        .collect();

    Ok(manifests)
}

pub(crate) fn parse_resource_manifest_payload(
    payload: &str,
) -> anyhow::Result<Vec<McpResourceManifest>> {
    let value: serde_json::Value =
        serde_json::from_str(payload).context("invalid JSON manifest payload")?;

    let resources_value = if let Some(obj) = value.as_object() {
        obj.get("resources").cloned()
    } else {
        None
    };

    let Some(resources_value) = resources_value else {
        return Ok(Vec::new());
    };

    let parsed: Vec<ResourceWire> =
        serde_json::from_value(resources_value).context("failed to parse resources payload")?;

    Ok(parsed
        .into_iter()
        .map(|r| McpResourceManifest {
            name: r.name,
            uri: r.uri,
            description: r.description,
            mime_type: r.mime_type,
        })
        .collect())
}

pub(crate) fn parse_prompt_manifest_payload(
    payload: &str,
) -> anyhow::Result<Vec<McpPromptManifest>> {
    let value: serde_json::Value =
        serde_json::from_str(payload).context("invalid JSON manifest payload")?;

    let prompts_value = if let Some(obj) = value.as_object() {
        obj.get("prompts").cloned()
    } else {
        None
    };

    let Some(prompts_value) = prompts_value else {
        return Ok(Vec::new());
    };

    let parsed: Vec<PromptWire> =
        serde_json::from_value(prompts_value).context("failed to parse prompts payload")?;

    Ok(parsed
        .into_iter()
        .map(|p| McpPromptManifest {
            name: p.name,
            description: p.description,
            arguments: p.arguments,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct MockMcpState {
        requests: Arc<Mutex<Vec<serde_json::Value>>>,
    }

    fn http_server_config(endpoint: String, token: &str) -> McpServerConfig {
        let mut env = BTreeMap::new();
        env.insert("MCP_AUTH_TOKEN".to_string(), token.to_string());
        McpServerConfig {
            name: "cerebro".into(),
            command: "__mcp_cerebro_http__".into(),
            args: vec![endpoint],
            env,
            call_timeout_ms: 5_000,
            ..McpServerConfig::default()
        }
    }

    async fn spawn_mcp_server(
        token: &'static str,
        status: StatusCode,
        payload: serde_json::Value,
    ) -> (String, MockMcpState) {
        async fn handler(
            State(state): State<MockMcpState>,
            headers: axum::http::HeaderMap,
            Json(body): Json<serde_json::Value>,
        ) -> impl IntoResponse {
            state.requests.lock().unwrap().push(body.clone());

            let auth = headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();

            let expected = format!("Bearer {}", body["params"].get("token_expect").and_then(|v| v.as_str()).unwrap_or("test-token"));
            if auth != expected {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": {"message": format!("invalid token {auth}")}})),
                );
            }

            let status = body["params"].get("status_code").and_then(|v| v.as_u64()).unwrap_or(200) as u16;
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
                Json(body["params"].get("response_payload").cloned().unwrap_or_else(|| json!({}))),
            )
        }

        let state = MockMcpState::default();
        let payload_clone = payload.clone();
        let app = Router::new()
            .route(
                "/",
                post(
                    move |State(state): State<MockMcpState>,
                          headers: axum::http::HeaderMap,
                          Json(mut body): Json<serde_json::Value>| {
                        let payload = payload_clone.clone();
                        async move {
                            body["params"]["token_expect"] = serde_json::Value::String(token.to_string());
                            body["params"]["status_code"] = serde_json::Value::from(status.as_u16());
                            body["params"]["response_payload"] = payload;
                            handler(State(state), headers, Json(body)).await
                        }
                    },
                ),
            )
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), state)
    }

    #[test]
    fn parse_payload_ignores_non_tool_capabilities_when_tools_exist() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search docs","parameters":{"type":"object"}}],
          "resources": [{"uri":"docs://index"}],
          "prompts": [{"name":"summarize"}]
        }"#;

        let tools = parse_tool_manifest_payload(payload).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "search");
    }

    #[test]
    fn parse_payload_with_only_non_tool_capabilities_returns_empty_tools() {
        let payload = r#"{
          "resources": [{"uri":"docs://index"}],
          "prompts": [{"name":"summarize"}]
        }"#;

        let tools = parse_tool_manifest_payload(payload).unwrap();
        assert!(tools.is_empty());
    }

    // ── Resource manifest parsing ────────────────────────────

    #[test]
    fn parse_resource_manifest_from_mixed_payload() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search docs"}],
          "resources": [
            {"name":"api-spec","uri":"docs://api-spec","description":"API specification","mime_type":"text/markdown"},
            {"name":"changelog","uri":"docs://changelog","description":"Changelog"}
          ]
        }"#;

        let resources = parse_resource_manifest_payload(payload).unwrap();
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].name, "api-spec");
        assert_eq!(resources[0].uri, "docs://api-spec");
        assert_eq!(resources[0].mime_type.as_deref(), Some("text/markdown"));
        assert_eq!(resources[1].name, "changelog");
        assert!(resources[1].mime_type.is_none());
    }

    #[test]
    fn parse_resource_manifest_returns_empty_when_no_resources_key() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search docs"}]
        }"#;

        let resources = parse_resource_manifest_payload(payload).unwrap();
        assert!(resources.is_empty());
    }

    #[test]
    fn parse_resource_manifest_returns_empty_for_plain_array() {
        let payload = r#"[{"name":"search"}]"#;
        let resources = parse_resource_manifest_payload(payload).unwrap();
        assert!(resources.is_empty());
    }

    // ── Prompt manifest parsing ──────────────────────────────

    #[test]
    fn parse_prompt_manifest_from_mixed_payload() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search docs"}],
          "prompts": [
            {
              "name":"code-review",
              "description":"Code review template",
              "arguments":[
                {"name":"language","description":"Programming language","required":true},
                {"name":"focus","description":"Review focus area","required":false}
              ]
            }
          ]
        }"#;

        let prompts = parse_prompt_manifest_payload(payload).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "code-review");
        assert_eq!(prompts[0].arguments.len(), 2);
        assert!(prompts[0].arguments[0].required);
        assert!(!prompts[0].arguments[1].required);
    }

    #[test]
    fn parse_prompt_manifest_returns_empty_when_no_prompts_key() {
        let payload = r#"{
          "tools": [{"name":"search","description":"Search docs"}]
        }"#;

        let prompts = parse_prompt_manifest_payload(payload).unwrap();
        assert!(prompts.is_empty());
    }

    #[test]
    fn parse_payload_only_resources_no_tools_returns_empty_tools_valid_resources() {
        let payload = r#"{
          "resources": [{"name":"index","uri":"docs://index","description":"Index"}]
        }"#;

        let tools = parse_tool_manifest_payload(payload).unwrap();
        assert!(tools.is_empty());

        let resources = parse_resource_manifest_payload(payload).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "index");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_tools_uses_live_http_discovery() {
        let (endpoint, state) = spawn_mcp_server(
            "secret-token",
            StatusCode::OK,
            json!({
                "result": {
                    "tools": [
                        {"name": "mem_search", "description": "Search", "parameters": {"type": "object"}},
                        {"name": "mem_stats", "description": "Stats", "parameters": {"type": "object"}}
                    ]
                }
            }),
        )
        .await;

        let client = McpClient::new(http_server_config(endpoint, "secret-token"));
        let tools = client.list_tools().unwrap();

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "mem_search");
        let requests = state.requests.lock().unwrap();
        assert_eq!(requests[0]["method"], "tools/list");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_tools_redacts_auth_token_from_failures() {
        let secret = "super-secret-token";
        let (endpoint, _state) = spawn_mcp_server(
            secret,
            StatusCode::UNAUTHORIZED,
            json!({
                "error": {
                    "message": format!("token rejected: {secret}")
                }
            }),
        )
        .await;

        let client = McpClient::new(http_server_config(endpoint, secret));
        let error = client.list_tools().unwrap_err().to_string();

        assert!(error.contains("HTTP 401"));
        assert!(!error.contains(secret));
    }
}
