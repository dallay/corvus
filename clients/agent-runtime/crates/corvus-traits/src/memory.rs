use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn slash_session_unsupported_error(backend: &str) -> anyhow::Error {
    anyhow::anyhow!("slash-session commands require sqlite memory backend (backend={backend})")
}

/// A single memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryValidationResult {
    pub valid: bool,
    #[serde(default)]
    pub violations: Vec<String>,
}

impl Default for MemoryValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
        }
    }
}

/// Session lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Ended,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Ended => "ended",
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "ended" => Ok(Self::Ended),
            other => Err(anyhow::anyhow!("unknown session status: {other}")),
        }
    }
}

/// A tracked session entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: SessionStatus,
    pub message_count: u32,
    pub last_activity: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlashSessionLifecycle {
    Active,
    Suspended,
}

impl SlashSessionLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }
}

impl std::str::FromStr for SlashSessionLifecycle {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            other => Err(anyhow::anyhow!("unknown slash session lifecycle: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSnapshotKind {
    Tldr,
    Compact,
}

impl SessionSnapshotKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tldr => "tldr",
            Self::Compact => "compact",
        }
    }
}

impl std::str::FromStr for SessionSnapshotKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tldr" => Ok(Self::Tldr),
            "compact" => Ok(Self::Compact),
            other => Err(anyhow::anyhow!("unknown session snapshot kind: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshotRecord {
    pub id: String,
    pub session_id: String,
    pub kind: SessionSnapshotKind,
    pub created_at: String,
    pub payload: serde_json::Value,
    pub resume_capable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStateRecord {
    pub session_id: String,
    pub lifecycle: SlashSessionLifecycle,
    pub latest_tldr_snapshot_id: Option<String>,
    pub latest_compact_snapshot_id: Option<String>,
    pub pending_hydration_snapshot_id: Option<String>,
    pub suspended_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStateMutation {
    pub session_id: String,
    pub lifecycle: SlashSessionLifecycle,
    pub latest_tldr_snapshot_id: Option<String>,
    pub latest_compact_snapshot_id: Option<String>,
    pub pending_hydration_snapshot_id: Option<String>,
    pub suspended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumableSessionEntry {
    pub session_id: String,
    pub started_at: String,
    pub last_activity: String,
    pub snapshot_id: String,
    pub snapshot_created_at: String,
    pub preview: String,
}

/// Aggregated memory statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: u64,
    pub by_category: HashMap<String, u64>,
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub backend: String,
    pub cerebro_configured: bool,
}

/// Memory categories for organization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// Long-term facts, preferences, decisions.
    Core,
    /// Daily session logs.
    Daily,
    /// Conversation context.
    Conversation,
    /// User-defined custom category.
    Custom(String),
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Daily => write!(f, "daily"),
            Self::Conversation => write!(f, "conversation"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Core memory trait — local short-term memory backends only.
/// Long-term memory is routed through Cerebro MCP tools.
#[async_trait]
pub trait Memory: Send + Sync {
    /// Backend name.
    fn name(&self) -> &str;

    /// Store a memory entry, optionally scoped to a session.
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Recall memories matching a query (keyword search), optionally scoped to a session.
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Get a specific memory by key.
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;

    /// List all memory keys, optionally filtered by category and/or session.
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Remove a memory by key.
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;

    /// Count total memories.
    async fn count(&self) -> anyhow::Result<usize>;

    /// Health check.
    async fn health_check(&self) -> bool;

    /// Validate an AI response against memory-backed domain rules.
    async fn validate_response(
        &self,
        _user_query: &str,
        _response: &str,
        _session_id: Option<&str>,
    ) -> anyhow::Result<MemoryValidationResult> {
        Ok(MemoryValidationResult::default())
    }

    /// Create or touch a session record (idempotent).
    async fn upsert_session(
        &self,
        _session_id: &str,
        _token_hash: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Mark a session as ended (idempotent — no-op if already ended).
    async fn end_session(&self, _session_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// Increment message count and update last_activity for an active session.
    async fn update_session_activity(
        &self,
        _session_id: &str,
        _token_hash: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// List sessions with optional status filter, pagination, sort, and order.
    async fn list_sessions(
        &self,
        _status: Option<SessionStatus>,
        _limit: u32,
        _offset: u32,
        _sort: &str,
        _order: &str,
    ) -> anyhow::Result<(Vec<SessionEntry>, u64)> {
        Ok((vec![], 0))
    }

    /// Get a single session by ID.
    async fn get_session(&self, _session_id: &str) -> anyhow::Result<Option<SessionEntry>> {
        Ok(None)
    }

    /// List sessions scoped to a specific token hash, with pagination.
    async fn list_sessions_for_token(
        &self,
        _token_hash: &str,
        _limit: u32,
        _offset: u32,
    ) -> anyhow::Result<(Vec<SessionEntry>, u64)> {
        Ok((vec![], 0))
    }

    /// Return aggregated memory and session statistics.
    async fn memory_stats(&self) -> anyhow::Result<MemoryStats> {
        Ok(MemoryStats::default())
    }

    async fn load_session_transcript_excerpt(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Err(slash_session_unsupported_error(self.name()))
    }

    async fn create_session_snapshot(
        &self,
        _session_id: &str,
        _kind: SessionSnapshotKind,
        _payload: serde_json::Value,
        _resume_capable: bool,
    ) -> anyhow::Result<SessionSnapshotRecord> {
        Err(slash_session_unsupported_error(self.name()))
    }

    async fn get_session_snapshot(
        &self,
        _snapshot_id: &str,
    ) -> anyhow::Result<Option<SessionSnapshotRecord>> {
        Err(slash_session_unsupported_error(self.name()))
    }

    async fn get_session_state_record(
        &self,
        _session_id: &str,
    ) -> anyhow::Result<Option<SessionStateRecord>> {
        Err(slash_session_unsupported_error(self.name()))
    }

    async fn update_session_state_record(
        &self,
        _state: SessionStateMutation,
    ) -> anyhow::Result<SessionStateRecord> {
        Err(slash_session_unsupported_error(self.name()))
    }

    async fn list_resumable_sessions(
        &self,
        _limit: u32,
        _offset: u32,
    ) -> anyhow::Result<Vec<ResumableSessionEntry>> {
        Err(slash_session_unsupported_error(self.name()))
    }

    async fn take_pending_resume_hydration(
        &self,
        _session_id: &str,
    ) -> anyhow::Result<Option<SessionSnapshotRecord>> {
        Err(slash_session_unsupported_error(self.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalMemory;

    #[async_trait]
    impl Memory for MinimalMemory {
        fn name(&self) -> &str {
            "minimal"
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
    }

    #[test]
    fn memory_category_display_outputs_expected_values() {
        assert_eq!(MemoryCategory::Core.to_string(), "core");
        assert_eq!(MemoryCategory::Daily.to_string(), "daily");
        assert_eq!(MemoryCategory::Conversation.to_string(), "conversation");
        assert_eq!(
            MemoryCategory::Custom("project_notes".into()).to_string(),
            "project_notes"
        );
    }

    #[test]
    fn session_status_from_str_handles_known_values() {
        assert!(matches!(
            "active".parse::<SessionStatus>(),
            Ok(SessionStatus::Active)
        ));
        assert!(matches!(
            "ended".parse::<SessionStatus>(),
            Ok(SessionStatus::Ended)
        ));
        assert!("other".parse::<SessionStatus>().is_err());
    }

    #[test]
    fn memory_entry_roundtrip_preserves_optional_fields() {
        let entry = MemoryEntry {
            id: "id-1".into(),
            key: "favorite_language".into(),
            content: "Rust".into(),
            category: MemoryCategory::Core,
            timestamp: "2026-02-16T00:00:00Z".into(),
            session_id: Some("session-abc".into()),
            score: Some(0.98),
        };

        let json = serde_json::to_string(&entry);
        assert!(json.is_ok());
        let parsed: Result<MemoryEntry, _> = serde_json::from_str(&json.unwrap_or_default());
        assert!(parsed.is_ok());

        let parsed = parsed.unwrap_or(MemoryEntry {
            id: String::new(),
            key: String::new(),
            content: String::new(),
            category: MemoryCategory::Core,
            timestamp: String::new(),
            session_id: None,
            score: None,
        });
        assert_eq!(parsed.id, "id-1");
        assert_eq!(parsed.session_id.as_deref(), Some("session-abc"));
        assert_eq!(parsed.score, Some(0.98));
    }

    #[tokio::test]
    async fn slash_session_defaults_fail_explicitly_for_non_sqlite_backends() {
        let memory = MinimalMemory;

        let transcript_error = memory
            .load_session_transcript_excerpt("session-1", 5)
            .await
            .unwrap_err();
        assert!(transcript_error.to_string().contains("require sqlite"));

        let snapshot_error = memory
            .create_session_snapshot(
                "session-1",
                SessionSnapshotKind::Compact,
                serde_json::json!({"preview": "hello"}),
                true,
            )
            .await
            .unwrap_err();
        assert!(snapshot_error.to_string().contains("backend=minimal"));

        let state_error = memory
            .update_session_state_record(SessionStateMutation {
                session_id: "session-1".into(),
                lifecycle: SlashSessionLifecycle::Active,
                latest_tldr_snapshot_id: None,
                latest_compact_snapshot_id: None,
                pending_hydration_snapshot_id: None,
                suspended_at: None,
            })
            .await
            .unwrap_err();
        assert!(state_error
            .to_string()
            .contains("slash-session commands require sqlite"));
    }
}
