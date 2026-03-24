//! Hardware memory map tool — returns flash/RAM address ranges for connected boards.
//!
//! Phase B: When user asks "what are the upper and lower memory addresses?", this tool
//! returns the memory map. Uses probe-rs for Nucleo/STM32 when available; otherwise
//! returns static maps from datasheets.

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;

/// Known memory maps (from datasheets). Used when probe-rs is unavailable.
const MEMORY_MAPS: &[(&str, &str)] = &[
    (
        "nucleo-f401re",
        "Flash: 0x0800_0000 - 0x0807_FFFF (512 KB)\nRAM: 0x2000_0000 - 0x2001_FFFF (128 KB)\nSTM32F401RET6, ARM Cortex-M4",
    ),
    (
        "nucleo-f411re",
        "Flash: 0x0800_0000 - 0x0807_FFFF (512 KB)\nRAM: 0x2000_0000 - 0x2001_FFFF (128 KB)\nSTM32F411RET6, ARM Cortex-M4",
    ),
    (
        "arduino-uno",
        "Flash: 0x0000 - 0x3FFF (16 KB, ATmega328P)\nSRAM: 0x0100 - 0x08FF (2 KB)\nEEPROM: 0x0000 - 0x03FF (1 KB)",
    ),
    (
        "arduino-mega",
        "Flash: 0x0000 - 0x3FFFF (256 KB, ATmega2560)\nSRAM: 0x0200 - 0x21FF (8 KB)\nEEPROM: 0x0000 - 0x0FFF (4 KB)",
    ),
    (
        "esp32",
        "Flash: 0x3F40_0000 - 0x3F7F_FFFF (4 MB typical)\nIRAM: 0x4000_0000 - 0x4005_FFFF\nDRAM: 0x3FFB_0000 - 0x3FFF_FFFF",
    ),
];

/// Build a structured error `ToolResult` with a code and message.
fn structured_err(code: &str, message: &str) -> ToolResult {
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(message.to_string()),
        structured: Some(json!({
            "code": code,
            "message": message,
        })),
    }
}

/// Resolve the board name from args, defaulting to the first configured board.
/// Returns `Ok(Ok(board))` on success, `Ok(Err(ToolResult))` for validation errors.
fn resolve_board_arg(
    args: &serde_json::Value,
    boards: &[String],
) -> anyhow::Result<Result<String, ToolResult>> {
    let obj = match args.as_object() {
        Some(obj) => obj,
        None => {
            return Ok(Err(structured_err(
                "invalid_args",
                "hardware_memory_map args must be a JSON object",
            )));
        }
    };

    let board = match obj.get("board") {
        Some(value) => match value.as_str() {
            Some(b) => b.to_string(),
            None => {
                return Ok(Err(structured_err(
                    "invalid_args",
                    "'board' must be a string",
                )));
            }
        },
        None => boards[0].clone(),
    };

    Ok(Ok(board))
}

/// Tool: report hardware memory map for connected boards.
pub struct HardwareMemoryMapTool {
    boards: Vec<String>,
}

impl HardwareMemoryMapTool {
    pub fn new(boards: Vec<String>) -> Self {
        Self { boards }
    }

    fn static_map_for_board(&self, board: &str) -> Option<&'static str> {
        MEMORY_MAPS
            .iter()
            .find(|(b, _)| *b == board)
            .map(|(_, m)| *m)
    }

    /// Look up the memory map for a board using probe-rs (if available) or static data.
    /// Returns `(output_text, map_text, source)`.
    fn lookup_memory_map(&self, board: &str) -> (String, Option<String>, &'static str) {
        let mut output = String::new();
        let mut map_text: Option<String> = None;
        let mut source = "unknown";

        #[cfg(feature = "probe")]
        let probe_ok = {
            if board == "nucleo-f401re" || board == "nucleo-f411re" {
                let chip = if board == "nucleo-f411re" {
                    "STM32F411RETx"
                } else {
                    "STM32F401RETx"
                };
                match probe_rs_memory_map(chip) {
                    Ok(probe_msg) => {
                        use std::fmt::Write;
                        let _ = writeln!(output, "**{}** (via probe-rs):\n{}", board, probe_msg);
                        map_text = Some(probe_msg);
                        source = "probe-rs";
                        true
                    }
                    Err(e) => {
                        use std::fmt::Write;
                        let _ = write!(output, "Probe-rs failed: {}. ", e);
                        false
                    }
                }
            } else {
                false
            }
        };

        #[cfg(not(feature = "probe"))]
        let probe_ok = false;

        if !probe_ok {
            if let Some(map) = self.static_map_for_board(board) {
                use std::fmt::Write;
                let _ = write!(output, "**{board}** (from datasheet):\n{map}");
                map_text = Some(map.to_string());
                source = "datasheet";
            } else {
                use std::fmt::Write;
                let known: Vec<&str> = MEMORY_MAPS.iter().map(|(b, _)| *b).collect();
                let _ = write!(
                    output,
                    "No memory map for board '{board}'. Known boards: {}",
                    known.join(", ")
                );
            }
        }

        (output, map_text, source)
    }
}

#[async_trait]
impl Tool for HardwareMemoryMapTool {
    fn name(&self) -> &str {
        "hardware_memory_map"
    }

    fn description(&self) -> &str {
        "Return the memory map (flash and RAM address ranges) for connected hardware. Use when: user asks for 'upper and lower memory addresses', 'memory map', 'address space', or 'readable addresses'. Returns flash/RAM ranges from datasheets."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "board": {
                    "type": "string",
                    "description": "Optional board name (e.g. nucleo-f401re, arduino-uno). If omitted, returns map for first configured board."
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if self.boards.is_empty() {
            return Ok(structured_err(
                "no_peripherals",
                "No peripherals configured. Add boards to config.toml [peripherals.boards].",
            ));
        }

        let board = match resolve_board_arg(&args, &self.boards)? {
            Ok(b) => b,
            Err(r) => return Ok(r),
        };

        if !self.boards.iter().any(|known| known == &board) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Board '{board}' is not configured. Configured boards: {}",
                    self.boards.join(", ")
                )),
                structured: Some(json!({
                    "error_code": "UNCONFIGURED_BOARD",
                    "board": board,
                    "configured_boards": self.boards,
                })),
            });
        }

        let (output, map_text, source) = self.lookup_memory_map(&board);

        if map_text.is_none() {
            return Ok(ToolResult {
                success: false,
                output,
                error: Some(format!("No memory map for board '{board}'")),
                structured: Some(json!({
                    "board": board,
                    "source": source,
                    "map": map_text,
                })),
            });
        }

        Ok(ToolResult {
            success: true,
            output,
            error: None,
            structured: Some(json!({
                "board": board,
                "source": source,
                "map": map_text,
            })),
        })
    }
}

#[cfg(feature = "probe")]
fn probe_rs_memory_map(chip: &str) -> anyhow::Result<String> {
    use probe_rs::config::MemoryRegion;
    use probe_rs::{Session, SessionConfig};
    use std::fmt::Write;

    let session = Session::auto_attach(chip, SessionConfig::default())
        .map_err(|e| anyhow::anyhow!("probe-rs attach failed: {}", e))?;

    let target = session.target();
    let mut out = String::new();

    for region in &target.memory_map {
        match region {
            MemoryRegion::Ram(ram) => {
                let start = ram.range.start;
                let end = ram.range.end;
                let size_kb = (end - start) / 1024;
                let _ = writeln!(out, "RAM: 0x{:08X} - 0x{:08X} ({} KB)", start, end, size_kb);
            }
            MemoryRegion::Nvm(flash) => {
                let start = flash.range.start;
                let end = flash.range.end;
                let size_kb = (end - start) / 1024;
                let _ = writeln!(
                    out,
                    "Flash: 0x{:08X} - 0x{:08X} ({} KB)",
                    start, end, size_kb
                );
            }
            MemoryRegion::Generic(_) => {}
        }
    }

    if out.is_empty() {
        out = "Could not read memory regions from probe.".to_string();
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_map_nucleo() {
        let tool = HardwareMemoryMapTool::new(vec!["nucleo-f401re".into()]);
        assert!(tool.static_map_for_board("nucleo-f401re").is_some());
        assert!(tool
            .static_map_for_board("nucleo-f401re")
            .unwrap()
            .contains("Flash"));
    }

    #[test]
    fn static_map_arduino() {
        let tool = HardwareMemoryMapTool::new(vec!["arduino-uno".into()]);
        assert!(tool.static_map_for_board("arduino-uno").is_some());
    }

    #[test]
    fn parameters_schema_has_additional_properties_false() {
        let tool = HardwareMemoryMapTool::new(vec!["nucleo-f401re".into()]);
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert_eq!(schema["additionalProperties"], json!(false));
        assert!(schema["properties"]["board"].is_object());
    }

    #[tokio::test]
    async fn unconfigured_board_returns_structured_error() {
        let tool = HardwareMemoryMapTool::new(vec!["nucleo-f401re".into()]);
        let result = tool
            .execute(json!({"board": "unknown-board"}))
            .await
            .unwrap();
        assert!(!result.success);
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["error_code"], "UNCONFIGURED_BOARD");
        assert_eq!(structured["board"], "unknown-board");
        assert!(structured["configured_boards"]
            .as_array()
            .unwrap()
            .contains(&json!("nucleo-f401re")));
    }

    #[tokio::test]
    async fn no_peripherals_returns_no_peripherals_error() {
        let tool = HardwareMemoryMapTool::new(vec![]);
        let result = tool.execute(json!({})).await.unwrap();
        assert!(!result.success);
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["code"], "no_peripherals");
    }

    #[tokio::test]
    async fn configured_board_returns_memory_map() {
        let tool = HardwareMemoryMapTool::new(vec!["nucleo-f401re".into()]);
        let result = tool
            .execute(json!({"board": "nucleo-f401re"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("Flash"));
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["board"], "nucleo-f401re");
        assert_eq!(structured["source"], "datasheet");
        assert!(structured["map"].as_str().unwrap().contains("0x0800_0000"));
    }

    #[tokio::test]
    async fn default_board_when_omitted() {
        let tool = HardwareMemoryMapTool::new(vec!["arduino-uno".into(), "nucleo-f401re".into()]);
        let result = tool.execute(json!({})).await.unwrap();
        assert!(result.success);
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["board"], "arduino-uno");
    }

    #[tokio::test]
    async fn invalid_args_not_object() {
        let tool = HardwareMemoryMapTool::new(vec!["nucleo-f401re".into()]);
        let result = tool.execute(json!("not an object")).await.unwrap();
        assert!(!result.success);
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["code"], "invalid_args");
    }

    #[tokio::test]
    async fn board_arg_not_string() {
        let tool = HardwareMemoryMapTool::new(vec!["nucleo-f401re".into()]);
        let result = tool.execute(json!({"board": 42})).await.unwrap();
        assert!(!result.success);
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["code"], "invalid_args");
    }

    #[test]
    fn lookup_unknown_board_no_map() {
        let tool = HardwareMemoryMapTool::new(vec!["custom-board".into()]);
        let (output, map_text, _source) = tool.lookup_memory_map("custom-board");
        assert!(map_text.is_none());
        assert!(output.contains("No memory map"));
    }

    #[test]
    fn static_map_esp32() {
        let tool = HardwareMemoryMapTool::new(vec!["esp32".into()]);
        let map = tool.static_map_for_board("esp32").unwrap();
        assert!(map.contains("DRAM"));
        assert!(map.contains("IRAM"));
    }

    #[test]
    fn name_and_description() {
        let tool = HardwareMemoryMapTool::new(vec![]);
        assert_eq!(tool.name(), "hardware_memory_map");
        assert!(!tool.description().is_empty());
    }
}
