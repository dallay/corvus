use anyhow::{anyhow, Result};

use crate::registry::{memory_availability, resolve_memory_backend_key, CapabilityAvailability};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryFactorySelection {
    pub key: &'static str,
}

pub fn select_memory_backend(name: &str) -> Result<MemoryFactorySelection> {
    let Some(key) = resolve_memory_backend_key(name) else {
        return Err(anyhow!("unknown memory backend '{name}'"));
    };

    match memory_availability(key) {
        Some(CapabilityAvailability::Constructible) => Ok(MemoryFactorySelection { key }),
        Some(CapabilityAvailability::Uncompiled) => {
            Err(anyhow!("memory backend '{key}' is known but not compiled"))
        }
        Some(CapabilityAvailability::PlatformUnavailable) => Err(anyhow!(
            "memory backend '{key}' is unavailable on this platform"
        )),
        None => Err(anyhow!("unknown memory backend '{name}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_memory_backend_returns_ok_for_sqlite() {
        let result = select_memory_backend("sqlite");
        assert!(result.is_ok(), "expected Ok for 'sqlite', got: {result:?}");
        assert_eq!(result.unwrap().key, "sqlite");
    }

    #[test]
    fn select_memory_backend_returns_ok_for_all_shipped_backends() {
        for name in &["sqlite", "lucid", "markdown", "none"] {
            let result = select_memory_backend(name);
            assert!(result.is_ok(), "expected Ok for '{name}', got: {result:?}");
        }
    }

    #[test]
    fn select_memory_backend_key_is_canonical_lowercase() {
        let selection = select_memory_backend("SQLITE").unwrap();
        assert_eq!(selection.key, "sqlite");
    }

    #[test]
    fn select_memory_backend_err_for_unknown_name() {
        let result = select_memory_backend("totally-unknown");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unknown memory backend"),
            "error message was: {msg}"
        );
    }

    #[test]
    fn select_memory_backend_err_for_empty_string() {
        let result = select_memory_backend("");
        assert!(result.is_err());
    }

    #[test]
    fn memory_factory_selection_is_copy() {
        let sel = MemoryFactorySelection { key: "sqlite" };
        let copy = sel;
        assert_eq!(sel, copy);
    }

    // Regression: selecting the same backend twice yields identical results.
    #[test]
    fn select_memory_backend_is_idempotent() {
        let first = select_memory_backend("markdown").unwrap();
        let second = select_memory_backend("markdown").unwrap();
        assert_eq!(first, second);
    }

    // Boundary: whitespace-padded name should resolve through trimming.
    #[test]
    fn select_memory_backend_accepts_padded_name() {
        let result = select_memory_backend("  none  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().key, "none");
    }
}
