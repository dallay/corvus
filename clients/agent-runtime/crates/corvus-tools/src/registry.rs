#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub key: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub compiled: bool,
    pub platform_supported: bool,
}

const TOOLS: &[ToolDescriptor] = &[
    ToolDescriptor {
        key: "shell",
        display_name: "Shell",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "code_search",
        display_name: "Code Search",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "file_read",
        display_name: "File Read",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "file_write",
        display_name: "File Write",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "cron_add",
        display_name: "Cron Add",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "cron_list",
        display_name: "Cron List",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "cron_remove",
        display_name: "Cron Remove",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "cron_update",
        display_name: "Cron Update",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "cron_run",
        display_name: "Cron Run",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "cron_runs",
        display_name: "Cron Runs",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "schedule",
        display_name: "Schedule",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "git_operations",
        display_name: "Git Operations",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "pushover",
        display_name: "Pushover",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "memory_store",
        display_name: "Memory Store",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "memory_recall",
        display_name: "Memory Recall",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "memory_forget",
        display_name: "Memory Forget",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "browser_open",
        display_name: "Browser Open",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "browser",
        display_name: "Browser",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "http_request",
        display_name: "HTTP Request",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "web_search_tool",
        display_name: "Web Search",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "screenshot",
        display_name: "Screenshot",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "image_info",
        display_name: "Image Info",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "delegate",
        display_name: "Delegate",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "composio",
        display_name: "Composio",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "hardware_board_info",
        display_name: "Hardware Board Info",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "hardware_memory_map",
        display_name: "Hardware Memory Map",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "hardware_memory_read",
        display_name: "Hardware Memory Read",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ToolDescriptor {
        key: "mcp.dynamic",
        display_name: "MCP Dynamic Tool",
        aliases: &[],
        compiled: false,
        platform_supported: true,
    },
];

pub fn list_tools() -> &'static [ToolDescriptor] {
    TOOLS
}

pub fn resolve_tool_key(name: &str) -> Option<&'static str> {
    let candidate = name.trim();
    TOOLS
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

pub fn tool_availability(name: &str) -> Option<CapabilityAvailability> {
    let key = resolve_tool_key(name)?;
    TOOLS
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
