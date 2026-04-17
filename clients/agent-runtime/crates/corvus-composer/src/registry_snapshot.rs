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

#[cfg(test)]
mod tests {
    use super::*;

    // --- CapabilityFamily::as_str ---

    #[test]
    fn capability_family_as_str_provider() {
        assert_eq!(CapabilityFamily::Provider.as_str(), "provider");
    }

    #[test]
    fn capability_family_as_str_channel() {
        assert_eq!(CapabilityFamily::Channel.as_str(), "channel");
    }

    #[test]
    fn capability_family_as_str_tool() {
        assert_eq!(CapabilityFamily::Tool.as_str(), "tool");
    }

    #[test]
    fn capability_family_as_str_memory() {
        assert_eq!(CapabilityFamily::Memory.as_str(), "memory");
    }

    #[test]
    fn capability_family_as_str_observer() {
        assert_eq!(CapabilityFamily::Observer.as_str(), "observer");
    }

    #[test]
    fn capability_family_as_str_security() {
        assert_eq!(CapabilityFamily::Security.as_str(), "security");
    }

    // --- RegistrySnapshot::collect ---

    #[test]
    fn registry_snapshot_collect_is_non_empty() {
        let snapshot = RegistrySnapshot::collect();
        assert!(!snapshot.records.is_empty());
    }

    #[test]
    fn registry_snapshot_contains_records_from_all_families() {
        let snapshot = RegistrySnapshot::collect();
        for family in &[
            CapabilityFamily::Provider,
            CapabilityFamily::Channel,
            CapabilityFamily::Tool,
            CapabilityFamily::Memory,
            CapabilityFamily::Observer,
            CapabilityFamily::Security,
        ] {
            let has_family = snapshot.records.iter().any(|r| r.family == *family);
            assert!(
                has_family,
                "snapshot is missing records for family '{}'",
                family.as_str()
            );
        }
    }

    #[test]
    fn registry_snapshot_contains_known_provider() {
        let snapshot = RegistrySnapshot::collect();
        let record = snapshot.find_in_family(CapabilityFamily::Provider, "anthropic");
        assert!(record.is_some(), "expected 'anthropic' in Provider family");
        assert_eq!(record.unwrap().key, "anthropic");
    }

    #[test]
    fn registry_snapshot_contains_known_channel() {
        let snapshot = RegistrySnapshot::collect();
        let record = snapshot.find_in_family(CapabilityFamily::Channel, "stdio");
        assert!(record.is_some(), "expected 'stdio' in Channel family");
        assert_eq!(record.unwrap().key, "stdio");
    }

    #[test]
    fn registry_snapshot_contains_known_memory_backend() {
        let snapshot = RegistrySnapshot::collect();
        let record = snapshot.find_in_family(CapabilityFamily::Memory, "sqlite");
        assert!(record.is_some(), "expected 'sqlite' in Memory family");
    }

    // --- find_in_family ---

    #[test]
    fn find_in_family_returns_none_for_unknown_key() {
        let snapshot = RegistrySnapshot::collect();
        let result = snapshot.find_in_family(CapabilityFamily::Provider, "totally-unknown-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn find_in_family_is_case_insensitive() {
        let snapshot = RegistrySnapshot::collect();
        let lower = snapshot.find_in_family(CapabilityFamily::Channel, "stdio");
        let upper = snapshot.find_in_family(CapabilityFamily::Channel, "STDIO");
        assert!(lower.is_some());
        assert!(upper.is_some());
        assert_eq!(lower.unwrap().key, upper.unwrap().key);
    }

    #[test]
    fn find_in_family_does_not_cross_family_boundary() {
        let snapshot = RegistrySnapshot::collect();
        // "stdio" is a Channel, not a Provider
        let result = snapshot.find_in_family(CapabilityFamily::Provider, "stdio");
        assert!(
            result.is_none(),
            "find_in_family must not cross family boundaries"
        );
    }

    // --- find_in_other_family ---

    #[test]
    fn find_in_other_family_finds_capability_in_different_family() {
        let snapshot = RegistrySnapshot::collect();
        // "shell" is a Tool. Looking for it in Provider family should find the Tool record.
        let result = snapshot.find_in_other_family(CapabilityFamily::Provider, "shell");
        assert!(
            result.is_some(),
            "expected to find 'shell' in a non-Provider family"
        );
        assert_eq!(result.unwrap().family, CapabilityFamily::Tool);
    }

    #[test]
    fn find_in_other_family_returns_none_when_key_only_in_specified_family() {
        let snapshot = RegistrySnapshot::collect();
        // "anthropic" is only a Provider. Looking for it in non-Provider families returns None.
        let result = snapshot.find_in_other_family(CapabilityFamily::Provider, "anthropic");
        assert!(
            result.is_none(),
            "expected None since 'anthropic' is only in the Provider family"
        );
    }

    #[test]
    fn find_in_other_family_is_case_insensitive() {
        let snapshot = RegistrySnapshot::collect();
        let lower = snapshot.find_in_other_family(CapabilityFamily::Provider, "shell");
        let upper = snapshot.find_in_other_family(CapabilityFamily::Provider, "SHELL");
        // Both should either both be Some or both be None.
        assert_eq!(lower.is_some(), upper.is_some());
    }

    // --- CapabilityStatus ---

    #[test]
    fn capability_status_variants_are_comparable() {
        assert_eq!(
            CapabilityStatus::Constructible,
            CapabilityStatus::Constructible
        );
        assert_ne!(
            CapabilityStatus::Constructible,
            CapabilityStatus::Uncompiled
        );
        assert_ne!(
            CapabilityStatus::Uncompiled,
            CapabilityStatus::PlatformUnavailable
        );
    }

    // Regression: every record in the snapshot has a non-empty key.
    #[test]
    fn all_snapshot_records_have_non_empty_key() {
        let snapshot = RegistrySnapshot::collect();
        for record in &snapshot.records {
            assert!(
                !record.key.is_empty(),
                "record with family '{}' has empty key",
                record.family.as_str()
            );
        }
    }
}
