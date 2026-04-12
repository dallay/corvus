use corvus_channels::{channel_availability, list_channels};
use corvus_memory::{list_memory_backends, memory_availability};
use corvus_observability::{list_observers, observer_availability};
use corvus_providers::{list_providers, provider_availability};
use corvus_security::{list_sandboxes, sandbox_availability};
use corvus_tools::{list_tools, tool_availability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityFamily {
    Provider,
    Channel,
    Tool,
    Memory,
    Observer,
    Security,
}

impl CapabilityFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Channel => "channel",
            Self::Tool => "tool",
            Self::Memory => "memory",
            Self::Observer => "observer",
            Self::Security => "security",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRecord {
    pub family: CapabilityFamily,
    pub key: &'static str,
    pub aliases: &'static [&'static str],
    pub status: CapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrySnapshot {
    pub records: Vec<CapabilityRecord>,
}

impl RegistrySnapshot {
    pub fn collect() -> Self {
        let mut records = Vec::new();
        records.extend(list_providers().iter().map(|descriptor| CapabilityRecord {
            family: CapabilityFamily::Provider,
            key: descriptor.key,
            aliases: descriptor.aliases,
            status: map_provider_status(descriptor.key),
        }));
        records.extend(list_channels().iter().map(|descriptor| CapabilityRecord {
            family: CapabilityFamily::Channel,
            key: descriptor.key,
            aliases: descriptor.aliases,
            status: map_channel_status(descriptor.key),
        }));
        records.extend(list_tools().iter().map(|descriptor| CapabilityRecord {
            family: CapabilityFamily::Tool,
            key: descriptor.key,
            aliases: descriptor.aliases,
            status: map_tool_status(descriptor.key),
        }));
        records.extend(
            list_memory_backends()
                .iter()
                .map(|descriptor| CapabilityRecord {
                    family: CapabilityFamily::Memory,
                    key: descriptor.key,
                    aliases: descriptor.aliases,
                    status: map_memory_status(descriptor.key),
                }),
        );
        records.extend(list_observers().iter().map(|descriptor| CapabilityRecord {
            family: CapabilityFamily::Observer,
            key: descriptor.key,
            aliases: descriptor.aliases,
            status: map_observer_status(descriptor.key),
        }));
        records.extend(list_sandboxes().iter().map(|descriptor| CapabilityRecord {
            family: CapabilityFamily::Security,
            key: descriptor.key,
            aliases: descriptor.aliases,
            status: map_security_status(descriptor.key),
        }));
        Self { records }
    }

    pub fn find_in_family(
        &self,
        family: CapabilityFamily,
        requested: &str,
    ) -> Option<&CapabilityRecord> {
        self.records.iter().find(|record| {
            record.family == family
                && (record.key.eq_ignore_ascii_case(requested)
                    || record
                        .aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(requested)))
        })
    }

    pub fn find_in_other_family(
        &self,
        family: CapabilityFamily,
        requested: &str,
    ) -> Option<&CapabilityRecord> {
        self.records.iter().find(|record| {
            record.family != family
                && (record.key.eq_ignore_ascii_case(requested)
                    || record
                        .aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(requested)))
        })
    }
}

fn map_provider_status(key: &str) -> CapabilityStatus {
    map_status(provider_availability(key))
}

fn map_channel_status(key: &str) -> CapabilityStatus {
    map_status(channel_availability(key))
}

fn map_tool_status(key: &str) -> CapabilityStatus {
    map_status(tool_availability(key))
}

fn map_memory_status(key: &str) -> CapabilityStatus {
    map_status(memory_availability(key))
}

fn map_observer_status(key: &str) -> CapabilityStatus {
    map_status(observer_availability(key))
}

fn map_security_status(key: &str) -> CapabilityStatus {
    map_status(sandbox_availability(key))
}

fn map_status<T>(status: Option<T>) -> CapabilityStatus
where
    T: IntoComposerStatus,
{
    status
        .map(IntoComposerStatus::into_composer_status)
        .unwrap_or(CapabilityStatus::Uncompiled)
}

trait IntoComposerStatus {
    fn into_composer_status(self) -> CapabilityStatus;
}

impl IntoComposerStatus for corvus_providers::CapabilityAvailability {
    fn into_composer_status(self) -> CapabilityStatus {
        match self {
            corvus_providers::CapabilityAvailability::Constructible => {
                CapabilityStatus::Constructible
            }
            corvus_providers::CapabilityAvailability::Uncompiled => CapabilityStatus::Uncompiled,
            corvus_providers::CapabilityAvailability::PlatformUnavailable => {
                CapabilityStatus::PlatformUnavailable
            }
        }
    }
}
impl IntoComposerStatus for corvus_channels::CapabilityAvailability {
    fn into_composer_status(self) -> CapabilityStatus {
        match self {
            corvus_channels::CapabilityAvailability::Constructible => {
                CapabilityStatus::Constructible
            }
            corvus_channels::CapabilityAvailability::Uncompiled => CapabilityStatus::Uncompiled,
            corvus_channels::CapabilityAvailability::PlatformUnavailable => {
                CapabilityStatus::PlatformUnavailable
            }
        }
    }
}
impl IntoComposerStatus for corvus_tools::CapabilityAvailability {
    fn into_composer_status(self) -> CapabilityStatus {
        match self {
            corvus_tools::CapabilityAvailability::Constructible => CapabilityStatus::Constructible,
            corvus_tools::CapabilityAvailability::Uncompiled => CapabilityStatus::Uncompiled,
            corvus_tools::CapabilityAvailability::PlatformUnavailable => {
                CapabilityStatus::PlatformUnavailable
            }
        }
    }
}
impl IntoComposerStatus for corvus_memory::CapabilityAvailability {
    fn into_composer_status(self) -> CapabilityStatus {
        match self {
            corvus_memory::CapabilityAvailability::Constructible => CapabilityStatus::Constructible,
            corvus_memory::CapabilityAvailability::Uncompiled => CapabilityStatus::Uncompiled,
            corvus_memory::CapabilityAvailability::PlatformUnavailable => {
                CapabilityStatus::PlatformUnavailable
            }
        }
    }
}
impl IntoComposerStatus for corvus_observability::CapabilityAvailability {
    fn into_composer_status(self) -> CapabilityStatus {
        match self {
            corvus_observability::CapabilityAvailability::Constructible => {
                CapabilityStatus::Constructible
            }
            corvus_observability::CapabilityAvailability::Uncompiled => {
                CapabilityStatus::Uncompiled
            }
            corvus_observability::CapabilityAvailability::PlatformUnavailable => {
                CapabilityStatus::PlatformUnavailable
            }
        }
    }
}
impl IntoComposerStatus for corvus_security::CapabilityAvailability {
    fn into_composer_status(self) -> CapabilityStatus {
        match self {
            corvus_security::CapabilityAvailability::Constructible => {
                CapabilityStatus::Constructible
            }
            corvus_security::CapabilityAvailability::Uncompiled => CapabilityStatus::Uncompiled,
            corvus_security::CapabilityAvailability::PlatformUnavailable => {
                CapabilityStatus::PlatformUnavailable
            }
        }
    }
}
