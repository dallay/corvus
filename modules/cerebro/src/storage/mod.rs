use crate::config::{CerebroConfig, StorageFallback, StorageMode};
use crate::errors::CerebroError;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use surrealdb::types::SurrealValue;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod surreal;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
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
    fn as_any(&self) -> &dyn Any;
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

#[derive(Debug)]
pub struct DiskBackedStorage {
    path: PathBuf,
    records: RwLock<HashMap<String, MemoryRecord>>,
}

impl DiskBackedStorage {
    pub fn new(path: PathBuf) -> Result<Arc<Self>, CerebroError> {
        let records = load_records(&path)?;
        Ok(Arc::new(Self {
            path,
            records: RwLock::new(records),
        }))
    }

    async fn persist_records(&self, records: Vec<MemoryRecord>) -> Result<(), CerebroError> {
        let path = self.path.clone();
        let path_for_task = path.clone();
        tokio::task::spawn_blocking(move || persist_records_to_path(&path_for_task, records))
            .await
            .map_err(|err| {
                CerebroError::Storage(format!(
                    "storage persist task failed for {}: {err}",
                    path.display()
                ))
            })??;
        Ok(())
    }
}

fn persist_records_to_path(
    path: &Path,
    mut records: Vec<MemoryRecord>,
) -> Result<(), CerebroError> {
    let Some(parent) = path.parent() else {
        return Err(CerebroError::Storage(
            "storage path must include a parent directory".to_string(),
        ));
    };
    fs::create_dir_all(parent).map_err(|err| {
        CerebroError::Storage(format!(
            "failed to create storage dir {}: {err}",
            parent.display()
        ))
    })?;
    records.sort_by(|a, b| a.memory_id.cmp(&b.memory_id));
    let encoded = serde_json::to_vec_pretty(&records).map_err(|err| {
        CerebroError::Storage(format!("failed to encode storage {}: {err}", path.display()))
    })?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, encoded).map_err(|err| {
        CerebroError::Storage(format!(
            "failed to write storage {}: {err}",
            tmp_path.display()
        ))
    })?;
    if path.exists() {
        fs::remove_file(path).map_err(|err| {
            CerebroError::Storage(format!(
                "failed to remove existing storage {}: {err}",
                path.display()
            ))
        })?;
    }
    fs::rename(&tmp_path, path).map_err(|err| {
        CerebroError::Storage(format!(
            "failed to commit storage {}: {err}",
            path.display()
        ))
    })?;
    Ok(())
}

#[async_trait]
impl Storage for InMemoryStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

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

#[async_trait]
impl Storage for DiskBackedStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn save(&self, record: MemoryRecord) -> Result<(), CerebroError> {
        let memory_id = record.memory_id.clone();
        let mut map = self.records.write().await;
        let previous = map.insert(memory_id.clone(), record.clone());
        let snapshot: Vec<MemoryRecord> = map.values().cloned().collect();
        drop(map);

        if let Err(error) = self.persist_records(snapshot).await {
            let mut map = self.records.write().await;
            if map.get(&memory_id).is_some_and(|current| current == &record) {
                match previous {
                    Some(prev) => {
                        map.insert(memory_id, prev);
                    }
                    None => {
                        map.remove(&memory_id);
                    }
                }
            }
            return Err(error);
        }

        Ok(())
    }

    async fn get(&self, memory_id: &str) -> Result<Option<MemoryRecord>, CerebroError> {
        let map = self.records.read().await;
        Ok(map.get(memory_id).cloned())
    }

    async fn delete(&self, memory_id: &str, hard_delete: bool) -> Result<bool, CerebroError> {
        let mut map = self.records.write().await;
        let previous = map.get(memory_id).cloned();
        let deleted = if hard_delete {
            map.remove(memory_id).is_some()
        } else if let Some(entry) = map.get_mut(memory_id) {
            entry.deleted = true;
            true
        } else {
            false
        };
        let snapshot: Vec<MemoryRecord> = map.values().cloned().collect();
        drop(map);

        if let Err(error) = self.persist_records(snapshot).await {
            let mut map = self.records.write().await;
            if hard_delete {
                if map.get(memory_id).is_none() {
                    if let Some(prev) = previous {
                        map.insert(memory_id.to_string(), prev);
                    }
                }
            } else if let Some(prev) = previous {
                if map
                    .get(memory_id)
                    .is_some_and(|current| current.deleted && current.memory_id == prev.memory_id)
                {
                    map.insert(memory_id.to_string(), prev);
                }
            }
            return Err(error);
        }

        Ok(deleted)
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

pub async fn storage_from_config(
    config: &CerebroConfig,
) -> Result<Arc<dyn Storage>, CerebroError> {
    config.validate_storage()?;
    match storage_from_mode(config, config.storage_mode.clone()).await {
        Ok(storage) => Ok(storage),
        Err(error) => match config.storage_fallback {
            StorageFallback::None => Err(error),
            StorageFallback::InMemory => {
                tracing::warn!(
                    fallback = "in_memory",
                    "storage fallback active after embedded surrealdb failure"
                );
                Ok(InMemoryStorage::new())
            }
            StorageFallback::Disk => {
                tracing::warn!(
                    fallback = "disk",
                    "storage fallback active after embedded surrealdb failure"
                );
                storage_from_mode(config, StorageMode::Disk)
                    .await
                    .map_err(|fallback_error| {
                        CerebroError::Storage(format!(
                            "primary storage failed ({error}); fallback failed ({fallback_error})"
                        ))
                    })
            }
            StorageFallback::RemoteSurreal => {
                tracing::warn!(
                    fallback = "remote_surreal",
                    "storage fallback active after embedded surrealdb failure"
                );
                storage_from_mode(config, StorageMode::RemoteSurreal)
                    .await
                    .map_err(|fallback_error| {
                        CerebroError::Storage(format!(
                            "primary storage failed ({error}); fallback failed ({fallback_error})"
                        ))
                    })
            }
        },
    }
}

async fn storage_from_mode(
    config: &CerebroConfig,
    mode: StorageMode,
) -> Result<Arc<dyn Storage>, CerebroError> {
    match mode {
        StorageMode::InMemory => Ok(InMemoryStorage::new()),
        StorageMode::Disk => {
            let path = config
                .storage_path
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./cerebro-data.json"));
            DiskBackedStorage::new(path).map(|storage| storage as Arc<dyn Storage>)
        }
        StorageMode::EmbeddedSurreal => surreal::SurrealStorage::new_embedded(config)
            .await
            .map(|storage| Arc::new(storage) as Arc<dyn Storage>),
        StorageMode::RemoteSurreal => surreal::SurrealStorage::new_remote(config)
            .map(|storage| Arc::new(storage) as Arc<dyn Storage>),
    }
}

fn load_records(path: &Path) -> Result<HashMap<String, MemoryRecord>, CerebroError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = fs::read(path).map_err(|err| {
        CerebroError::Storage(format!("failed to read storage {}: {err}", path.display()))
    })?;
    let records: Vec<MemoryRecord> = serde_json::from_slice(&data).map_err(|err| {
        CerebroError::Storage(format!(
            "failed to parse storage file {}: {err}",
            path.display()
        ))
    })?;
    Ok(records
        .into_iter()
        .map(|record| (record.memory_id.clone(), record))
        .collect())
}
