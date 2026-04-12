#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelDescriptor {
    pub key: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub compiled: bool,
    pub platform_supported: bool,
}

const CHANNELS: &[ChannelDescriptor] = &[
    ChannelDescriptor {
        key: "stdio",
        display_name: "Standard IO",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "telegram",
        display_name: "Telegram",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "discord",
        display_name: "Discord",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "slack",
        display_name: "Slack",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "mattermost",
        display_name: "Mattermost",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "imessage",
        display_name: "iMessage",
        aliases: &[],
        compiled: cfg!(target_os = "macos"),
        platform_supported: cfg!(target_os = "macos"),
    },
    ChannelDescriptor {
        key: "matrix",
        display_name: "Matrix",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "signal",
        display_name: "Signal",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "whatsapp",
        display_name: "WhatsApp",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "email",
        display_name: "Email",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "irc",
        display_name: "IRC",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "lark",
        display_name: "Lark",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "dingtalk",
        display_name: "DingTalk",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "qq",
        display_name: "QQ",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ChannelDescriptor {
        key: "webhook",
        display_name: "Webhook",
        aliases: &[],
        // Placeholder: webhook channel is not yet implemented (deferred).
        // callers of channel_availability() receive Uncompiled for this key.
        compiled: false,
        platform_supported: true,
    },
];

pub fn list_channels() -> &'static [ChannelDescriptor] {
    CHANNELS
}

pub fn resolve_channel_key(name: &str) -> Option<&'static str> {
    let candidate = name.trim();
    CHANNELS
        .iter()
        .find(|descriptor| {
            descriptor.key.eq_ignore_ascii_case(candidate)
                || descriptor
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(candidate))
        })
        .map(|descriptor| descriptor.key)
}

pub fn channel_availability(name: &str) -> Option<CapabilityAvailability> {
    let key = resolve_channel_key(name)?;
    CHANNELS
        .iter()
        .find(|descriptor| descriptor.key == key)
        .map(|descriptor| {
            if !descriptor.platform_supported {
                CapabilityAvailability::PlatformUnavailable
            } else if !descriptor.compiled {
                CapabilityAvailability::Uncompiled
            } else {
                CapabilityAvailability::Constructible
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- list_channels ---

    #[test]
    fn list_channels_is_non_empty() {
        assert!(!list_channels().is_empty());
    }

    #[test]
    fn list_channels_includes_all_expected_keys() {
        let channels = list_channels();
        let keys: Vec<&str> = channels.iter().map(|c| c.key).collect();
        for expected in &[
            "stdio", "telegram", "discord", "slack", "mattermost", "imessage", "matrix",
            "signal", "whatsapp", "email", "irc", "lark", "dingtalk", "qq", "webhook",
        ] {
            assert!(
                keys.contains(expected),
                "expected channel key '{expected}' not found in registry"
            );
        }
    }

    #[test]
    fn list_channels_has_unique_keys() {
        let channels = list_channels();
        let mut seen = std::collections::HashSet::new();
        for descriptor in channels {
            assert!(
                seen.insert(descriptor.key),
                "duplicate channel key: '{}'",
                descriptor.key
            );
        }
    }

    #[test]
    fn all_channel_descriptors_have_non_empty_display_name() {
        for descriptor in list_channels() {
            assert!(
                !descriptor.display_name.is_empty(),
                "channel '{}' has empty display_name",
                descriptor.key
            );
        }
    }

    // --- resolve_channel_key ---

    #[test]
    fn resolve_channel_key_returns_none_for_unknown() {
        assert_eq!(resolve_channel_key("not-a-real-channel"), None);
        assert_eq!(resolve_channel_key(""), None);
    }

    #[test]
    fn resolve_channel_key_matches_exact_key() {
        assert_eq!(resolve_channel_key("stdio"), Some("stdio"));
        assert_eq!(resolve_channel_key("telegram"), Some("telegram"));
        assert_eq!(resolve_channel_key("webhook"), Some("webhook"));
    }

    #[test]
    fn resolve_channel_key_is_case_insensitive() {
        assert_eq!(resolve_channel_key("STDIO"), Some("stdio"));
        assert_eq!(resolve_channel_key("Telegram"), Some("telegram"));
        assert_eq!(resolve_channel_key("DISCORD"), Some("discord"));
        assert_eq!(resolve_channel_key("Webhook"), Some("webhook"));
    }

    #[test]
    fn resolve_channel_key_trims_leading_and_trailing_whitespace() {
        assert_eq!(resolve_channel_key("  stdio  "), Some("stdio"));
        assert_eq!(resolve_channel_key("\ttelegram\n"), Some("telegram"));
    }

    #[test]
    fn resolve_channel_key_returns_canonical_static_key() {
        // The returned key must be the same static reference, not user-supplied casing.
        let key = resolve_channel_key("SLACK").unwrap();
        assert_eq!(key, "slack");
    }

    // --- channel_availability ---

    #[test]
    fn channel_availability_returns_none_for_unknown() {
        assert_eq!(channel_availability("does-not-exist"), None);
    }

    #[test]
    fn channel_availability_constructible_for_always_on_channels() {
        for name in &["stdio", "telegram", "discord", "slack", "mattermost"] {
            assert_eq!(
                channel_availability(name),
                Some(CapabilityAvailability::Constructible),
                "expected Constructible for '{name}'"
            );
        }
    }

    #[test]
    fn channel_availability_uncompiled_for_webhook() {
        // webhook is explicitly marked compiled=false in the registry.
        assert_eq!(
            channel_availability("webhook"),
            Some(CapabilityAvailability::Uncompiled)
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn channel_availability_platform_unavailable_for_imessage_on_non_macos() {
        assert_eq!(
            channel_availability("imessage"),
            Some(CapabilityAvailability::PlatformUnavailable)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn channel_availability_constructible_for_imessage_on_macos() {
        assert_eq!(
            channel_availability("imessage"),
            Some(CapabilityAvailability::Constructible)
        );
    }

    #[test]
    fn channel_availability_is_case_insensitive() {
        assert_eq!(
            channel_availability("STDIO"),
            Some(CapabilityAvailability::Constructible)
        );
        assert_eq!(
            channel_availability("Webhook"),
            Some(CapabilityAvailability::Uncompiled)
        );
    }

    // --- CapabilityAvailability enum ---

    #[test]
    fn capability_availability_variants_are_eq_comparable() {
        assert_eq!(
            CapabilityAvailability::Constructible,
            CapabilityAvailability::Constructible
        );
        assert_ne!(
            CapabilityAvailability::Constructible,
            CapabilityAvailability::Uncompiled
        );
        assert_ne!(
            CapabilityAvailability::Uncompiled,
            CapabilityAvailability::PlatformUnavailable
        );
    }

    // Regression: whitespace-only input must not match any channel.
    #[test]
    fn resolve_channel_key_whitespace_only_returns_none() {
        assert_eq!(resolve_channel_key("   "), None);
        assert_eq!(resolve_channel_key("\t"), None);
    }
}