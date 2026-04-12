//! Corvus security registry surfaces for manifest composition.

pub mod factory;
pub mod registry;

pub use corvus_traits::security::{NoopSandbox, Sandbox};
pub use factory::{select_sandbox, SandboxFactorySelection};
pub use registry::{
    list_sandboxes, resolve_sandbox_key, sandbox_availability, CapabilityAvailability,
    SandboxDescriptor,
};
