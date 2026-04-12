#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryDescriptor {
    pub key: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub compiled: bool,
    pub platform_supported: bool,
}

const MEMORY_BACKENDS: &[MemoryDescriptor] = &[
    MemoryDescriptor {
        key: "sqlite",
        display_name: "SQLite",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    MemoryDescriptor {
        key: "lucid",
        display_name: "Lucid",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    MemoryDescriptor {
        key: "markdown",
        display_name: "Markdown",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    MemoryDescriptor {
        key: "none",
        display_name: "None",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
];

pub fn list_memory_backends() -> &'static [MemoryDescriptor] {
    MEMORY_BACKENDS
}

pub fn resolve_memory_backend_key(name: &str) -> Option<&'static str> {
    let candidate = name.trim();
    MEMORY_BACKENDS
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

pub fn memory_availability(name: &str) -> Option<CapabilityAvailability> {
    let key = resolve_memory_backend_key(name)?;
    MEMORY_BACKENDS
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
