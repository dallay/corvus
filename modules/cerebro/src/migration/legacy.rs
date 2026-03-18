use crate::errors::CerebroError;
use crate::storage::MemoryRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("invalid observation in memory record {0}: {1}")]
    InvalidObservation(String, String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct LegacyExport {
    pub memory: Vec<LegacyMemoryRecord>,
    pub session: Vec<LegacySessionRecord>,
    pub prompt: Vec<LegacyPromptRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacyMemoryRecord {
    pub id: String,
    pub scope: String,
    pub topic_key: String,
    pub observation: Value,
    pub summary: String,
    pub deleted: bool,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacySessionRecord {
    pub id: String,
    pub agent: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacyPromptRecord {
    pub id: String,
    pub session_id: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NormalizedExport {
    pub memory: Vec<MemoryRecord>,
    pub session: Vec<LegacySessionRecord>,
    pub prompt: Vec<LegacyPromptRecord>,
}

pub fn read_legacy_export(path: &Path) -> Result<LegacyExport, CerebroError> {
    let data = std::fs::read_to_string(path).map_err(|err| {
        CerebroError::Storage(format!(
            "failed to read legacy export {}: {err}",
            path.display()
        ))
    })?;
    serde_json::from_str(&data).map_err(|err| {
        CerebroError::Validation(format!(
            "failed to parse legacy export {}: {err}",
            path.display()
        ))
    })
}

pub fn normalize_export(export: LegacyExport) -> Result<NormalizedExport, MigrationError> {
    let mut memory: Vec<MemoryRecord> = Vec::new();
    for record in export.memory {
        let memory_id = normalize_memory_id(&record.id);
        validate_observation(&record.observation, &memory_id)?;
        memory.push(MemoryRecord {
            memory_id,
            scope: record.scope,
            topic_key: record.topic_key,
            observation: record.observation,
            summary: record.summary,
            deleted: record.deleted,
            timestamp: record.timestamp,
        });
    }
    memory.sort_by(|a, b| a.memory_id.cmp(&b.memory_id));

    let mut session = export.session;
    session.sort_by(|a, b| a.id.cmp(&b.id));

    let mut prompt = export.prompt;
    prompt.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(NormalizedExport {
        memory,
        session,
        prompt,
    })
}

fn validate_observation(observation: &Value, memory_id: &str) -> Result<(), MigrationError> {
    if !observation.is_object() {
        return Err(MigrationError::InvalidObservation(
            memory_id.to_string(),
            "observation must be an object".to_string(),
        ));
    }
    if observation.get("content").is_none() {
        tracing::warn!(
            memory_id = %memory_id,
            "observation missing 'content' field, allowing with empty content"
        );
    }
    Ok(())
}

fn normalize_memory_id(value: &str) -> String {
    value.strip_prefix("memory:").unwrap_or(value).to_string()
}
