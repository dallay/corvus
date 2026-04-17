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

#[cfg(test)]
mod tests {
    use super::*;

    // --- list_memory_backends ---

    #[test]
    fn list_memory_backends_is_non_empty() {
        assert!(!list_memory_backends().is_empty());
    }

    #[test]
    fn list_memory_backends_contains_exactly_four_backends() {
        assert_eq!(list_memory_backends().len(), 4);
    }

    #[test]
    fn list_memory_backends_includes_all_expected_keys() {
        let backends = list_memory_backends();
        let keys: Vec<&str> = backends.iter().map(|b| b.key).collect();
        for expected in &["sqlite", "lucid", "markdown", "none"] {
            assert!(
                keys.contains(expected),
                "expected memory backend '{expected}' not found in registry"
            );
        }
    }

    #[test]
    fn list_memory_backends_has_unique_keys() {
        let mut seen = std::collections::HashSet::new();
        for descriptor in list_memory_backends() {
            assert!(
                seen.insert(descriptor.key),
                "duplicate memory backend key: '{}'",
                descriptor.key
            );
        }
    }

    #[test]
    fn all_memory_descriptors_have_non_empty_display_name() {
        for descriptor in list_memory_backends() {
            assert!(
                !descriptor.display_name.is_empty(),
                "memory backend '{}' has empty display_name",
                descriptor.key
            );
        }
    }

    // --- resolve_memory_backend_key ---

    #[test]
    fn resolve_memory_backend_key_returns_none_for_unknown() {
        assert_eq!(resolve_memory_backend_key("not-a-backend"), None);
        assert_eq!(resolve_memory_backend_key(""), None);
    }

    #[test]
    fn resolve_memory_backend_key_matches_all_known_backends() {
        for key in &["sqlite", "lucid", "markdown", "none"] {
            assert_eq!(
                resolve_memory_backend_key(key),
                Some(*key),
                "failed for key '{key}'"
            );
        }
    }

    #[test]
    fn resolve_memory_backend_key_is_case_insensitive() {
        assert_eq!(resolve_memory_backend_key("SQLITE"), Some("sqlite"));
        assert_eq!(resolve_memory_backend_key("Markdown"), Some("markdown"));
        assert_eq!(resolve_memory_backend_key("NONE"), Some("none"));
        assert_eq!(resolve_memory_backend_key("LUCID"), Some("lucid"));
    }

    #[test]
    fn resolve_memory_backend_key_trims_whitespace() {
        assert_eq!(resolve_memory_backend_key("  sqlite  "), Some("sqlite"));
        assert_eq!(resolve_memory_backend_key("\tnone\n"), Some("none"));
    }

    #[test]
    fn resolve_memory_backend_key_returns_canonical_lowercase_key() {
        let key = resolve_memory_backend_key("MARKDOWN").unwrap();
        assert_eq!(key, "markdown");
    }

    // Regression: whitespace-only must not match anything.
    #[test]
    fn resolve_memory_backend_key_whitespace_only_returns_none() {
        assert_eq!(resolve_memory_backend_key("   "), None);
    }

    // --- memory_availability ---

    #[test]
    fn memory_availability_returns_none_for_unknown() {
        assert_eq!(memory_availability("unknown-backend"), None);
    }

    #[test]
    fn memory_availability_constructible_for_all_shipped_backends() {
        for name in &["sqlite", "lucid", "markdown", "none"] {
            assert_eq!(
                memory_availability(name),
                Some(CapabilityAvailability::Constructible),
                "expected Constructible for '{name}'"
            );
        }
    }

    #[test]
    fn memory_availability_is_case_insensitive() {
        assert_eq!(
            memory_availability("SQLITE"),
            Some(CapabilityAvailability::Constructible)
        );
        assert_eq!(
            memory_availability("None"),
            Some(CapabilityAvailability::Constructible)
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
}
