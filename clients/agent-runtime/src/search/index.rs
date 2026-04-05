use crate::search::discovery::{
    discover_indexable_files, discover_searchable_files_with_stats, DiscoveryRules,
    DiscoveryRules as SearchDiscoveryRules,
};
use crate::search::sqlite::{
    create_temp_db_path, delete_file_tx, init_schema, open_connection, read_candidate_paths,
    read_files, read_index_file_count, read_metadata, replace_file_tx, required_tables_exist,
    write_metadata_tx, PersistedFileRecord, BUILDER_VERSION, BUILD_STATE_BUILDING,
    BUILD_STATE_READY, DISCOVERY_RULES_VERSION, FORMAT_VERSION, REQUIRED_METADATA_KEYS,
    SCHEMA_VERSION,
};
use crate::search::trigram::{trigram_counts, validate_utf8_text};
use crate::security::SecurityPolicy;
use anyhow::{bail, Context};
use chrono::Utc;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedIndex {
    pub db_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBuildReport {
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub trigrams_written: usize,
    pub built_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRefreshDecision {
    pub action: RefreshAction,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshAction {
    LoadExisting,
    Rebuild,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTrigramIndex {
    workspace_dir: PathBuf,
    db_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateCoverage {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRequest {
    pub relative_root: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub raw_pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePlan {
    pub coverage: CandidateCoverage,
    pub ordered_paths: Vec<String>,
    pub reason: String,
}

impl WorkspaceTrigramIndex {
    pub fn for_workspace(workspace_dir: &Path) -> Self {
        Self {
            workspace_dir: workspace_dir.to_path_buf(),
            db_path: workspace_dir.join("state/code-search/index.db"),
        }
    }

    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Ensure the workspace directory of this index matches the security policy's workspace.
    fn ensure_workspace_matches(&self, security: &SecurityPolicy) -> anyhow::Result<()> {
        let index_workspace = self.workspace_dir.canonicalize().with_context(|| {
            format!(
                "failed to canonicalize index workspace dir '{}'",
                self.workspace_dir.display()
            )
        })?;
        let security_workspace = security.workspace_dir.canonicalize().with_context(|| {
            format!(
                "failed to canonicalize security workspace dir '{}'",
                security.workspace_dir.display()
            )
        })?;
        anyhow::ensure!(
            index_workspace == security_workspace,
            "workspace mismatch: index expects '{}' but security policy uses '{}'",
            index_workspace.display(),
            security_workspace.display()
        );
        Ok(())
    }

    fn checked_state_paths(&self) -> anyhow::Result<CheckedStatePaths> {
        let workspace_root = self.workspace_dir.canonicalize().with_context(|| {
            format!(
                "failed to canonicalize workspace root '{}'",
                self.workspace_dir.display()
            )
        })?;

        let state_root = workspace_root.join("state");
        let state_dir = state_root.join("code-search");
        let db_path = state_dir.join("index.db");

        reject_if_symlink(&state_root, "state root")?;
        reject_if_symlink(&state_dir, "code search state dir")?;
        reject_if_symlink(&db_path, "index db")?;

        anyhow::ensure!(state_root.starts_with(&workspace_root));
        anyhow::ensure!(state_dir.starts_with(&workspace_root));
        anyhow::ensure!(db_path.starts_with(&workspace_root));

        Ok(CheckedStatePaths {
            workspace_root,
            state_dir,
            db_path,
        })
    }

    pub fn load(&self) -> anyhow::Result<LoadedIndex> {
        let checked = self.checked_state_paths()?;
        if !checked.db_path.exists() {
            bail!(
                "workspace trigram index is missing at '{}'; run refresh_or_rebuild first",
                checked.db_path.display()
            );
        }
        let conn = open_connection(&checked.db_path)?;
        let decision = self.compatibility_decision(&conn)?;
        if decision.action == RefreshAction::Rebuild {
            bail!(
                "workspace trigram index is not loadable: {}",
                decision.reasons.join(", ")
            );
        }
        Ok(LoadedIndex {
            db_path: checked.db_path,
        })
    }

    pub fn plan_candidates(
        &self,
        security: &SecurityPolicy,
        request: &CandidateRequest,
        max_file_size_bytes: u64,
    ) -> anyhow::Result<CandidatePlan> {
        self.ensure_workspace_matches(security)?;

        let required_trigrams = match extract_required_trigrams(request) {
            Ok(required_trigrams) => required_trigrams,
            Err(reason) => {
                return Ok(CandidatePlan {
                    coverage: CandidateCoverage::Unavailable,
                    ordered_paths: Vec::new(),
                    reason,
                });
            }
        };

        let loaded = match self.load() {
            Ok(loaded) => loaded,
            Err(_) => {
                return Ok(CandidatePlan {
                    coverage: CandidateCoverage::Unavailable,
                    ordered_paths: Vec::new(),
                    reason: "index_unavailable".to_string(),
                });
            }
        };

        let conn = match open_connection(&loaded.db_path) {
            Ok(conn) => conn,
            Err(_) => {
                return Ok(CandidatePlan {
                    coverage: CandidateCoverage::Unavailable,
                    ordered_paths: Vec::new(),
                    reason: "index_query_failed".to_string(),
                });
            }
        };

        let root_prefix = normalized_relative_root(&request.relative_root);
        let mut ordered_paths =
            match read_candidate_paths(&conn, &required_trigrams, root_prefix.as_deref()) {
                Ok(paths) => paths,
                Err(_) => {
                    return Ok(CandidatePlan {
                        coverage: CandidateCoverage::Unavailable,
                        ordered_paths: Vec::new(),
                        reason: "index_query_failed".to_string(),
                    });
                }
            };

        let discovered = discover_searchable_files_with_stats(
            security,
            &request.relative_root,
            &request.include,
            &request.exclude,
            SearchDiscoveryRules {
                max_file_size_bytes,
                max_files: None,
                ..SearchDiscoveryRules::default()
            },
        )?;

        let discovered_paths: BTreeSet<String> = discovered
            .files
            .iter()
            .map(|entry| entry.file.relative_path.clone())
            .collect();
        ordered_paths.retain(|path| discovered_paths.contains(path));

        let indexed_files = read_files(&conn)?;
        let indexed_file_count = read_index_file_count(&conn)?;
        let parity_complete = indexed_file_count >= discovered_paths.len()
            && discovered.files.iter().all(|entry| {
                indexed_files
                    .get(&entry.file.relative_path)
                    .map(|record| {
                        record.size_bytes == entry.file.size_bytes
                            && record.modified_unix_ms == entry.file.modified_unix_ms
                            && record.modified_unix_ms != 0
                    })
                    .unwrap_or(false)
            });

        let (coverage, reason) = if parity_complete {
            (CandidateCoverage::Complete, "indexed_candidates_complete")
        } else {
            (CandidateCoverage::Partial, "index_parity_requires_fallback")
        };

        Ok(CandidatePlan {
            coverage,
            ordered_paths,
            reason: reason.to_string(),
        })
    }

    pub fn build(&self, security: Arc<SecurityPolicy>) -> anyhow::Result<IndexBuildReport> {
        self.ensure_workspace_matches(&security)?;

        let checked = self.checked_state_paths()?;
        fs::create_dir_all(&checked.state_dir).with_context(|| {
            format!(
                "failed to create index state directory '{}'",
                checked.state_dir.display()
            )
        })?;

        // Acquire exclusive build lock before expensive work
        let _lock = acquire_build_lock(&checked.state_dir)?;

        self.build_locked(&security, &checked)
    }

    fn build_locked(
        &self,
        security: &SecurityPolicy,
        checked: &CheckedStatePaths,
    ) -> anyhow::Result<IndexBuildReport> {
        let discovered = discover_indexable_files(
            security,
            DiscoveryRules {
                use_workspace_local_ignores_only: true,
                ..DiscoveryRules::default()
            },
        )?;
        let built_at = Utc::now().to_rfc3339();
        let workspace_fingerprint = workspace_fingerprint(&checked.workspace_root)?;

        let temp_db_path = create_temp_db_path(&checked.state_dir);
        let build_result = self.build_into_path(
            &temp_db_path,
            &discovered,
            &workspace_fingerprint,
            &built_at,
        );

        let build_report = match build_result {
            Ok(report) => report,
            Err(error) => {
                let _ = fs::remove_file(&temp_db_path);
                return Err(error);
            }
        };

        publish_index_db(&temp_db_path, &checked.db_path)?;

        Ok(build_report)
    }

    pub fn refresh_or_rebuild(
        &self,
        security: Arc<SecurityPolicy>,
    ) -> anyhow::Result<IndexBuildReport> {
        self.ensure_workspace_matches(&security)?;

        let checked = self.checked_state_paths()?;
        fs::create_dir_all(&checked.state_dir).with_context(|| {
            format!(
                "failed to create index state directory '{}'",
                checked.state_dir.display()
            )
        })?;

        // Acquire exclusive build lock before any work
        let _lock = acquire_build_lock(&checked.state_dir)?;

        if !checked.db_path.exists() {
            return self.build_locked(&security, &checked);
        }

        let conn = match open_connection(&checked.db_path) {
            Ok(conn) => conn,
            Err(_) => return self.build_locked(&security, &checked),
        };

        let decision = match self.compatibility_decision(&conn) {
            Ok(decision) => decision,
            Err(_) => {
                drop(conn);
                return self.build_locked(&security, &checked);
            }
        };

        if decision.action == RefreshAction::Rebuild {
            drop(conn);
            return self.build_locked(&security, &checked);
        }

        let metadata = read_metadata(&conn)?;
        let existing_files = read_files(&conn)?;
        let discovered = discover_indexable_files(
            &security,
            DiscoveryRules {
                use_workspace_local_ignores_only: true,
                ..DiscoveryRules::default()
            },
        )?;
        let current_rows = prepare_current_rows(&discovered)?;

        let removed_paths: Vec<_> = existing_files
            .keys()
            .filter(|path| !current_rows.contains_key(*path))
            .cloned()
            .collect();
        let changed_paths: Vec<_> = current_rows
            .iter()
            .filter(|(path, current)| match existing_files.get(*path) {
                Some(existing) => {
                    // Treat modified_unix_ms = 0 as unknown/stale, forcing re-index
                    existing.modified_unix_ms == 0
                        || current.record.modified_unix_ms == 0
                        || existing != &current.record
                }
                None => true,
            })
            .map(|(path, _)| path.clone())
            .collect();

        if changed_paths.is_empty() && removed_paths.is_empty() {
            return Ok(IndexBuildReport {
                files_indexed: existing_files.len(),
                files_skipped: existing_files.len(),
                trigrams_written: 0,
                built_at: metadata.get("built_at").cloned().unwrap_or_default(),
            });
        }

        let built_at = Utc::now().to_rfc3339();
        let mut next_metadata =
            build_metadata(&workspace_fingerprint(&checked.workspace_root)?, &built_at);
        next_metadata.insert("build_state".to_string(), BUILD_STATE_READY.to_string());

        let tx = conn
            .unchecked_transaction()
            .context("failed to open workspace trigram refresh transaction")?;
        for path in &removed_paths {
            delete_file_tx(&tx, path)?;
        }
        let mut trigrams_written = 0usize;
        for path in &changed_paths {
            let row = current_rows
                .get(path)
                .expect("changed path must exist in current rows");
            replace_file_tx(&tx, &row.file, &row.record.content_sha256, &row.trigrams)?;
            trigrams_written += row.trigrams.len();
        }
        write_metadata_tx(&tx, &next_metadata)?;
        tx.commit()
            .context("failed to commit workspace trigram refresh transaction")?;

        Ok(IndexBuildReport {
            files_indexed: current_rows.len(),
            files_skipped: current_rows.len().saturating_sub(changed_paths.len()),
            trigrams_written,
            built_at,
        })
    }

    fn build_into_path(
        &self,
        db_path: &Path,
        discovered: &[crate::search::discovery::DiscoveredFileContent],
        workspace_fingerprint: &str,
        built_at: &str,
    ) -> anyhow::Result<IndexBuildReport> {
        let mut conn = open_connection(db_path)?;
        init_schema(&conn)?;

        let current_rows = prepare_current_rows(discovered)?;
        let mut metadata = build_metadata(workspace_fingerprint, built_at);
        metadata.insert("build_state".to_string(), BUILD_STATE_BUILDING.to_string());

        let tx = conn
            .transaction()
            .context("failed to open workspace trigram build transaction")?;
        write_metadata_tx(&tx, &metadata)?;

        let mut trigrams_written = 0usize;
        for row in current_rows.values() {
            replace_file_tx(&tx, &row.file, &row.record.content_sha256, &row.trigrams)?;
            trigrams_written += row.trigrams.len();
        }

        metadata.insert("build_state".to_string(), BUILD_STATE_READY.to_string());
        write_metadata_tx(&tx, &metadata)?;
        tx.commit()
            .context("failed to commit workspace trigram build transaction")?;
        conn.execute(
            "INSERT INTO metadata(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            ["build_state", BUILD_STATE_READY],
        )
        .context("failed to finalize workspace trigram build state")?;

        Ok(IndexBuildReport {
            files_indexed: current_rows.len(),
            files_skipped: 0,
            trigrams_written,
            built_at: built_at.to_string(),
        })
    }

    fn compatibility_decision(
        &self,
        conn: &rusqlite::Connection,
    ) -> anyhow::Result<IndexRefreshDecision> {
        let mut reasons = Vec::new();
        if !required_tables_exist(conn)? {
            reasons.push("required tables missing".to_string());
        }

        let metadata = read_metadata(conn)?;
        for key in REQUIRED_METADATA_KEYS {
            if !metadata.contains_key(*key) {
                reasons.push(format!("missing metadata key '{key}'"));
            }
        }

        if metadata.get("schema_version").map(String::as_str) != Some(SCHEMA_VERSION) {
            reasons.push("schema version mismatch".to_string());
        }
        if metadata.get("format_version").map(String::as_str) != Some(FORMAT_VERSION) {
            reasons.push("format version mismatch".to_string());
        }
        if metadata.get("discovery_rules_version").map(String::as_str)
            != Some(DISCOVERY_RULES_VERSION)
        {
            reasons.push("discovery rules version mismatch".to_string());
        }
        if metadata.get("builder_version").map(String::as_str) != Some(BUILDER_VERSION) {
            reasons.push("builder version mismatch".to_string());
        }
        if metadata.get("build_state").map(String::as_str) != Some(BUILD_STATE_READY) {
            reasons.push("build state is not ready".to_string());
        }

        let checked = self.checked_state_paths()?;
        let expected_fingerprint = workspace_fingerprint(&checked.workspace_root)?;
        if metadata.get("workspace_fingerprint").map(String::as_str)
            != Some(expected_fingerprint.as_str())
        {
            reasons.push("workspace fingerprint mismatch".to_string());
        }

        if reasons.is_empty() {
            Ok(IndexRefreshDecision {
                action: RefreshAction::LoadExisting,
                reasons,
            })
        } else {
            Ok(IndexRefreshDecision {
                action: RefreshAction::Rebuild,
                reasons,
            })
        }
    }
}

fn normalized_relative_root(relative_root: &str) -> Option<String> {
    let trimmed = relative_root.trim();
    if trimmed.is_empty() || trimmed == "." {
        None
    } else {
        Some(trimmed.trim_matches('/').to_string())
    }
}

fn extract_required_trigrams(request: &CandidateRequest) -> Result<Vec<[u8; 3]>, String> {
    if request.is_regex {
        return Err("query_regex_not_supported".to_string());
    }
    if !request.case_sensitive {
        return Err("query_case_insensitive_not_supported".to_string());
    }
    if request.whole_word {
        return Err("query_whole_word_not_supported".to_string());
    }

    let bytes = request.raw_pattern.as_bytes();
    if bytes.len() < 3 {
        return Err("query_pattern_too_short".to_string());
    }

    let mut trigrams = BTreeSet::new();
    for window in bytes.windows(3) {
        trigrams.insert([window[0], window[1], window[2]]);
    }
    if trigrams.is_empty() {
        return Err("query_pattern_too_short".to_string());
    }

    Ok(trigrams.into_iter().collect())
}

#[derive(Debug, Clone)]
struct CurrentRow {
    file: crate::search::discovery::DiscoveredFile,
    record: PersistedFileRecord,
    trigrams: BTreeMap<[u8; 3], u32>,
}

#[derive(Debug, Clone)]
struct CheckedStatePaths {
    workspace_root: PathBuf,
    state_dir: PathBuf,
    db_path: PathBuf,
}

fn reject_if_symlink(path: &Path, label: &str) -> anyhow::Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        anyhow::ensure!(
            !metadata.file_type().is_symlink(),
            "{} '{}' must not be a symlink",
            label,
            path.display()
        );
    }
    Ok(())
}

/// Acquire an exclusive workspace-wide lock for index building.
/// Returns the lock file handle which must be kept alive until the build completes.
fn acquire_build_lock(state_dir: &Path) -> anyhow::Result<File> {
    let lock_path = state_dir.join(".index-build.lock");
    reject_if_symlink(&lock_path, "index build lock")?;
    let lock_file = File::create(&lock_path).with_context(|| {
        format!(
            "failed to create index build lock file at '{}'",
            lock_path.display()
        )
    })?;

    // Try to acquire exclusive lock with short timeout (matching SQLite busy_timeout)
    lock_file.try_lock_exclusive().with_context(|| {
        format!(
            "index build already in progress (lock held at '{}')",
            lock_path.display()
        )
    })?;

    Ok(lock_file)
}

fn build_metadata(workspace_fingerprint: &str, built_at: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("schema_version".to_string(), SCHEMA_VERSION.to_string()),
        ("format_version".to_string(), FORMAT_VERSION.to_string()),
        (
            "workspace_fingerprint".to_string(),
            workspace_fingerprint.to_string(),
        ),
        (
            "discovery_rules_version".to_string(),
            DISCOVERY_RULES_VERSION.to_string(),
        ),
        ("built_at".to_string(), built_at.to_string()),
        ("build_state".to_string(), BUILD_STATE_BUILDING.to_string()),
        ("builder_version".to_string(), BUILDER_VERSION.to_string()),
    ])
}

fn prepare_current_rows(
    discovered: &[crate::search::discovery::DiscoveredFileContent],
) -> anyhow::Result<BTreeMap<String, CurrentRow>> {
    let mut rows = BTreeMap::new();
    for entry in discovered {
        validate_utf8_text(&entry.bytes)?;
        let trigrams = trigram_counts(&entry.bytes);
        let record = PersistedFileRecord {
            relative_path: entry.file.relative_path.clone(),
            size_bytes: entry.file.size_bytes,
            modified_unix_ms: entry.file.modified_unix_ms,
            trigram_count: u32::try_from(trigrams.len()).unwrap_or(u32::MAX),
            content_sha256: sha256_hex(&entry.bytes),
        };
        rows.insert(
            entry.file.relative_path.clone(),
            CurrentRow {
                file: entry.file.clone(),
                record,
                trigrams,
            },
        );
    }
    Ok(rows)
}

fn workspace_fingerprint(workspace_dir: &Path) -> anyhow::Result<String> {
    let canonical = workspace_dir.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize workspace root '{}' for fingerprint",
            workspace_dir.display()
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    hasher.update(b"\0workspace-trigram-index\0");
    hasher.update(SCHEMA_VERSION.as_bytes());
    hasher.update(FORMAT_VERSION.as_bytes());
    hasher.update(DISCOVERY_RULES_VERSION.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(not(windows))]
fn publish_index_db(temp_db_path: &Path, db_path: &Path) -> anyhow::Result<()> {
    fs::rename(temp_db_path, db_path).with_context(|| {
        format!(
            "failed to atomically publish workspace trigram index from '{}' to '{}'",
            temp_db_path.display(),
            db_path.display()
        )
    })?;
    Ok(())
}

#[cfg(windows)]
fn publish_index_db(temp_db_path: &Path, db_path: &Path) -> anyhow::Result<()> {
    let backup_path = db_path.with_extension("db.bak");

    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    if db_path.exists() {
        fs::rename(db_path, &backup_path).with_context(|| {
            format!(
                "failed to move previous workspace trigram index '{}' to backup '{}'",
                db_path.display(),
                backup_path.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(temp_db_path, db_path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, db_path);
        }
        return Err(error).with_context(|| {
            format!(
                "failed to publish workspace trigram index from '{}' to '{}'",
                temp_db_path.display(),
                db_path.display()
            )
        });
    }

    if backup_path.exists() {
        fs::remove_file(&backup_path).with_context(|| {
            format!(
                "failed to remove workspace trigram backup '{}' after publish",
                backup_path.display()
            )
        })?;
    }

    Ok(())
}
