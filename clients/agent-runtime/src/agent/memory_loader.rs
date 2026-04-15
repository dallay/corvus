use crate::config::MemoryCerebroConfig;
use crate::memory::{is_slash_session_unsupported_error, Memory, MemoryEntry, SessionSnapshotKind};
use crate::security::egress::enforce_cerebro_egress;
use crate::security::policy::ToolOperation;
use crate::tools::mcp::{cerebro, normalize};
use crate::tools::traits::Tool;
use anyhow::Context as _;
use async_trait::async_trait;
use serde_json::json;
use std::fmt::Write;

#[async_trait]
pub trait MemoryLoader: Send + Sync {
    async fn load_context(
        &self,
        memory: &dyn Memory,
        user_message: &str,
        session_id: Option<&str>,
    ) -> anyhow::Result<String>;
}

pub struct DefaultMemoryLoader {
    limit: usize,
    min_relevance_score: f64,
}

pub struct CerebroMemoryLoader {
    config: MemoryCerebroConfig,
    limit: usize,
    min_relevance_score: f64,
}

impl Default for DefaultMemoryLoader {
    fn default() -> Self {
        Self {
            limit: 5,
            min_relevance_score: 0.4,
        }
    }
}

impl DefaultMemoryLoader {
    pub fn new(limit: usize, min_relevance_score: f64) -> Self {
        Self {
            limit: limit.max(1),
            min_relevance_score,
        }
    }
}

impl CerebroMemoryLoader {
    pub fn new(config: MemoryCerebroConfig, limit: usize, min_relevance_score: f64) -> Self {
        Self {
            config,
            limit: limit.max(1),
            min_relevance_score,
        }
    }
}

#[async_trait]
impl MemoryLoader for DefaultMemoryLoader {
    async fn load_context(
        &self,
        memory: &dyn Memory,
        user_message: &str,
        session_id: Option<&str>,
    ) -> anyhow::Result<String> {
        // Call recall first - if it fails, don't consume the one-shot resume context
        let entries = memory.recall(user_message, self.limit, session_id).await?;

        // Now try to append pending resume context (safe to call after recall succeeded)
        let mut context = String::new();
        let added_resume = append_pending_resume_context(&mut context, memory, session_id).await?;

        let added = append_local_entries(&mut context, &entries, self.min_relevance_score);
        if !added && !added_resume {
            return Ok(String::new());
        }

        context.push('\n');
        Ok(context)
    }
}

#[async_trait]
impl MemoryLoader for CerebroMemoryLoader {
    async fn load_context(
        &self,
        memory: &dyn Memory,
        user_message: &str,
        session_id: Option<&str>,
    ) -> anyhow::Result<String> {
        // Call recall first - if it fails, don't consume the one-shot resume context
        let entries = memory.recall(user_message, self.limit, session_id).await?;

        // Now append pending resume context after recall succeeded
        let mut context = String::new();
        let added_resume = append_pending_resume_context(&mut context, memory, session_id).await?;

        let mut added = append_local_entries(&mut context, &entries, self.min_relevance_score);
        added = added || added_resume;

        let endpoint = self
            .config
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if endpoint.is_none() {
            if added {
                context.push('\n');
                return Ok(context);
            }
            anyhow::bail!("Cerebro MCP endpoint is not configured");
        }

        if let Some(endpoint) = endpoint {
            if enforce_cerebro_egress(endpoint, &self.config, ToolOperation::Read).is_err() {
                return Ok(finalize_context(context, added));
            }
        }

        if session_id.is_some() {
            tracing::warn!(
                "Skipping Cerebro remote recall for session-scoped turn until mem_search supports session filtering"
            );
            return Ok(finalize_context(context, added));
        }

        let results = match fetch_cerebro_results(&self.config, user_message, self.limit).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "Cerebro remote recall failed, using local context");
                if added || !context.is_empty() {
                    return Ok(finalize_context(context, added));
                }
                return Err(e);
            }
        };
        added = append_cerebro_results(&mut context, &results, self.min_relevance_score) || added;

        if !added {
            return Ok(String::new());
        }

        context.push('\n');
        Ok(context)
    }
}

fn append_local_entries(
    context: &mut String,
    entries: &[MemoryEntry],
    min_relevance_score: f64,
) -> bool {
    let mut added = false;
    for entry in entries {
        if let Some(score) = entry.score {
            if score < min_relevance_score {
                continue;
            }
        }
        if context.is_empty() {
            context.push_str("[Memory context]\n");
        }
        let _ = writeln!(context, "- {}: {}", entry.key, entry.content);
        added = true;
    }
    added
}

async fn append_pending_resume_context(
    context: &mut String,
    memory: &dyn Memory,
    session_id: Option<&str>,
) -> anyhow::Result<bool> {
    let Some(session_id) = session_id else {
        return Ok(false);
    };
    let pending = match memory.take_pending_resume_hydration(session_id).await {
        Ok(snapshot) => snapshot,
        Err(error) if is_slash_session_unsupported_error(&error) => return Ok(false),
        Err(error) => {
            return Err(error).context(format!(
                "pending resume hydration failed for session {session_id}"
            ))
        }
    };
    let Some(snapshot) = pending else {
        return Ok(false);
    };
    if snapshot.kind != SessionSnapshotKind::Compact {
        anyhow::bail!("pending resume hydration must use a compact snapshot");
    }
    let summary = snapshot
        .payload
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("No summary available.");
    let resume_context = snapshot
        .payload
        .get("resume_context")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("compact snapshot missing resume_context"))?;
    let _ = writeln!(context, "[Resumed session context]");
    let _ = writeln!(context, "- Snapshot: compact");
    let _ = writeln!(context, "- Session: {}", snapshot.session_id);
    let _ = writeln!(context, "- Summary: {summary}");
    let _ = writeln!(context, "- Resume context: {resume_context}");
    Ok(true)
}

/// Finalize a context string: append newline if entries were added, else return empty.
fn finalize_context(mut context: String, added: bool) -> String {
    if added {
        context.push('\n');
    }
    context
}

/// Call the Cerebro MCP adapter and return the parsed results array.
async fn fetch_cerebro_results(
    config: &MemoryCerebroConfig,
    user_message: &str,
    limit: usize,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let adapter = cerebro::cerebro_tool_adapter(config, normalize::CEREBRO_TOOL_RECALL)?;
    let payload = json!({
        "input": {
            "query": user_message,
            "limit": limit
        }
    });
    let response = adapter.execute(payload).await?;
    if !response.success {
        let message = response
            .error
            .unwrap_or_else(|| "Cerebro mem_search failed".to_string());
        anyhow::bail!(message);
    }

    let mut value: serde_json::Value = serde_json::from_str(&response.output)?;
    let results = value
        .get_mut("results")
        .and_then(serde_json::Value::as_array_mut)
        .map(std::mem::take)
        .ok_or_else(|| anyhow::anyhow!("Cerebro response missing results"))?;
    Ok(results)
}

/// Append Cerebro remote recall results to the context string.
fn append_cerebro_results(
    context: &mut String,
    results: &[serde_json::Value],
    min_relevance_score: f64,
) -> bool {
    let mut added = false;
    for entry in results {
        let score = entry.get("score").and_then(serde_json::Value::as_f64);
        if let Some(score) = score {
            if score < min_relevance_score {
                continue;
            }
        }
        let key = entry
            .get("topic_key")
            .or_else(|| entry.get("memory_id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("memory");
        let summary = entry
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(summary) = summary {
            if context.is_empty() {
                context.push_str("[Memory context]\n");
            }
            let _ = writeln!(context, "- {key}: {summary}");
            added = true;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};

    struct MockMemory;

    #[derive(Default)]
    struct SessionTrackingMemory {
        recall_sessions: std::sync::Mutex<Vec<Option<String>>>,
        pending_snapshot: std::sync::Mutex<Option<crate::memory::SessionSnapshotRecord>>,
    }

    #[async_trait]
    impl Memory for MockMemory {
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
            limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            if limit == 0 {
                return Ok(vec![]);
            }
            Ok(vec![MemoryEntry {
                id: "1".into(),
                key: "k".into(),
                content: "v".into(),
                category: MemoryCategory::Conversation,
                timestamp: "now".into(),
                session_id: None,
                score: None,
            }])
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(vec![])
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(true)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[async_trait]
    impl Memory for SessionTrackingMemory {
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
            limit: usize,
            session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            self.recall_sessions
                .lock()
                .unwrap()
                .push(session_id.map(str::to_string));
            if limit == 0 {
                return Ok(vec![]);
            }
            Ok(vec![MemoryEntry {
                id: "session-entry".into(),
                key: "session-key".into(),
                content: "session scoped memory".into(),
                category: MemoryCategory::Conversation,
                timestamp: "now".into(),
                session_id: session_id.map(str::to_string),
                score: Some(1.0),
            }])
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(vec![])
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(true)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "session-tracking"
        }

        async fn take_pending_resume_hydration(
            &self,
            _session_id: &str,
        ) -> anyhow::Result<Option<crate::memory::SessionSnapshotRecord>> {
            Ok(self.pending_snapshot.lock().unwrap().take())
        }
    }

    #[tokio::test]
    async fn default_loader_formats_context() {
        let loader = DefaultMemoryLoader::default();
        let context = loader
            .load_context(&MockMemory, "hello", None)
            .await
            .unwrap();
        assert!(context.contains("[Memory context]"));
        assert!(context.contains("- k: v"));
    }

    #[tokio::test]
    async fn cerebro_loader_returns_local_context_when_endpoint_missing() {
        let loader = CerebroMemoryLoader::new(MemoryCerebroConfig::default(), 5, 0.4);
        let context = loader
            .load_context(&MockMemory, "hello", None)
            .await
            .unwrap();
        assert!(context.contains("[Memory context]"));
        assert!(context.contains("- k: v"));
    }

    #[tokio::test]
    async fn cerebro_loader_returns_local_context_when_egress_is_blocked() {
        let loader = CerebroMemoryLoader::new(
            MemoryCerebroConfig {
                endpoint: Some("http://public.example.com/mcp".to_string()),
                auth_token: None,
                request_timeout_ms: 30_000,
                allow_insecure_loopback: false,
            },
            5,
            0.4,
        );

        let context = loader
            .load_context(&MockMemory, "hello", None)
            .await
            .unwrap();
        assert!(context.contains("[Memory context]"));
        assert!(context.contains("- k: v"));
    }

    #[tokio::test]
    async fn default_loader_uses_explicit_session_scope() {
        let loader = DefaultMemoryLoader::default();
        let memory = SessionTrackingMemory::default();

        let _ = loader
            .load_context(&memory, "hello", Some("webhook-session-1"))
            .await
            .unwrap();

        assert_eq!(
            memory.recall_sessions.lock().unwrap().clone(),
            vec![Some("webhook-session-1".to_string())]
        );
    }

    #[test]
    fn append_cerebro_results_filters_empty_summaries() {
        let results = vec![
            serde_json::json!({"topic_key": "k1", "summary": "valid summary", "score": 0.9}),
            serde_json::json!({"topic_key": "k2", "summary": "", "score": 0.9}),
            serde_json::json!({"topic_key": "k3", "score": 0.9}),
        ];
        let mut context = String::new();
        let added = append_cerebro_results(&mut context, &results, 0.0);
        assert!(added);
        assert!(context.contains("k1"));
        assert!(context.contains("valid summary"));
        assert!(!context.contains("k2"));
        assert!(!context.contains("k3"));
    }

    #[test]
    fn append_cerebro_results_filters_low_scores() {
        let results = vec![
            serde_json::json!({"topic_key": "k1", "summary": "high", "score": 0.9}),
            serde_json::json!({"topic_key": "k2", "summary": "low", "score": 0.1}),
        ];
        let mut context = String::new();
        let added = append_cerebro_results(&mut context, &results, 0.5);
        assert!(added);
        assert!(context.contains("high"));
        assert!(!context.contains("low"));
    }

    #[test]
    fn append_cerebro_results_empty_returns_false() {
        let mut context = String::new();
        let added = append_cerebro_results(&mut context, &[], 0.0);
        assert!(!added);
        assert!(context.is_empty());
    }

    #[test]
    fn finalize_context_appends_newline_when_added() {
        let result = finalize_context("some content".to_string(), true);
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn finalize_context_returns_as_is_when_not_added() {
        let result = finalize_context("leftover".to_string(), false);
        assert_eq!(result, "leftover");
    }

    #[tokio::test]
    async fn cerebro_loader_uses_explicit_session_scope_for_local_fallback() {
        let loader = CerebroMemoryLoader::new(MemoryCerebroConfig::default(), 5, 0.4);
        let memory = SessionTrackingMemory::default();
        memory
            .store(
                "local",
                "hello from local memory",
                MemoryCategory::Conversation,
                Some("webhook-session-1"),
            )
            .await
            .unwrap();

        let _ = loader
            .load_context(&memory, "hello", Some("webhook-session-1"))
            .await
            .unwrap();

        assert_eq!(
            memory.recall_sessions.lock().unwrap().clone(),
            vec![Some("webhook-session-1".to_string())]
        );
    }

    #[tokio::test]
    async fn default_loader_prepends_persisted_resume_context_once() {
        let loader = DefaultMemoryLoader::default();
        let memory = SessionTrackingMemory::default();
        *memory.pending_snapshot.lock().unwrap() = Some(crate::memory::SessionSnapshotRecord {
            id: "snapshot-1".into(),
            session_id: "webhook-session-1".into(),
            kind: SessionSnapshotKind::Compact,
            created_at: "now".into(),
            payload: serde_json::json!({
                "summary": "Discuss release checklist",
                "resume_context": "Pick up from the last checklist item",
            }),
            resume_capable: true,
        });

        let first = loader
            .load_context(&memory, "hello", Some("webhook-session-1"))
            .await
            .unwrap();
        let second = loader
            .load_context(&memory, "hello", Some("webhook-session-1"))
            .await
            .unwrap();

        assert!(first.contains("[Resumed session context]"));
        assert!(first.contains("Pick up from the last checklist item"));
        assert!(!second.contains("[Resumed session context]"));
    }
}
