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
