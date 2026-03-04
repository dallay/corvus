use crate::config::McpServerConfig;
use anyhow::Context;
use serde::Deserialize;
use serde_json::json;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct McpToolManifest {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
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
            _ => self.list_tools_from_command(),
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

    fn list_tools_from_mock_payload(&self) -> anyhow::Result<Vec<McpToolManifest>> {
        let payload = self
            .server
            .args
            .first()
            .context("mock MCP server requires one JSON payload argument")?;
        parse_tool_manifest_payload(payload)
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
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start MCP server '{}' for discovery",
                    self.server.name
                )
            })?;

        let timeout = Duration::from_millis(self.server.startup_timeout_ms);
        let start = Instant::now();
        let output = loop {
            if let Some(status) = child.try_wait()? {
                let output = child
                    .wait_with_output()
                    .context("failed to read MCP discovery output")?;
                if !status.success() {
                    anyhow::bail!(
                        "MCP server '{}' exited during discovery with status {}",
                        self.server.name,
                        status
                    );
                }
                break output;
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

        let stdout =
            String::from_utf8(output.stdout).context("MCP discovery output was not UTF-8")?;
        parse_tool_manifest_payload(&stdout)
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

fn parse_tool_manifest_payload(payload: &str) -> anyhow::Result<Vec<McpToolManifest>> {
    let value: serde_json::Value =
        serde_json::from_str(payload).context("invalid JSON tool manifest payload")?;

    let parsed: Vec<ToolWire> = if value.is_array() {
        serde_json::from_value(value).context("failed to parse tool list payload")?
    } else if let Some(obj) = value.as_object() {
        let has_resources = obj.contains_key("resources");
        let has_prompts = obj.contains_key("prompts");
        if has_resources || has_prompts {
            tracing::warn!(
                has_resources,
                has_prompts,
                "MCP payload advertised unsupported non-tool capabilities; ignoring for v1",
            );
        }

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
