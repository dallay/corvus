use crate::config::CerebroConfig;
use crate::errors::CerebroError;
use crate::migration::legacy::{LegacyPromptRecord, LegacySessionRecord, NormalizedExport};
use crate::storage::{MemoryRecord, Storage};
use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;
use std::path::PathBuf;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::types::Variables;
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
        let mut statements = Vec::new();
        statements.push("BEGIN;".to_string());
        let mut variables = Variables::new();
        let mut index = 0usize;

        for record in &export.memory {
            let id_key = format!("mem_id_{index}");
            let data_key = format!("mem_data_{index}");
            let statement =
                format!("UPSERT type::record('memory', ${id_key}) CONTENT ${data_key};");
            let payload = serde_json::to_value(record).map_err(|err| {
                CerebroError::Storage(format!("failed to encode memory record: {err}"))
            })?;
            statements.push(statement);
            variables.insert(id_key, record.memory_id.clone());
            variables.insert(data_key, payload);
            index += 1;
        }

        for record in &export.session {
            let record_id = self.normalize_table_id("session", &record.id);
            let mut payload = serde_json::to_value(record).map_err(|err| {
                CerebroError::Storage(format!("failed to encode session record: {err}"))
            })?;
            if let Value::Object(object) = &mut payload {
                object.remove("id");
            }
            let id_key = format!("session_id_{index}");
            let data_key = format!("session_data_{index}");
            let statement =
                format!("UPSERT type::record('session', ${id_key}) CONTENT ${data_key};");
            statements.push(statement);
            variables.insert(id_key, record_id);
            variables.insert(data_key, payload);
            index += 1;
        }

        for record in &export.prompt {
            let record_id = self.normalize_table_id("prompt", &record.id);
            let mut payload = serde_json::to_value(record).map_err(|err| {
                CerebroError::Storage(format!("failed to encode prompt record: {err}"))
            })?;
            if let Value::Object(object) = &mut payload {
                object.remove("id");
            }
            let id_key = format!("prompt_id_{index}");
            let data_key = format!("prompt_data_{index}");
            let statement =
                format!("UPSERT type::record('prompt', ${id_key}) CONTENT ${data_key};");
            statements.push(statement);
            variables.insert(id_key, record_id);
            variables.insert(data_key, payload);
            index += 1;
        }

        statements.push("COMMIT;".to_string());
        let response = self
            .db
            .query(statements.join(" "))
            .bind(variables)
            .await
            .map_err(|err| {
                CerebroError::Storage(format!("surrealdb batch transaction failed: {err}"))
            })?;
        response.check().map_err(|err| {
            CerebroError::Storage(format!("surrealdb batch transaction failed: {err}"))
        })?;
        Ok(())
    }

    pub async fn export_collections(&self) -> Result<NormalizedExport, CerebroError> {
        let memory_json = self.select_table("memory").await?;
        let mut memory: Vec<MemoryRecord> = parse_legacy_records(memory_json, "memory")?;
        memory.sort_by(|a, b| a.memory_id.cmp(&b.memory_id));
        let session_json = self.select_table("session").await?;
        let mut session: Vec<LegacySessionRecord> = parse_legacy_records(session_json, "session")?;
        session.sort_by(|a, b| a.id.cmp(&b.id));
        let prompt_json = self.select_table("prompt").await?;
        let mut prompt: Vec<LegacyPromptRecord> = parse_legacy_records(prompt_json, "prompt")?;
        prompt.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(NormalizedExport {
            memory,
            session,
            prompt,
        })
    }

    async fn select_table(&self, table: &str) -> Result<Vec<Value>, CerebroError> {
        match self.db.select(table).await {
            Ok(records) => Ok(records),
            Err(err) if err.to_string().contains("does not exist") => Ok(Vec::new()),
            Err(err) => Err(CerebroError::Storage(format!(
                "surrealdb export failed: {err}"
            ))),
        }
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
        if let Some(id_string) = id_value.as_str() {
            let cleaned = id_string.replace('`', "");
            if cleaned != id_string {
                object.insert("id".to_string(), Value::String(cleaned));
            }
        } else {
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
            if let Some(thing_value) = object.get("Thing").or_else(|| object.get("thing")) {
                return thing_value_to_string(thing_value);
            }
            let table = object.get("tb")?.as_str()?;
            let id_value = object.get("id")?;
            let id = extract_thing_id(id_value)?;
            Some(format!("{table}:{id}"))
        }
        _ => None,
    }
}

fn thing_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Object(object) => {
            let table = object.get("tb")?.as_str()?;
            let id_value = object.get("id")?;
            let id = extract_thing_id(id_value)?;
            Some(format!("{table}:{id}"))
        }
        Value::Array(items) => {
            if items.len() != 2 {
                return None;
            }
            let table = items.first()?.as_str()?;
            let id = extract_thing_id(items.get(1)?)?;
            Some(format!("{table}:{id}"))
        }
        _ => None,
    }
}

fn extract_thing_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Object(inner) => {
            if let Some(thing_value) = inner.get("Thing").or_else(|| inner.get("thing")) {
                return thing_value_to_id(thing_value);
            }
            inner.iter().next().and_then(|(_, v)| match v {
                Value::String(value) => Some(value.clone()),
                Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
        }
        Value::Array(items) => {
            if items.len() != 1 {
                return None;
            }
            extract_thing_id(items.first()?)
        }
        _ => None,
    }
}

fn thing_value_to_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Object(object) => extract_thing_id(object.get("id")?),
        Value::Array(items) => {
            if items.len() != 2 {
                return None;
            }
            extract_thing_id(items.get(1)?)
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
        let record_id = record.memory_id.clone();
        let response = self
            .db
            .query("UPSERT type::record('memory', $id) CONTENT $data")
            .bind(("id", record_id))
            .bind(("data", record))
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb save failed: {err}")))?;
        response
            .check()
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
        let mut clauses: Vec<String> = Vec::new();
        let mut variables = Variables::new();
        if !include_deleted {
            clauses.push("deleted = false".to_string());
        }
        if let Some(scope) = scope {
            clauses.push("scope = $scope".to_string());
            variables.insert("scope", scope.to_string());
        }
        if let Some(topic_key) = topic_key {
            clauses.push("topic_key = $topic_key".to_string());
            variables.insert("topic_key", topic_key.to_string());
        }
        let normalized_query = query.trim().to_ascii_lowercase();
        if !normalized_query.is_empty() {
            clauses.push(
                "(string::contains(string::lowercase(summary), $query) OR string::contains(string::lowercase(topic_key), $query))"
                    .to_string(),
            );
            variables.insert("query", normalized_query);
        }

        variables.insert("limit", limit as i64);
        let mut statement = String::from("SELECT * FROM memory");
        if !clauses.is_empty() {
            statement.push_str(" WHERE ");
            statement.push_str(&clauses.join(" AND "));
        }
        statement.push_str(" ORDER BY timestamp DESC LIMIT $limit;");

        let response = self
            .db
            .query(statement)
            .bind(variables)
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb search failed: {err}")))?;
        let mut response = response
            .check()
            .map_err(|err| CerebroError::Storage(format!("surrealdb search failed: {err}")))?;
        let records: Vec<MemoryRecord> = response
            .take(0)
            .map_err(|err| CerebroError::Storage(format!("surrealdb search failed: {err}")))?;
        Ok(records)
    }

    async fn timeline(
        &self,
        memory_id: &str,
        before: usize,
        after: usize,
        include_deleted: bool,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        // First, get the anchor row's timestamp
        let memory_id = memory_id.to_owned();
        let mut anchor_response = self
            .db
            .query("SELECT timestamp FROM memory WHERE memory_id = $id")
            .bind(("id", memory_id))
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb timeline failed: {err}")))?;
        let anchor_records: Vec<MemoryRecord> = anchor_response
            .take(0)
            .map_err(|err| CerebroError::Storage(format!("surrealdb timeline failed: {err}")))?;

        let Some(anchor_record) = anchor_records.first() else {
            return Ok(Vec::new());
        };

        let anchor_ts = anchor_record.timestamp.clone();

        // Build the query with timestamp range and deleted filter
        let mut variables = Variables::new();
        variables.insert("anchor_ts", anchor_ts);
        variables.insert("before_limit", before as i64);
        variables.insert("after_limit", after as i64);

        let deleted_clause = if include_deleted {
            ""
        } else {
            " AND deleted = false"
        };

        // Query for records before the anchor (reverse order, then reverse the results)
        let before_query = format!(
            "SELECT * FROM memory WHERE timestamp < $anchor_ts{} ORDER BY timestamp DESC LIMIT $before_limit",
            deleted_clause
        );

        // Query for the anchor and records after it
        let after_query = format!(
            "SELECT * FROM memory WHERE timestamp >= $anchor_ts{} ORDER BY timestamp ASC LIMIT $after_limit + 1",
            deleted_clause
        );

        let mut before_response = self
            .db
            .query(&before_query)
            .bind(variables.clone())
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb timeline failed: {err}")))?;
        let mut before_records: Vec<MemoryRecord> = before_response
            .take(0)
            .map_err(|err| CerebroError::Storage(format!("surrealdb timeline failed: {err}")))?;

        let mut after_response = self
            .db
            .query(&after_query)
            .bind(variables)
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb timeline failed: {err}")))?;
        let after_records: Vec<MemoryRecord> = after_response
            .take(0)
            .map_err(|err| CerebroError::Storage(format!("surrealdb timeline failed: {err}")))?;

        // Reverse before_records since they were fetched in DESC order
        before_records.reverse();

        // Combine the results
        let mut result = before_records;
        result.extend(after_records);

        Ok(result)
    }

    async fn count(&self) -> Result<usize, CerebroError> {
        let records = self
            .select_table("memory")
            .await
            .map_err(|err| CerebroError::Storage(format!("surrealdb count failed: {err}")))?;
        Ok(records.len())
    }

    async fn ready(&self) -> Result<(), CerebroError> {
        self.db.query("RETURN 1;").await.map(|_| ()).map_err(|err| {
            CerebroError::Storage(format!("surrealdb readiness probe failed: {err}"))
        })
    }
}
