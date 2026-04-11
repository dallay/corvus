//! Corvus Memory Registry
//!
//! Re-exports memory types and provides registry functions.

pub use corvus_traits::memory::{
    Memory, MemoryCategory, MemoryEntry, MemoryStats, SessionEntry, SessionStatus,
};

/// Information about a memory backend.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub name: &'static str,
    pub display_name: &'static str,
}
