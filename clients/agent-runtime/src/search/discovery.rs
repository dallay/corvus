use crate::security::SecurityPolicy;
use anyhow::{bail, Context};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct DiscoveryRules {
    pub max_file_size_bytes: u64,
    pub follow_links: bool,
    pub include_hidden: bool,
    pub max_files: Option<usize>,
}

impl Default for DiscoveryRules {
    fn default() -> Self {
        Self {
            max_file_size_bytes: DEFAULT_MAX_FILE_SIZE_BYTES,
            follow_links: false,
            include_hidden: false,
            max_files: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub resolved_path: PathBuf,
    pub relative_path: String,
    pub size_bytes: u64,
    pub modified_unix_ms: i64,
}

#[derive(Debug, Clone)]
pub struct DiscoveredFileContent {
    pub file: DiscoveredFile,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub files: Vec<DiscoveredFileContent>,
    pub visited_files: usize,
    pub hit_max_files: bool,
}

pub fn validate_search_root(
    security: &SecurityPolicy,
    relative_root: &str,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    if !security.is_path_allowed(relative_root) {
        bail!("Path not allowed by security policy: {relative_root}");
    }

    let full_path = security.workspace_dir.join(relative_root);
    let resolved_root = match fs::canonicalize(&full_path) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            bail!("Search path not found: {relative_root}");
        }
        Err(error) => {
            bail!("Failed to resolve search path: {error}");
        }
    };

    if !security.is_resolved_path_allowed(&resolved_root) {
        bail!(
            "Resolved path escapes workspace: {}",
            resolved_root.display()
        );
    }

    let metadata = fs::metadata(&resolved_root).context("Failed to read search path metadata")?;
    if !metadata.is_dir() {
        bail!("Search path is not a directory: {relative_root}");
    }

    let workspace_root = security
        .workspace_dir
        .canonicalize()
        .unwrap_or_else(|_| security.workspace_dir.clone());

    Ok((workspace_root, resolved_root))
}

pub fn normalize_relative_path(
    workspace_root: &Path,
    resolved_path: &Path,
) -> anyhow::Result<String> {
    let relative_path = resolved_path
        .strip_prefix(workspace_root)
        .with_context(|| {
            format!(
                "resolved path '{}' is outside workspace root '{}'",
                resolved_path.display(),
                workspace_root.display()
            )
        })?
        .to_string_lossy()
        .replace('\\', "/");

    Ok(relative_path)
}

pub fn is_index_artifact_path(relative_path: &str) -> bool {
    let normalized = relative_path.trim_start_matches("./").replace('\\', "/");
    normalized == "state/code-search/index.db"
        || normalized == "state/code-search/.index-build.lock"
        || normalized.starts_with("state/code-search/index.db-")
        || normalized.starts_with("state/code-search/.index-build.lock.")
        || normalized.starts_with("state/code-search/.index.db.tmp-")
}

pub fn is_binary_file(bytes: &[u8]) -> bool {
    // Check for text BOMs first (UTF-8, UTF-16, UTF-32)
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        // UTF-8 BOM
        return false;
    }
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 LE/BE BOM
        return false;
    }
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) || bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
    {
        // UTF-32 LE/BE BOM
        return false;
    }

    // Fall back to null-byte heuristic
    let sample_len = bytes.len().min(8 * 1024);
    bytes[..sample_len].contains(&0)
}

pub fn discover_searchable_files(
    security: &SecurityPolicy,
    relative_root: &str,
    include: &[String],
    exclude: &[String],
    rules: DiscoveryRules,
) -> anyhow::Result<Vec<DiscoveredFileContent>> {
    let result =
        discover_searchable_files_with_stats(security, relative_root, include, exclude, rules)?;
    Ok(result.files)
}

pub fn discover_searchable_files_with_stats(
    security: &SecurityPolicy,
    relative_root: &str,
    include: &[String],
    exclude: &[String],
    rules: DiscoveryRules,
) -> anyhow::Result<DiscoveryResult> {
    let (workspace_root, search_root) = validate_search_root(security, relative_root)?;
    let mut builder = WalkBuilder::new(&search_root);
    builder.standard_filters(true);
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);
    builder.parents(true);
    builder.follow_links(rules.follow_links);
    builder.hidden(!rules.include_hidden);
    builder.require_git(false);

    if !include.is_empty() || !exclude.is_empty() {
        let mut overrides = OverrideBuilder::new(&search_root);
        for pattern in include {
            overrides
                .add(pattern)
                .with_context(|| format!("Invalid include glob '{pattern}'"))?;
        }
        for pattern in exclude {
            overrides
                .add(&format!("!{pattern}"))
                .with_context(|| format!("Invalid exclude glob '{pattern}'"))?;
        }
        let overrides = overrides.build().context("Invalid search glob override")?;
        builder.overrides(overrides);
    }

    let mut discovered = Vec::new();
    let mut visited_files = 0usize;
    let mut hit_max_files = false;

    for entry in builder.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::debug!(error = %error, "search discovery skipped unreadable entry");
                continue;
            }
        };

        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        visited_files += 1;
        if let Some(max_files) = rules.max_files {
            if visited_files > max_files {
                hit_max_files = true;
                tracing::debug!(
                    max_files,
                    visited_files,
                    "search discovery stopped at file visit limit"
                );
                break;
            }
        }

        let entry_path = entry.into_path();
        let resolved_path = match fs::canonicalize(&entry_path) {
            Ok(path) => path,
            Err(error) => {
                tracing::debug!(path = %entry_path.display(), error = %error, "search discovery skipped unresolved path");
                continue;
            }
        };

        if !security.is_resolved_path_allowed(&resolved_path) {
            tracing::debug!(path = %resolved_path.display(), "search discovery skipped path escaping workspace");
            continue;
        }

        let metadata = match fs::metadata(&resolved_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                tracing::debug!(path = %resolved_path.display(), error = %error, "search discovery skipped unreadable metadata");
                continue;
            }
        };

        if !metadata.is_file() || metadata.len() > rules.max_file_size_bytes {
            continue;
        }

        let relative_path = match normalize_relative_path(&workspace_root, &resolved_path) {
            Ok(relative_path) => relative_path,
            Err(error) => {
                tracing::debug!(path = %resolved_path.display(), error = %error, "search discovery skipped non-workspace path");
                continue;
            }
        };

        if is_index_artifact_path(&relative_path) {
            continue;
        }

        let bytes = match fs::read(&resolved_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::debug!(path = %resolved_path.display(), error = %error, "search discovery skipped unreadable file");
                continue;
            }
        };

        if is_binary_file(&bytes) {
            continue;
        }

        // Note: modified_unix_ms = 0 signals unknown modification time (metadata read failure).
        // Downstream freshness checks must treat 0 as stale/unknown to force re-indexing.
        let modified_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or(0);

        discovered.push(DiscoveredFileContent {
            file: DiscoveredFile {
                resolved_path,
                relative_path,
                size_bytes: metadata.len(),
                modified_unix_ms,
            },
            bytes,
        });
    }

    discovered.sort_by(|left, right| left.file.relative_path.cmp(&right.file.relative_path));
    Ok(DiscoveryResult {
        files: discovered,
        visited_files,
        hit_max_files,
    })
}

pub fn discover_indexable_files(
    security: &SecurityPolicy,
    rules: DiscoveryRules,
) -> anyhow::Result<Vec<DiscoveredFileContent>> {
    let discovered = discover_searchable_files(security, ".", &[], &[], rules)?;
    Ok(discovered
        .into_iter()
        .filter(|entry| std::str::from_utf8(&entry.bytes).is_ok())
        .collect())
}
