use crate::errors::CerebroError;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub memory_id: String,
    pub scope: String,
    pub topic_key: String,
    pub observation: Value,
    pub summary: String,
    pub deleted: bool,
    pub timestamp: String,
}

impl MemoryRecord {
    pub fn new(memory_id: String, scope: String, topic_key: String, observation: Value) -> Self {
        let summary = observation
            .get("content")
            .and_then(Value::as_str)
            .map(truncate_summary)
            .unwrap_or_default();
        let timestamp = Utc::now().to_rfc3339();
        Self {
            memory_id,
            scope,
            topic_key,
            observation,
            summary,
            deleted: false,
            timestamp,
        }
    }
}

fn truncate_summary(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.len() <= 160 {
        return trimmed.to_string();
    }
    let mut end = 160;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &trimmed[..end])
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn save(&self, record: MemoryRecord) -> Result<(), CerebroError>;
    async fn get(&self, memory_id: &str) -> Result<Option<MemoryRecord>, CerebroError>;
    async fn delete(&self, memory_id: &str, hard_delete: bool) -> Result<bool, CerebroError>;
    async fn search(
        &self,
        query: &str,
        limit: usize,
        include_deleted: bool,
        scope: Option<&str>,
        topic_key: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, CerebroError>;
    async fn count(&self) -> Result<usize, CerebroError>;
}

#[derive(Debug, Default)]
pub struct InMemoryStorage {
    records: RwLock<HashMap<String, MemoryRecord>>,
}

impl InMemoryStorage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            records: RwLock::new(HashMap::new()),
        })
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn save(&self, record: MemoryRecord) -> Result<(), CerebroError> {
        let mut map = self.records.write().await;
        map.insert(record.memory_id.clone(), record);
        Ok(())
    }

    async fn get(&self, memory_id: &str) -> Result<Option<MemoryRecord>, CerebroError> {
        let map = self.records.read().await;
        Ok(map.get(memory_id).cloned())
    }

    async fn delete(&self, memory_id: &str, hard_delete: bool) -> Result<bool, CerebroError> {
        let mut map = self.records.write().await;
        if hard_delete {
            return Ok(map.remove(memory_id).is_some());
        }

        if let Some(entry) = map.get_mut(memory_id) {
            entry.deleted = true;
            return Ok(true);
        }

        Ok(false)
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
        include_deleted: bool,
        scope: Option<&str>,
        topic_key: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        let map = self.records.read().await;
        let query = query.to_ascii_lowercase();
        let mut results: Vec<MemoryRecord> = map
            .values()
            .filter(|record| {
                if !include_deleted && record.deleted {
                    return false;
                }
                if let Some(scope) = scope {
                    if record.scope != scope {
                        return false;
                    }
                }
                if let Some(topic_key) = topic_key {
                    if record.topic_key != topic_key {
                        return false;
                    }
                }
                let haystack =
                    format!("{} {}", record.summary, record.topic_key).to_ascii_lowercase();
                haystack.contains(&query)
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results.truncate(limit);
        Ok(results)
    }

    async fn count(&self) -> Result<usize, CerebroError> {
        let map = self.records.read().await;
        Ok(map.len())
    }
}
