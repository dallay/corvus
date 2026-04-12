use anyhow::{anyhow, Result};

use crate::registry::{resolve_sandbox_key, sandbox_availability, CapabilityAvailability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxFactorySelection {
    pub key: &'static str,
}

pub fn select_sandbox(name: &str) -> Result<SandboxFactorySelection> {
    let Some(key) = resolve_sandbox_key(name) else {
        return Err(anyhow!("unknown sandbox '{name}'"));
    };

    match sandbox_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(SandboxFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("sandbox '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => {
            Err(anyhow!("sandbox '{key}' is unavailable on this platform"))
        }
        None => Err(anyhow!("unknown sandbox '{name}'")),
    }
}
