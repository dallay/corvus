use crate::errors::CerebroError;
use crate::server::AuthContext;
use crate::storage::{MemoryRecord, Storage};
use crate::validation::require_non_empty;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct ToolInput<T> {
    input: T,
}

#[derive(Debug, Deserialize)]
struct MemSaveObservation {
    content: String,
    #[serde(default)]
    what: Option<String>,
    #[serde(default)]
    why: Option<String>,
    #[serde(default)]
    #[serde(rename = "where")]
    where_: Option<String>,
    #[serde(default)]
    learned: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct MemSaveRequest {
    scope: String,
    topic_key: String,
    observation: MemSaveObservation,
}

#[derive(Debug, Deserialize)]
struct MemSearchRequest {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    topic_key: Option<String>,
    #[serde(default)]
    include_deleted: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MemDeleteRequest {
    #[serde(default)]
    memory_id: Option<String>,
    #[serde(default)]
    topic_key: Option<String>,
    #[serde(default)]
    hard_delete: Option<bool>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MemGetObservationRequest {
    memory_id: String,
    #[serde(default)]
    include_deleted: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemUpdateRequest {
    memory_id: String,
    #[serde(default)]
    observation: Option<Value>,
    #[serde(default)]
    topic_key: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemSuggestTopicKeyRequest {
    seed: String,
    #[serde(default)]
    scope: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemTimelineRequest {
    memory_id: String,
    #[serde(default)]
    before: Option<usize>,
    #[serde(default)]
    after: Option<usize>,
    #[serde(default)]
    include_deleted: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemSavePromptRequest {
    prompt: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemSessionStartRequest {
    session_id: String,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemSessionEndRequest {
    session_id: String,
    #[serde(default)]
    ended_at: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemSessionSummaryRequest {
    session_id: String,
    summary: Value,
    #[serde(default)]
    metadata: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MemContextRequest {
    session_id: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Clone)]
pub struct CerebroTools {
    storage: Arc<dyn Storage>,
}

const MAX_MEM_SEARCH_LIMIT: usize = 100;
const MAX_TIMELINE_ITEMS: usize = 100;

impl CerebroTools {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub async fn handle(
        &self,
        tool: &str,
        payload: Value,
        auth: &AuthContext,
    ) -> Result<Value, CerebroError> {
        match tool {
            "mem_save" => self.mem_save(payload).await,
            "mem_search" => self.mem_search(payload).await,
            "mem_delete" => self.mem_delete(payload).await,
            "mem_get_observation" => self.mem_get_observation(payload).await,
            "mem_update" => self.mem_update(payload).await,
            "mem_suggest_topic_key" => self.mem_suggest_topic_key(payload).await,
            "mem_timeline" => self.mem_timeline(payload, auth).await,
            "mem_save_prompt" => Err(CerebroError::NotImplemented(
                "mem_save_prompt".to_string(),
            )),
            "mem_session_start" => Err(CerebroError::NotImplemented(
                "mem_session_start".to_string(),
            )),
            "mem_session_end" => Err(CerebroError::NotImplemented(
                "mem_session_end".to_string(),
            )),
            "mem_session_summary" => Err(CerebroError::NotImplemented(
                "mem_session_summary".to_string(),
            )),
            "mem_context" => Err(CerebroError::NotImplemented(
                "mem_context".to_string(),
            )),
            "mem_stats" => self.mem_stats(payload).await,
            _ => Err(CerebroError::Validation(format!(
                "unsupported tool '{tool}'",
            ))),
        }
    }

    async fn mem_save(&self, payload: Value) -> Result<Value, CerebroError> {
        let input: ToolInput<MemSaveRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        require_non_empty("scope", &input.input.scope)?;
        require_non_empty("topic_key", &input.input.topic_key)?;
        require_non_empty("observation.content", &input.input.observation.content)?;

        let observation = json!({
          "content": input.input.observation.content,
          "what": input.input.observation.what,
          "why": input.input.observation.why,
          "where": input.input.observation.where_,
          "learned": input.input.observation.learned,
          "source": input.input.observation.source,
          "tags": input.input.observation.tags,
          "metadata": input.input.observation.metadata,
        });

        let memory_id = input.input.topic_key.clone();
        let record = MemoryRecord::new(
            memory_id.clone(),
            input.input.scope,
            input.input.topic_key,
            observation,
        );
        self.storage.save(record).await?;

        Ok(json!({
          "memory_id": memory_id,
          "status": "saved"
        }))
    }

    async fn mem_search(&self, payload: Value) -> Result<Value, CerebroError> {
        let input: ToolInput<MemSearchRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        require_non_empty("query", &input.input.query)?;

        let limit = input
            .input
            .limit
            .unwrap_or(5)
            .clamp(1, MAX_MEM_SEARCH_LIMIT);
        let include_deleted = input.input.include_deleted.unwrap_or(false);

        let results = self
            .storage
            .search(
                &input.input.query,
                limit,
                include_deleted,
                input.input.scope.as_deref(),
                input.input.topic_key.as_deref(),
            )
            .await?;

        let items: Vec<Value> = results
            .into_iter()
            .map(|record| {
                json!({
                  "memory_id": record.memory_id,
                  "summary": record.summary,
                  "score": 1.0,
                  "topic_key": record.topic_key,
                  "scope": record.scope,
                  "timestamp": record.timestamp
                })
            })
            .collect();

        Ok(json!({
          "results": items,
          "truncated": false
        }))
    }

    async fn mem_delete(&self, payload: Value) -> Result<Value, CerebroError> {
        let input: ToolInput<MemDeleteRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        let hard_delete = input.input.hard_delete.unwrap_or(false);
        let memory_id = match (
            input.input.memory_id.as_deref(),
            input.input.topic_key.as_deref(),
        ) {
            (Some(id), _) => {
                require_non_empty("memory_id", id)?;
                id.to_string()
            }
            (None, Some(topic_key)) => {
                require_non_empty("topic_key", topic_key)?;
                let record = self
                    .storage
                    .search("", 1, true, None, Some(topic_key))
                    .await?
                    .into_iter()
                    .next()
                    .ok_or(CerebroError::NotFound)?;
                record.memory_id
            }
            _ => {
                return Err(CerebroError::Validation(
                    "memory_id or topic_key must be provided".to_string(),
                ));
            }
        };

        let deleted = self.storage.delete(&memory_id, hard_delete).await?;
        let status = if hard_delete {
            "hard_deleted"
        } else {
            "soft_deleted"
        };

        Ok(json!({
          "memory_id": memory_id,
          "status": status,
          "deleted": deleted
        }))
    }

    async fn mem_get_observation(&self, payload: Value) -> Result<Value, CerebroError> {
        let input: ToolInput<MemGetObservationRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        require_non_empty("memory_id", &input.input.memory_id)?;

        let record = self
            .storage
            .get(&input.input.memory_id)
            .await?
            .ok_or(CerebroError::NotFound)?;

        if record.deleted && !input.input.include_deleted.unwrap_or(false) {
            return Ok(json!({
              "memory_id": record.memory_id,
              "status": "deleted"
            }));
        }

        let status = if record.deleted { "deleted" } else { "active" };
        Ok(json!({
          "memory_id": record.memory_id,
          "status": status,
          "observation": record.observation,
          "metadata": {
            "topic_key": record.topic_key,
            "scope": record.scope,
            "timestamp": record.timestamp
          }
        }))
    }

    async fn mem_update(&self, payload: Value) -> Result<Value, CerebroError> {
        let input: ToolInput<MemUpdateRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        require_non_empty("memory_id", &input.input.memory_id)?;

        let mut record = self
            .storage
            .get(&input.input.memory_id)
            .await?
            .ok_or(CerebroError::NotFound)?;

        if let Some(observation) = input.input.observation {
            record.observation = observation;
            record.summary = record
                .observation
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
        }

        if let Some(topic_key) = input.input.topic_key {
            record.topic_key = topic_key;
        }

        if let Some(scope) = input.input.scope {
            record.scope = scope;
        }

        self.storage.save(record).await?;

        Ok(json!({
          "memory_id": input.input.memory_id,
          "status": "updated"
        }))
    }

    async fn mem_suggest_topic_key(&self, payload: Value) -> Result<Value, CerebroError> {
        let input: ToolInput<MemSuggestTopicKeyRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        require_non_empty("seed", &input.input.seed)?;

        let slug = input
            .input
            .seed
            .trim()
            .to_ascii_lowercase()
            .replace(' ', "_");

        Ok(json!({
          "topic_key": slug,
          "candidates": []
        }))
    }

    async fn mem_timeline(
        &self,
        payload: Value,
        auth: &AuthContext,
    ) -> Result<Value, CerebroError> {
        let input: ToolInput<MemTimelineRequest> = serde_json::from_value(payload)
            .map_err(|err| CerebroError::Validation(err.to_string()))?;
        require_non_empty("memory_id", &input.input.memory_id)?;

        let before = input.input.before.unwrap_or(0);
        let after = input.input.after.unwrap_or(0);
        if before > MAX_TIMELINE_ITEMS || after > MAX_TIMELINE_ITEMS {
            return Err(CerebroError::Validation(format!(
                "before/after must be <= {MAX_TIMELINE_ITEMS}"
            )));
        }

        if input.input.include_deleted.unwrap_or(false) && !auth.is_audit {
            return Err(CerebroError::Forbidden(
                "include_deleted requires audit permissions".to_string(),
            ));
        }

        Err(CerebroError::NotImplemented("mem_timeline".to_string()))
    }

    async fn mem_stats(&self, _payload: Value) -> Result<Value, CerebroError> {
        let count = self.storage.count().await?;
        Ok(json!({
          "memory_count": count,
          "session_count": 0,
          "prompt_count": 0,
          "worker": {
            "enabled": false,
            "queue_depth": 0
          }
        }))
    }
}
