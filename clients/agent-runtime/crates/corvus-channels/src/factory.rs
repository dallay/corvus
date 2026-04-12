use anyhow::{anyhow, Result};

use crate::registry::{channel_availability, resolve_channel_key, CapabilityAvailability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelFactorySelection {
    pub key: &'static str,
}

pub fn select_channel(name: &str) -> Result<ChannelFactorySelection> {
    let Some(key) = resolve_channel_key(name) else {
        return Err(anyhow!("unknown channel '{name}'"));
    };

    match channel_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(ChannelFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("channel '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => {
            Err(anyhow!("channel '{key}' is unavailable on this platform"))
        }
        None => Err(anyhow!("unknown channel '{name}'")),
    }
}
