use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Default official skills repository.
pub const OFFICIAL_REPO: &str = "https://github.com/dallay/corvus-skills";

/// Raw GitHub URL for fetching the latest index.
pub const OFFICIAL_INDEX_URL: &str =
    "https://raw.githubusercontent.com/dallay/corvus-skills/main/index.toml";

/// Default cache TTL in hours.
pub const DEFAULT_CACHE_TTL_HOURS: u64 = 24;

/// HTTP fetch timeout in seconds.
const INDEX_FETCH_TIMEOUT_SECS: u64 = 3;

/// Embedded catalog index snapshot (compiled in at build time).
const EMBEDDED_INDEX: &str = include_str!(concat!(env!("OUT_DIR"), "/catalog_index.toml"));

/// Catalog index schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogIndex {
    pub meta: CatalogMeta,
    #[serde(default)]
    pub skills: BTreeMap<String, CatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMeta {
    pub version: u32,
    pub generated_at: String,
    #[serde(default)]
    pub repo_url: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    pub path: String,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Parse a TOML string into a `CatalogIndex`.
/// Validates schema version == 1.
pub fn parse_index(content: &str) -> anyhow::Result<CatalogIndex> {
    let index: CatalogIndex = toml::from_str(content)?;
    if index.meta.version != 1 {
        anyhow::bail!(
            "unsupported catalog index version: {} (expected 1)",
            index.meta.version
        );
    }
    Ok(index)
}

/// Returns true if the source string looks like a bare catalog name
/// (no `/`, `\`, `.`, or `:`).
pub fn is_bare_name(source: &str) -> bool {
    !source.is_empty()
        && !source.contains('/')
        && !source.contains('\\')
        && !source.contains('.')
        && !source.contains(':')
}

/// Resolve the best available catalog index.
/// Priority: fresh cache → fetch (if stale) → embedded.
pub fn resolve_index(
    workspace_dir: &Path,
    config: &crate::config::SkillsConfig,
) -> anyhow::Result<CatalogIndex> {
    let cache_dir = workspace_dir.join(".catalog-cache");
    let cache_path = cache_dir.join("index.toml");
    let ttl_hours = config
        .catalog_cache_ttl_hours
        .unwrap_or(DEFAULT_CACHE_TTL_HOURS);
    let ttl = Duration::from_secs(ttl_hours * 3600);

    // Try cached index if fresh
    if let Some(index) = try_cached_index(&cache_path, ttl) {
        return Ok(index);
    }

    // Try fetching fresh index
    let index_url = config
        .catalog_repo_url
        .as_deref()
        .unwrap_or(OFFICIAL_INDEX_URL);

    if let Some(index) = try_fetch_index(index_url, &cache_dir, &cache_path) {
        return Ok(index);
    }

    // Fall back to embedded
    tracing::debug!("using embedded catalog index");
    parse_index(EMBEDDED_INDEX)
}

fn try_cached_index(cache_path: &Path, ttl: Duration) -> Option<CatalogIndex> {
    let content = std::fs::read_to_string(cache_path).ok()?;
    let metadata = std::fs::metadata(cache_path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::MAX);

    if age <= ttl {
        match parse_index(&content) {
            Ok(index) => {
                tracing::debug!("using cached catalog index");
                Some(index)
            }
            Err(err) => {
                tracing::warn!("corrupt cached catalog index: {err}");
                None
            }
        }
    } else {
        tracing::debug!("cached catalog index is stale ({age:?} old)");
        None
    }
}

fn try_fetch_index(url: &str, cache_dir: &Path, cache_path: &Path) -> Option<CatalogIndex> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(INDEX_FETCH_TIMEOUT_SECS))
        .build()
        .ok()?;

    let resp = match client.get(url).send() {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            tracing::debug!("catalog index fetch returned {}", r.status());
            return None;
        }
        Err(err) => {
            tracing::debug!("catalog index fetch failed: {err}");
            return None;
        }
    };

    let body = resp.text().ok()?;
    let index = match parse_index(&body) {
        Ok(idx) => idx,
        Err(err) => {
            tracing::warn!("fetched catalog index is invalid: {err}");
            return None;
        }
    };

    // Write to cache
    if let Err(err) = std::fs::create_dir_all(cache_dir) {
        tracing::debug!("failed to create catalog cache dir: {err}");
    } else if let Err(err) = std::fs::write(cache_path, &body) {
        tracing::debug!("failed to write catalog cache: {err}");
    }

    Some(index)
}

/// Parse the embedded index (for offline-only operations).
pub fn embedded_index() -> anyhow::Result<CatalogIndex> {
    parse_index(EMBEDDED_INDEX)
}

/// Search the catalog for entries matching the query.
/// Case-insensitive substring match against name, description, and tags.
pub fn search<'a>(index: &'a CatalogIndex, query: &str) -> Vec<&'a CatalogEntry> {
    let query_lower = query.to_lowercase();
    index
        .skills
        .values()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&query_lower)
                || entry.description.to_lowercase().contains(&query_lower)
                || entry
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_index_toml() -> &'static str {
        r#"
[meta]
version = 1
generated_at = "2026-03-24T00:00:00Z"
repo_url = "https://github.com/dallay/corvus-skills"
commit = "abc123"

[skills.git-expert]
name = "git-expert"
description = "Git operations and workflow expert"
version = "0.1.0"
path = "skills/git-expert"
content_hash = "sha256:aaa"
author = "Corvus Team"
tags = ["git", "vcs", "workflow"]

[skills.rust-expert]
name = "rust-expert"
description = "Rust language patterns and best practices"
version = "0.2.0"
path = "skills/rust-expert"
author = "Corvus Team"
tags = ["rust", "systems"]
"#
    }

    // ── 1. Parse valid index with skills ─────────────────────────

    #[test]
    fn parse_valid_index_with_skills() {
        let index = parse_index(sample_index_toml()).unwrap();
        assert_eq!(index.meta.version, 1);
        assert_eq!(index.skills.len(), 2);
        let git = &index.skills["git-expert"];
        assert_eq!(git.name, "git-expert");
        assert_eq!(git.path, "skills/git-expert");
        assert_eq!(git.tags, vec!["git", "vcs", "workflow"]);
    }

    // ── 2. Unknown version → error containing version number ─────

    #[test]
    fn parse_index_unknown_version() {
        let content = r#"
[meta]
version = 99
generated_at = "2026-01-01T00:00:00Z"
"#;
        let err = parse_index(content).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("99"), "error should contain version: {msg}");
        assert!(
            msg.contains("unsupported"),
            "error should mention unsupported: {msg}"
        );
    }

    // ── 3. Missing required field → error ────────────────────────

    #[test]
    fn parse_index_missing_required_field() {
        // Missing `generated_at` in meta
        let content = r#"
[meta]
version = 1
"#;
        let err = parse_index(content).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("generated_at"),
            "error should mention missing field: {msg}"
        );
    }

    // ── 4-7. is_bare_name tests ──────────────────────────────────

    #[test]
    fn is_bare_name_simple_name() {
        assert!(is_bare_name("git-expert"));
    }

    #[test]
    fn is_bare_name_url_false() {
        assert!(!is_bare_name("https://github.com/x/y"));
    }

    #[test]
    fn is_bare_name_local_path_false() {
        assert!(!is_bare_name("./local"));
    }

    #[test]
    fn is_bare_name_empty_false() {
        assert!(!is_bare_name(""));
    }

    // ── 8. Search with partial match ─────────────────────────────

    #[test]
    fn search_partial_match() {
        let index = parse_index(sample_index_toml()).unwrap();
        let results = search(&index, "git");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "git-expert");
    }

    // ── 9. Search case-insensitive ───────────────────────────────

    #[test]
    fn search_case_insensitive() {
        let index = parse_index(sample_index_toml()).unwrap();
        let results = search(&index, "RUST");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-expert");
    }

    // ── 10. Search by tag ────────────────────────────────────────

    #[test]
    fn search_by_tag() {
        let index = parse_index(sample_index_toml()).unwrap();
        let results = search(&index, "vcs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "git-expert");
    }

    // ── 11. Search no results ────────────────────────────────────

    #[test]
    fn search_no_results() {
        let index = parse_index(sample_index_toml()).unwrap();
        let results = search(&index, "nonexistent-xyz");
        assert!(results.is_empty());
    }

    // ── 12. Embedded index parses successfully ───────────────────

    #[test]
    fn embedded_index_parses_successfully() {
        let index = embedded_index().expect("embedded index should parse");
        assert_eq!(index.meta.version, 1);
    }

    // ── 13. resolve_index uses embedded when no cache ────────────

    #[test]
    fn resolve_index_uses_embedded_when_no_cache() {
        let dir = tempfile::tempdir().unwrap();
        let config = crate::config::SkillsConfig::default();
        // No cache file exists, fetch will fail (no network in test)
        // Should fall back to embedded
        let result = resolve_index(dir.path(), &config);
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.meta.version, 1);
    }

    // ── 14. resolve_index returns cached when fresh ──────────────

    #[test]
    fn resolve_index_returns_cached_when_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join(".catalog-cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        // Write a valid index as cache (it's fresh — just written)
        std::fs::write(cache_dir.join("index.toml"), sample_index_toml()).unwrap();

        let config = crate::config::SkillsConfig::default();
        let result = resolve_index(dir.path(), &config);
        assert!(result.is_ok());
        let index = result.unwrap();
        // Should return our cached index with 2 skills, not the embedded one
        assert_eq!(index.skills.len(), 2);
        assert!(index.skills.contains_key("git-expert"));
    }

    // ── 15. resolve_index skips stale cache and falls back ───────

    #[test]
    fn resolve_index_skips_stale_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join(".catalog-cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_path = cache_dir.join("index.toml");
        std::fs::write(&cache_path, sample_index_toml()).unwrap();

        // TTL of 0 hours means any non-zero age is stale
        let config = crate::config::SkillsConfig {
            catalog_cache_ttl_hours: Some(0),
            ..Default::default()
        };
        // Cache is stale (age > 0s), fetch will fail (no network), falls back to embedded
        let result = resolve_index(dir.path(), &config);
        assert!(result.is_ok());
    }
}
