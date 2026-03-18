use crate::config::CerebroConfig;
use crate::errors::CerebroError;
use crate::migration::checksum::canonical_json_checksum;
use crate::migration::legacy::{normalize_export, read_legacy_export, NormalizedExport};
use crate::migration::report::{CollectionReport, MigrationReport, MigrationStatus};
use crate::storage::surreal::SurrealStorage;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

pub mod checksum;
pub mod legacy;
pub mod report;

static STORAGE_CACHE: OnceLock<Mutex<HashMap<String, SurrealStorage>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct MigrationOptions {
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub dry_run: bool,
}

pub async fn import_legacy_export(
    source: &Path,
    target: &Path,
    options: &MigrationOptions,
) -> Result<MigrationReport, CerebroError> {
    let export = read_legacy_export(source)?;
    let export = normalize_export(export).map_err(|e| {
        CerebroError::Validation(format!("migration normalization failed: {}", e))
    })?;
    if !options.dry_run {
        let storage = embedded_storage(target, options).await?;
        storage.write_batches(&export).await?;
    }
    let collections = collection_reports(&export)?;
    Ok(MigrationReport {
        source: source.display().to_string(),
        target: target.display().to_string(),
        collections,
        status: MigrationStatus::Ok,
    })
}

pub async fn validate_legacy_export(
    source: &Path,
    target: &Path,
    options: &MigrationOptions,
) -> Result<MigrationReport, CerebroError> {
    let export = normalize_export(read_legacy_export(source)?).map_err(|e| {
        CerebroError::Validation(format!("migration normalization failed: {}", e))
    })?;
    let expected = collection_reports(&export)?;

    let storage = embedded_storage(target, options).await?;
    let actual_export = storage.export_collections().await?;
    let actual = collection_reports(&actual_export)?;

    let status = if expected == actual {
        MigrationStatus::Ok
    } else {
        MigrationStatus::Mismatch
    };

    Ok(MigrationReport {
        source: source.display().to_string(),
        target: target.display().to_string(),
        collections: actual,
        status,
    })
}

async fn embedded_storage(
    target: &Path,
    options: &MigrationOptions,
) -> Result<SurrealStorage, CerebroError> {
    let mut config = CerebroConfig::default();
    config.surreal.storage_path = Some(target.display().to_string());
    if let Some(namespace) = options.namespace.as_ref() {
        config.surreal.namespace = namespace.clone();
    }
    if let Some(database) = options.database.as_ref() {
        config.surreal.database = database.clone();
    }
    let cache_key = format!(
        "{}::{}::{}",
        config
            .surreal
            .storage_path
            .clone()
            .unwrap_or_else(|| "./cerebro.db".to_string()),
        config.surreal.namespace,
        config.surreal.database,
    );
    let cache = STORAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = cache.lock() {
        if let Some(storage) = cache.get(&cache_key) {
            return Ok(storage.clone());
        }
    }

    let storage = SurrealStorage::new_embedded(&config).await?;
    if let Ok(mut cache) = cache.lock() {
        if let Some(existing) = cache.get(&cache_key) {
            return Ok(existing.clone());
        }
        cache.insert(cache_key, storage.clone());
    }
    Ok(storage)
}

fn collection_reports(export: &NormalizedExport) -> Result<BTreeMap<String, CollectionReport>, CerebroError> {
    let mut collections = BTreeMap::new();
    collections.insert(
        "memory".to_string(),
        collection_report(&export.memory)?,
    );
    collections.insert(
        "session".to_string(),
        collection_report(&export.session)?,
    );
    collections.insert(
        "prompt".to_string(),
        collection_report(&export.prompt)?,
    );
    Ok(collections)
}

fn collection_report<T: Serialize>(records: &[T]) -> Result<CollectionReport, CerebroError> {
    let value = serde_json::to_value(records).map_err(|err| {
        CerebroError::Internal(format!("failed to encode collection: {err}"))
    })?;
    Ok(CollectionReport {
        count: records.len(),
        checksum: canonical_json_checksum(&value),
    })
}
