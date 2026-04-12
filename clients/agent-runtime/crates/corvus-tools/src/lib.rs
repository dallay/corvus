//! Corvus tool registry surfaces for manifest composition.

pub mod factory;
pub mod registry;

pub use corvus_traits::tools::{Tool, ToolResult, ToolSpec};
pub use factory::{select_tool, ToolFactorySelection};
pub use registry::{
    list_tools, resolve_tool_key, tool_availability, CapabilityAvailability, ToolDescriptor,
};
