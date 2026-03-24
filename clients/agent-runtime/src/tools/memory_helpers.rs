use super::traits::ToolResult;
use crate::config::MemoryCerebroConfig;

/// Extract a trimmed, non-empty string from a JSON args object.
pub(crate) fn extract_trimmed_str<'a>(args: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// Build a failure `ToolResult` with the given error message.
pub(crate) fn err_result(msg: &str) -> ToolResult {
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(msg.to_string()),
        structured: None,
    }
}

/// Validate and return the Cerebro endpoint, or a failure `ToolResult`.
pub(crate) fn validated_endpoint<'a>(
    cerebro: &'a MemoryCerebroConfig,
    tool_name: &str,
) -> Result<&'a str, ToolResult> {
    cerebro
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| err_result(&format!("Cerebro MCP endpoint is required for {tool_name}")))
}
