pub mod backend;
pub mod chunker;
pub mod embeddings;
pub mod hygiene;
pub mod lucid;
pub mod markdown;
pub mod none;
pub mod response_cache;
pub mod snapshot;
pub mod sqlite;
pub mod traits;
pub mod vector;

#[allow(unused_imports)]
pub use backend::{
    classify_memory_backend, default_memory_backend_key, memory_backend_profile,
    selectable_memory_backends, MemoryBackendKind, MemoryBackendProfile,
};
pub use lucid::LucidMemory;
pub use markdown::MarkdownMemory;
pub use none::NoneMemory;
pub use response_cache::ResponseCache;
pub use sqlite::SqliteMemory;
pub use traits::Memory;
#[allow(unused_imports)]
pub use traits::{MemoryCategory, MemoryEntry, MemoryValidationResult};

use crate::config::MemoryConfig;
use std::path::Path;
use std::sync::Arc;

pub fn cerebro_configured(config: &MemoryConfig) -> bool {
    config
        .cerebro
        .endpoint
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

fn build_sqlite_memory(
    config: &MemoryConfig,
    workspace_dir: &Path,
    api_key: Option<&str>,
) -> anyhow::Result<SqliteMemory> {
    let embedder: Arc<dyn embeddings::EmbeddingProvider> =
        Arc::from(embeddings::create_embedding_provider(
            &config.embedding_provider,
            api_key,
            &config.embedding_model,
            config.embedding_dimensions,
        ));

    #[allow(clippy::cast_possible_truncation)]
    let mem = SqliteMemory::with_embedder(
        workspace_dir,
        embedder,
        config.vector_weight as f32,
        config.keyword_weight as f32,
        config.embedding_cache_size,
        config.sqlite_open_timeout_secs,
    )?;
    Ok(mem)
}


/// Factory: create the right memory backend from config
pub fn create_memory(
    config: &MemoryConfig,
    workspace_dir: &Path,
    api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    if cerebro_configured(config) {
        tracing::info!("Cerebro MCP configured; local memory remains short-term only");
    }
    // Best-effort memory hygiene/retention pass (throttled by state file).
    if let Err(e) = hygiene::run_if_due(config, workspace_dir) {
        tracing::warn!("memory hygiene skipped: {e}");
    }

    // If snapshot_on_hygiene is enabled, export core memories during hygiene.
    if config.snapshot_enabled && config.snapshot_on_hygiene {
        if let Err(e) = snapshot::export_snapshot(workspace_dir) {
            tracing::warn!("memory snapshot skipped: {e}");
        }
    }

    // Auto-hydration: if brain.db is missing but MEMORY_SNAPSHOT.md exists,
    // restore the "soul" from the snapshot before creating the backend.
    if config.auto_hydrate
        && matches!(
            classify_memory_backend(&config.backend),
            MemoryBackendKind::Sqlite | MemoryBackendKind::Lucid
        )
        && snapshot::should_hydrate(workspace_dir)
    {
        tracing::info!("🧬 Cold boot detected — hydrating from MEMORY_SNAPSHOT.md");
        match snapshot::hydrate_from_snapshot(workspace_dir) {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("🧬 Hydrated {count} core memories from snapshot");
                }
            }
            Err(e) => {
                tracing::warn!("memory hydration failed: {e}");
            }
        }
    }

    match classify_memory_backend(&config.backend) {
        MemoryBackendKind::Sqlite => Ok(Box::new(build_sqlite_memory(
            config,
            workspace_dir,
            api_key,
        )?)),
        MemoryBackendKind::Lucid => {
            let local = build_sqlite_memory(config, workspace_dir, api_key)?;
            Ok(Box::new(LucidMemory::new(workspace_dir, local)))
        }
        MemoryBackendKind::Markdown => Ok(Box::new(MarkdownMemory::new(workspace_dir))),
        MemoryBackendKind::None => Ok(Box::new(NoneMemory::new())),
        MemoryBackendKind::LegacySurreal => {
            anyhow::bail!(
                "SurrealDB backend has been removed; use the Cerebro backend for long-term memory. See https://github.com/dallay/corvus/blob/main/clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md"
            );
        }
        MemoryBackendKind::Unknown => {
            tracing::warn!(
                "Unknown memory backend '{}', falling back to markdown",
                config.backend
            );
            Ok(Box::new(MarkdownMemory::new(workspace_dir)))
        }
    }
}

pub fn create_memory_for_migration(
    backend: &str,
    workspace_dir: &Path,
) -> anyhow::Result<Box<dyn Memory>> {
    if matches!(classify_memory_backend(backend), MemoryBackendKind::None) {
        anyhow::bail!(
            "memory backend 'none' disables persistence; choose sqlite, lucid, or markdown before migration"
        );
    }

    match classify_memory_backend(backend) {
        MemoryBackendKind::Sqlite => Ok(Box::new(SqliteMemory::new(workspace_dir)?)),
        MemoryBackendKind::Lucid => {
            let local = SqliteMemory::new(workspace_dir)?;
            Ok(Box::new(LucidMemory::new(workspace_dir, local)))
        }
        MemoryBackendKind::Markdown | MemoryBackendKind::Unknown => {
            Ok(Box::new(MarkdownMemory::new(workspace_dir)))
        }
        MemoryBackendKind::LegacySurreal => {
            anyhow::bail!(
                "SurrealDB backend has been removed; use the Cerebro backend for long-term memory. See https://github.com/dallay/corvus/blob/main/clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md"
            );
        }
        MemoryBackendKind::None => unreachable!("checked above"),
    }
}

/// Factory: create an optional response cache from config.
pub fn create_response_cache(config: &MemoryConfig, workspace_dir: &Path) -> Option<ResponseCache> {
    if !config.response_cache_enabled {
        return None;
    }

    match ResponseCache::new(
        workspace_dir,
        config.response_cache_ttl_minutes,
        config.response_cache_max_entries,
    ) {
        Ok(cache) => {
            tracing::info!(
                "💾 Response cache enabled (TTL: {}min, max: {} entries)",
                config.response_cache_ttl_minutes,
                config.response_cache_max_entries
            );
            Some(cache)
        }
        Err(e) => {
            tracing::warn!("Response cache disabled due to error: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn factory_sqlite() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "sqlite".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn factory_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn factory_lucid() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "lucid".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "lucid");
    }

    #[test]
    fn factory_none_uses_noop_memory() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "none".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "none");
    }

    #[test]
    fn factory_unknown_falls_back_to_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "redis".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn migration_factory_lucid() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("lucid", tmp.path()).unwrap();
        assert_eq!(mem.name(), "lucid");
    }

    #[test]
    fn migration_factory_none_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let error = create_memory_for_migration("none", tmp.path())
            .err()
            .expect("backend=none should be rejected for migration");
        assert!(error.to_string().contains("disables persistence"));
    }


    #[test]
    fn factory_creates_correct_backend_types() {
        let tmp = TempDir::new().unwrap();

        for backend_key in ["sqlite", "lucid", "markdown", "none"] {
            let cfg = MemoryConfig {
                backend: backend_key.into(),
                ..MemoryConfig::default()
            };
            let mem = create_memory(&cfg, tmp.path(), None).unwrap();

            // Verify backend name matches expected
            match backend_key {
                "sqlite" => assert_eq!(mem.name(), "sqlite"),
                "lucid" => assert_eq!(mem.name(), "lucid"),
                "markdown" => assert_eq!(mem.name(), "markdown"),
                "none" => assert_eq!(mem.name(), "none"),
                _ => panic!("unexpected backend"),
            }
        }
    }

    #[test]
    fn response_cache_factory_disabled_by_default() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            response_cache_enabled: false,
            ..MemoryConfig::default()
        };
        let cache = create_response_cache(&cfg, tmp.path());
        assert!(cache.is_none());
    }

    #[test]
    fn response_cache_factory_creates_cache_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            response_cache_enabled: true,
            response_cache_ttl_minutes: 60,
            response_cache_max_entries: 100,
            ..MemoryConfig::default()
        };
        let cache = create_response_cache(&cfg, tmp.path());
        assert!(cache.is_some());
    }


    #[test]
    fn migration_factory_accepts_markdown() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("markdown", tmp.path()).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn migration_factory_accepts_sqlite() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("sqlite", tmp.path()).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn factory_handles_unknown_backend_gracefully() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "future-backend-2030".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        // Unknown backends should fall back to markdown
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn factory_with_api_key_passes_to_embedding_provider() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "sqlite".into(),
            embedding_provider: "openai".into(),
            ..MemoryConfig::default()
        };
        // Should not panic even with fake API key
        let _mem = create_memory(&cfg, tmp.path(), Some("sk-fake-key"));
    }

    #[test]
    fn migration_factory_with_unknown_backend_uses_markdown() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("redis", tmp.path()).unwrap();
        assert_eq!(mem.name(), "markdown");
    }
}
