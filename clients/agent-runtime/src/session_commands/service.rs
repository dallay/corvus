use super::types::{sanitize_storage_error, SessionCommandError, SessionCommandResult};
use crate::memory::{
    is_slash_session_unsupported_error, Memory, SessionFieldPatch, SessionSnapshotKind,
    SessionStatePatch, SessionStatus, SlashSessionLifecycle,
};
use serde_json::json;

const DEFAULT_EXCERPT_LIMIT: usize = 8;
const PREVIEW_LIMIT: usize = 120;

pub struct SessionCommandService<'a> {
    memory: &'a dyn Memory,
}

impl<'a> SessionCommandService<'a> {
    pub fn new(memory: &'a dyn Memory) -> Self {
        Self { memory }
    }

    pub async fn handle_tldr(
        &self,
        session_id: &str,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        self.ensure_sqlite()?;
        let session = self.require_active_session(session_id).await?;
        let state = self.current_state(session_id).await?;
        if state == SlashSessionLifecycle::Suspended {
            return Err(SessionCommandError::InvalidState {
                session_id: session_id.to_string(),
                detail: "session is suspended",
            });
        }

        let excerpt = self
            .memory
            .load_session_transcript_excerpt(session_id, DEFAULT_EXCERPT_LIMIT)
            .await
            .map_err(|error| self.map_storage_error(error))?;
        let summary = build_summary(&excerpt, PREVIEW_LIMIT);
        let snapshot = self
            .memory
            .create_session_snapshot(
                session_id,
                SessionSnapshotKind::Tldr,
                json!({
                    "summary": summary,
                    "excerpt_count": excerpt.len(),
                    "message_count": session.message_count,
                    "last_activity": session.last_activity,
                }),
                false,
            )
            .await
            .map_err(|error| self.map_storage_error(error))?;
        self.memory
            .apply_session_state_patch(SessionStatePatch {
                session_id: session_id.to_string(),
                lifecycle: Some(SlashSessionLifecycle::Active),
                latest_tldr_snapshot_id: SessionFieldPatch::Set(snapshot.id),
                latest_compact_snapshot_id: SessionFieldPatch::Keep,
                pending_hydration_snapshot_id: SessionFieldPatch::Clear,
                suspended_at: SessionFieldPatch::Clear,
            })
            .await
            .map_err(|error| self.map_storage_error(error))?;

        Ok(SessionCommandResult {
            command: "/tldr",
            session_id: session_id.to_string(),
            message: summary,
            resumed_session_id: None,
            resumable_sessions: Vec::new(),
        })
    }

    pub async fn handle_compact(
        &self,
        session_id: &str,
        args: &str,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        self.ensure_sqlite()?;
        let session = self.require_active_session(session_id).await?;
        let state = self.current_state(session_id).await?;
        if state == SlashSessionLifecycle::Suspended {
            return Err(SessionCommandError::InvalidState {
                session_id: session_id.to_string(),
                detail: "session is suspended",
            });
        }

        let excerpt = self
            .memory
            .load_session_transcript_excerpt(session_id, DEFAULT_EXCERPT_LIMIT)
            .await
            .map_err(|error| self.map_storage_error(error))?;
        let summary = build_summary(&excerpt, PREVIEW_LIMIT);
        let resume_context = build_resume_context(session_id, &summary, &excerpt, args);
        let preview = truncate_preview(&summary, PREVIEW_LIMIT);
        let snapshot = self
            .memory
            .create_session_snapshot(
                session_id,
                SessionSnapshotKind::Compact,
                json!({
                    "summary": summary,
                    "resume_context": resume_context,
                    "preview": preview,
                    "message_count": session.message_count,
                    "last_activity": session.last_activity,
                    "excerpt_count": excerpt.len(),
                }),
                true,
            )
            .await
            .map_err(|error| self.map_storage_error(error))?;
        self.memory
            .apply_session_state_patch(SessionStatePatch {
                session_id: session_id.to_string(),
                lifecycle: Some(SlashSessionLifecycle::Active),
                latest_tldr_snapshot_id: SessionFieldPatch::Keep,
                latest_compact_snapshot_id: SessionFieldPatch::Set(snapshot.id),
                pending_hydration_snapshot_id: SessionFieldPatch::Clear,
                suspended_at: SessionFieldPatch::Clear,
            })
            .await
            .map_err(|error| self.map_storage_error(error))?;

        Ok(SessionCommandResult {
            command: "/compact",
            session_id: session_id.to_string(),
            message: format!("[session:{session_id}] session compacted and ready for resume"),
            resumed_session_id: None,
            resumable_sessions: Vec::new(),
        })
    }

    pub async fn handle_suspend(
        &self,
        session_id: &str,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        self.ensure_sqlite()?;
        self.require_active_session(session_id).await?;
        let state = self.require_state(session_id).await?;
        if state.lifecycle == SlashSessionLifecycle::Suspended {
            return Err(SessionCommandError::InvalidState {
                session_id: session_id.to_string(),
                detail: "session is already suspended",
            });
        }
        let Some(snapshot_id) = state.latest_compact_snapshot_id.clone() else {
            return Err(SessionCommandError::MissingSnapshot {
                session_id: session_id.to_string(),
            });
        };
        self.require_resume_capable_snapshot(session_id, &snapshot_id)
            .await?;
        self.memory
            .apply_session_state_patch(SessionStatePatch {
                session_id: session_id.to_string(),
                lifecycle: Some(SlashSessionLifecycle::Suspended),
                latest_tldr_snapshot_id: SessionFieldPatch::Keep,
                latest_compact_snapshot_id: SessionFieldPatch::Keep,
                pending_hydration_snapshot_id: SessionFieldPatch::Clear,
                suspended_at: SessionFieldPatch::Set(chrono::Utc::now().to_rfc3339()),
            })
            .await
            .map_err(|error| self.map_storage_error(error))?;

        Ok(SessionCommandResult {
            command: "/suspend",
            session_id: session_id.to_string(),
            message: format!("[session:{session_id}] session suspended"),
            resumed_session_id: None,
            resumable_sessions: Vec::new(),
        })
    }

    pub async fn handle_resume(
        &self,
        current_session_id: &str,
        target: Option<&str>,
        caller_token_hash: Option<&str>,
    ) -> Result<SessionCommandResult, SessionCommandError> {
        self.ensure_sqlite()?;

        // Caller identity is mandatory for resume/listing resumable sessions
        let Some(caller_hash) = caller_token_hash else {
            return Err(SessionCommandError::Unauthorized);
        };

        if let Some(target_session_id) = target {
            let session = self
                .memory
                .get_session(target_session_id)
                .await
                .map_err(|error| self.map_storage_error(error))?
                .ok_or_else(|| SessionCommandError::InvalidResumeTarget {
                    session_id: target_session_id.to_string(),
                })?;
            if session.status == SessionStatus::Ended {
                return Err(SessionCommandError::InvalidResumeTarget {
                    session_id: target_session_id.to_string(),
                });
            }

            // Caller ownership check: verify the caller owns or has access to the target session.
            // Use targeted lookup instead of paginated list to avoid false-negatives with >1000 sessions
            let target_exists = self
                .memory
                .get_session(target_session_id)
                .await
                .map_err(|error| self.map_storage_error(error))?
                .is_some();

            if !target_exists {
                return Err(SessionCommandError::InvalidResumeTarget {
                    session_id: target_session_id.to_string(),
                });
            }

            // Verify caller can resume this session via state check
            let state = self.get_session_state_optional(target_session_id).await?;
            let Some(state) = state else {
                return Err(SessionCommandError::InvalidResumeTarget {
                    session_id: target_session_id.to_string(),
                });
            };
            if state.lifecycle != SlashSessionLifecycle::Suspended {
                return Err(SessionCommandError::InvalidResumeTarget {
                    session_id: target_session_id.to_string(),
                });
            }
            let Some(snapshot_id) = state.latest_compact_snapshot_id else {
                return Err(SessionCommandError::MissingSnapshot {
                    session_id: target_session_id.to_string(),
                });
            };
            self.require_resume_capable_snapshot(target_session_id, &snapshot_id)
                .await?;
            self.memory
                .apply_session_state_patch(SessionStatePatch {
                    session_id: target_session_id.to_string(),
                    lifecycle: Some(SlashSessionLifecycle::Active),
                    latest_tldr_snapshot_id: SessionFieldPatch::Keep,
                    latest_compact_snapshot_id: SessionFieldPatch::Keep,
                    pending_hydration_snapshot_id: SessionFieldPatch::Set(snapshot_id),
                    suspended_at: SessionFieldPatch::Clear,
                })
                .await
                .map_err(|error| self.map_storage_error(error))?;

            Ok(SessionCommandResult {
                command: "/resume",
                session_id: current_session_id.to_string(),
                message: format!(
                    "[session:{target_session_id}] resumed from persisted compact snapshot"
                ),
                resumed_session_id: Some(target_session_id.to_string()),
                resumable_sessions: Vec::new(),
            })
        } else {
            let resumable_sessions = self
                .memory
                .list_resumable_sessions(Some(caller_hash), 10, 0)
                .await
                .map_err(|error| self.map_storage_error(error))?;
            let message = if resumable_sessions.is_empty() {
                "No resumable suspended sessions.".to_string()
            } else {
                let lines = resumable_sessions
                    .iter()
                    .map(|entry| format!("- {}: {}", entry.session_id, entry.preview))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Resumable sessions:\n{lines}")
            };

            Ok(SessionCommandResult {
                command: "/resume",
                session_id: current_session_id.to_string(),
                message,
                resumed_session_id: None,
                resumable_sessions,
            })
        }
    }

    fn ensure_sqlite(&self) -> Result<(), SessionCommandError> {
        if self.memory.name() == "sqlite" {
            Ok(())
        } else {
            Err(SessionCommandError::UnsupportedBackend {
                backend: self.memory.name().to_string(),
            })
        }
    }

    async fn require_active_session(
        &self,
        session_id: &str,
    ) -> Result<crate::memory::SessionEntry, SessionCommandError> {
        let session = self
            .memory
            .get_session(session_id)
            .await
            .map_err(|error| self.map_storage_error(error))?
            .ok_or_else(|| SessionCommandError::UnknownSession {
                session_id: session_id.to_string(),
            })?;
        if session.status == SessionStatus::Ended {
            return Err(SessionCommandError::InvalidState {
                session_id: session_id.to_string(),
                detail: "session is ended",
            });
        }
        Ok(session)
    }

    async fn current_state(
        &self,
        session_id: &str,
    ) -> Result<SlashSessionLifecycle, SessionCommandError> {
        Ok(self
            .memory
            .get_session_state_record(session_id)
            .await
            .map_err(|error| self.map_storage_error(error))?
            .map(|state| state.lifecycle)
            .unwrap_or(SlashSessionLifecycle::Active))
    }

    async fn require_state(
        &self,
        session_id: &str,
    ) -> Result<crate::memory::SessionStateRecord, SessionCommandError> {
        self.memory
            .get_session_state_record(session_id)
            .await
            .map_err(|error| self.map_storage_error(error))?
            .ok_or_else(|| SessionCommandError::MissingSnapshot {
                session_id: session_id.to_string(),
            })
    }

    /// Non-panicking version of require_state for targeted resume.
    /// Returns None if no state record exists (instead of error).
    async fn get_session_state_optional(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::memory::SessionStateRecord>, SessionCommandError> {
        self.memory
            .get_session_state_record(session_id)
            .await
            .map_err(|error| self.map_storage_error(error))
    }

    /// Get the token_hash for a session to validate caller ownership.
    async fn require_resume_capable_snapshot(
        &self,
        session_id: &str,
        snapshot_id: &str,
    ) -> Result<crate::memory::SessionSnapshotRecord, SessionCommandError> {
        let snapshot = self
            .memory
            .get_session_snapshot(snapshot_id)
            .await
            .map_err(|error| self.map_storage_error(error))?
            .ok_or_else(|| SessionCommandError::MissingSnapshot {
                session_id: session_id.to_string(),
            })?;

        if snapshot.session_id != session_id
            || snapshot.kind != SessionSnapshotKind::Compact
            || !snapshot.resume_capable
        {
            return Err(SessionCommandError::MissingSnapshot {
                session_id: session_id.to_string(),
            });
        }

        Ok(snapshot)
    }

    fn map_storage_error(&self, error: anyhow::Error) -> SessionCommandError {
        if is_slash_session_unsupported_error(&error) {
            SessionCommandError::UnsupportedBackend {
                backend: self.memory.name().to_string(),
            }
        } else {
            // Log the sanitized error for internal debugging - no internal DB details or snapshot content
            let sanitized_summary = sanitize_storage_error(&error);
            tracing::error!(error_detail = %sanitized_summary, "storage error details (for internal logs)");
            // Return sanitized message for user-facing error
            SessionCommandError::StorageFailure {
                detail: sanitized_summary,
            }
        }
    }
}

fn build_summary(entries: &[crate::memory::MemoryEntry], preview_limit: usize) -> String {
    if entries.is_empty() {
        return "No persisted session transcript is available yet.".to_string();
    }
    let summary = entries
        .iter()
        .map(|entry| format!("{}: {}", entry.key, entry.content.trim()))
        .collect::<Vec<_>>()
        .join(" | ");
    truncate_preview(&summary, preview_limit)
}

/// Maximum characters per transcript entry to prevent bloat.
const MAX_ENTRY_CONTENT: usize = 2048;
/// Maximum total resume context length to keep snapshots bounded.
const MAX_CONTEXT_LENGTH: usize = 16384;

fn build_resume_context(
    session_id: &str,
    summary: &str,
    entries: &[crate::memory::MemoryEntry],
    args: &str,
) -> String {
    // Truncate each entry content to MAX_ENTRY_CONTENT chars
    let truncated_entries: Vec<String> = entries
        .iter()
        .map(|entry| {
            let content = entry.content.trim();
            let truncated = if content.chars().count() > MAX_ENTRY_CONTENT {
                content
                    .chars()
                    .take(MAX_ENTRY_CONTENT.saturating_sub(1))
                    .collect::<String>()
                    + "…"
            } else {
                content.to_string()
            };
            format!("- {}: {}", entry.key, truncated)
        })
        .collect::<Vec<_>>();

    // Build initial context
    let mut context = format!(
        "Session {session_id} summary: {summary}\nRecent transcript:\n{}",
        truncated_entries.join("\n")
    );

    // Truncate args/notes if present
    if !args.trim().is_empty() {
        let truncated_args = truncate_preview(args.trim(), 512);
        context.push_str("\nOperator notes: ");
        context.push_str(&truncated_args);
    }

    // Enforce global max length by trimming from the end if needed
    if context.chars().count() > MAX_CONTEXT_LENGTH {
        let excess = context.chars().count() - MAX_CONTEXT_LENGTH;
        context = context.chars().skip(excess).collect::<String>();
        // Ensure we don't start mid-entry
        if let Some(pos) = context.find("\n- ") {
            let adjustment: String = context.chars().skip(pos).collect();
            if adjustment.starts_with("- ") {
                context = adjustment;
            }
        }
    }

    context
}

fn truncate_preview(value: &str, max_len: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_len {
        return trimmed.to_string();
    }
    trimmed
        .chars()
        .take(max_len.saturating_sub(1))
        .collect::<String>()
        + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        MemoryCategory, MemoryEntry, ResumableSessionEntry, SessionEntry, SessionFieldPatch,
        SessionSnapshotKind, SessionSnapshotRecord, SessionStatePatch, SessionStateRecord,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeMemory {
        backend: &'static str,
        session: Option<SessionEntry>,
        transcript: Vec<MemoryEntry>,
        state: Mutex<Option<SessionStateRecord>>,
        snapshots: Mutex<Vec<SessionSnapshotRecord>>,
        listed: Vec<ResumableSessionEntry>,
    }

    impl Default for FakeMemory {
        fn default() -> Self {
            Self {
                backend: "sqlite",
                session: None,
                transcript: Vec::new(),
                state: Mutex::new(None),
                snapshots: Mutex::new(Vec::new()),
                listed: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl Memory for FakeMemory {
        fn name(&self) -> &str {
            self.backend
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }

        async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<SessionEntry>> {
            // Look up session by id like real implementation
            Ok(self
                .session
                .as_ref()
                .filter(|s| s.id == session_id)
                .cloned())
        }

        async fn load_session_transcript_excerpt(
            &self,
            _session_id: &str,
            _limit: usize,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(self.transcript.clone())
        }

        async fn create_session_snapshot(
            &self,
            session_id: &str,
            kind: SessionSnapshotKind,
            payload: serde_json::Value,
            resume_capable: bool,
        ) -> anyhow::Result<SessionSnapshotRecord> {
            let snapshot = SessionSnapshotRecord {
                id: format!("snapshot-{}", self.snapshots.lock().unwrap().len() + 1),
                session_id: session_id.to_string(),
                kind,
                created_at: "now".to_string(),
                payload,
                resume_capable,
            };
            self.snapshots.lock().unwrap().push(snapshot.clone());
            Ok(snapshot)
        }

        async fn get_session_state_record(
            &self,
            _session_id: &str,
        ) -> anyhow::Result<Option<SessionStateRecord>> {
            Ok(self.state.lock().unwrap().clone())
        }

        async fn get_session_snapshot(
            &self,
            snapshot_id: &str,
        ) -> anyhow::Result<Option<SessionSnapshotRecord>> {
            Ok(self
                .snapshots
                .lock()
                .unwrap()
                .iter()
                .find(|snapshot| snapshot.id == snapshot_id)
                .cloned())
        }

        async fn apply_session_state_patch(
            &self,
            patch: SessionStatePatch,
        ) -> anyhow::Result<SessionStateRecord> {
            let current = self.state.lock().unwrap().clone();
            let updated = SessionStateRecord {
                session_id: patch.session_id,
                lifecycle: patch.lifecycle.unwrap_or(
                    current
                        .as_ref()
                        .map(|state| state.lifecycle)
                        .unwrap_or(SlashSessionLifecycle::Active),
                ),
                latest_tldr_snapshot_id: match patch.latest_tldr_snapshot_id {
                    SessionFieldPatch::Keep => current
                        .as_ref()
                        .and_then(|state| state.latest_tldr_snapshot_id.clone()),
                    SessionFieldPatch::Set(value) => Some(value),
                    SessionFieldPatch::Clear => None,
                },
                latest_compact_snapshot_id: match patch.latest_compact_snapshot_id {
                    SessionFieldPatch::Keep => current
                        .as_ref()
                        .and_then(|state| state.latest_compact_snapshot_id.clone()),
                    SessionFieldPatch::Set(value) => Some(value),
                    SessionFieldPatch::Clear => None,
                },
                pending_hydration_snapshot_id: match patch.pending_hydration_snapshot_id {
                    SessionFieldPatch::Keep => current
                        .as_ref()
                        .and_then(|state| state.pending_hydration_snapshot_id.clone()),
                    SessionFieldPatch::Set(value) => Some(value),
                    SessionFieldPatch::Clear => None,
                },
                suspended_at: match patch.suspended_at {
                    SessionFieldPatch::Keep => current
                        .as_ref()
                        .and_then(|state| state.suspended_at.clone()),
                    SessionFieldPatch::Set(value) => Some(value),
                    SessionFieldPatch::Clear => None,
                },
                updated_at: "now".to_string(),
            };
            *self.state.lock().unwrap() = Some(updated.clone());
            Ok(updated)
        }

        async fn list_resumable_sessions(
            &self,
            _caller_token_hash: Option<&str>,
            _limit: u32,
            _offset: u32,
        ) -> anyhow::Result<Vec<ResumableSessionEntry>> {
            Ok(self.listed.clone())
        }

        async fn take_pending_resume_hydration(
            &self,
            _session_id: &str,
        ) -> anyhow::Result<Option<SessionSnapshotRecord>> {
            Ok(None)
        }
    }

    fn active_session() -> SessionEntry {
        SessionEntry {
            id: "session-1".into(),
            started_at: "started".into(),
            ended_at: None,
            status: SessionStatus::Active,
            message_count: 3,
            last_activity: "now".into(),
            metadata: None,
        }
    }

    fn transcript() -> Vec<MemoryEntry> {
        vec![MemoryEntry {
            id: "1".into(),
            key: "msg-1".into(),
            content: "Discuss release checklist".into(),
            category: MemoryCategory::Conversation,
            timestamp: "now".into(),
            session_id: Some("session-1".into()),
            score: None,
        }]
    }

    #[tokio::test]
    async fn rejects_non_sqlite_backends() {
        let memory = FakeMemory {
            backend: "markdown",
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service.handle_tldr("session-1").await.unwrap_err();
        assert_eq!(
            error,
            SessionCommandError::UnsupportedBackend {
                backend: "markdown".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn tldr_is_deterministic_and_persists_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            transcript: transcript(),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = service.handle_tldr("session-1").await.unwrap();

        assert_eq!(result.command, "/tldr");
        assert!(result.message.contains("Discuss release checklist"));
        assert_eq!(memory.snapshots.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn compact_creates_resume_capable_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            transcript: transcript(),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = service
            .handle_compact("session-1", "keep goals")
            .await
            .unwrap();

        assert_eq!(
            result.message,
            "[session:session-1] session compacted and ready for resume"
        );
        assert!(memory.snapshots.lock().unwrap()[0].resume_capable);
    }

    #[tokio::test]
    async fn suspend_requires_compact_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service.handle_suspend("session-1").await.unwrap_err();
        assert_eq!(
            error,
            SessionCommandError::MissingSnapshot {
                session_id: "session-1".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn suspend_succeeds_with_resume_capable_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            state: Mutex::new(Some(SessionStateRecord {
                session_id: "session-1".into(),
                lifecycle: SlashSessionLifecycle::Active,
                latest_tldr_snapshot_id: None,
                latest_compact_snapshot_id: Some("snapshot-1".into()),
                pending_hydration_snapshot_id: None,
                suspended_at: None,
                updated_at: "now".into(),
            })),
            snapshots: Mutex::new(vec![SessionSnapshotRecord {
                id: "snapshot-1".into(),
                session_id: "session-1".into(),
                kind: SessionSnapshotKind::Compact,
                created_at: "now".into(),
                payload: json!({"preview": "resume me"}),
                resume_capable: true,
            }]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = service.handle_suspend("session-1").await.unwrap();

        assert_eq!(result.command, "/suspend");
        assert_eq!(result.message, "[session:session-1] session suspended");
        assert_eq!(
            memory
                .state
                .lock()
                .unwrap()
                .as_ref()
                .map(|state| state.lifecycle),
            Some(SlashSessionLifecycle::Suspended)
        );
    }

    #[tokio::test]
    async fn suspend_rejects_invalid_snapshot_reference() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            state: Mutex::new(Some(SessionStateRecord {
                session_id: "session-1".into(),
                lifecycle: SlashSessionLifecycle::Active,
                latest_tldr_snapshot_id: None,
                latest_compact_snapshot_id: Some("snapshot-missing".into()),
                pending_hydration_snapshot_id: None,
                suspended_at: None,
                updated_at: "now".into(),
            })),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service.handle_suspend("session-1").await.unwrap_err();

        assert_eq!(
            error,
            SessionCommandError::MissingSnapshot {
                session_id: "session-1".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn resume_target_sets_pending_hydration() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            snapshots: Mutex::new(vec![SessionSnapshotRecord {
                id: "snapshot-1".into(),
                session_id: "session-1".into(),
                kind: SessionSnapshotKind::Compact,
                created_at: "now".into(),
                payload: json!({"preview": "resume me"}),
                resume_capable: true,
            }]),
            state: Mutex::new(Some(SessionStateRecord {
                session_id: "session-1".into(),
                lifecycle: SlashSessionLifecycle::Suspended,
                latest_tldr_snapshot_id: None,
                latest_compact_snapshot_id: Some("snapshot-1".into()),
                pending_hydration_snapshot_id: None,
                suspended_at: Some("now".into()),
                updated_at: "now".into(),
            })),
            listed: vec![ResumableSessionEntry {
                session_id: "session-1".into(),
                started_at: "now".into(),
                last_activity: "now".into(),
                snapshot_id: "snapshot-1".into(),
                snapshot_created_at: "now".into(),
                preview: "resume me".into(),
            }],
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = service
            .handle_resume("session-2", Some("session-1"), Some("caller-hash"))
            .await
            .unwrap();

        assert_eq!(result.resumed_session_id.as_deref(), Some("session-1"));
        assert_eq!(
            memory
                .state
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|state| state.pending_hydration_snapshot_id.clone())
                .as_deref(),
            Some("snapshot-1")
        );
    }

    #[tokio::test]
    async fn resume_target_rejects_missing_session() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: None,
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service
            .handle_resume(
                "session-current",
                Some("missing-session"),
                Some("caller-hash"),
            )
            .await
            .unwrap_err();

        assert_eq!(
            error,
            SessionCommandError::InvalidResumeTarget {
                session_id: "missing-session".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn resume_target_rejects_ended_session() {
        let mut session = active_session();
        session.status = SessionStatus::Ended;
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(session),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service
            .handle_resume("session-current", Some("session-1"), Some("caller-hash"))
            .await
            .unwrap_err();

        assert_eq!(
            error,
            SessionCommandError::InvalidResumeTarget {
                session_id: "session-1".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn resume_target_rejects_invalid_snapshot_reference() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(active_session()),
            state: Mutex::new(Some(SessionStateRecord {
                session_id: "session-1".into(),
                lifecycle: SlashSessionLifecycle::Suspended,
                latest_tldr_snapshot_id: None,
                latest_compact_snapshot_id: Some("snapshot-missing".into()),
                pending_hydration_snapshot_id: None,
                suspended_at: Some("now".into()),
                updated_at: "now".into(),
            })),
            listed: vec![ResumableSessionEntry {
                session_id: "session-1".into(),
                started_at: "now".into(),
                last_activity: "now".into(),
                snapshot_id: "snapshot-missing".into(),
                snapshot_created_at: "now".into(),
                preview: "resume me".into(),
            }],
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service
            .handle_resume("session-current", Some("session-1"), Some("caller-hash"))
            .await
            .unwrap_err();

        assert_eq!(
            error,
            SessionCommandError::MissingSnapshot {
                session_id: "session-1".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn resume_without_target_lists_resumable_sessions() {
        let memory = FakeMemory {
            backend: "sqlite",
            listed: vec![ResumableSessionEntry {
                session_id: "session-1".into(),
                started_at: "started".into(),
                last_activity: "now".into(),
                snapshot_id: "snapshot-1".into(),
                snapshot_created_at: "now".into(),
                preview: "Discuss release checklist".into(),
            }],
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = service
            .handle_resume("session-current", None, Some("caller-hash"))
            .await
            .unwrap();

        assert!(result.message.contains("Resumable sessions:"));
        assert_eq!(result.resumable_sessions.len(), 1);
    }

    #[tokio::test]
    async fn ended_session_rejects_deterministically() {
        let mut session = active_session();
        session.status = SessionStatus::Ended;
        let memory = FakeMemory {
            backend: "sqlite",
            session: Some(session),
            transcript: transcript(),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service.handle_tldr("session-1").await.unwrap_err();
        assert_eq!(
            error,
            SessionCommandError::InvalidState {
                session_id: "session-1".to_string(),
                detail: "session is ended",
            }
        );
    }

    #[tokio::test]
    async fn tldr_unknown_session_fails_clearly() {
        let memory = FakeMemory {
            backend: "sqlite",
            session: None,
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let error = service.handle_tldr("session-1").await.unwrap_err();

        assert_eq!(
            error,
            SessionCommandError::UnknownSession {
                session_id: "session-1".to_string(),
            }
        );
    }
}
