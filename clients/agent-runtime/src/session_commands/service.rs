use super::types::{
    sanitize_storage_error, CommandContext, SessionCommandFailure, SessionCommandFailureKind,
    SessionCommandHelpEntry, SessionCommandInspectGap, SessionCommandInspectGapCode,
    SessionCommandInspectSessionRecord, SessionCommandInspectSnapshot,
    SessionCommandInspectSnapshotSlot, SessionCommandInspectSnapshots,
    SessionCommandInspectStateRecord, SessionCommandOutcome, SessionCommandSessionInspect,
    SessionCommandSessionStatus, SessionCommandSuccess, SessionCommandSuccessData,
    SessionCommandToolEntry, SessionCommandToolSourceKind,
};
use crate::memory::{
    is_slash_session_unsupported_error, Memory, SessionEntry, SessionFieldPatch, SessionListEntry,
    SessionSnapshotKind, SessionSnapshotRecord, SessionStatePatch, SessionStateRecord,
    SessionStatus, SlashSessionLifecycle,
};
use serde_json::json;
use std::collections::HashMap;

const DEFAULT_EXCERPT_LIMIT: usize = 8;
const PREVIEW_LIMIT: usize = 120;
const SESSION_LIST_LIMIT: u32 = 50;
const SESSION_HELP_ENTRIES: &[SessionCommandHelpEntry] = &[
    SessionCommandHelpEntry {
        name: "/session",
        usage: "/session",
        description: "Show session command help and discoverability guidance.",
    },
    SessionCommandHelpEntry {
        name: "/session status",
        usage: "/session status",
        description: "Show a compact current-session summary without mutating session state.",
    },
    SessionCommandHelpEntry {
        name: "/session inspect",
        usage: "/session inspect",
        description:
            "Show a richer current-session inspection view without mutating session state.",
    },
    SessionCommandHelpEntry {
        name: "/session list",
        usage: "/session list",
        description: "List caller-scoped accessible sessions without mutating session state.",
    },
];

struct CurrentSessionReadModel {
    session: Option<SessionEntry>,
    state: Option<SessionStateRecord>,
    snapshots: ResolvedInspectSnapshots,
}

#[derive(Default)]
struct ResolvedInspectSnapshots {
    latest_tldr: ResolvedInspectSnapshotSlot,
    latest_compact: ResolvedInspectSnapshotSlot,
    pending_hydration: ResolvedInspectSnapshotSlot,
}

#[derive(Default)]
struct ResolvedInspectSnapshotSlot {
    reference_id: Option<String>,
    snapshot: Option<SessionSnapshotRecord>,
    gap: Option<SessionCommandInspectGap>,
}

pub struct SessionCommandService<'a> {
    memory: &'a dyn Memory,
    tool_snapshot: &'a [SessionCommandToolEntry],
}

impl<'a> SessionCommandService<'a> {
    pub fn new(memory: &'a dyn Memory) -> Self {
        Self {
            memory,
            tool_snapshot: &[],
        }
    }

    pub fn with_tool_snapshot(
        memory: &'a dyn Memory,
        tool_snapshot: &'a [SessionCommandToolEntry],
    ) -> Self {
        Self {
            memory,
            tool_snapshot,
        }
    }

    pub fn handle_tools(&self, session_id: &str) -> SessionCommandOutcome {
        let mut tools = self.tool_snapshot.to_vec();
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        SessionCommandOutcome::Success(SessionCommandSuccess {
            command: "/tools",
            session_id: session_id.to_string(),
            message: format_tool_listing_message(&tools),
            data: SessionCommandSuccessData::ToolListing { tools },
        })
    }

    pub async fn handle_session(
        &self,
        context: &CommandContext,
        raw_args: &str,
    ) -> SessionCommandOutcome {
        let trimmed = raw_args.trim();
        let session_id = context.session.session_id.as_str();

        if trimmed.is_empty() {
            return SessionCommandOutcome::Success(SessionCommandSuccess {
                command: "/session",
                session_id: session_id.to_string(),
                message: format_session_help_message(),
                data: SessionCommandSuccessData::SessionHelp {
                    entries: SESSION_HELP_ENTRIES.to_vec(),
                },
            });
        }

        let result = match trimmed {
            "status" => self.handle_session_status(session_id).await,
            "inspect" => self.handle_session_inspect(session_id).await,
            "list" => self.handle_session_list(context).await,
            _ => Err(self.failure(
                "/session",
                SessionCommandFailureKind::InvalidArguments,
                Some(session_id),
                invalid_session_usage_message(trimmed),
            )),
        };

        Self::outcome_from_result(result)
    }

    async fn handle_session_status(
        &self,
        session_id: &str,
    ) -> Result<SessionCommandSuccess, SessionCommandFailure> {
        self.ensure_sqlite("/session", Some(session_id))?;

        let read_model = self
            .load_current_session_read_model("/session", session_id)
            .await?;
        let status = assemble_session_status(session_id, read_model.session, read_model.state);
        Ok(SessionCommandSuccess {
            command: "/session",
            session_id: session_id.to_string(),
            message: format_session_status_message(&status),
            data: SessionCommandSuccessData::SessionStatus { status },
        })
    }

    async fn handle_session_inspect(
        &self,
        session_id: &str,
    ) -> Result<SessionCommandSuccess, SessionCommandFailure> {
        self.ensure_sqlite("/session", Some(session_id))?;

        let read_model = self
            .load_current_session_read_model("/session", session_id)
            .await?;
        let inspect = assemble_session_inspect(session_id, read_model);
        Ok(SessionCommandSuccess {
            command: "/session",
            session_id: session_id.to_string(),
            message: format_session_inspect_message(&inspect),
            data: SessionCommandSuccessData::SessionInspect {
                inspect: Box::new(inspect),
            },
        })
    }

    async fn handle_session_list(
        &self,
        context: &CommandContext,
    ) -> Result<SessionCommandSuccess, SessionCommandFailure> {
        let session_id = context.session.session_id.as_str();
        self.ensure_sqlite("/session", Some(session_id))?;

        let caller_scope_key = context.caller.scope_key().ok_or_else(|| {
            self.failure(
                "/session",
                SessionCommandFailureKind::MissingCallerScope,
                Some(session_id),
                format!("[session:{session_id}] caller scope is required for /session list"),
            )
        })?;

        let sessions = self
            .memory
            .list_session_rows_for_scope(caller_scope_key, SESSION_LIST_LIMIT, 0)
            .await
            .map_err(|error| self.map_storage_error("/session", Some(session_id), error))?;

        Ok(SessionCommandSuccess {
            command: "/session",
            session_id: session_id.to_string(),
            message: format_session_list_message(&sessions),
            data: SessionCommandSuccessData::SessionList { sessions },
        })
    }

    async fn load_current_session_read_model(
        &self,
        command: &'static str,
        session_id: &str,
    ) -> Result<CurrentSessionReadModel, SessionCommandFailure> {
        let session = self
            .memory
            .get_session(session_id)
            .await
            .map_err(|error| self.map_storage_error(command, Some(session_id), error))?;

        let Some(session) = session else {
            return Ok(CurrentSessionReadModel {
                session: None,
                state: None,
                snapshots: ResolvedInspectSnapshots::default(),
            });
        };

        let state = self.get_session_state_optional(command, session_id).await?;
        let snapshots = match state.as_ref() {
            Some(state) => {
                self.resolve_inspect_snapshots(command, session_id, state)
                    .await?
            }
            None => ResolvedInspectSnapshots::default(),
        };

        Ok(CurrentSessionReadModel {
            session: Some(session),
            state,
            snapshots,
        })
    }

    async fn resolve_inspect_snapshots(
        &self,
        command: &'static str,
        session_id: &str,
        state: &SessionStateRecord,
    ) -> Result<ResolvedInspectSnapshots, SessionCommandFailure> {
        let mut cache = HashMap::<String, Option<SessionSnapshotRecord>>::new();

        Ok(ResolvedInspectSnapshots {
            latest_tldr: self
                .resolve_inspect_snapshot_slot(
                    command,
                    session_id,
                    state.latest_tldr_snapshot_id.as_deref(),
                    SessionSnapshotKind::Tldr,
                    "latest TLDR",
                    &mut cache,
                )
                .await?,
            latest_compact: self
                .resolve_inspect_snapshot_slot(
                    command,
                    session_id,
                    state.latest_compact_snapshot_id.as_deref(),
                    SessionSnapshotKind::Compact,
                    "latest compact",
                    &mut cache,
                )
                .await?,
            pending_hydration: self
                .resolve_inspect_snapshot_slot(
                    command,
                    session_id,
                    state.pending_hydration_snapshot_id.as_deref(),
                    SessionSnapshotKind::Compact,
                    "pending hydration",
                    &mut cache,
                )
                .await?,
        })
    }

    async fn resolve_inspect_snapshot_slot(
        &self,
        command: &'static str,
        session_id: &str,
        reference_id: Option<&str>,
        expected_kind: SessionSnapshotKind,
        slot_label: &'static str,
        cache: &mut HashMap<String, Option<SessionSnapshotRecord>>,
    ) -> Result<ResolvedInspectSnapshotSlot, SessionCommandFailure> {
        let Some(reference_id) = reference_id else {
            return Ok(ResolvedInspectSnapshotSlot::default());
        };

        let snapshot = if let Some(snapshot) = cache.get(reference_id) {
            snapshot.clone()
        } else {
            let snapshot = self
                .memory
                .get_session_snapshot(reference_id)
                .await
                .map_err(|error| self.map_storage_error(command, Some(session_id), error))?;
            cache.insert(reference_id.to_string(), snapshot.clone());
            snapshot
        };

        let mut slot = ResolvedInspectSnapshotSlot {
            reference_id: Some(reference_id.to_string()),
            snapshot: None,
            gap: None,
        };

        match snapshot {
            None => {
                slot.gap = Some(SessionCommandInspectGap {
                    code: SessionCommandInspectGapCode::ReferencedSnapshotMissing,
                    reference_id: Some(reference_id.to_string()),
                    detail: format!(
                        "referenced snapshot {reference_id} is missing from authoritative storage"
                    ),
                });
            }
            Some(snapshot) if snapshot.session_id != session_id => {
                slot.gap = Some(SessionCommandInspectGap {
                    code: SessionCommandInspectGapCode::ReferencedSnapshotOwnershipMismatch,
                    reference_id: Some(reference_id.to_string()),
                    detail: format!(
                        "referenced snapshot {reference_id} belongs to a different session"
                    ),
                });
            }
            Some(snapshot) if snapshot.kind != expected_kind => {
                slot.gap = Some(SessionCommandInspectGap {
                    code: SessionCommandInspectGapCode::ReferencedSnapshotKindMismatch,
                    reference_id: Some(reference_id.to_string()),
                    detail: format!(
                        "referenced snapshot {reference_id} has unexpected kind for {slot_label}"
                    ),
                });
            }
            Some(snapshot) => {
                slot.snapshot = Some(snapshot);
            }
        }

        Ok(slot)
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

    pub fn handle_model(&self, session_id: &str, raw_args: &str) -> SessionCommandOutcome {
        let trimmed = raw_args.trim();
        let (message, current) = if trimmed.is_empty() {
            (
                "Current model: (not set). Pass a model name to change it.".to_string(),
                String::new(),
            )
        } else {
            (format!("Model set to: {trimmed}"), trimmed.to_string())
        };
        SessionCommandOutcome::Success(SessionCommandSuccess {
            command: "/model",
            session_id: session_id.to_string(),
            message,
            data: SessionCommandSuccessData::ModelInfo {
                current,
                available: vec![],
            },
        })
    }

    pub fn handle_provider(&self, session_id: &str, raw_args: &str) -> SessionCommandOutcome {
        let trimmed = raw_args.trim();
        let (message, current) = if trimmed.is_empty() {
            (
                "Current provider: (not set). Pass a provider name to change it.".to_string(),
                String::new(),
            )
        } else {
            (format!("Provider set to: {trimmed}"), trimmed.to_string())
        };
        SessionCommandOutcome::Success(SessionCommandSuccess {
            command: "/provider",
            session_id: session_id.to_string(),
            message,
            data: SessionCommandSuccessData::ProviderInfo {
                current,
                available: vec![],
            },
        })
    }

    pub fn handle_temperature(&self, session_id: &str) -> SessionCommandOutcome {
        let current = 0.7_f32;
        SessionCommandOutcome::Success(SessionCommandSuccess {
            command: "/temperature",
            session_id: session_id.to_string(),
            message: format!("Current temperature: {current}"),
            data: SessionCommandSuccessData::TemperatureInfo { current },
        })
    }

    pub fn handle_mcp(&self, session_id: &str, raw_args: &str) -> SessionCommandOutcome {
        let trimmed = raw_args.trim();
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let subcommand = parts.next().unwrap_or("").trim();
        let rest = parts.next().unwrap_or("").trim();

        match subcommand {
            "list" => SessionCommandOutcome::Success(SessionCommandSuccess {
                command: "/mcp",
                session_id: session_id.to_string(),
                message: "MCP servers: (none registered)".to_string(),
                data: SessionCommandSuccessData::McpList { servers: vec![] },
            }),
            "add" => {
                if rest.is_empty() {
                    SessionCommandOutcome::Failure(self.failure(
                        "/mcp",
                        SessionCommandFailureKind::InvalidArguments,
                        Some(session_id),
                        "Usage: /mcp add <server-name>".to_string(),
                    ))
                } else {
                    SessionCommandOutcome::Success(SessionCommandSuccess {
                        command: "/mcp",
                        session_id: session_id.to_string(),
                        message: format!("MCP server added: {rest}"),
                        data: SessionCommandSuccessData::McpAdded {
                            server: rest.to_string(),
                        },
                    })
                }
            }
            "remove" => {
                if rest.is_empty() {
                    SessionCommandOutcome::Failure(self.failure(
                        "/mcp",
                        SessionCommandFailureKind::InvalidArguments,
                        Some(session_id),
                        "Usage: /mcp remove <server-name>".to_string(),
                    ))
                } else {
                    SessionCommandOutcome::Success(SessionCommandSuccess {
                        command: "/mcp",
                        session_id: session_id.to_string(),
                        message: format!("MCP server removed: {rest}"),
                        data: SessionCommandSuccessData::McpRemoved {
                            server: rest.to_string(),
                        },
                    })
                }
            }
            other => SessionCommandOutcome::Failure(self.failure(
                "/mcp",
                SessionCommandFailureKind::InvalidArguments,
                Some(session_id),
                format!("Unknown /mcp subcommand: '{other}'. Use list, add, or remove."),
            )),
        }
    }

    pub fn handle_tool_manage(&self, session_id: &str, raw_args: &str) -> SessionCommandOutcome {
        let trimmed = raw_args.trim();
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let subcommand = parts.next().unwrap_or("").trim();
        let name = parts.next().unwrap_or("").trim();

        match subcommand {
            "enable" => {
                if name.is_empty() {
                    SessionCommandOutcome::Failure(self.failure(
                        "/tool",
                        SessionCommandFailureKind::InvalidArguments,
                        Some(session_id),
                        "Usage: /tool enable <tool-name>".to_string(),
                    ))
                } else {
                    SessionCommandOutcome::Success(SessionCommandSuccess {
                        command: "/tool",
                        session_id: session_id.to_string(),
                        message: format!("Tool enabled: {name}"),
                        data: SessionCommandSuccessData::ToolEnabled {
                            name: name.to_string(),
                        },
                    })
                }
            }
            "disable" => {
                if name.is_empty() {
                    SessionCommandOutcome::Failure(self.failure(
                        "/tool",
                        SessionCommandFailureKind::InvalidArguments,
                        Some(session_id),
                        "Usage: /tool disable <tool-name>".to_string(),
                    ))
                } else {
                    SessionCommandOutcome::Success(SessionCommandSuccess {
                        command: "/tool",
                        session_id: session_id.to_string(),
                        message: format!("Tool disabled: {name}"),
                        data: SessionCommandSuccessData::ToolDisabled {
                            name: name.to_string(),
                        },
                    })
                }
            }
            other => SessionCommandOutcome::Failure(self.failure(
                "/tool",
                SessionCommandFailureKind::InvalidArguments,
                Some(session_id),
                format!("Unknown /tool subcommand: '{other}'. Use enable or disable."),
            )),
        }
    }
}

fn format_session_help_message() -> String {
    let entries = SESSION_HELP_ENTRIES
        .iter()
        .map(|entry| format!("- {} — {}", entry.usage, entry.description))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Session commands:\n{entries}\nRelated lifecycle commands: /resume, /suspend, /compact, /tldr"
    )
}

fn invalid_session_usage_message(raw_args: &str) -> String {
    format!(
        "Unknown /session subcommand: '{raw_args}'. Usage: /session, /session status, /session inspect, or /session list"
    )
}

fn format_session_list_message(sessions: &[SessionListEntry]) -> String {
    if sessions.is_empty() {
        return "No accessible sessions for the current caller scope.".to_string();
    }

    let rows = sessions
        .iter()
        .map(|session| {
            format!(
                "- {} — {}, resumable={}, last activity {}",
                session.id,
                session.lifecycle.as_str(),
                if session.resumable { "yes" } else { "no" },
                session.last_activity
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("Accessible sessions ({}):\n{rows}", sessions.len())
}

fn assemble_session_status(
    session_id: &str,
    session: Option<crate::memory::SessionEntry>,
    state: Option<crate::memory::SessionStateRecord>,
) -> SessionCommandSessionStatus {
    let Some(session) = session else {
        return SessionCommandSessionStatus {
            session_id: session_id.to_string(),
            current_session_known: false,
            session_status: None,
            slash_lifecycle: None,
            started_at: None,
            last_activity: None,
            ended_at: None,
            message_count: None,
            has_tldr_snapshot: None,
            has_compact_snapshot: None,
            resume_hydration_pending: None,
            suspended_at: None,
            recommendation: None,
        };
    };

    let slash_lifecycle = state
        .as_ref()
        .map(|record| record.lifecycle)
        .unwrap_or(SlashSessionLifecycle::Active);
    let has_tldr_snapshot = state
        .as_ref()
        .map(|record| record.latest_tldr_snapshot_id.is_some())
        .unwrap_or(false);
    let has_compact_snapshot = state
        .as_ref()
        .map(|record| record.latest_compact_snapshot_id.is_some())
        .unwrap_or(false);
    let resume_hydration_pending = state
        .as_ref()
        .map(|record| record.pending_hydration_snapshot_id.is_some())
        .unwrap_or(false);
    let suspended_at = if slash_lifecycle == SlashSessionLifecycle::Suspended {
        state
            .as_ref()
            .and_then(|record| record.suspended_at.clone())
    } else {
        None
    };
    let recommendation =
        session_status_recommendation(slash_lifecycle, has_compact_snapshot).map(str::to_string);

    SessionCommandSessionStatus {
        session_id: session.id,
        current_session_known: true,
        session_status: Some(session.status),
        slash_lifecycle: Some(slash_lifecycle),
        started_at: Some(session.started_at),
        last_activity: Some(session.last_activity),
        ended_at: session.ended_at,
        message_count: Some(session.message_count),
        has_tldr_snapshot: Some(has_tldr_snapshot),
        has_compact_snapshot: Some(has_compact_snapshot),
        resume_hydration_pending: Some(resume_hydration_pending),
        suspended_at,
        recommendation,
    }
}

fn session_status_recommendation(
    slash_lifecycle: SlashSessionLifecycle,
    has_compact_snapshot: bool,
) -> Option<&'static str> {
    match (slash_lifecycle, has_compact_snapshot) {
        (SlashSessionLifecycle::Active, false) => Some("/compact"),
        (SlashSessionLifecycle::Active, true) => Some("/suspend"),
        (SlashSessionLifecycle::Suspended, true) => Some("/resume"),
        (SlashSessionLifecycle::Suspended, false) => None,
    }
}

fn assemble_session_inspect(
    session_id: &str,
    read_model: CurrentSessionReadModel,
) -> SessionCommandSessionInspect {
    let CurrentSessionReadModel {
        session,
        state,
        snapshots,
    } = read_model;

    let Some(session) = session else {
        return SessionCommandSessionInspect {
            session_id: session_id.to_string(),
            current_session_known: false,
            session: None,
            state: None,
            snapshots: SessionCommandInspectSnapshots::default(),
            gaps: Vec::new(),
        };
    };

    let mut gaps = Vec::new();
    let state_record = state.map(|record| SessionCommandInspectStateRecord {
        lifecycle: record.lifecycle,
        latest_tldr_snapshot_id: record.latest_tldr_snapshot_id,
        latest_compact_snapshot_id: record.latest_compact_snapshot_id,
        pending_hydration_snapshot_id: record.pending_hydration_snapshot_id,
        suspended_at: record.suspended_at,
        updated_at: record.updated_at,
    });

    if state_record.is_none() {
        gaps.push(SessionCommandInspectGap {
            code: SessionCommandInspectGapCode::SlashSessionStateMissing,
            reference_id: None,
            detail: "slash-session state is missing from authoritative storage".to_string(),
        });
        gaps.push(SessionCommandInspectGap {
            code: SessionCommandInspectGapCode::SnapshotUnavailableWithoutState,
            reference_id: None,
            detail:
                "snapshot-derived details are unavailable because slash-session state is missing"
                    .to_string(),
        });
    }

    let inspect_snapshots = SessionCommandInspectSnapshots {
        latest_tldr: assemble_inspect_snapshot_slot(snapshots.latest_tldr, &mut gaps),
        latest_compact: assemble_inspect_snapshot_slot(snapshots.latest_compact, &mut gaps),
        pending_hydration: assemble_inspect_snapshot_slot(snapshots.pending_hydration, &mut gaps),
    };

    SessionCommandSessionInspect {
        session_id: session.id,
        current_session_known: true,
        session: Some(SessionCommandInspectSessionRecord {
            status: session.status,
            started_at: session.started_at,
            last_activity: session.last_activity,
            ended_at: session.ended_at,
            message_count: session.message_count,
        }),
        state: state_record,
        snapshots: inspect_snapshots,
        gaps,
    }
}

fn assemble_inspect_snapshot_slot(
    resolved: ResolvedInspectSnapshotSlot,
    gaps: &mut Vec<SessionCommandInspectGap>,
) -> SessionCommandInspectSnapshotSlot {
    if let Some(gap) = resolved.gap {
        gaps.push(gap);
    }

    SessionCommandInspectSnapshotSlot {
        reference_id: resolved.reference_id,
        snapshot: resolved
            .snapshot
            .map(|snapshot| SessionCommandInspectSnapshot {
                snapshot_id: snapshot.id,
                kind: snapshot.kind,
                created_at: snapshot.created_at,
                resume_capable: snapshot.resume_capable,
                payload: snapshot.payload,
            }),
    }
}

fn format_session_status_message(status: &SessionCommandSessionStatus) -> String {
    if !status.current_session_known {
        return format!(
            "[session:{}] current session is unknown to slash-session state",
            status.session_id
        );
    }

    let lifecycle = status
        .slash_lifecycle
        .map(SlashSessionLifecycle::as_str)
        .unwrap_or("unknown");
    let compact = bool_label(status.has_compact_snapshot);
    let tldr = bool_label(status.has_tldr_snapshot);
    let recommendation = status
        .recommendation
        .as_deref()
        .map(|value| format!("\nRecommended next command: {value}"))
        .unwrap_or_default();
    let suspended = status
        .suspended_at
        .as_deref()
        .map(|value| format!("\nSuspended at: {value}"))
        .unwrap_or_default();

    format!(
        "[session:{}] current session status: {lifecycle}\nTLDR snapshot available: {tldr}\nCompact snapshot available: {compact}{suspended}{recommendation}",
        status.session_id
    )
}

fn format_session_inspect_message(inspect: &SessionCommandSessionInspect) -> String {
    if !inspect.current_session_known {
        return format!(
            "[session:{}] current session is unknown to slash-session state",
            inspect.session_id
        );
    }

    let mut lines = vec![format!(
        "[session:{}] current session inspection",
        inspect.session_id
    )];

    if let Some(session) = inspect.session.as_ref() {
        let ended = session
            .ended_at
            .as_deref()
            .map(|ended_at| format!(", ended {ended_at}"))
            .unwrap_or_default();
        lines.push(format!(
            "Session record: {}, started {}, last activity {}, messages {}{}",
            session.status.as_str(),
            session.started_at,
            session.last_activity,
            session.message_count,
            ended,
        ));
    }

    match inspect.state.as_ref() {
        Some(state) => {
            let suspended = state
                .suspended_at
                .as_deref()
                .map(|value| format!(", suspended at {value}"))
                .unwrap_or_default();
            lines.push(format!(
                "Slash state: {}, updated {}{}",
                state.lifecycle.as_str(),
                state.updated_at,
                suspended,
            ));
        }
        None => lines.push("Slash state: missing from authoritative storage".to_string()),
    }

    lines.push("Snapshots:".to_string());
    lines.push(format_snapshot_line(
        "TLDR",
        &inspect.snapshots.latest_tldr,
        inspect.state.is_some(),
    ));
    lines.push(format_snapshot_line(
        "Compact",
        &inspect.snapshots.latest_compact,
        inspect.state.is_some(),
    ));
    lines.push(format_snapshot_line(
        "Pending hydration",
        &inspect.snapshots.pending_hydration,
        inspect.state.is_some(),
    ));

    if !inspect.gaps.is_empty() {
        lines.push("Gaps:".to_string());
        lines.extend(inspect.gaps.iter().map(|gap| format!("- {}", gap.detail)));
    }

    lines.join("\n")
}

fn format_snapshot_line(
    label: &str,
    slot: &SessionCommandInspectSnapshotSlot,
    has_state: bool,
) -> String {
    match (
        slot.reference_id.as_deref(),
        slot.snapshot.as_ref(),
        has_state,
    ) {
        (Some(_reference_id), Some(snapshot), _) => {
            let resume_capable = if snapshot.resume_capable {
                " (resume-capable)"
            } else {
                ""
            };
            format!(
                "- {label}: {} @ {}{}",
                snapshot.snapshot_id, snapshot.created_at, resume_capable
            )
        }
        (Some(reference_id), None, _) => {
            format!("- {label}: referenced snapshot {reference_id} not available")
        }
        (None, None, false) => format!("- {label}: unavailable (slash-session state missing)"),
        (None, None, true) => format!("- {label}: none"),
        (None, Some(snapshot), _) => {
            let resume_capable = if snapshot.resume_capable {
                " (resume-capable)"
            } else {
                ""
            };
            format!(
                "- {label}: {} @ {}{}",
                snapshot.snapshot_id, snapshot.created_at, resume_capable
            )
        }
    }
}

fn bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
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

fn format_tool_listing_message(tools: &[SessionCommandToolEntry]) -> String {
    if tools.is_empty() {
        return "No tools are currently available.".to_string();
    }

    let lines = tools
        .iter()
        .map(format_tool_listing_line)
        .collect::<Vec<_>>()
        .join("\n");
    format!("Available tools ({}):\n{lines}", tools.len())
}

fn format_tool_listing_line(tool: &SessionCommandToolEntry) -> String {
    match tool.source_kind {
        SessionCommandToolSourceKind::Native => {
            format!("- {} — {}", tool.name, tool.description)
        }
        SessionCommandToolSourceKind::McpTool => format_mcp_tool_line(tool, "mcp tool"),
        SessionCommandToolSourceKind::McpResource => format_mcp_tool_line(tool, "mcp resource"),
        SessionCommandToolSourceKind::McpPrompt => format_mcp_tool_line(tool, "mcp prompt"),
    }
}

fn format_mcp_tool_line(tool: &SessionCommandToolEntry, source: &str) -> String {
    match tool.source_label.as_deref() {
        Some(label) if !label.trim().is_empty() => {
            format!("- {} — {} [{source}: {label}]", tool.name, tool.description)
        }
        _ => format!("- {} — {} [{source}]", tool.name, tool.description),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutionMode;
    use crate::memory::{
        MemoryCategory, MemoryEntry, ResumableSessionEntry, SessionEntry, SessionFieldPatch,
        SessionListEntry, SessionSnapshotKind, SessionSnapshotRecord, SessionStatePatch,
        SessionStateRecord,
    };
    use crate::session_commands::{
        CommandContext, CommandSessionSource, SessionCommandInspectGapCode,
        SessionCommandSuccessData, SessionCommandToolEntry, SessionCommandToolSourceKind,
    };
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
        session_rows_by_scope: HashMap<String, Vec<SessionListEntry>>,
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
                session_rows_by_scope: HashMap::new(),
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

        async fn list_session_rows_for_scope(
            &self,
            caller_scope_key: &str,
            _limit: u32,
            _offset: u32,
        ) -> anyhow::Result<Vec<SessionListEntry>> {
            Ok(self
                .session_rows_by_scope
                .get(caller_scope_key)
                .cloned()
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

    fn tldr_snapshot(session_id: &str, snapshot_id: &str) -> SessionSnapshotRecord {
        SessionSnapshotRecord {
            id: snapshot_id.to_string(),
            session_id: session_id.to_string(),
            kind: SessionSnapshotKind::Tldr,
            created_at: "tldr-created".to_string(),
            payload: json!({"summary": "short summary"}),
            resume_capable: false,
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
    async fn tools_are_sorted_and_exposed_as_machine_readable_listing() {
        let memory = FakeMemory::default();
        let tools = vec![
            SessionCommandToolEntry {
                name: "shell".to_string(),
                description: "Execute shell commands".to_string(),
                source_kind: SessionCommandToolSourceKind::Native,
                source_label: None,
            },
            SessionCommandToolEntry {
                name: "file_read".to_string(),
                description: "Read files".to_string(),
                source_kind: SessionCommandToolSourceKind::Native,
                source_label: None,
            },
            SessionCommandToolEntry {
                name: "mcp.docs.search".to_string(),
                description: "Search docs".to_string(),
                source_kind: SessionCommandToolSourceKind::McpTool,
                source_label: Some("docs".to_string()),
            },
        ];
        let service = SessionCommandService::with_tool_snapshot(&memory, &tools);

        let success = expect_success(service.handle_tools("session-tools"));

        assert_eq!(success.command, "/tools");
        assert_eq!(success.session_id, "session-tools");
        assert_eq!(
            success.message,
            "Available tools (3):\n- file_read — Read files\n- mcp.docs.search — Search docs [mcp tool: docs]\n- shell — Execute shell commands"
        );
        assert_eq!(
            success.data,
            SessionCommandSuccessData::ToolListing {
                tools: vec![
                    SessionCommandToolEntry {
                        name: "file_read".to_string(),
                        description: "Read files".to_string(),
                        source_kind: SessionCommandToolSourceKind::Native,
                        source_label: None,
                    },
                    SessionCommandToolEntry {
                        name: "mcp.docs.search".to_string(),
                        description: "Search docs".to_string(),
                        source_kind: SessionCommandToolSourceKind::McpTool,
                        source_label: Some("docs".to_string()),
                    },
                    SessionCommandToolEntry {
                        name: "shell".to_string(),
                        description: "Execute shell commands".to_string(),
                        source_kind: SessionCommandToolSourceKind::Native,
                        source_label: None,
                    },
                ],
            }
        );
    }

    #[tokio::test]
    async fn tools_returns_explicit_empty_state_success() {
        let memory = FakeMemory::default();
        let tools = Vec::new();
        let service = SessionCommandService::with_tool_snapshot(&memory, &tools);

        let success = expect_success(service.handle_tools("session-tools"));

        assert_eq!(success.message, "No tools are currently available.");
        assert_eq!(
            success.data,
            SessionCommandSuccessData::ToolListing { tools: vec![] }
        );
    }

    #[tokio::test]
    async fn tools_formats_mixed_native_and_mcp_sources() {
        let memory = FakeMemory::default();
        let tools = vec![
            SessionCommandToolEntry {
                name: "mcp.docs.prompt.ask".to_string(),
                description: "Prompt docs".to_string(),
                source_kind: SessionCommandToolSourceKind::McpPrompt,
                source_label: Some("docs".to_string()),
            },
            SessionCommandToolEntry {
                name: "mcp.docs.resource.index".to_string(),
                description: "Docs index".to_string(),
                source_kind: SessionCommandToolSourceKind::McpResource,
                source_label: Some("docs".to_string()),
            },
            SessionCommandToolEntry {
                name: "file_write".to_string(),
                description: "Write files".to_string(),
                source_kind: SessionCommandToolSourceKind::Native,
                source_label: None,
            },
        ];
        let service = SessionCommandService::with_tool_snapshot(&memory, &tools);

        let success = expect_success(service.handle_tools("session-tools"));

        assert!(success.message.contains("- file_write — Write files"));
        assert!(success
            .message
            .contains("- mcp.docs.prompt.ask — Prompt docs [mcp prompt: docs]"));
        assert!(success
            .message
            .contains("- mcp.docs.resource.index — Docs index [mcp resource: docs]"));
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

    #[tokio::test]
    async fn session_root_help_returns_discoverability_guidance_without_mutation() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "").await);

        assert_eq!(result.command, "/session");
        assert!(result.message.contains("/session status"));
        assert!(result.message.contains("/session inspect"));
        assert!(result.message.contains("/session list"));
        assert!(result.message.contains("/resume"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionHelp { ref entries }
            if entries.iter().any(|entry| entry.usage == "/session status")
                && entries.iter().any(|entry| entry.usage == "/session inspect")
                && entries.iter().any(|entry| entry.usage == "/session list")
        ));
        assert!(memory.states.lock().unwrap().is_empty());
        assert!(memory.snapshots.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn session_root_help_mentions_session_list() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(Some("scope-a")), "").await);

        assert!(result.message.contains("/session list"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionHelp { ref entries }
                if entries.iter().any(|entry| entry.usage == "/session list")
        ));
    }

    #[tokio::test]
    async fn session_list_returns_caller_scoped_rows_in_desc_order_with_balanced_output() {
        let rows = vec![
            SessionListEntry {
                id: "sess-c".to_string(),
                last_activity: "2026-04-19T12:01:00Z".to_string(),
                lifecycle: SlashSessionLifecycle::Suspended,
                resumable: true,
            },
            SessionListEntry {
                id: "sess-a".to_string(),
                last_activity: "2026-04-19T12:00:00Z".to_string(),
                lifecycle: SlashSessionLifecycle::Active,
                resumable: false,
            },
        ];
        let memory = FakeMemory {
            session_rows_by_scope: HashMap::from([("scope-a".to_string(), rows.clone())]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(
            service
                .handle_session(&context(Some("scope-a")), "list")
                .await,
        );

        assert!(result.message.contains("sess-c"));
        assert!(result.message.contains("sess-a"));
        assert_eq!(
            result.data,
            SessionCommandSuccessData::SessionList { sessions: rows }
        );
    }

    #[tokio::test]
    async fn session_list_returns_empty_success_for_scope_with_no_visible_sessions() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let result = expect_success(
            service
                .handle_session(&context(Some("scope-a")), "list")
                .await,
        );

        assert!(result.message.contains("No accessible sessions"));
        assert_eq!(
            result.data,
            SessionCommandSuccessData::SessionList {
                sessions: Vec::new(),
            }
        );
    }

    #[tokio::test]
    async fn session_list_requires_caller_scope() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(service.handle_session(&context(None), "list").await);

        assert_eq!(failure.kind, SessionCommandFailureKind::MissingCallerScope);
    }

    #[tokio::test]
    async fn session_list_rejects_extra_tokens_after_supported_subcommand() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(
            service
                .handle_session(&context(Some("scope-a")), "list extra")
                .await,
        );

        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    #[tokio::test]
    async fn session_status_reports_active_current_session_and_recommends_compact() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "status").await);

        assert_eq!(result.command, "/session");
        assert!(result
            .message
            .contains("Recommended next command: /compact"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionStatus { ref status }
                if status.session_id == "session-current"
                    && status.current_session_known
                    && status.session_status == Some(SessionStatus::Active)
                    && status.slash_lifecycle == Some(SlashSessionLifecycle::Active)
                    && status.has_compact_snapshot == Some(false)
                    && status.recommendation.as_deref() == Some("/compact")
        ));
    }

    #[tokio::test]
    async fn session_status_reports_suspended_state_and_recommends_resume() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-current".to_string(),
                SessionStateRecord {
                    session_id: "session-current".to_string(),
                    lifecycle: SlashSessionLifecycle::Suspended,
                    latest_tldr_snapshot_id: Some("tldr-1".to_string()),
                    latest_compact_snapshot_id: Some("compact-1".to_string()),
                    pending_hydration_snapshot_id: None,
                    suspended_at: Some("suspended-at".to_string()),
                    updated_at: "now".to_string(),
                },
            )])),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "status").await);

        assert!(result.message.contains("Recommended next command: /resume"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionStatus { ref status }
                if status.slash_lifecycle == Some(SlashSessionLifecycle::Suspended)
                    && status.has_tldr_snapshot == Some(true)
                    && status.has_compact_snapshot == Some(true)
                    && status.suspended_at.as_deref() == Some("suspended-at")
                    && status.recommendation.as_deref() == Some("/resume")
        ));
    }

    #[tokio::test]
    async fn session_status_defaults_missing_state_to_active_and_recommends_compact() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "status").await);

        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionStatus { ref status }
                if status.slash_lifecycle == Some(SlashSessionLifecycle::Active)
                    && status.has_tldr_snapshot == Some(false)
                    && status.has_compact_snapshot == Some(false)
                    && status.resume_hydration_pending == Some(false)
        ));
    }

    #[tokio::test]
    async fn session_status_reports_unknown_current_session_without_inventing_state() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let missing = CommandContext::for_cli(
            "session-missing",
            CommandSessionSource::Existing,
            ExecutionMode::Standard,
            None,
        );
        let result = expect_success(service.handle_session(&missing, "status").await);

        assert!(result
            .message
            .contains("current session is unknown to slash-session state"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionStatus { ref status }
                if status.session_id == "session-missing"
                    && !status.current_session_known
                    && status.session_status.is_none()
                    && status.slash_lifecycle.is_none()
                    && status.has_tldr_snapshot.is_none()
                    && status.has_compact_snapshot.is_none()
                    && status.recommendation.is_none()
        ));
    }

    #[tokio::test]
    async fn session_status_requires_sqlite_backend_only_for_status_branch() {
        let memory = FakeMemory {
            backend: "markdown",
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let help = expect_success(service.handle_session(&context(None), "").await);
        let failure = expect_failure(service.handle_session(&context(None), "status").await);

        assert!(matches!(
            help.data,
            SessionCommandSuccessData::SessionHelp { .. }
        ));
        assert_eq!(failure.kind, SessionCommandFailureKind::UnsupportedBackend);
    }

    #[tokio::test]
    async fn session_status_recommends_suspend_when_compact_snapshot_exists() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-current".to_string(),
                SessionStateRecord {
                    session_id: "session-current".to_string(),
                    lifecycle: SlashSessionLifecycle::Active,
                    latest_tldr_snapshot_id: None,
                    latest_compact_snapshot_id: Some("compact-1".to_string()),
                    pending_hydration_snapshot_id: None,
                    suspended_at: None,
                    updated_at: "now".to_string(),
                },
            )])),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "status").await);

        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionStatus { ref status }
                if status.recommendation.as_deref() == Some("/suspend")
        ));
    }

    #[tokio::test]
    async fn session_rejects_invalid_subcommands_with_usage_guidance() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(service.handle_session(&context(None), "archive").await);

        assert_eq!(failure.command, "/session");
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
        assert!(failure.message.contains("Usage: /session"));
        assert!(failure.message.contains("/session status"));
        assert!(failure.message.contains("/session inspect"));
        assert!(failure.message.contains("/session list"));
    }

    #[tokio::test]
    async fn session_status_rejects_extra_tokens_after_supported_subcommand() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(service.handle_session(&context(None), "status extra").await);

        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    #[tokio::test]
    async fn session_inspect_returns_richer_current_session_view_when_authoritative_data_is_complete(
    ) {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-current".to_string(),
                SessionStateRecord {
                    session_id: "session-current".to_string(),
                    lifecycle: SlashSessionLifecycle::Suspended,
                    latest_tldr_snapshot_id: Some("tldr-1".to_string()),
                    latest_compact_snapshot_id: Some("compact-1".to_string()),
                    pending_hydration_snapshot_id: Some("compact-1".to_string()),
                    suspended_at: Some("suspended-at".to_string()),
                    updated_at: "updated-at".to_string(),
                },
            )])),
            snapshots: Mutex::new(vec![
                tldr_snapshot("session-current", "tldr-1"),
                compact_snapshot("session-current", "compact-1"),
            ]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "inspect").await);

        assert_eq!(result.command, "/session");
        assert!(result.message.contains("current session inspection"));
        assert!(result.message.contains("Session record: active"));
        assert!(result.message.contains("Slash state: suspended"));
        assert!(result.message.contains("TLDR: tldr-1 @ tldr-created"));
        assert!(result
            .message
            .contains("Compact: compact-1 @ now (resume-capable)"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionInspect { ref inspect }
                if inspect.session_id == "session-current"
                    && inspect.current_session_known
                    && inspect.gaps.is_empty()
                    && inspect.session.as_ref().map(|session| session.message_count) == Some(3)
                    && inspect.state.as_ref().map(|state| state.lifecycle)
                        == Some(SlashSessionLifecycle::Suspended)
                    && inspect.snapshots.latest_tldr.reference_id.as_deref() == Some("tldr-1")
                    && inspect.snapshots.latest_tldr.snapshot.as_ref().map(|snapshot| snapshot.snapshot_id.as_str())
                        == Some("tldr-1")
                    && inspect.snapshots.latest_compact.reference_id.as_deref() == Some("compact-1")
                    && inspect
                        .snapshots
                        .latest_compact
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.resume_capable)
                    && inspect.snapshots.pending_hydration.reference_id.as_deref() == Some("compact-1")
                    && inspect.snapshots.pending_hydration.snapshot.as_ref().map(|snapshot| snapshot.snapshot_id.as_str())
                        == Some("compact-1")
        ));
    }

    #[tokio::test]
    async fn session_inspect_returns_partial_data_when_state_is_missing() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "inspect").await);

        assert!(result
            .message
            .contains("Slash state: missing from authoritative storage"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionInspect { ref inspect }
                if inspect.current_session_known
                    && inspect.session.is_some()
                    && inspect.state.is_none()
                    && inspect.snapshots.latest_tldr.reference_id.is_none()
                    && inspect.snapshots.latest_tldr.snapshot.is_none()
                    && inspect.gaps.iter().any(|gap| gap.code == SessionCommandInspectGapCode::SlashSessionStateMissing)
                    && inspect.gaps.iter().any(|gap| gap.code == SessionCommandInspectGapCode::SnapshotUnavailableWithoutState)
        ));
    }

    #[tokio::test]
    async fn session_inspect_reports_missing_and_mismatched_referenced_snapshots_without_inventing_details(
    ) {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            states: Mutex::new(HashMap::from([(
                "session-current".to_string(),
                SessionStateRecord {
                    session_id: "session-current".to_string(),
                    lifecycle: SlashSessionLifecycle::Active,
                    latest_tldr_snapshot_id: Some("missing-tldr".to_string()),
                    latest_compact_snapshot_id: Some("other-session-compact".to_string()),
                    pending_hydration_snapshot_id: Some("wrong-kind".to_string()),
                    suspended_at: None,
                    updated_at: "updated-at".to_string(),
                },
            )])),
            snapshots: Mutex::new(vec![
                compact_snapshot("other-session", "other-session-compact"),
                tldr_snapshot("session-current", "wrong-kind"),
            ]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let result = expect_success(service.handle_session(&context(None), "inspect").await);

        assert!(result
            .message
            .contains("referenced snapshot missing-tldr is missing from authoritative storage"));
        assert!(result
            .message
            .contains("referenced snapshot other-session-compact belongs to a different session"));
        assert!(result
            .message
            .contains("referenced snapshot wrong-kind has unexpected kind for pending hydration"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionInspect { ref inspect }
                if inspect.current_session_known
                    && inspect.state.is_some()
                    && inspect.snapshots.latest_tldr.reference_id.as_deref() == Some("missing-tldr")
                    && inspect.snapshots.latest_tldr.snapshot.is_none()
                    && inspect.snapshots.latest_compact.reference_id.as_deref() == Some("other-session-compact")
                    && inspect.snapshots.latest_compact.snapshot.is_none()
                    && inspect.snapshots.pending_hydration.reference_id.as_deref() == Some("wrong-kind")
                    && inspect.snapshots.pending_hydration.snapshot.is_none()
                    && inspect.gaps.iter().any(|gap| gap.code == SessionCommandInspectGapCode::ReferencedSnapshotMissing && gap.reference_id.as_deref() == Some("missing-tldr"))
                    && inspect.gaps.iter().any(|gap| gap.code == SessionCommandInspectGapCode::ReferencedSnapshotOwnershipMismatch && gap.reference_id.as_deref() == Some("other-session-compact"))
                    && inspect.gaps.iter().any(|gap| gap.code == SessionCommandInspectGapCode::ReferencedSnapshotKindMismatch && gap.reference_id.as_deref() == Some("wrong-kind"))
        ));
    }

    #[tokio::test]
    async fn session_inspect_reports_unknown_current_session_without_inventing_state() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);

        let missing = CommandContext::for_cli(
            "session-missing",
            CommandSessionSource::Existing,
            ExecutionMode::Standard,
            None,
        );
        let result = expect_success(service.handle_session(&missing, "inspect").await);

        assert!(result
            .message
            .contains("current session is unknown to slash-session state"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::SessionInspect { ref inspect }
                if inspect.session_id == "session-missing"
                    && !inspect.current_session_known
                    && inspect.session.is_none()
                    && inspect.state.is_none()
                    && inspect.gaps.is_empty()
        ));
    }

    #[tokio::test]
    async fn session_inspect_requires_sqlite_backend_only_for_inspect_branch() {
        let memory = FakeMemory {
            backend: "markdown",
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let help = expect_success(service.handle_session(&context(None), "").await);
        let failure = expect_failure(service.handle_session(&context(None), "inspect").await);

        assert!(matches!(
            help.data,
            SessionCommandSuccessData::SessionHelp { .. }
        ));
        assert_eq!(failure.kind, SessionCommandFailureKind::UnsupportedBackend);
    }

    #[tokio::test]
    async fn session_inspect_rejects_extra_tokens_after_supported_subcommand() {
        let memory = FakeMemory {
            sessions: HashMap::from([(
                "session-current".to_string(),
                SessionEntry {
                    id: "session-current".into(),
                    ..active_session()
                },
            )]),
            ..Default::default()
        };
        let service = SessionCommandService::new(&memory);

        let failure = expect_failure(
            service
                .handle_session(&context(None), "inspect extra")
                .await,
        );

        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    // --- handle_model ---

    #[test]
    fn model_no_args_returns_current_placeholder() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_model("sess-1", ""));
        assert_eq!(result.command, "/model");
        assert!(result.message.contains("not set"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::ModelInfo { ref current, ref available }
            if current.is_empty() && available.is_empty()
        ));
    }

    #[test]
    fn model_with_args_sets_model_name() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_model("sess-1", "gpt-4o"));
        assert_eq!(result.command, "/model");
        assert!(result.message.contains("gpt-4o"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::ModelInfo { ref current, .. }
            if current == "gpt-4o"
        ));
    }

    // --- handle_provider ---

    #[test]
    fn provider_no_args_returns_current_placeholder() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_provider("sess-1", ""));
        assert_eq!(result.command, "/provider");
        assert!(result.message.contains("not set"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::ProviderInfo { ref current, .. }
            if current.is_empty()
        ));
    }

    #[test]
    fn provider_with_args_sets_provider_name() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_provider("sess-1", "openai"));
        assert!(result.message.contains("openai"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::ProviderInfo { ref current, .. }
            if current == "openai"
        ));
    }

    // --- handle_temperature ---

    #[test]
    fn temperature_returns_default_value() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_temperature("sess-1"));
        assert_eq!(result.command, "/temperature");
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::TemperatureInfo { current }
            if (current - 0.7_f32).abs() < f32::EPSILON
        ));
    }

    // --- handle_mcp ---

    #[test]
    fn mcp_list_returns_empty_server_list() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_mcp("sess-1", "list"));
        assert_eq!(result.command, "/mcp");
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::McpList { ref servers }
            if servers.is_empty()
        ));
    }

    #[test]
    fn mcp_add_with_name_succeeds() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_mcp("sess-1", "add my-server"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::McpAdded { ref server }
            if server == "my-server"
        ));
    }

    #[test]
    fn mcp_add_without_name_fails() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let failure = expect_failure_sync(service.handle_mcp("sess-1", "add"));
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    #[test]
    fn mcp_remove_with_name_succeeds() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_mcp("sess-1", "remove my-server"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::McpRemoved { ref server }
            if server == "my-server"
        ));
    }

    #[test]
    fn mcp_remove_without_name_fails() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let failure = expect_failure_sync(service.handle_mcp("sess-1", "remove"));
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    #[test]
    fn mcp_unknown_subcommand_fails() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let failure = expect_failure_sync(service.handle_mcp("sess-1", "bogus"));
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
        assert!(failure.message.contains("bogus"));
    }

    // --- handle_tool_manage ---

    #[test]
    fn tool_enable_with_name_succeeds() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_tool_manage("sess-1", "enable shell"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::ToolEnabled { ref name }
            if name == "shell"
        ));
    }

    #[test]
    fn tool_enable_without_name_fails() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let failure = expect_failure_sync(service.handle_tool_manage("sess-1", "enable"));
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    #[test]
    fn tool_disable_with_name_succeeds() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let result = expect_success_sync(service.handle_tool_manage("sess-1", "disable shell"));
        assert!(matches!(
            result.data,
            SessionCommandSuccessData::ToolDisabled { ref name }
            if name == "shell"
        ));
    }

    #[test]
    fn tool_disable_without_name_fails() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let failure = expect_failure_sync(service.handle_tool_manage("sess-1", "disable"));
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
    }

    #[test]
    fn tool_unknown_subcommand_fails() {
        let memory = FakeMemory::default();
        let service = SessionCommandService::new(&memory);
        let failure = expect_failure_sync(service.handle_tool_manage("sess-1", "toggle shell"));
        assert_eq!(failure.kind, SessionCommandFailureKind::InvalidArguments);
        assert!(failure.message.contains("toggle"));
    }

    // --- sync helpers for non-async handlers ---

    fn expect_success_sync(outcome: SessionCommandOutcome) -> SessionCommandSuccess {
        match outcome {
            SessionCommandOutcome::Success(s) => s,
            SessionCommandOutcome::Failure(f) => {
                panic!("expected success but got failure: {:?}", f)
            }
        }
    }

    fn expect_failure_sync(outcome: SessionCommandOutcome) -> SessionCommandFailure {
        match outcome {
            SessionCommandOutcome::Failure(f) => f,
            SessionCommandOutcome::Success(s) => {
                panic!("expected failure but got success: {:?}", s)
            }
        }
    }
}
