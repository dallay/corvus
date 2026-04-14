//! Compatibility shim for extracted memory contracts.

#[allow(unused_imports)]
pub use corvus_traits::memory::{
    slash_session_unsupported_error, Memory, MemoryCategory, MemoryEntry, MemoryStats,
    MemoryValidationResult, ResumableSessionEntry, SessionEntry, SessionSnapshotKind,
    SessionSnapshotRecord, SessionStateMutation, SessionStateRecord, SessionStatus,
    SlashSessionLifecycle,
};
