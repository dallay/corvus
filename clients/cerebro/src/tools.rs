use crate::errors::CerebroError;
use crate::server::AuthContext;
use crate::storage::{MemoryRecord, Storage};
use crate::validation::{require_non_empty, require_optional_non_empty};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct ToolRedaction {
    pub allowed_arg_fields: &'static [&'static str],
    pub allowed_output_fields: &'static [&'static str],
}

impl ToolRedaction {
    pub fn for_tool(tool: &str) -> Self {
        match tool {
            "mem_save" => Self {
                allowed_arg_fields: &["scope", "topic_key"],
                allowed_output_fields: &["memory_id", "status"],
            },
            "mem_search" => Self {
                allowed_arg_fields: &["limit", "scope", "topic_key", "include_deleted"],
                allowed_output_fields: &["results_count", "truncated"],
            },
            "mem_delete" => Self {
                allowed_arg_fields: &["memory_id", "topic_key", "hard_delete"],
                allowed_output_fields: &["memory_id", "status", "deleted"],
            },
            "mem_get_observation" => Self {
                allowed_arg_fields: &["memory_id", "include_deleted"],
                allowed_output_fields: &["memory_id", "status"],
            },
            "mem_update" => Self {
                allowed_arg_fields: &["memory_id"],
                allowed_output_fields: &["memory_id", "status"],
            },
            "mem_suggest_topic_key" => Self {
                allowed_arg_fields: &["scope"],
                allowed_output_fields: &["topic_key", "candidates_count"],
            },
            "mem_timeline" => Self {
                allowed_arg_fields: &["memory_id", "before", "after", "include_deleted"],
                allowed_output_fields: &["items_count"],
            },
            "mem_stats" => Self {
                allowed_arg_fields: &[],
                allowed_output_fields: &[
                    "memory_count",
                    "session_count",
                    "prompt_count",
                    "worker_enabled",
                    "worker_queue_depth",
                ],
            },
            _ => Self {
                allowed_arg_fields: &[],
                allowed_output_fields: &[],
            },
        }
    }
}

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

#[derive(Debug, Clone, Serialize)]
pub struct ToolManifest {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

#[derive(Clone)]
pub struct CerebroTools {
    storage: Arc<dyn Storage>,
}

pub const IMPLEMENTED_TOOL_NAMES: [&str; 8] = [
    "mem_save",
    "mem_search",
    "mem_delete",
    "mem_get_observation",
    "mem_update",
    "mem_suggest_topic_key",
    "mem_timeline",
    "mem_stats",
];

pub const DEFERRED_TOOL_NAMES: [&str; 5] = [
    "mem_save_prompt",
    "mem_session_start",
    "mem_session_end",
    "mem_session_summary",
    "mem_context",
];

const MAX_MEM_SEARCH_LIMIT: usize = 100;
const MAX_TIMELINE_ITEMS: usize = 100;

/// Merge `incoming` metadata into the `"metadata"` field of `observation`.
///
/// - If `observation` already has a `"metadata"` object **and** `incoming` is
///   also an object, the keys are shallow-merged.
/// - Otherwise `incoming` replaces the existing `"metadata"` value, or is
///   inserted when absent.
/// - Returns an error when `observation` itself is not an object (nowhere to
///   attach metadata).
fn merge_metadata(observation: &mut Value, incoming: Value) -> Result<(), CerebroError> {
    let Some(obs_obj) = observation.as_object_mut() else {
        return Err(CerebroError::Validation(
            "cannot merge metadata: observation is not an object".to_string(),
        ));
    };

    let Some(existing) = obs_obj.get_mut("metadata") else {
        obs_obj.insert("metadata".to_string(), incoming);
        return Ok(());
    };

    let Some(existing_obj) = existing.as_object_mut() else {
        *existing = incoming;
        return Ok(());
    };

    let Some(new_obj) = incoming.as_object() else {
        *existing = incoming;
        return Ok(());
    };

    for (key, value) in new_obj {
        existing_obj.insert(key.clone(), value.clone());
    }

    Ok(())
}

impl CerebroTools {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub fn redaction_for_tool(&self, tool: &str) -> ToolRedaction {
        ToolRedaction::for_tool(tool)
    }

    pub fn list_manifest(&self) -> Vec<ToolManifest> {
        IMPLEMENTED_TOOL_NAMES
            .iter()
            .map(|name| ToolManifest {
                name,
                description: name,
                parameters: json!({ "type": "object" }),
            })
            .collect()
    }

    pub fn extract_safe_args(&self, tool: &str, payload: &Value) -> Option<Value> {
        fn parse_input<T: DeserializeOwned>(payload: &Value) -> Option<ToolInput<T>> {
            serde_json::from_value(payload.clone()).ok()
        }
        match tool {
            "mem_save" => {
                let input: ToolInput<MemSaveRequest> = parse_input(payload)?;
                Some(json!({
                    "scope": input.input.scope,
                    "topic_key": input.input.topic_key,
                }))
            }
            "mem_search" => {
                let input: ToolInput<MemSearchRequest> = parse_input(payload)?;
                Some(json!({
                    "limit": input.input.limit,
                    "scope": input.input.scope,
                    "topic_key": input.input.topic_key,
                    "include_deleted": input.input.include_deleted,
                }))
            }
            "mem_delete" => {
                let input: ToolInput<MemDeleteRequest> = parse_input(payload)?;
                Some(json!({
                    "memory_id": input.input.memory_id,
                    "topic_key": input.input.topic_key,
                    "hard_delete": input.input.hard_delete,
                }))
            }
            "mem_get_observation" => {
                let input: ToolInput<MemGetObservationRequest> = parse_input(payload)?;
                Some(json!({
                    "memory_id": input.input.memory_id,
                    "include_deleted": input.input.include_deleted,
                }))
            }
            "mem_update" => {
                let input: ToolInput<MemUpdateRequest> = parse_input(payload)?;
                Some(json!({
                    "memory_id": input.input.memory_id,
                }))
            }
            "mem_suggest_topic_key" => {
                let input: ToolInput<MemSuggestTopicKeyRequest> = parse_input(payload)?;
                Some(json!({
                    "scope": input.input.scope,
                }))
            }
            "mem_timeline" => {
                let input: ToolInput<MemTimelineRequest> = parse_input(payload)?;
                Some(json!({
                    "memory_id": input.input.memory_id,
                    "before": input.input.before,
                    "after": input.input.after,
                    "include_deleted": input.input.include_deleted,
                }))
            }
            "mem_stats" => Some(json!({})),
            _ => None,
        }
    }

    pub fn extract_safe_output(&self, tool: &str, output: &Value) -> Option<Value> {
        match tool {
            "mem_save" => Some(json!({
                "memory_id": output.get("memory_id"),
                "status": output.get("status"),
            })),
            "mem_search" => {
                let results_count = output
                    .get("results")
                    .and_then(|value| value.as_array())
                    .map(|value| value.len());
                Some(json!({
                    "results_count": results_count,
                    "truncated": output.get("truncated"),
                }))
            }
            "mem_delete" => Some(json!({
                "memory_id": output.get("memory_id"),
                "status": output.get("status"),
                "deleted": output.get("deleted"),
            })),
            "mem_get_observation" => Some(json!({
                "memory_id": output.get("memory_id"),
                "status": output.get("status"),
            })),
            "mem_update" => Some(json!({
                "memory_id": output.get("memory_id"),
                "status": output.get("status"),
            })),
            "mem_suggest_topic_key" => {
                let candidates_count = output
                    .get("candidates")
                    .and_then(|value| value.as_array())
                    .map(|value| value.len());
                Some(json!({
                    "topic_key": output.get("topic_key"),
                    "candidates_count": candidates_count,
                }))
            }
            "mem_timeline" => {
                let items_count = output
                    .get("items_count")
                    .and_then(|value| value.as_u64())
                    .or_else(|| {
                        output
                            .get("items")
                            .and_then(|value| value.as_array())
                            .map(|value| value.len() as u64)
                    });
                Some(json!({ "items_count": items_count }))
            }
            "mem_stats" => Some(json!({
                "memory_count": output.get("memory_count"),
                "session_count": output.get("session_count"),
                "prompt_count": output.get("prompt_count"),
                "worker_enabled": output
                    .get("worker")
                    .and_then(|value| value.get("enabled")),
                "worker_queue_depth": output
                    .get("worker")
                    .and_then(|value| value.get("queue_depth")),
            })),
            _ => None,
        }
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
            "mem_stats" => self.mem_stats(payload).await,
            _ if DEFERRED_TOOL_NAMES.contains(&tool) => {
                Err(CerebroError::NotImplemented(tool.to_string()))
            }
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

        let memory_id = Uuid::new_v4().to_string();
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
        let _ = input.input.reason.as_deref();
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
        require_optional_non_empty("topic_key", input.input.topic_key.as_deref())?;
        require_optional_non_empty("scope", input.input.scope.as_deref())?;

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

        if let Some(metadata) = input.input.metadata {
            merge_metadata(&mut record.observation, metadata)?;
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

        let items = self
            .storage
            .timeline(
                &input.input.memory_id,
                before,
                after,
                input.input.include_deleted.unwrap_or(false),
            )
            .await?;

        Ok(json!({ "items": items }))
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
