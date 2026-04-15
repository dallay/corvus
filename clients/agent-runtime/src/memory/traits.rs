//! Compatibility shim for extracted memory contracts.

#[allow(unused_imports)]
pub use corvus_traits::memory::{
    is_slash_session_unsupported_error, slash_session_unsupported_error, Memory, MemoryCategory,
    MemoryEntry, MemoryStats, MemoryValidationResult, ResumableSessionEntry, SessionEntry,
    SessionFieldPatch, SessionSnapshotKind, SessionSnapshotRecord, SessionStateMutation,
    SessionStatePatch, SessionStateRecord, SessionStatus, SlashSessionLifecycle,
};
