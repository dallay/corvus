use crate::search::discovery::DiscoveredFile;
use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

pub const SCHEMA_VERSION: &str = "1";
pub const FORMAT_VERSION: &str = "1";
pub const DISCOVERY_RULES_VERSION: &str = "1";
pub const BUILDER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const BUILD_STATE_BUILDING: &str = "building";
pub const BUILD_STATE_READY: &str = "ready";
pub const BUILD_STATE_FAILED: &str = "failed";

pub const REQUIRED_METADATA_KEYS: &[&str] = &[
    "schema_version",
    "format_version",
    "workspace_fingerprint",
    "discovery_rules_version",
    "built_at",
    "build_state",
    "builder_version",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedFileRecord {
    pub relative_path: String,
    pub size_bytes: u64,
    pub modified_unix_ms: i64,
    pub trigram_count: u32,
    pub content_sha256: String,
}

pub fn create_temp_db_path(state_dir: &Path) -> PathBuf {
    state_dir.join(format!(".index.db.tmp-{}", Uuid::new_v4()))
}

pub fn open_connection(db_path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("failed to open SQLite database at '{}'", db_path.display()))?;
    conn.busy_timeout(Duration::from_millis(250))
        .context("failed to configure SQLite busy timeout")?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;",
    )
    .context("failed to configure SQLite pragmas")?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS files (
            file_id INTEGER PRIMARY KEY,
            relative_path TEXT NOT NULL UNIQUE,
            size_bytes INTEGER NOT NULL,
            modified_unix_ms INTEGER NOT NULL,
            trigram_count INTEGER NOT NULL,
            content_sha256 TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_files_relative_path ON files(relative_path);

        CREATE TABLE IF NOT EXISTS trigram_postings (
            trigram BLOB NOT NULL,
            file_id INTEGER NOT NULL,
            occurrences INTEGER NOT NULL,
            PRIMARY KEY (trigram, file_id),
            FOREIGN KEY (file_id) REFERENCES files(file_id) ON DELETE CASCADE
        ) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS idx_trigram_postings_file ON trigram_postings(file_id);",
    )
    .context("failed to initialize workspace trigram index schema")?;
    Ok(())
}

pub fn required_tables_exist(conn: &Connection) -> anyhow::Result<bool> {
    for table in ["metadata", "files", "trigram_postings"] {
        let exists: Option<String> = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .optional()
            .context("failed to inspect SQLite schema")?;
        if exists.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn read_metadata(conn: &Connection) -> anyhow::Result<BTreeMap<String, String>> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM metadata")
        .context("failed to prepare metadata query")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .context("failed to read metadata rows")?;

    let mut metadata = BTreeMap::new();
    for row in rows {
        let (key, value) = row.context("failed to decode metadata row")?;
        metadata.insert(key, value);
    }
    Ok(metadata)
}

pub fn write_metadata_tx(
    tx: &Transaction<'_>,
    metadata: &BTreeMap<String, String>,
) -> anyhow::Result<()> {
    for (key, value) in metadata {
        tx.execute(
            "INSERT INTO metadata(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .with_context(|| format!("failed to upsert metadata key '{key}'"))?;
    }
    Ok(())
}

pub fn read_files(conn: &Connection) -> anyhow::Result<BTreeMap<String, PersistedFileRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT relative_path, size_bytes, modified_unix_ms, trigram_count, content_sha256
             FROM files",
        )
        .context("failed to prepare files query")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PersistedFileRecord {
                relative_path: row.get(0)?,
                size_bytes: row.get::<_, i64>(1)?.try_into().unwrap_or(u64::MAX),
                modified_unix_ms: row.get(2)?,
                trigram_count: row.get::<_, i64>(3)?.try_into().unwrap_or(u32::MAX),
                content_sha256: row.get(4)?,
            })
        })
        .context("failed to read file rows")?;

    let mut files = BTreeMap::new();
    for row in rows {
        let record = row.context("failed to decode file row")?;
        files.insert(record.relative_path.clone(), record);
    }
    Ok(files)
}

pub fn replace_file_tx(
    tx: &Transaction<'_>,
    file: &DiscoveredFile,
    content_sha256: &str,
    trigram_counts: &BTreeMap<[u8; 3], u32>,
) -> anyhow::Result<()> {
    tx.execute(
        "DELETE FROM files WHERE relative_path = ?1",
        [&file.relative_path],
    )
    .with_context(|| format!("failed to delete prior file row '{}'", file.relative_path))?;

    tx.execute(
        "INSERT INTO files(relative_path, size_bytes, modified_unix_ms, trigram_count, content_sha256)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            file.relative_path,
            i64::try_from(file.size_bytes).unwrap_or(i64::MAX),
            file.modified_unix_ms,
            i64::try_from(trigram_counts.len()).unwrap_or(i64::MAX),
            content_sha256,
        ],
    )
    .with_context(|| format!("failed to insert file row '{}'", file.relative_path))?;

    let file_id = tx.last_insert_rowid();
    for (trigram, occurrences) in trigram_counts {
        tx.execute(
            "INSERT INTO trigram_postings(trigram, file_id, occurrences) VALUES(?1, ?2, ?3)",
            params![trigram.as_slice(), file_id, i64::from(*occurrences)],
        )
        .with_context(|| {
            format!(
                "failed to insert trigram postings for '{}'",
                file.relative_path
            )
        })?;
    }

    Ok(())
}

pub fn delete_file_tx(tx: &Transaction<'_>, relative_path: &str) -> anyhow::Result<()> {
    tx.execute(
        "DELETE FROM files WHERE relative_path = ?1",
        [relative_path],
    )
    .with_context(|| format!("failed to delete removed file row '{relative_path}'"))?;
    Ok(())
}
