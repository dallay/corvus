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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_registry_case_insensitive_uniqueness() {
        // Collect all lowercased keys and aliases
        let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        let mut failures: Vec<String> = Vec::new();

        for descriptor in TOOLS {
            // Check the primary key
            let key_lower = descriptor.key.to_lowercase();
            if let Some(existing) = seen.insert(&key_lower, descriptor.key) {
                failures.push(format!(
                    "Collision: key '{}' (from '{}') already registered by '{}'",
                    key_lower, descriptor.key, existing
                ));
            }

            // Check each alias
            for alias in descriptor.aliases {
                let alias_lower = alias.to_lowercase();
                if let Some(existing) = seen.insert(&alias_lower, descriptor.key) {
                    failures.push(format!(
                        "Collision: alias '{}' (from '{}') already registered by '{}'",
                        alias_lower, alias, existing
                    ));
                }
            }
        }

        if !failures.is_empty() {
            panic!(
                "Tool registry case-insensitive uniqueness violations:\n{}",
                failures.join("\n")
            );
        }
    }
}
