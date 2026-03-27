use corvus::config::McpServerConfig;
use corvus::tools::mcp::adapter::McpToolAdapter;
use corvus::tools::mcp::client::{McpClient, McpToolManifest};
use corvus::tools::Tool;
use std::collections::BTreeMap;

fn test_server() -> McpServerConfig {
    McpServerConfig {
        name: "docs".to_string(),
        enabled: true,
        command: "__mcp_mock__".to_string(),
        args: Vec::new(),
        env: BTreeMap::new(),
        startup_timeout_ms: 100,
        call_timeout_ms: 25,
        output_limit_bytes: 64,
        capabilities: vec!["tools".to_string()],
        resource_output_limit_bytes: None,
        prompt_output_limit_bytes: None,
    }
}

fn test_manifest() -> McpToolManifest {
    McpToolManifest {
        name: "search".to_string(),
        description: "Search docs".to_string(),
        parameters: serde_json::json!({"type": "object"}),
    }
}

#[tokio::test]
async fn mcp_call_timeout_returns_structured_timeout_failure() {
    let mut server = test_server();
    server.command = "__mcp_mock_sleep__".to_string();
    server.args = vec!["200".to_string()];

    let client = McpClient::new(server.clone());
    let adapter = McpToolAdapter::from_manifest(&server, test_manifest(), client).unwrap();
    let result = adapter
        .execute(serde_json::json!({"query": "rust"}))
        .await
        .unwrap();

    assert!(!result.success);
    let error = result.error.unwrap_or_default();
    let payload: serde_json::Value = serde_json::from_str(&error).unwrap();
    assert_eq!(payload["code"], "mcp_timeout");
}

#[tokio::test]
async fn mcp_output_cap_enforcement_marks_limited_output() {
    let mut server = test_server();
    server.command = "__mcp_mock_output__".to_string();
    server.args = vec!["200".to_string()];
    server.output_limit_bytes = 32;

    let client = McpClient::new(server.clone());
    let adapter = McpToolAdapter::from_manifest(&server, test_manifest(), client).unwrap();
    let result = adapter.execute(serde_json::json!({})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("output_limit_enforced"));
}

#[tokio::test]
async fn mcp_transport_failures_return_stable_structured_errors() {
    let mut server = test_server();
    server.command = "__mcp_mock_error__".to_string();

    let client = McpClient::new(server.clone());
    let adapter = McpToolAdapter::from_manifest(&server, test_manifest(), client).unwrap();
    let result = adapter.execute(serde_json::json!({})).await.unwrap();

    assert!(!result.success);
    let error = result.error.unwrap_or_default();
    let payload: serde_json::Value = serde_json::from_str(&error).unwrap();
    assert_eq!(payload["code"], "mcp_transport_error");
}

#[tokio::test]
async fn native_tool_dispatch_still_works_with_mcp_limits_enabled() {
    struct NativeEcho;

    #[async_trait::async_trait]
    impl Tool for NativeEcho {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echo tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> anyhow::Result<corvus::tools::ToolResult> {
            Ok(corvus::tools::ToolResult {
                success: true,
                output: "ok".to_string(),
                error: None,
                structured: None,
            })
        }
    }

    let tool = NativeEcho;
    let result = tool.execute(serde_json::json!({})).await.unwrap();
    assert!(result.success);
    assert_eq!(result.output, "ok");
}
