use crate::config::MemoryCerebroConfig;
use crate::memory::{Memory, MemoryEntry};
use crate::security::egress::enforce_cerebro_egress;
use crate::security::policy::ToolOperation;
use crate::tools::mcp::{cerebro, normalize};
use crate::tools::traits::Tool;
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
        let entries = memory.recall(user_message, self.limit, session_id).await?;
        let mut context = String::new();
        let added = append_local_entries(&mut context, &entries, self.min_relevance_score);
        if !added {
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
        let entries = memory.recall(user_message, self.limit, session_id).await?;
        let mut context = String::new();
        let mut added = append_local_entries(&mut context, &entries, self.min_relevance_score);

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
                if added {
                    context.push('\n');
                    return Ok(context);
                }
                return Ok(String::new());
            }
        }

        let adapter = cerebro::cerebro_tool_adapter(&self.config, normalize::CEREBRO_TOOL_RECALL)?;
        // Cerebro's current mem_search contract does not accept a session filter, so remote recall
        // remains global even when the local memory fallback is session-scoped.
        let payload = json!({
            "input": {
                "query": user_message,
                "limit": self.limit
            }
        });
        let response = adapter.execute(payload).await?;
        if !response.success {
            let message = response
                .error
                .unwrap_or_else(|| "Cerebro mem_search failed".to_string());
            anyhow::bail!(message);
        }

        let value: serde_json::Value = serde_json::from_str(&response.output)?;
        let results = value
            .get("results")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Cerebro response missing results"))?;

        if !results.is_empty() {
            for entry in results {
                let score = entry.get("score").and_then(serde_json::Value::as_f64);
                if let Some(score) = score {
                    if score < self.min_relevance_score {
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
                    .unwrap_or_default();
                if context.is_empty() {
                    context.push_str("[Memory context]\n");
                }
                let _ = writeln!(context, "- {key}: {summary}");
                added = true;
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};

    struct MockMemory;

    #[derive(Default)]
    struct SessionTrackingMemory {
        recall_sessions: std::sync::Mutex<Vec<Option<String>>>,
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
            _limit: usize,
            session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            self.recall_sessions
                .lock()
                .unwrap()
                .push(session_id.map(str::to_string));
            Ok(vec![])
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

    #[tokio::test]
    async fn cerebro_loader_uses_explicit_session_scope_for_local_fallback() {
        let loader = CerebroMemoryLoader::new(
            MemoryCerebroConfig {
                endpoint: Some("http://127.0.0.1:7777/mcp".to_string()),
                auth_token: None,
                request_timeout_ms: 1_000,
                allow_insecure_loopback: false,
            },
            5,
            0.4,
        );
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
}
