//! Corvus memory registry surfaces for manifest composition.

pub mod factory;
pub mod registry;

pub use corvus_traits::memory::{
    Memory, MemoryCategory, MemoryEntry, MemoryStats, SessionEntry, SessionStatus,
};
pub use factory::{select_memory_backend, MemoryFactorySelection};
pub use registry::{
    list_memory_backends, memory_availability, resolve_memory_backend_key, CapabilityAvailability,
    MemoryDescriptor,
};
