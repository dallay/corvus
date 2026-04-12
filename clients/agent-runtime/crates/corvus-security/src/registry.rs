#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxDescriptor {
    pub key: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub compiled: bool,
    pub platform_supported: bool,
}

const SANDBOXES: &[SandboxDescriptor] = &[
    SandboxDescriptor {
        key: "auto",
        display_name: "Auto Detect",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    SandboxDescriptor {
        key: "none",
        display_name: "No Sandbox",
        aliases: &["noop"],
        compiled: true,
        platform_supported: true,
    },
    SandboxDescriptor {
        key: "docker",
        display_name: "Docker",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    SandboxDescriptor {
        key: "firejail",
        display_name: "Firejail",
        aliases: &[],
        compiled: cfg!(target_os = "linux"),
        platform_supported: cfg!(target_os = "linux"),
    },
    SandboxDescriptor {
        key: "landlock",
        display_name: "Landlock",
        aliases: &[],
        compiled: false,
        platform_supported: cfg!(target_os = "linux"),
    },
    SandboxDescriptor {
        key: "bubblewrap",
        display_name: "Bubblewrap",
        aliases: &[],
        compiled: false,
        platform_supported: cfg!(any(target_os = "linux", target_os = "macos")),
    },
];

pub fn list_sandboxes() -> &'static [SandboxDescriptor] {
    SANDBOXES
}

pub fn resolve_sandbox_key(name: &str) -> Option<&'static str> {
    let candidate = name.trim();
    SANDBOXES
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

pub fn sandbox_availability(name: &str) -> Option<CapabilityAvailability> {
    let key = resolve_sandbox_key(name)?;
    SANDBOXES
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
