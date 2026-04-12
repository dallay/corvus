use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAvailability {
    Constructible,
    Uncompiled,
    PlatformUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverDescriptor {
    pub key: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub compiled: bool,
    pub platform_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverFactorySelection {
    pub key: &'static str,
}

const OBSERVERS: &[ObserverDescriptor] = &[
    ObserverDescriptor {
        key: "none",
        display_name: "Noop",
        aliases: &["noop"],
        compiled: true,
        platform_supported: true,
    },
    ObserverDescriptor {
        key: "log",
        display_name: "Log",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ObserverDescriptor {
        key: "prometheus",
        display_name: "Prometheus",
        aliases: &[],
        compiled: true,
        platform_supported: true,
    },
    ObserverDescriptor {
        key: "otel",
        display_name: "OpenTelemetry",
        aliases: &["opentelemetry", "otlp"],
        compiled: true,
        platform_supported: true,
    },
];

pub fn list_observers() -> &'static [ObserverDescriptor] {
    OBSERVERS
}

pub fn resolve_observer_key(name: &str) -> Option<&'static str> {
    let candidate = name.trim();
    OBSERVERS
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

pub fn observer_availability(name: &str) -> Option<CapabilityAvailability> {
    let key = resolve_observer_key(name)?;
    OBSERVERS
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

pub fn select_observer(name: &str) -> Result<ObserverFactorySelection> {
    let Some(key) = resolve_observer_key(name) else {
        return Err(anyhow!("unknown observer '{name}'"));
    };

    match observer_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(ObserverFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("observer '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => {
            Err(anyhow!("observer '{key}' is unavailable on this platform"))
        }
        // SAFETY: resolve_observer_key guarantees a canonical key that exists in the registry.
        // If this None case is reached, it indicates an internal invariant violation.
        None => {
            unreachable!("invariant: observer_availability returned None for resolved key '{key}'")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_observer_aliases() {
        assert_eq!(resolve_observer_key("noop"), Some("none"));
        assert_eq!(resolve_observer_key("otlp"), Some("otel"));
    }
}
