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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_channel_returns_ok_for_constructible_channel() {
        let result = select_channel("stdio");
        assert!(result.is_ok(), "expected Ok for 'stdio', got: {result:?}");
        assert_eq!(result.unwrap().key, "stdio");
    }

    #[test]
    fn select_channel_returns_ok_for_multiple_always_on_channels() {
        for name in &["telegram", "discord", "slack", "mattermost", "matrix"] {
            let result = select_channel(name);
            assert!(
                result.is_ok(),
                "expected Ok for '{name}', got: {result:?}"
            );
        }
    }

    #[test]
    fn select_channel_key_is_canonical_static_str() {
        // Input can be uppercase; returned key must be the lowercase canonical key.
        let selection = select_channel("STDIO").unwrap();
        assert_eq!(selection.key, "stdio");
    }

    #[test]
    fn select_channel_err_for_completely_unknown_name() {
        let result = select_channel("not-a-channel");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unknown channel"),
            "error message was: {msg}"
        );
    }

    #[test]
    fn select_channel_err_for_empty_string() {
        let result = select_channel("");
        assert!(result.is_err());
    }

    #[test]
    fn select_channel_err_with_not_compiled_for_webhook() {
        // webhook is compiled=false in the registry → Uncompiled
        let result = select_channel("webhook");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not compiled"),
            "expected 'not compiled' in error, got: {msg}"
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn select_channel_err_platform_unavailable_for_imessage_on_non_macos() {
        let result = select_channel("imessage");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unavailable on this platform"),
            "expected platform-unavailable error, got: {msg}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn select_channel_ok_for_imessage_on_macos() {
        let result = select_channel("imessage");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().key, "imessage");
    }

    #[test]
    fn channel_factory_selection_is_copy() {
        let sel = ChannelFactorySelection { key: "stdio" };
        let copy = sel;
        assert_eq!(sel, copy);
    }

    // Regression: selecting a channel twice should produce identical results.
    #[test]
    fn select_channel_is_idempotent() {
        let first = select_channel("discord").unwrap();
        let second = select_channel("discord").unwrap();
        assert_eq!(first, second);
    }
}