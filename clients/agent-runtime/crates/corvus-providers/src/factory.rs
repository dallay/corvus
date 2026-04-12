use anyhow::{anyhow, Result};

use crate::registry::{provider_availability, resolve_provider_key, CapabilityAvailability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderFactorySelection {
    pub key: &'static str,
}

pub fn select_provider(name: &str) -> Result<ProviderFactorySelection> {
    let Some(key) = resolve_provider_key(name) else {
        return Err(anyhow!("unknown provider '{name}'"));
    };

    match provider_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(ProviderFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("provider '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => {
            Err(anyhow!("provider '{key}' is unavailable on this platform"))
        }
        None => Err(anyhow!("unknown provider '{name}'")),
    }
}
