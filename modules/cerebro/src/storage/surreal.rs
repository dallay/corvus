use crate::config::CerebroConfig;
use crate::errors::CerebroError;
use crate::migration::legacy::{LegacyPromptRecord, LegacySessionRecord, NormalizedExport};
use crate::storage::{MemoryRecord, Storage};
use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;
use std::path::PathBuf;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

#[derive(Clone)]
pub struct SurrealStorage {
    db: Surreal<Db>,
    _namespace: String,
    _database: String,
}

impl SurrealStorage {
    pub async fn new_embedded(config: &CerebroConfig) -> Result<Self, CerebroError> {
        let path = config
            .surreal
            .storage_path
            .clone()
            .or_else(|| config.storage_path.clone())
            .unwrap_or_else(|| "./cerebro.db".to_string());

        if cfg!(debug_assertions) && std::env::var("CEREBRO_TEST_FAIL_STORAGE").is_ok() {
            return Err(CerebroError::Storage(
                "forced storage failure for test".to_string(),
            ));
        }

        let db = Surreal::new::<RocksDb>(PathBuf::from(path))
            .await
            .map_err(|err| {
                CerebroError::Storage(format!("failed to open embedded surrealdb: {err}"))
            })?;
        let namespace = config.surreal.namespace.clone();
        let database = config.surreal.database.clone();
        db.use_ns(namespace.clone())
            .use_db(database.clone())
            .await
            .map_err(|err| CerebroError::Storage(format!("failed to select surrealdb: {err}")))?;
        Ok(Self {
            db,
            _namespace: namespace,
            _database: database,
        })
    }

    pub fn new_remote(_config: &CerebroConfig) -> Result<Self, CerebroError> {
        Err(CerebroError::NotImplemented(
            "remote surrealdb storage is not available in this build".to_string(),
        ))
    }

    fn normalize_table_id(&self, table: &str, id: &str) -> String {
        id.strip_prefix(&format!("{table}:"))
            .unwrap_or(id)
            .to_string()
    }

    pub async fn write_batches(&self, export: &NormalizedExport) -> Result<(), CerebroError> {
        self.write_batch_memory(&export.memory).await?;
        self.write_batch_sessions(&export.session).await?;
        self.write_batch_prompts(&export.prompt).await?;
        Ok(())
    }

    pub async fn export_collections(&self) -> Result<NormalizedExport, CerebroError> {
        let mut memory: Vec<MemoryRecord> = self
            .db
            .select::<Vec<MemoryRecord>>("memory")
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb export failed: {err}")))?;
        memory.sort_by(|a, b| a.memory_id.cmp(&b.memory_id));

        let session_raw: Vec<Value> = self
            .db
            .select::<Vec<Value>>("session")
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb export failed: {err}")))?;
        let mut session: Vec<LegacySessionRecord> =
            parse_legacy_records(session_raw, "session")?;
        session.sort_by(|a, b| a.id.cmp(&b.id));

        let prompt_raw: Vec<Value> = self
            .db
            .select::<Vec<Value>>("prompt")
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb export failed: {err}")))?;
        let mut prompt: Vec<LegacyPromptRecord> = parse_legacy_records(prompt_raw, "prompt")?;
        prompt.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(NormalizedExport {
            memory,
            session,
            prompt,
        })
    }

    async fn write_batch_memory(&self, records: &[MemoryRecord]) -> Result<(), CerebroError> {
        let mut written = Vec::new();
        for record in records {
            let record_id = record.memory_id.clone();
            if let Err(err) = self
                .db
                .update::<Option<MemoryRecord>>(("memory", record_id.as_str()))
                .content(record)
                .await
            {
                self.rollback_records("memory", &written).await?;
                return Err(CerebroError::Storage(format!(
                    "surrealdb memory batch failed: {err}"
                )));
            }
            written.push(record_id);
        }
        Ok(())
    }

    async fn write_batch_sessions(
        &self,
        records: &[LegacySessionRecord],
    ) -> Result<(), CerebroError> {
        let mut written = Vec::new();
        for record in records {
            let record_id = self.normalize_table_id("session", &record.id);
            let mut payload = serde_json::to_value(record).map_err(|err| {
                CerebroError::Storage(format!("failed to encode session record: {err}"))
            })?;
            if let Value::Object(object) = &mut payload {
                object.remove("id");
            }
            if let Err(err) = self
                .db
                .update::<Option<Value>>(("session", record_id.as_str()))
                .content(payload)
                .await
            {
                self.rollback_records("session", &written).await?;
                return Err(CerebroError::Storage(format!(
                    "surrealdb session batch failed: {err}"
                )));
            }
            written.push(record_id);
        }
        Ok(())
    }

    async fn write_batch_prompts(
        &self,
        records: &[LegacyPromptRecord],
    ) -> Result<(), CerebroError> {
        let mut written = Vec::new();
        for record in records {
            let record_id = self.normalize_table_id("prompt", &record.id);
            let mut payload = serde_json::to_value(record).map_err(|err| {
                CerebroError::Storage(format!("failed to encode prompt record: {err}"))
            })?;
            if let Value::Object(object) = &mut payload {
                object.remove("id");
            }
            if let Err(err) = self
                .db
                .update::<Option<Value>>(("prompt", record_id.as_str()))
                .content(payload)
                .await
            {
                self.rollback_records("prompt", &written).await?;
                return Err(CerebroError::Storage(format!(
                    "surrealdb prompt batch failed: {err}"
                )));
            }
            written.push(record_id);
        }
        Ok(())
    }

    async fn rollback_records(
        &self,
        table: &str,
        record_ids: &[String],
    ) -> Result<(), CerebroError> {
        for record_id in record_ids.iter().rev() {
            let _ = self
                .db
                .delete::<Option<serde_json::Value>>((table, record_id.as_str()))
                .await;
        }
        Ok(())
    }
}

fn parse_legacy_records<T>(records: Vec<Value>, table: &str) -> Result<Vec<T>, CerebroError>
where
    T: serde::de::DeserializeOwned,
{
    records
        .into_iter()
        .map(|value| normalize_id_field(value, table))
        .map(|value| {
            value.and_then(|normalized| {
                serde_json::from_value(normalized).map_err(|err| {
                    CerebroError::Storage(format!(
                        "surrealdb export failed: failed to decode {table} record: {err}"
                    ))
                })
            })
        })
        .collect::<Result<Vec<_>, CerebroError>>()
}

fn normalize_id_field(value: Value, table: &str) -> Result<Value, CerebroError> {
    let mut object = match value {
        Value::Object(object) => object,
        _ => {
            return Err(CerebroError::Storage(format!(
                "surrealdb export failed: {table} record is not an object"
            )))
        }
    };

    if let Some(id_value) = object.get("id").cloned() {
        if !id_value.is_string() {
            let id_string = thing_to_string(&id_value).ok_or_else(|| {
                CerebroError::Storage(format!(
                    "surrealdb export failed: {table} record id could not be normalized"
                ))
            })?;
            object.insert("id".to_string(), Value::String(id_string));
        }
    }

    Ok(Value::Object(object))
}

fn thing_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Object(object) => {
            let table = object.get("tb")?.as_str()?;
            let id_value = object.get("id")?;
            let id = match id_value {
                Value::String(value) => value.clone(),
                Value::Number(value) => value.to_string(),
                Value::Object(inner) => inner.iter().next().and_then(|(_, v)| match v {
                    Value::String(value) => Some(value.clone()),
                    Value::Number(value) => Some(value.to_string()),
                    _ => None,
                })?,
                _ => return None,
            };
            Some(format!("{table}:{id}"))
        }
        _ => None,
    }
}

#[async_trait]
impl Storage for SurrealStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn save(&self, record: MemoryRecord) -> Result<(), CerebroError> {
        let record_id = record.memory_id.as_str();
        self.db
            .update::<Option<MemoryRecord>>(("memory", record_id))
            .content(record)
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb save failed: {err}")))?;
        Ok(())
    }

    async fn get(&self, memory_id: &str) -> Result<Option<MemoryRecord>, CerebroError> {
        let record: Option<MemoryRecord> = self
            .db
            .select::<Option<MemoryRecord>>(("memory", memory_id))
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb get failed: {err}")))?;
        Ok(record)
    }

    async fn delete(&self, memory_id: &str, hard_delete: bool) -> Result<bool, CerebroError> {
        if hard_delete {
            let record: Option<MemoryRecord> = self
                .db
                .delete::<Option<MemoryRecord>>(("memory", memory_id))
                .await
                .map_err(|err| CerebroError::Storage(format!("surrealdb delete failed: {err}")))?;
            return Ok(record.is_some());
        }

        let record: Option<MemoryRecord> = self
            .db
            .update::<Option<MemoryRecord>>(("memory", memory_id))
            .merge(serde_json::json!({ "deleted": true }))
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb update failed: {err}")))?;
        Ok(record.is_some())
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
        include_deleted: bool,
        scope: Option<&str>,
        topic_key: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        let mut records: Vec<MemoryRecord> = self
            .db
            .select::<Vec<MemoryRecord>>("memory")
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb search failed: {err}")))?;

        let query = query.to_ascii_lowercase();
        records.retain(|record| {
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
            let haystack = format!("{} {}", record.summary, record.topic_key).to_ascii_lowercase();
            haystack.contains(&query)
        });

        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        records.truncate(limit);
        Ok(records)
    }

    async fn count(&self) -> Result<usize, CerebroError> {
        let records: Vec<MemoryRecord> = self
            .db
            .select::<Vec<MemoryRecord>>("memory")
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb count failed: {err}")))?;
        Ok(records.len())
    }
}
