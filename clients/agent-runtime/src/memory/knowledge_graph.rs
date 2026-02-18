use super::traits::{Memory, MemoryCategory, MemoryEntry};
use anyhow::{anyhow, ensure, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_KEY_LEN: usize = 256;
const MAX_CONTENT_LEN: usize = 8_192;
const MAX_QUERY_LEN: usize = 512;
const DEFAULT_HALF_LIFE_DAYS: f64 = 21.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MemoryKind {
    Episodic,
    Semantic,
    Procedural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityNode {
    id: String,
    label: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationEdge {
    id: String,
    subject_id: String,
    predicate: String,
    object_id: String,
    confidence: f64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphMemory {
    id: String,
    key: String,
    content: String,
    category: MemoryCategory,
    session_id: Option<String>,
    kind: MemoryKind,
    base_relevance: f64,
    access_count: u32,
    created_at: String,
    updated_at: String,
    last_accessed_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct KnowledgeGraphState {
    entities: HashMap<String, EntityNode>,
    relations: HashMap<String, RelationEdge>,
    memories: HashMap<String, GraphMemory>,
}

pub struct KnowledgeGraphMemory {
    state: Mutex<KnowledgeGraphState>,
    state_path: PathBuf,
    half_life_days: f64,
}

impl KnowledgeGraphMemory {
    pub fn new(workspace_dir: &Path) -> anyhow::Result<Self> {
        let state_path = workspace_dir.join("memory").join("knowledge_graph.json");
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create memory directory: {parent:?}"))?;
        }

        let state = Self::load_state(&state_path)?;
        Ok(Self {
            state: Mutex::new(state),
            state_path,
            half_life_days: DEFAULT_HALF_LIFE_DAYS,
        })
    }

    fn load_state(path: &Path) -> anyhow::Result<KnowledgeGraphState> {
        if !path.exists() {
            return Ok(KnowledgeGraphState::default());
        }

        let raw = std::fs::read(path)
            .with_context(|| format!("failed to read knowledge graph state from {path:?}"))?;

        if raw.is_empty() {
            return Ok(KnowledgeGraphState::default());
        }

        let parsed = serde_json::from_slice(&raw)
            .with_context(|| format!("failed to parse knowledge graph state at {path:?}"))?;
        Ok(parsed)
    }

    fn save_state(path: &Path, state: &KnowledgeGraphState) -> anyhow::Result<()> {
        let encoded = serde_json::to_vec_pretty(state)?;
        let tmp_path = path.with_extension("json.tmp");

        std::fs::write(&tmp_path, encoded)
            .with_context(|| format!("failed to write temp graph state: {tmp_path:?}"))?;
        std::fs::rename(&tmp_path, path).with_context(|| {
            format!("failed to atomically persist graph state to destination: {path:?}")
        })?;
        Ok(())
    }

    fn validate_key(key: &str) -> anyhow::Result<&str> {
        let trimmed = key.trim();
        ensure!(!trimmed.is_empty(), "key cannot be empty");
        ensure!(trimmed.len() <= MAX_KEY_LEN, "key is too long");
        Ok(trimmed)
    }

    fn validate_content(content: &str) -> anyhow::Result<&str> {
        let trimmed = content.trim();
        ensure!(!trimmed.is_empty(), "content cannot be empty");
        ensure!(trimmed.len() <= MAX_CONTENT_LEN, "content is too long");
        Ok(trimmed)
    }

    fn validate_query(query: &str) -> anyhow::Result<&str> {
        let trimmed = query.trim();
        ensure!(!trimmed.is_empty(), "query cannot be empty");
        ensure!(trimmed.len() <= MAX_QUERY_LEN, "query is too long");
        Ok(trimmed)
    }

    fn category_to_string(category: &MemoryCategory) -> String {
        match category {
            MemoryCategory::Core => "core".to_string(),
            MemoryCategory::Daily => "daily".to_string(),
            MemoryCategory::Conversation => "conversation".to_string(),
            MemoryCategory::Custom(name) => name.to_ascii_lowercase(),
        }
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|token| {
                token
                    .chars()
                    .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                    .collect::<String>()
                    .to_ascii_lowercase()
            })
            .filter(|token| !token.is_empty())
            .collect()
    }

    fn entity_id(label: &str) -> String {
        let normalized = Self::tokenize(label).join("_");
        if normalized.is_empty() {
            "entity:unknown".to_string()
        } else {
            format!("entity:{normalized}")
        }
    }

    fn parse_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(timestamp)
            .ok()
            .map(|value| value.with_timezone(&Utc))
    }

    fn memory_score(&self, memory: &GraphMemory, query: &str, now: DateTime<Utc>) -> f64 {
        let query_tokens = Self::tokenize(query);
        if query_tokens.is_empty() {
            return 0.0;
        }

        let memory_text = format!(
            "{} {} {} {}",
            memory.key,
            memory.content,
            Self::category_to_string(&memory.category),
            memory.session_id.clone().unwrap_or_default(),
        )
        .to_ascii_lowercase();

        let matched = query_tokens
            .iter()
            .filter(|token| memory_text.contains(token.as_str()))
            .count();

        if matched == 0 {
            return 0.0;
        }

        let lexical = matched as f64 / query_tokens.len() as f64;

        let created_at = Self::parse_timestamp(&memory.created_at).unwrap_or(now);
        let age_seconds = (now - created_at).num_seconds().max(0) as f64;
        let half_life_seconds = self.half_life_days * 24.0 * 3600.0;
        let decay = 0.5_f64.powf(age_seconds / half_life_seconds);

        let access_boost = f64::from(memory.access_count).ln_1p();
        let kind_weight = match memory.kind {
            MemoryKind::Semantic => 1.2,
            MemoryKind::Procedural => 1.0,
            MemoryKind::Episodic => 0.9,
        };

        kind_weight * (0.6 * lexical + 0.3 * memory.base_relevance * decay + 0.1 * access_boost)
    }

    pub async fn store_fact(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
        confidence: f64,
    ) -> anyhow::Result<()> {
        ensure!((0.0..=1.0).contains(&confidence), "confidence must be in [0,1]");

        let subject = Self::validate_content(subject)?;
        let predicate = Self::validate_content(predicate)?;
        let object = Self::validate_content(object)?;

        let now = Utc::now().to_rfc3339();
        let subject_id = Self::entity_id(subject);
        let object_id = Self::entity_id(object);
        let relation_id = format!("edge:{subject_id}:{predicate}:{object_id}");
        let memory_key = format!("kg:{}:{}:{}", subject, predicate, object);

        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;

        guard.entities.insert(
            subject_id.clone(),
            EntityNode {
                id: subject_id.clone(),
                label: subject.to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        );

        guard.entities.insert(
            object_id.clone(),
            EntityNode {
                id: object_id.clone(),
                label: object.to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        );

        guard.relations.insert(
            relation_id.clone(),
            RelationEdge {
                id: relation_id,
                subject_id,
                predicate: predicate.to_string(),
                object_id,
                confidence,
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        );

        guard.memories.insert(
            memory_key.clone(),
            GraphMemory {
                id: uuid::Uuid::new_v4().to_string(),
                key: memory_key,
                content: format!("{subject} {predicate} {object}"),
                category,
                session_id: session_id.map(str::to_owned),
                kind: MemoryKind::Semantic,
                base_relevance: confidence,
                access_count: 0,
                created_at: now.clone(),
                updated_at: now.clone(),
                last_accessed_at: now,
            },
        );

        Self::save_state(&self.state_path, &guard)
    }
}

#[async_trait]
impl Memory for KnowledgeGraphMemory {
    fn name(&self) -> &str {
        "knowledge_graph"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let key = Self::validate_key(key)?;
        let content = Self::validate_content(content)?;

        let now = Utc::now().to_rfc3339();
        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;

        let entry = guard.memories.entry(key.to_string()).or_insert_with(|| GraphMemory {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            content: String::new(),
            category: MemoryCategory::Core,
            session_id: None,
            kind: MemoryKind::Episodic,
            base_relevance: 0.65,
            access_count: 0,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_accessed_at: now.clone(),
        });

        entry.content = content.to_string();
        entry.category = category;
        entry.session_id = session_id.map(str::to_owned);
        entry.updated_at = now.clone();
        if entry.created_at.is_empty() {
            entry.created_at = now.clone();
        }

        Self::save_state(&self.state_path, &guard)
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let query = Self::validate_query(query)?;
        ensure!(limit > 0 && limit <= 100, "limit must be in range 1..=100");

        let now = Utc::now();
        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;

        let mut scored = guard
            .memories
            .values_mut()
            .filter(|memory| {
                session_id
                    .map(|target| memory.session_id.as_deref() == Some(target))
                    .unwrap_or(true)
            })
            .map(|memory| {
                let score = self.memory_score(memory, query, now);
                if score > 0.0 {
                    memory.access_count = memory.access_count.saturating_add(1);
                    memory.last_accessed_at = now.to_rfc3339();
                }
                (score, memory.clone())
            })
            .filter(|(score, _)| *score > 0.0)
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
        });

        let result = scored
            .into_iter()
            .take(limit)
            .map(|(score, memory)| MemoryEntry {
                id: memory.id,
                key: memory.key,
                content: memory.content,
                category: memory.category,
                timestamp: memory.created_at,
                session_id: memory.session_id,
                score: Some(score),
            })
            .collect::<Vec<_>>();

        Self::save_state(&self.state_path, &guard)?;
        Ok(result)
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let key = Self::validate_key(key)?;
        let now = Utc::now().to_rfc3339();

        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;

        let item = guard.memories.get_mut(key).map(|memory| {
            memory.access_count = memory.access_count.saturating_add(1);
            memory.last_accessed_at = now;
            MemoryEntry {
                id: memory.id.clone(),
                key: memory.key.clone(),
                content: memory.content.clone(),
                category: memory.category.clone(),
                timestamp: memory.created_at.clone(),
                session_id: memory.session_id.clone(),
                score: Some(memory.base_relevance),
            }
        });

        if item.is_some() {
            Self::save_state(&self.state_path, &guard)?;
        }

        Ok(item)
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;

        let mut entries = guard
            .memories
            .values()
            .filter(|memory| category.map(|cat| &memory.category == cat).unwrap_or(true))
            .filter(|memory| {
                session_id
                    .map(|target| memory.session_id.as_deref() == Some(target))
                    .unwrap_or(true)
            })
            .map(|memory| MemoryEntry {
                id: memory.id.clone(),
                key: memory.key.clone(),
                content: memory.content.clone(),
                category: memory.category.clone(),
                timestamp: memory.created_at.clone(),
                session_id: memory.session_id.clone(),
                score: None,
            })
            .collect::<Vec<_>>();

        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        Ok(entries)
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let key = Self::validate_key(key)?;

        let mut guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;

        let removed = guard.memories.remove(key).is_some();
        if removed {
            Self::save_state(&self.state_path, &guard)?;
        }
        Ok(removed)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        let guard = self
            .state
            .lock()
            .map_err(|_| anyhow!("knowledge graph state lock poisoned"))?;
        Ok(guard.memories.len())
    }

    async fn health_check(&self) -> bool {
        self.state_path
            .parent()
            .map(std::path::Path::exists)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, KnowledgeGraphMemory) {
        let temp = TempDir::new().unwrap();
        let memory = KnowledgeGraphMemory::new(temp.path()).unwrap();
        (temp, memory)
    }

    #[tokio::test]
    async fn knowledge_graph_store_and_get() {
        let (_tmp, memory) = setup();

        memory
            .store(
                "user_pref_lang",
                "User prefers Rust",
                MemoryCategory::Core,
                Some("session-a"),
            )
            .await
            .unwrap();

        let item = memory.get("user_pref_lang").await.unwrap().unwrap();
        assert_eq!(item.content, "User prefers Rust");
        assert_eq!(item.session_id.as_deref(), Some("session-a"));
    }

    #[tokio::test]
    async fn knowledge_graph_recall_filters_by_session() {
        let (_tmp, memory) = setup();

        memory
            .store(
                "a",
                "Rust memory context",
                MemoryCategory::Conversation,
                Some("session-a"),
            )
            .await
            .unwrap();
        memory
            .store(
                "b",
                "Rust but from another session",
                MemoryCategory::Conversation,
                Some("session-b"),
            )
            .await
            .unwrap();

        let session_a = memory.recall("rust", 10, Some("session-a")).await.unwrap();
        assert_eq!(session_a.len(), 1);
        assert_eq!(session_a[0].key, "a");
    }

    #[tokio::test]
    async fn knowledge_graph_list_and_forget() {
        let (_tmp, memory) = setup();

        memory
            .store("a", "Core fact", MemoryCategory::Core, None)
            .await
            .unwrap();
        memory
            .store("b", "Daily note", MemoryCategory::Daily, None)
            .await
            .unwrap();

        let core = memory.list(Some(&MemoryCategory::Core), None).await.unwrap();
        assert_eq!(core.len(), 1);
        assert_eq!(core[0].key, "a");

        assert!(memory.forget("a").await.unwrap());
        assert!(memory.get("a").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn knowledge_graph_store_fact_creates_semantic_memory() {
        let (_tmp, memory) = setup();

        memory
            .store_fact(
                "project:corvus",
                "uses",
                "surrealdb",
                MemoryCategory::Core,
                None,
                0.9,
            )
            .await
            .unwrap();

        let results = memory.recall("surrealdb", 5, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("surrealdb"));
    }

    #[tokio::test]
    async fn knowledge_graph_persists_to_disk() {
        let temp = TempDir::new().unwrap();
        let memory = KnowledgeGraphMemory::new(temp.path()).unwrap();

        memory
            .store("persist_key", "Persist me", MemoryCategory::Core, None)
            .await
            .unwrap();

        let reloaded = KnowledgeGraphMemory::new(temp.path()).unwrap();
        let loaded = reloaded.get("persist_key").await.unwrap();
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn knowledge_graph_rejects_invalid_inputs() {
        let (_tmp, memory) = setup();

        let empty_key = memory.store("   ", "value", MemoryCategory::Core, None).await;
        assert!(empty_key.is_err());

        let invalid_limit = memory.recall("rust", 0, None).await;
        assert!(invalid_limit.is_err());

        let invalid_confidence = memory
            .store_fact("a", "b", "c", MemoryCategory::Core, None, 1.2)
            .await;
        assert!(invalid_confidence.is_err());
    }
}
