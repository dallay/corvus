//! Corvus Security Registry
//!
//! Re-exports security types and provides registry functions.

pub use corvus_traits::security::{NoopSandbox, Sandbox};

/// Information about a security implementation.
#[derive(Debug, Clone)]
pub struct SecurityInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    pub is_sandbox: bool,
}
