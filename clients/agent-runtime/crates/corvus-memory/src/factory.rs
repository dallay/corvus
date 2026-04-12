use anyhow::{anyhow, Result};

use crate::registry::{memory_availability, resolve_memory_backend_key, CapabilityAvailability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryFactorySelection {
    pub key: &'static str,
}

pub fn select_memory_backend(name: &str) -> Result<MemoryFactorySelection> {
    let Some(key) = resolve_memory_backend_key(name) else {
        return Err(anyhow!("unknown memory backend '{name}'"));
    };

    match memory_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(MemoryFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("memory backend '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => Err(anyhow!(
            "memory backend '{key}' is unavailable on this platform"
        )),
        None => Err(anyhow!("unknown memory backend '{name}'")),
    }
}
