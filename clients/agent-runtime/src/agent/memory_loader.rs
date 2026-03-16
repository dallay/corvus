use crate::config::MemoryCerebroConfig;
use crate::memory::Memory;
use crate::tools::mcp::{cerebro, normalize};
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::json;
use std::fmt::Write;

#[async_trait]
pub trait MemoryLoader: Send + Sync {
    async fn load_context(&self, memory: &dyn Memory, user_message: &str)
        -> anyhow::Result<String>;
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
    ) -> anyhow::Result<String> {
        let entries = memory.recall(user_message, self.limit, None).await?;
        if entries.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::from("[Memory context]\n");
        for entry in entries {
            if let Some(score) = entry.score {
                if score < self.min_relevance_score {
                    continue;
                }
            }
            let _ = writeln!(context, "- {}: {}", entry.key, entry.content);
        }

        // If all entries were below threshold, return empty
        if context == "[Memory context]\n" {
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
        _memory: &dyn Memory,
        user_message: &str,
    ) -> anyhow::Result<String> {
        let endpoint = self
            .config
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if endpoint.is_none() {
            anyhow::bail!("Cerebro MCP endpoint is missing for memory loader");
        }

        let adapter =
            cerebro::cerebro_tool_adapter(&self.config, normalize::CEREBRO_TOOL_RECALL)?;
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
        let results = match value
            .get("results")
            .and_then(serde_json::Value::as_array)
        {
            Some(results) => results,
            None => return Ok(String::new()),
        };

        if results.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::from("[Memory context]\n");
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
            let _ = writeln!(context, "- {key}: {summary}");
        }

        if context == "[Memory context]\n" {
            return Ok(String::new());
        }

        context.push('\n');
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};

    struct MockMemory;

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

    #[tokio::test]
    async fn default_loader_formats_context() {
        let loader = DefaultMemoryLoader::default();
        let context = loader.load_context(&MockMemory, "hello").await.unwrap();
        assert!(context.contains("[Memory context]"));
        assert!(context.contains("- k: v"));
    }
}
