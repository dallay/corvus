use anyhow::{anyhow, Result};

use crate::registry::{resolve_tool_key, tool_availability, CapabilityAvailability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolFactorySelection {
    pub key: &'static str,
}

pub fn select_tool(name: &str) -> Result<ToolFactorySelection> {
    let Some(key) = resolve_tool_key(name) else {
        return Err(anyhow!("unknown tool '{name}'"));
    };

    match tool_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(ToolFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("tool '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => {
            Err(anyhow!("tool '{key}' is unavailable on this platform"))
        }
        None => Err(anyhow!("unknown tool '{name}'")),
    }
}
