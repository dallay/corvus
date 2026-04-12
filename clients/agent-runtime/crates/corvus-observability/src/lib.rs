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
    let Some(descriptor) = OBSERVERS.iter().find(|descriptor| {
        let candidate = name.trim();
        descriptor.key.eq_ignore_ascii_case(candidate)
            || descriptor
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(candidate))
    }) else {
        return Err(anyhow!("unknown observer '{name}'"));
    };

    let key = descriptor.key;
    if !descriptor.platform_supported {
        return Err(anyhow!("observer '{key}' is unavailable on this platform"));
    }
    if !descriptor.compiled {
        return Err(anyhow!("observer '{key}' is known but not compiled"));
    }

    Ok(ObserverFactorySelection { key })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_observer_aliases() {
        assert_eq!(resolve_observer_key("noop"), Some("none"));
        assert_eq!(resolve_observer_key("otlp"), Some("otel"));
    }

    #[test]
    fn resolve_observer_key_trims_whitespace() {
        assert_eq!(resolve_observer_key("  none  "), Some("none"));
        assert_eq!(resolve_observer_key("\totel\t"), Some("otel"));
    }

    #[test]
    fn resolve_observer_key_is_case_insensitive() {
        assert_eq!(resolve_observer_key("NONE"), Some("none"));
        assert_eq!(resolve_observer_key("OTEL"), Some("otel"));
        assert_eq!(resolve_observer_key("NoOp"), Some("none"));
        assert_eq!(resolve_observer_key("Otlp"), Some("otel"));
    }

    #[test]
    fn select_observer_returns_selection_for_valid_keys() {
        assert_eq!(select_observer("none").unwrap().key, "none");
        assert_eq!(select_observer("noop").unwrap().key, "none");
        assert_eq!(select_observer("otel").unwrap().key, "otel");
        assert_eq!(select_observer("otlp").unwrap().key, "otel");
    }

    #[test]
    fn select_observer_errors_on_unknown_input() {
        let result = select_observer("unknown");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unknown observer"));
        assert!(msg.contains("unknown"));
    }

    #[test]
    fn select_observer_errors_on_unknown_after_trimming() {
        let result = select_observer("   ");
        assert!(result.is_err());
    }
}
