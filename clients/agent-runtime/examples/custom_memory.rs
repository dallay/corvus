//! Example: Implementing a custom Knowledge Graph Memory backend for Corvus
//!
//! This demonstrates a hybrid memory design that can later be persisted in SurrealDB:
//! - Knowledge Graph (entities + relations)
//! - Episodic Memory (events/timeline)
//! - Semantic Memory (facts)
//! - Procedural Memory (instructions/policies)
//! - Relevance scoring with simple time-decay
//!
//! Run: cargo run --example custom_memory

use anyhow::{anyhow, ensure};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Mutex;

// ── Re-define the trait types (in your app, import from corvus::memory) ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryCategory {
    Core,
    Daily,
    Conversation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryKind {
    Episodic,
    Semantic,
    Procedural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
    pub score: Option<f64>,
}

#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    async fn store(&self, key: &str, content: &str, category: MemoryCategory)
        -> anyhow::Result<()>;
    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;
    async fn count(&self) -> anyhow::Result<usize>;
}

// ── Knowledge graph memory model ───────────────────────────────────

#[derive(Debug, Clone)]
struct EntityNode {
    id: String,
    label: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct RelationEdge {
    id: String,
    subject_id: String,
    predicate: String,
    object_id: String,
    confidence: f64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct GraphMemory {
    id: String,
    key: String,
    kind: MemoryKind,
    category: MemoryCategory,
    content: String,
    created_at: DateTime<Utc>,
    last_accessed_at: DateTime<Utc>,
    access_count: u32,
    base_relevance: f64,
}

#[derive(Default)]
struct KnowledgeGraphStore {
    entities: HashMap<String, EntityNode>,
    relations: HashMap<String, RelationEdge>,
    memories: HashMap<String, GraphMemory>,
}

/// In-memory knowledge graph backend.
///
/// This mirrors a schema that could map to SurrealDB records/tables while
/// staying dependency-light for local development and tests.
pub struct KnowledgeGraphMemoryBackend {
    store: Mutex<KnowledgeGraphStore>,
    half_life_days: f64,
}

impl Default for KnowledgeGraphMemoryBackend {
    fn default() -> Self {
        Self {
            store: Mutex::new(KnowledgeGraphStore::default()),
            half_life_days: 21.0,
        }
    }
}

impl KnowledgeGraphMemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn store_fact(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        kind: MemoryKind,
        category: MemoryCategory,
        confidence: f64,
    ) -> anyhow::Result<()> {
        ensure!((0.0..=1.0).contains(&confidence), "confidence must be in [0,1]");
        Self::validate_text("subject", subject)?;
        Self::validate_text("predicate", predicate)?;
        Self::validate_text("object", object)?;

        let now = Utc::now();
        let subject_id = Self::entity_id(subject);
        let object_id = Self::entity_id(object);
        let relation_id = format!("edge:{}:{}:{}", subject_id, predicate.trim(), object_id);
        let key = format!("kg:{}:{}:{}", subject.trim(), predicate.trim(), object.trim());
        let content = format!("{} {} {}", subject.trim(), predicate.trim(), object.trim());

        let mut store = self.store.lock().map_err(|e| anyhow!("{e}"))?;

        store.entities.insert(
            subject_id.clone(),
            EntityNode {
                id: subject_id.clone(),
                label: subject.trim().to_string(),
                updated_at: now,
            },
        );
        store.entities.insert(
            object_id.clone(),
            EntityNode {
                id: object_id.clone(),
                label: object.trim().to_string(),
                updated_at: now,
            },
        );

        store.relations.insert(
            relation_id.clone(),
            RelationEdge {
                id: relation_id,
                subject_id,
                predicate: predicate.trim().to_string(),
                object_id,
                confidence,
                updated_at: now,
            },
        );

        store.memories.insert(
            key.clone(),
            GraphMemory {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                kind,
                category,
                content,
                created_at: now,
                last_accessed_at: now,
                access_count: 0,
                base_relevance: confidence,
            },
        );

        Ok(())
    }

    fn validate_text(field: &str, value: &str) -> anyhow::Result<()> {
        let trimmed = value.trim();
        ensure!(!trimmed.is_empty(), "{field} cannot be empty");
        ensure!(trimmed.len() <= 256, "{field} is too long");
        Ok(())
    }

    fn entity_id(label: &str) -> String {
        let normalized = label
            .trim()
            .to_lowercase()
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();

        if normalized.is_empty() {
            "entity:unknown".to_string()
        } else {
            format!("entity:{normalized}")
        }
    }

    fn decay_score(&self, memory: &GraphMemory, now: DateTime<Utc>) -> f64 {
        let age_seconds = (now - memory.created_at).num_seconds().max(0) as f64;
        let half_life_seconds = self.half_life_days * 24.0 * 3600.0;
        let decay = 0.5_f64.powf(age_seconds / half_life_seconds);

        let access_boost = (memory.access_count as f64 + 1.0).ln_1p();
        memory.base_relevance * decay + 0.15 * access_boost
    }

    fn keyword_score(content: &str, query: &str) -> f64 {
        let query_tokens = query
            .split_whitespace()
            .map(|s| s.to_ascii_lowercase())
            .collect::<Vec<_>>();

        if query_tokens.is_empty() {
            return 0.0;
        }

        let content_lower = content.to_ascii_lowercase();
        let matched = query_tokens
            .iter()
            .filter(|token| content_lower.contains(token.as_str()))
            .count();

        matched as f64 / query_tokens.len() as f64
    }

    pub async fn graph_stats(&self) -> anyhow::Result<(usize, usize, usize)> {
        let store = self.store.lock().map_err(|e| anyhow!("{e}"))?;
        // Touch fields to keep the sample exhaustive and warning-free.
        let _entity_checksum = store
            .entities
            .values()
            .fold(0usize, |acc, n| {
                acc ^ n.id.len() ^ n.label.len() ^ n.updated_at.timestamp() as usize
            });
        let _relation_checksum = store.relations.values().fold(0usize, |acc, e| {
            acc ^ e.id.len()
                ^ e.subject_id.len()
                ^ e.predicate.len()
                ^ e.object_id.len()
                ^ (e.confidence * 1000.0) as usize
                ^ e.updated_at.timestamp() as usize
        });
        Ok((
            store.entities.len(),
            store.relations.len(),
            store.memories.len(),
        ))
    }
}

#[async_trait]
impl Memory for KnowledgeGraphMemoryBackend {
    fn name(&self) -> &str {
        "knowledge-graph-memory"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
    ) -> anyhow::Result<()> {
        Self::validate_text("key", key)?;
        Self::validate_text("content", content)?;

        let now = Utc::now();
        let mut store = self.store.lock().map_err(|e| anyhow!("{e}"))?;

        store.memories.insert(
            key.to_string(),
            GraphMemory {
                id: uuid::Uuid::new_v4().to_string(),
                key: key.trim().to_string(),
                kind: MemoryKind::Episodic,
                category,
                content: content.trim().to_string(),
                created_at: now,
                last_accessed_at: now,
                access_count: 0,
                base_relevance: 0.65,
            },
        );

        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        Self::validate_text("query", query)?;
        ensure!(limit > 0 && limit <= 100, "limit must be in range 1..=100");

        let now = Utc::now();
        let mut store = self.store.lock().map_err(|e| anyhow!("{e}"))?;

        let mut scored = store
            .memories
            .values_mut()
            .map(|memory| {
                let semantic = Self::keyword_score(&memory.content, query);
                let recency = self.decay_score(memory, now);
                let kind_weight = match memory.kind {
                    MemoryKind::Semantic => 1.2,
                    MemoryKind::Procedural => 1.0,
                    MemoryKind::Episodic => 0.9,
                };
                let score = kind_weight * (0.65 * semantic + 0.35 * recency);

                if score > 0.0 {
                    memory.last_accessed_at = now;
                    memory.access_count = memory.access_count.saturating_add(1);
                }

                (score, memory.clone())
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
        });

        let results = scored
            .into_iter()
            .filter(|(score, _)| *score > 0.0)
            .take(limit)
            .map(|(score, memory)| MemoryEntry {
                id: memory.id,
                key: memory.key,
                content: memory.content,
                category: memory.category,
                timestamp: memory.created_at.to_rfc3339(),
                score: Some(score),
            })
            .collect();

        Ok(results)
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Self::validate_text("key", key)?;

        let mut store = self.store.lock().map_err(|e| anyhow!("{e}"))?;
        let now = Utc::now();

        let item = store.memories.get_mut(key).map(|memory| {
            memory.last_accessed_at = now;
            memory.access_count = memory.access_count.saturating_add(1);
            MemoryEntry {
                id: memory.id.clone(),
                key: memory.key.clone(),
                content: memory.content.clone(),
                category: memory.category.clone(),
                timestamp: memory.created_at.to_rfc3339(),
                score: Some(self.decay_score(memory, now)),
            }
        });

        Ok(item)
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        Self::validate_text("key", key)?;

        let mut store = self.store.lock().map_err(|e| anyhow!("{e}"))?;
        Ok(store.memories.remove(key).is_some())
    }

    async fn count(&self) -> anyhow::Result<usize> {
        let store = self.store.lock().map_err(|e| anyhow!("{e}"))?;
        Ok(store.memories.len())
    }
}

// ── Demo usage ─────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let memory = KnowledgeGraphMemoryBackend::new();

    println!("🧠 Corvus Memory Demo — KnowledgeGraphMemoryBackend\n");

    memory
        .store_fact(
            "user:yuniel",
            "prefers_language",
            "rust",
            MemoryKind::Semantic,
            MemoryCategory::Core,
            0.92,
        )
        .await?;

    memory
        .store_fact(
            "project:corvus",
            "uses_database",
            "surrealdb",
            MemoryKind::Procedural,
            MemoryCategory::Core,
            0.87,
        )
        .await?;

    memory
        .store(
            "episode:today",
            "Discussed dynamic subgraph extraction for minimum viable context.",
            MemoryCategory::Daily,
        )
        .await?;

    println!("Backend: {}", memory.name());
    println!("Stored memories: {}", memory.count().await?);

    let (entities, relations, memories) = memory.graph_stats().await?;
    println!(
        "Graph stats → entities: {entities}, relations: {relations}, memories: {memories}",
    );

    let recall = memory.recall("rust surrealdb context", 5).await?;
    println!("\nRecall results ({})", recall.len());
    for entry in &recall {
        println!(
            "  score={:.3} [{:?}] {} => {}",
            entry.score.unwrap_or_default(),
            entry.category,
            entry.key,
            entry.content,
        );
    }

    if let Some(item) = memory.get("episode:today").await? {
        println!("\nGet episode:today => {}", item.content);
    }

    let removed = memory.forget("episode:today").await?;
    println!("Forget episode:today => removed: {removed}");
    println!("Remaining memories: {}", memory.count().await?);

    println!(
        "\n✅ Knowledge graph backend works. Replace the storage layer with SurrealDB for production.",
    );
    Ok(())
}
