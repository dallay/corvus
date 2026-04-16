use super::types::{
    sanitize_storage_error, CommandContext, SessionCommandFailure, SessionCommandFailureKind,
    SessionCommandOutcome, SessionCommandSuccess, SessionCommandSuccessData,
};
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

    pub async fn handle_tldr(&self, session_id: &str) -> SessionCommandOutcome {
        let result: Result<SessionCommandSuccess, SessionCommandFailure> = async {
            self.ensure_sqlite("/tldr", Some(session_id))?;
            let session = self.require_active_session("/tldr", session_id).await?;
            let state = self.current_state("/tldr", session_id).await?;
            if state == SlashSessionLifecycle::Suspended {
                return Err(self.failure(
                    "/tldr",
                    SessionCommandFailureKind::InvalidState,
                    Some(session_id),
                    format!("[session:{session_id}] session is suspended"),
                ));
            }

            let excerpt = self
                .memory
                .load_session_transcript_excerpt(session_id, DEFAULT_EXCERPT_LIMIT)
                .await
                .map_err(|error| self.map_storage_error("/tldr", Some(session_id), error))?;
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
                .map_err(|error| self.map_storage_error("/tldr", Some(session_id), error))?;
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
                .map_err(|error| self.map_storage_error("/tldr", Some(session_id), error))?;

            Ok(SessionCommandSuccess {
                command: "/tldr",
                session_id: session_id.to_string(),
                message: summary,
                data: SessionCommandSuccessData::None,
            })
        }
        .await;

        Self::outcome_from_result(result)
    }

    pub async fn handle_compact(&self, session_id: &str, args: &str) -> SessionCommandOutcome {
        let result: Result<SessionCommandSuccess, SessionCommandFailure> = async {
            self.ensure_sqlite("/compact", Some(session_id))?;
            let session = self.require_active_session("/compact", session_id).await?;
            let state = self.current_state("/compact", session_id).await?;
            if state == SlashSessionLifecycle::Suspended {
                return Err(self.failure(
                    "/compact",
                    SessionCommandFailureKind::InvalidState,
                    Some(session_id),
                    format!("[session:{session_id}] session is suspended"),
                ));
            }

            let excerpt = self
                .memory
                .load_session_transcript_excerpt(session_id, DEFAULT_EXCERPT_LIMIT)
                .await
                .map_err(|error| self.map_storage_error("/compact", Some(session_id), error))?;
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
                .map_err(|error| self.map_storage_error("/compact", Some(session_id), error))?;
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
                .map_err(|error| self.map_storage_error("/compact", Some(session_id), error))?;

            Ok(SessionCommandSuccess {
                command: "/compact",
                session_id: session_id.to_string(),
                message: format!("[session:{session_id}] session compacted and ready for resume"),
                data: SessionCommandSuccessData::None,
            })
        }
        .await;

        Self::outcome_from_result(result)
    }

    pub async fn handle_suspend(&self, session_id: &str) -> SessionCommandOutcome {
        let result: Result<SessionCommandSuccess, SessionCommandFailure> = async {
            self.ensure_sqlite("/suspend", Some(session_id))?;
            self.require_active_session("/suspend", session_id).await?;
            let state = self.require_state("/suspend", session_id).await?;
            if state.lifecycle == SlashSessionLifecycle::Suspended {
                return Err(self.failure(
                    "/suspend",
                    SessionCommandFailureKind::InvalidState,
                    Some(session_id),
                    format!("[session:{session_id}] session is already suspended"),
                ));
            }
            let Some(snapshot_id) = state.latest_compact_snapshot_id.clone() else {
                return Err(self.failure(
                    "/suspend",
                    SessionCommandFailureKind::MissingSnapshot,
                    Some(session_id),
                    format!("[session:{session_id}] missing resume-capable compact snapshot"),
                ));
            };
            self.require_resume_capable_snapshot("/suspend", session_id, &snapshot_id)
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
                .map_err(|error| self.map_storage_error("/suspend", Some(session_id), error))?;

            Ok(SessionCommandSuccess {
                command: "/suspend",
                session_id: session_id.to_string(),
                message: format!("[session:{session_id}] session suspended"),
                data: SessionCommandSuccessData::None,
            })
        }
        .await;

        Self::outcome_from_result(result)
    }

    pub async fn handle_resume(
        &self,
        context: &CommandContext,
        target: Option<&str>,
    ) -> SessionCommandOutcome {
        let current_session_id = context.session.session_id.as_str();
        let result: Result<SessionCommandSuccess, SessionCommandFailure> = async {
            self.ensure_sqlite("/resume", Some(current_session_id))?;

            let Some(caller_scope_key) = context.caller.scope_key() else {
                return Err(self.failure(
                    "/resume",
                    SessionCommandFailureKind::MissingCallerScope,
                    Some(current_session_id),
                    "permission denied: caller scope unavailable".to_string(),
                ));
            };

            if let Some(target_session_id) = target {
                let scoped_target = self
                    .memory
                    .get_resumable_session_for_scope(target_session_id, caller_scope_key)
                    .await
                    .map_err(|error| {
                        self.map_storage_error("/resume", Some(target_session_id), error)
                    })?
                    .ok_or_else(|| {
                        self.failure(
                            "/resume",
                            SessionCommandFailureKind::PermissionDenied,
                            Some(target_session_id),
                            format!("[session:{target_session_id}] permission denied"),
                        )
                    })?;

                let session = self
                    .memory
                    .get_session(target_session_id)
                    .await
                    .map_err(|error| {
                        self.map_storage_error("/resume", Some(target_session_id), error)
                    })?
                    .ok_or_else(|| {
                        self.failure(
                            "/resume",
                            SessionCommandFailureKind::InvalidResumeTarget,
                            Some(target_session_id),
                            format!("[session:{target_session_id}] invalid resume target"),
                        )
                    })?;
                if session.status == SessionStatus::Ended {
                    return Err(self.failure(
                        "/resume",
                        SessionCommandFailureKind::InvalidResumeTarget,
                        Some(target_session_id),
                        format!("[session:{target_session_id}] invalid resume target"),
                    ));
                }

                let state = self
                    .get_session_state_optional("/resume", target_session_id)
                    .await?;
                let Some(state) = state else {
                    return Err(self.failure(
                        "/resume",
                        SessionCommandFailureKind::InvalidResumeTarget,
                        Some(target_session_id),
                        format!("[session:{target_session_id}] invalid resume target"),
                    ));
                };
                if state.lifecycle != SlashSessionLifecycle::Suspended {
                    return Err(self.failure(
                        "/resume",
                        SessionCommandFailureKind::InvalidResumeTarget,
                        Some(target_session_id),
                        format!("[session:{target_session_id}] invalid resume target"),
                    ));
                }
                let Some(snapshot_id) = state.latest_compact_snapshot_id else {
                    return Err(self.failure(
                        "/resume",
                        SessionCommandFailureKind::MissingSnapshot,
                        Some(target_session_id),
                        format!(
                            "[session:{target_session_id}] missing resume-capable compact snapshot"
                        ),
                    ));
                };
                self.require_resume_capable_snapshot("/resume", target_session_id, &snapshot_id)
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
                    .map_err(|error| {
                        self.map_storage_error("/resume", Some(target_session_id), error)
                    })?;

                Ok(SessionCommandSuccess {
                    command: "/resume",
                    session_id: current_session_id.to_string(),
                    message: format!(
                        "[session:{target_session_id}] resumed from persisted compact snapshot"
                    ),
                    data: SessionCommandSuccessData::Resumed {
                        resumed_session_id: scoped_target.session_id,
                    },
                })
            } else {
                let resumable_sessions = self
                    .memory
                    .list_resumable_sessions(Some(caller_scope_key), 10, 0)
                    .await
                    .map_err(|error| {
                        self.map_storage_error("/resume", Some(current_session_id), error)
                    })?;
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

                Ok(SessionCommandSuccess {
                    command: "/resume",
                    session_id: current_session_id.to_string(),
                    message,
                    data: SessionCommandSuccessData::ResumableSessions {
                        sessions: resumable_sessions,
                    },
                })
            }
        }
        .await;

        Self::outcome_from_result(result)
    }

    fn ensure_sqlite(
        &self,
        command: &'static str,
        session_id: Option<&str>,
    ) -> Result<(), SessionCommandFailure> {
        if self.memory.name() == "sqlite" {
            Ok(())
        } else {
            Err(self.failure(
                command,
                SessionCommandFailureKind::UnsupportedBackend,
                session_id,
                format!(
                    "slash-session commands require sqlite memory backend (backend={})",
                    self.memory.name()
                ),
            ))
        }
    }

    async fn require_active_session(
        &self,
        command: &'static str,
        session_id: &str,
    ) -> Result<crate::memory::SessionEntry, SessionCommandFailure> {
        let session = self
            .memory
            .get_session(session_id)
            .await
            .map_err(|error| self.map_storage_error(command, Some(session_id), error))?
            .ok_or_else(|| {
                self.failure(
                    command,
                    SessionCommandFailureKind::UnknownSession,
                    Some(session_id),
                    format!("[session:{session_id}] unknown session"),
                )
            })?;
        if session.status == SessionStatus::Ended {
            return Err(self.failure(
                command,
                SessionCommandFailureKind::InvalidState,
                Some(session_id),
                format!("[session:{session_id}] session is ended"),
            ));
        }
        Ok(session)
    }

    async fn current_state(
        &self,
        command: &'static str,
        session_id: &str,
    ) -> Result<SlashSessionLifecycle, SessionCommandFailure> {
        Ok(self
            .memory
            .get_session_state_record(session_id)
            .await
            .map_err(|error| self.map_storage_error(command, Some(session_id), error))?
            .map(|state| state.lifecycle)
            .unwrap_or(SlashSessionLifecycle::Active))
    }

    async fn require_state(
        &self,
        command: &'static str,
        session_id: &str,
    ) -> Result<crate::memory::SessionStateRecord, SessionCommandFailure> {
        self.memory
            .get_session_state_record(session_id)
            .await
            .map_err(|error| self.map_storage_error(command, Some(session_id), error))?
            .ok_or_else(|| {
                self.failure(
                    command,
                    SessionCommandFailureKind::MissingSnapshot,
                    Some(session_id),
                    format!("[session:{session_id}] missing resume-capable compact snapshot"),
                )
            })
    }

    async fn get_session_state_optional(
        &self,
        command: &'static str,
        session_id: &str,
    ) -> Result<Option<crate::memory::SessionStateRecord>, SessionCommandFailure> {
        self.memory
            .get_session_state_record(session_id)
            .await
            .map_err(|error| self.map_storage_error(command, Some(session_id), error))
    }

    async fn require_resume_capable_snapshot(
        &self,
        command: &'static str,
        session_id: &str,
        snapshot_id: &str,
    ) -> Result<crate::memory::SessionSnapshotRecord, SessionCommandFailure> {
        let snapshot = self
            .memory
            .get_session_snapshot(snapshot_id)
            .await
            .map_err(|error| self.map_storage_error(command, Some(session_id), error))?
            .ok_or_else(|| {
                self.failure(
                    command,
                    SessionCommandFailureKind::MissingSnapshot,
                    Some(session_id),
                    format!("[session:{session_id}] missing resume-capable compact snapshot"),
                )
            })?;

        if snapshot.session_id != session_id
            || snapshot.kind != SessionSnapshotKind::Compact
            || !snapshot.resume_capable
        {
            return Err(self.failure(
                command,
                SessionCommandFailureKind::MissingSnapshot,
                Some(session_id),
                format!("[session:{session_id}] missing resume-capable compact snapshot"),
            ));
        }

        Ok(snapshot)
    }

    fn map_storage_error(
        &self,
        command: &'static str,
        session_id: Option<&str>,
        error: anyhow::Error,
    ) -> SessionCommandFailure {
        if is_slash_session_unsupported_error(&error) {
            self.failure(
                command,
                SessionCommandFailureKind::UnsupportedBackend,
                session_id,
                format!(
                    "slash-session commands require sqlite memory backend (backend={})",
                    self.memory.name()
                ),
            )
        } else {
            let sanitized_summary = sanitize_storage_error(&error);
            tracing::error!(error_detail = %sanitized_summary, "storage error details (for internal logs)");
            self.failure(
                command,
                SessionCommandFailureKind::StorageFailure,
                session_id,
                format!("slash-session storage failure: {sanitized_summary}"),
            )
        }
    }

    fn failure(
        &self,
        command: &'static str,
        kind: SessionCommandFailureKind,
        session_id: Option<&str>,
        message: String,
    ) -> SessionCommandFailure {
        SessionCommandFailure {
            command,
            kind,
            session_id: session_id.map(str::to_string),
            message,
        }
    }

    fn outcome_from_result(
        result: Result<SessionCommandSuccess, SessionCommandFailure>,
    ) -> SessionCommandOutcome {
        match result {
            Ok(success) => SessionCommandOutcome::Success(success),
            Err(failure) => SessionCommandOutcome::Failure(failure),
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
        // Ensure we don't start mid-entry: skip past the newline to land on the "- " of the list entry
        if let Some(pos) = context.find("\n- ") {
            let adjustment: String = context.chars().skip(pos + 1).collect();
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
    use crate::config::ExecutionMode;
    use crate::memory::{
        MemoryCategory, MemoryEntry, ResumableSessionEntry, SessionEntry, SessionFieldPatch,
        SessionSnapshotKind, SessionSnapshotRecord, SessionStatePatch, SessionStateRecord,
    };
    use crate::session_commands::{CommandContext, CommandSessionSource};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeMemory {
        backend: &'static str,
        sessions: HashMap<String, SessionEntry>,
        transcript: Vec<MemoryEntry>,
        states: Mutex<HashMap<String, SessionStateRecord>>,
        snapshots: Mutex<Vec<SessionSnapshotRecord>>,
        listed_by_scope: HashMap<String, Vec<ResumableSessionEntry>>,
        scoped_targets: HashMap<(String, String), ResumableSessionEntry>,
        list_error: Option<String>,
    }

    impl Default for FakeMemory {
        fn default() -> Self {
            Self {
                backend: "sqlite",
                sessions: HashMap::new(),
                transcript: Vec::new(),
                states: Mutex::new(HashMap::new()),
                snapshots: Mutex::new(Vec::new()),
                listed_by_scope: HashMap::new(),
                scoped_targets: HashMap::new(),
                list_error: None,
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
            Ok(self.sessions.get(session_id).cloned())
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
            session_id: &str,
        ) -> anyhow::Result<Option<SessionStateRecord>> {
            Ok(self.states.lock().unwrap().get(session_id).cloned())
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
            let mut states = self.states.lock().unwrap();
            let current = states.get(&patch.session_id).cloned();
            let updated = SessionStateRecord {
                session_id: patch.session_id.clone(),
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
            states.insert(patch.session_id, updated.clone());
            Ok(updated)
        }

        async fn list_resumable_sessions(
            &self,
            caller_token_hash: Option<&str>,
            _limit: u32,
            _offset: u32,
        ) -> anyhow::Result<Vec<ResumableSessionEntry>> {
            if let Some(error) = &self.list_error {
                return Err(anyhow::anyhow!(error.clone()));
            }

            Ok(caller_token_hash
                .and_then(|scope| self.listed_by_scope.get(scope).cloned())
                .unwrap_or_default())
        }

        async fn get_resumable_session_for_scope(
            &self,
            session_id: &str,
            caller_scope_key: &str,
        ) -> anyhow::Result<Option<ResumableSessionEntry>> {
            Ok(self
                .scoped_targets
                .get(&(session_id.to_string(), caller_scope_key.to_string()))
                .cloned())
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

    fn context(scope_key: Option<&str>) -> CommandContext {
        CommandContext::for_cli(
            "session-current",
            CommandSessionSource::Existing,
            ExecutionMode::Standard,
            scope_key.map(str::to_string),
        )
    }

    fn scoped_entry(session_id: &str, snapshot_id: &str, preview: &str) -> ResumableSessionEntry {
        ResumableSessionEntry {
            session_id: session_id.to_string(),
            started_at: "started".to_string(),
            last_activity: "now".to_string(),
            snapshot_id: snapshot_id.to_string(),
            snapshot_created_at: "now".to_string(),
            preview: preview.to_string(),
        }
    }

    fn suspended_state(session_id: &str, snapshot_id: &str) -> SessionStateRecord {
        SessionStateRecord {
            session_id: session_id.to_string(),
            lifecycle: SlashSessionLifecycle::Suspended,
            latest_tldr_snapshot_id: None,
            latest_compact_snapshot_id: Some(snapshot_id.to_string()),
            pending_hydration_snapshot_id: None,
            suspended_at: Some("now".to_string()),
            updated_at: "now".to_string(),
        }
    }

    fn active_state(session_id: &str, snapshot_id: &str) -> SessionStateRecord {
        SessionStateRecord {
            session_id: session_id.to_string(),
            lifecycle: SlashSessionLifecycle::Active,
            latest_tldr_snapshot_id: None,
            latest_compact_snapshot_id: Some(snapshot_id.to_string()),
            pending_hydration_snapshot_id: None,
            suspended_at: None,
            updated_at: "now".to_string(),
        }
    }

    fn compact_snapshot(session_id: &str, snapshot_id: &str) -> SessionSnapshotRecord {
        SessionSnapshotRecord {
            id: snapshot_id.to_string(),
            session_id: session_id.to_string(),
            kind: SessionSnapshotKind::Compact,
            created_at: "now".to_string(),
            payload: json!({"preview": "resume me"}),
            resume_capable: true,
        }
    }

    fn expect_failure(outcome: SessionCommandOutcome) -> SessionCommandFailure {
        match outcome {
            SessionCommandOutcome::Failure(failure) => failure,
            SessionCommandOutcome::Success(success) => {
                panic!("expected failure outcome, got success: {success:?}")
            }
        }
    }

    fn expect_success(outcome: SessionCommandOutcome) -> SessionCommandSuccess {
        match outcome {
            SessionCommandOutcome::Success(success) => success,
            SessionCommandOutcome::Failure(failure) => {
                panic!("expected success outcome, got failure: {failure:?}")
            }
        }
    }

    #[tokio::test]
    async fn rejects_non_sqlite_backends() {
        let memory = FakeMemory {
            backend: "markdown",
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(service.handle_tldr("session-1").await);
        assert_eq!(failure.kind, SessionCommandFailureKind::UnsupportedBackend);
    }

    #[tokio::test]
    async fn tldr_is_deterministic_and_persists_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            sessions: HashMap::from([("session-1".to_string(), active_session())]),
            transcript: transcript(),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_tldr("session-1").await);

        assert_eq!(result.command, "/tldr");
        assert!(result.message.contains("Discuss release checklist"));
        assert_eq!(memory.snapshots.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn compact_creates_resume_capable_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            sessions: HashMap::from([("session-1".to_string(), active_session())]),
            transcript: transcript(),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_compact("session-1", "keep goals").await);

        assert_eq!(
            result.message,
            "[session:session-1] session compacted and ready for resume"
        );
        assert!(memory.snapshots.lock().unwrap()[0].resume_capable);
    }

    #[tokio::test]
    async fn suspend_succeeds_with_resume_capable_snapshot() {
        let memory = FakeMemory {
            backend: "sqlite",
            sessions: HashMap::from([("session-1".to_string(), active_session())]),
            states: Mutex::new(HashMap::from([(
                "session-1".to_string(),
                active_state("session-1", "snapshot-1"),
            )])),
            snapshots: Mutex::new(vec![compact_snapshot("session-1", "snapshot-1")]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_suspend("session-1").await);

        assert_eq!(result.command, "/suspend");
        assert_eq!(result.message, "[session:session-1] session suspended");
        assert_eq!(
            memory
                .states
                .lock()
                .unwrap()
                .get("session-1")
                .map(|state| state.lifecycle),
            Some(SlashSessionLifecycle::Suspended)
        );
    }

    #[tokio::test]
    async fn resume_requires_caller_scope_for_targeted_resume() {
        let memory = FakeMemory {
            backend: "sqlite",
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(
            service
                .handle_resume(&context(None), Some("session-1"))
                .await,
        );

        assert_eq!(failure.kind, SessionCommandFailureKind::MissingCallerScope);
    }

    #[tokio::test]
    async fn resume_target_denies_sessions_outside_caller_scope_without_mutation() {
        let memory = FakeMemory {
            backend: "sqlite",
            sessions: HashMap::from([(
                "session-denied".to_string(),
                SessionEntry {
                    id: "session-denied".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-denied".to_string(),
                suspended_state("session-denied", "snapshot-1"),
            )])),
            snapshots: Mutex::new(vec![compact_snapshot("session-denied", "snapshot-1")]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(
            service
                .handle_resume(&context(Some("authorized-scope")), Some("session-denied"))
                .await,
        );

        assert_eq!(failure.kind, SessionCommandFailureKind::PermissionDenied);
        assert_eq!(
            memory
                .states
                .lock()
                .unwrap()
                .get("session-denied")
                .and_then(|state| state.pending_hydration_snapshot_id.clone())
                .as_deref(),
            None
        );
    }

    #[tokio::test]
    async fn resume_target_rejects_invalid_target_state() {
        let memory = FakeMemory {
            backend: "sqlite",
            sessions: HashMap::from([(
                "session-active".to_string(),
                SessionEntry {
                    id: "session-active".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-active".to_string(),
                active_state("session-active", "snapshot-1"),
            )])),
            snapshots: Mutex::new(vec![compact_snapshot("session-active", "snapshot-1")]),
            scoped_targets: HashMap::from([(
                ("session-active".to_string(), "scope-a".to_string()),
                scoped_entry("session-active", "snapshot-1", "preview"),
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(
            service
                .handle_resume(&context(Some("scope-a")), Some("session-active"))
                .await,
        );

        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidResumeTarget);
    }

    #[tokio::test]
    async fn resume_target_sets_pending_hydration_for_authorized_scope() {
        let memory = FakeMemory {
            backend: "sqlite",
            sessions: HashMap::from([(
                "session-1".to_string(),
                SessionEntry {
                    id: "session-1".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-1".to_string(),
                suspended_state("session-1", "snapshot-1"),
            )])),
            snapshots: Mutex::new(vec![compact_snapshot("session-1", "snapshot-1")]),
            scoped_targets: HashMap::from([(
                ("session-1".to_string(), "caller-hash".to_string()),
                scoped_entry("session-1", "snapshot-1", "resume me"),
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(
            service
                .handle_resume(&context(Some("caller-hash")), Some("session-1"))
                .await,
        );

        assert_eq!(
            result.data,
            SessionCommandSuccessData::Resumed {
                resumed_session_id: "session-1".to_string(),
            }
        );
        assert_eq!(
            memory
                .states
                .lock()
                .unwrap()
                .get("session-1")
                .and_then(|state| state.pending_hydration_snapshot_id.clone())
                .as_deref(),
            Some("snapshot-1")
        );
    }

    #[tokio::test]
    async fn resume_list_storage_failures_are_sanitized() {
        let memory = FakeMemory {
            backend: "sqlite",
            list_error: Some("permission denied: /tmp/secret.db".to_string()),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(service.handle_resume(&context(Some("scope-a")), None).await);

        assert_eq!(failure.kind, SessionCommandFailureKind::StorageFailure);
        assert!(failure.message.contains("storage access denied"));
        assert!(!failure.message.contains("/tmp/secret.db"));
    }
}
