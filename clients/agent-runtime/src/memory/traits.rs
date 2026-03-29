use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single memory entry
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

/// A tracked session entry
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

/// Aggregated memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: u64,
    pub by_category: HashMap<String, u64>,
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub backend: String,
    pub cerebro_configured: bool,
}

/// Memory categories for organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// Long-term facts, preferences, decisions
    Core,
    /// Daily session logs
    Daily,
    /// Conversation context
    Conversation,
    /// User-defined custom category
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
///
/// Default implementations return `Ok(empty)` so that non-SQLite backends
/// (e.g. markdown-only) work without overriding every session method.
#[async_trait]
pub trait Memory: Send + Sync {
    /// Backend name
    fn name(&self) -> &str;

    /// Store a memory entry, optionally scoped to a session
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Recall memories matching a query (keyword search), optionally scoped to a session
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Get a specific memory by key
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;

    /// List all memory keys, optionally filtered by category and/or session
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Remove a memory by key
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;

    /// Count total memories
    async fn count(&self) -> anyhow::Result<usize>;

    /// Health check
    async fn health_check(&self) -> bool;

    /// Validate an AI response against memory-backed domain rules.
    ///
    /// Backends that do not provide ontology/rule validation should rely on
    /// the default permissive implementation.
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
    /// When a token hash is present, implementations should preserve token-scoped ownership.
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn memory_category_serde_uses_snake_case() {
        let core = serde_json::to_string(&MemoryCategory::Core).unwrap();
        let daily = serde_json::to_string(&MemoryCategory::Daily).unwrap();
        let conversation = serde_json::to_string(&MemoryCategory::Conversation).unwrap();

        assert_eq!(core, "\"core\"");
        assert_eq!(daily, "\"daily\"");
        assert_eq!(conversation, "\"conversation\"");
    }

    #[test]
    fn session_entry_serde_roundtrip() {
        let entry = SessionEntry {
            id: "sess-1".into(),
            started_at: "2026-03-28T10:00:00Z".into(),
            ended_at: None,
            status: SessionStatus::Active,
            message_count: 5,
            last_activity: "2026-03-28T10:05:00Z".into(),
            metadata: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "sess-1");
        assert_eq!(parsed.status, SessionStatus::Active);
        assert_eq!(parsed.message_count, 5);
        assert!(parsed.ended_at.is_none());
        assert!(parsed.metadata.is_none());
    }

    #[test]
    fn session_entry_with_ended_at_and_metadata() {
        let entry = SessionEntry {
            id: "sess-2".into(),
            started_at: "2026-03-28T10:00:00Z".into(),
            ended_at: Some("2026-03-28T11:00:00Z".into()),
            status: SessionStatus::Ended,
            message_count: 10,
            last_activity: "2026-03-28T10:55:00Z".into(),
            metadata: Some(serde_json::json!({"source": "web"})),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.ended_at.as_deref(), Some("2026-03-28T11:00:00Z"));
        assert_eq!(parsed.status, SessionStatus::Ended);
        assert!(parsed.metadata.is_some());
    }

    #[test]
    fn memory_stats_default_is_empty() {
        let stats = MemoryStats::default();
        assert_eq!(stats.total_entries, 0);
        assert!(stats.by_category.is_empty());
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
        assert!(stats.backend.is_empty());
        assert!(!stats.cerebro_configured);
    }

    #[test]
    fn memory_stats_serde_roundtrip() {
        let mut by_cat = HashMap::new();
        by_cat.insert("core".to_string(), 10);
        by_cat.insert("daily".to_string(), 5);

        let stats = MemoryStats {
            total_entries: 15,
            by_category: by_cat,
            total_sessions: 3,
            active_sessions: 1,
            backend: "sqlite".into(),
            cerebro_configured: true,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: MemoryStats = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total_entries, 15);
        assert_eq!(parsed.by_category.get("core"), Some(&10));
        assert_eq!(parsed.total_sessions, 3);
        assert_eq!(parsed.active_sessions, 1);
        assert_eq!(parsed.backend, "sqlite");
        assert!(parsed.cerebro_configured);
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

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "id-1");
        assert_eq!(parsed.key, "favorite_language");
        assert_eq!(parsed.content, "Rust");
        assert_eq!(parsed.category, MemoryCategory::Core);
        assert_eq!(parsed.session_id.as_deref(), Some("session-abc"));
        assert_eq!(parsed.score, Some(0.98));
    }
}
