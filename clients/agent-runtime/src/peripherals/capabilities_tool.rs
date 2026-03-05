//! Hardware capabilities tool — Phase C: query device for reported GPIO pins.

use super::serial::SerialTransport;
use crate::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Tool: query device capabilities (GPIO pins, LED pin) from firmware.
pub struct HardwareCapabilitiesTool {
    /// (board_name, transport) for each serial board.
    boards: Vec<(String, Arc<SerialTransport>)>,
}

impl HardwareCapabilitiesTool {
    pub(crate) fn new(boards: Vec<(String, Arc<SerialTransport>)>) -> Self {
        Self { boards }
    }
}

fn includes_board(filter: Option<&str>, board_name: &str) -> bool {
    match filter {
        Some(expected) => expected == board_name,
        None => true,
    }
}

fn format_capabilities_success(board_name: &str, output: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(output) {
        let gpio = parsed.get("gpio").cloned().unwrap_or_else(|| json!([]));
        let led_pin = parsed
            .get("led_pin")
            .cloned()
            .unwrap_or_else(|| json!(null));

        return format!("{}: gpio {:?}, led_pin {:?}", board_name, gpio, led_pin);
    }

    format!("{}: {}", board_name, output)
}

fn format_capabilities_result(board_name: &str, result: &ToolResult) -> String {
    if result.success {
        return format_capabilities_success(board_name, &result.output);
    }

    format!(
        "{}: {}",
        board_name,
        result.error.as_deref().unwrap_or("unknown")
    )
}

fn empty_capabilities_message(filter: Option<&str>) -> String {
    match filter {
        Some(_) => "No matching board or capabilities not supported.".to_string(),
        None => "No serial boards configured or capabilities not supported.".to_string(),
    }
}

#[async_trait]
impl Tool for HardwareCapabilitiesTool {
    fn name(&self) -> &str {
        "hardware_capabilities"
    }

    fn description(&self) -> &str {
        "Query connected hardware for reported GPIO pins and LED pin. Use when: user asks what pins are available."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "board": {
                    "type": "string",
                    "description": "Optional board name. If omitted, queries all."
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let filter = args.get("board").and_then(|v| v.as_str());
        let mut outputs = Vec::new();

        for (board_name, transport) in &self.boards {
            if !includes_board(filter, board_name) {
                continue;
            }

            match transport.capabilities().await {
                Ok(result) => {
                    outputs.push(format_capabilities_result(board_name, &result));
                }
                Err(e) => {
                    outputs.push(format!("{}: error - {}", board_name, e));
                }
            }
        }

        let output = if outputs.is_empty() {
            empty_capabilities_message(filter)
        } else {
            outputs.join("\n")
        };

        Ok(ToolResult {
            success: !outputs.is_empty(),
            output,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_json_capabilities_output() {
        let output = format_capabilities_success("uno", r#"{"gpio":[2,13],"led_pin":13}"#);
        assert!(output.contains("uno: gpio"));
        assert!(output.contains("13"));
    }

    #[test]
    fn formats_plain_text_capabilities_output() {
        let output = format_capabilities_success("uno", "raw capabilities");
        assert_eq!(output, "uno: raw capabilities");
    }

    #[test]
    fn formats_error_result_with_unknown_fallback() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: None,
        };
        let output = format_capabilities_result("uno", &result);
        assert_eq!(output, "uno: unknown");
    }

    #[test]
    fn includes_board_filter_matches_expected_name() {
        assert!(includes_board(None, "uno"));
        assert!(includes_board(Some("uno"), "uno"));
        assert!(!includes_board(Some("esp32"), "uno"));
    }
}
