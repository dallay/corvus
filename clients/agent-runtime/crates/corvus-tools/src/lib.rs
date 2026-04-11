//! Corvus Tools Registry
//!
//! Re-exports tool types and provides registry functions.

pub use corvus_traits::tools::{Tool, ToolResult, ToolSpec};

/// Information about a tool.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub display_name: &'static str,
}
