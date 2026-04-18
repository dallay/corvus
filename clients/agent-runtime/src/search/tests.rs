use super::discovery::{
    discover_indexable_files, discover_metadata_files_with_stats, discover_searchable_files,
    DiscoveryRules,
};
use super::index::{CandidateCoverage, CandidateRequest, WorkspaceTrigramIndex};
use super::sqlite::{
    read_candidate_paths, read_index_file_count, BUILD_STATE_BUILDING, BUILD_STATE_READY,
    FORMAT_VERSION,
};
use super::trigram::trigram_counts;
use crate::security::{AutonomyLevel, SecurityPolicy};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn test_security(workspace: &TempDir) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace.path().to_path_buf(),
        ..SecurityPolicy::default()
    })
}

fn file_rows(workspace: &TempDir) -> Vec<(String, i64, String)> {
    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT relative_path, trigram_count, content_sha256 FROM files ORDER BY relative_path",
        )
        .unwrap();
    stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

fn metadata_value(workspace: &TempDir, key: &str) -> String {
    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    conn.query_row("SELECT value FROM metadata WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .unwrap()
}

fn plan_for_literal(
    index: &WorkspaceTrigramIndex,
    security: &Arc<SecurityPolicy>,
    pattern: &str,
) -> super::index::CandidatePlan {
    index
        .plan_candidates(
            security,
            &CandidateRequest {
                relative_root: ".".to_string(),
                include: Vec::new(),
                exclude: Vec::new(),
                raw_pattern: pattern.to_string(),
                is_regex: false,
                case_sensitive: true,
                whole_word: false,
            },
            DiscoveryRules::default().max_file_size_bytes,
        )
        .unwrap()
}

fn trigram_rows(workspace: &TempDir) -> Vec<(Vec<u8>, String, i64)> {
    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    let mut stmt = conn
        .prepare(
        "SELECT p.trigram, f.relative_path, p.occurrences\n         FROM trigram_postings p\n         JOIN files f ON f.file_id = p.file_id\n         ORDER BY f.relative_path, p.trigram",
    )
        .unwrap();
    stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

#[test]
fn discovery_excludes_invalid_utf8_and_binary_content() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("good.txt"), "hello world\n").unwrap();
    std::fs::write(workspace.path().join("bad.bin"), [0, 1, 2, 3]).unwrap();
    std::fs::write(workspace.path().join("bad.txt"), [0xFF, 0xFE, 0xFD]).unwrap();

    let files =
        discover_indexable_files(&test_security(&workspace), DiscoveryRules::default()).unwrap();
    let paths: Vec<_> = files
        .iter()
        .map(|entry| entry.file.relative_path.as_str())
        .collect();
    assert_eq!(paths, vec!["good.txt"]);
}

#[test]
fn metadata_discovery_returns_workspace_relative_matches_in_stable_order() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/lib")).unwrap();
    std::fs::write(workspace.path().join("src/main.ts"), "main\n").unwrap();
    std::thread::sleep(Duration::from_millis(10));
    std::fs::write(workspace.path().join("src/lib/util.ts"), "util\n").unwrap();

    let result = discover_metadata_files_with_stats(
        &test_security(&workspace),
        ".",
        "src/**/*.ts",
        DiscoveryRules::default(),
    )
    .unwrap();

    let paths: Vec<_> = result
        .files
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect();
    assert_eq!(paths, vec!["src/lib/util.ts", "src/main.ts"]);
    assert!(!result.hit_max_files);
}

#[test]
fn discovery_excludes_hidden_ignored_oversized_and_index_artifacts() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::create_dir_all(workspace.path().join(".hidden")).unwrap();
    std::fs::create_dir_all(workspace.path().join("target")).unwrap();
    std::fs::create_dir_all(workspace.path().join("state/code-search")).unwrap();
    std::fs::write(workspace.path().join(".gitignore"), "target/\n").unwrap();
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn ok() {}\n").unwrap();
    std::fs::write(workspace.path().join(".hidden/secret.rs"), "secret\n").unwrap();
    std::fs::write(workspace.path().join("target/generated.rs"), "ignored\n").unwrap();
    std::fs::write(
        workspace.path().join("state/code-search/index.db"),
        b"sqlite",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("state/code-search/index.db-wal"),
        b"sidecar",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("too-large.txt"),
        vec![b'x'; (10 * 1024 * 1024) + 1],
    )
    .unwrap();

    let files =
        discover_indexable_files(&test_security(&workspace), DiscoveryRules::default()).unwrap();
    let paths: Vec<_> = files
        .into_iter()
        .map(|entry| entry.file.relative_path)
        .collect();
    assert_eq!(paths, vec!["src/lib.rs"]);
}

#[cfg(unix)]
#[test]
fn discovery_excludes_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let workspace = root.path().join("workspace");
    let outside = root.path().join("outside");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(workspace.join("inside.rs"), "inside\n").unwrap();
    std::fs::write(outside.join("secret.rs"), "secret\n").unwrap();
    symlink(outside.join("secret.rs"), workspace.join("escape.rs")).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    std::fs::remove_dir(temp_workspace.path()).unwrap();
    std::os::unix::fs::symlink(&workspace, temp_workspace.path()).unwrap();

    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace.clone(),
        ..SecurityPolicy::default()
    });

    let files = discover_indexable_files(&security, DiscoveryRules::default()).unwrap();
    let paths: Vec<_> = files
        .into_iter()
        .map(|entry| entry.file.relative_path)
        .collect();
    assert_eq!(paths, vec!["inside.rs"]);
}

#[cfg(unix)]
#[test]
fn discovery_skips_unreadable_files() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    let readable = workspace.path().join("readable.txt");
    let unreadable = workspace.path().join("unreadable.txt");
    std::fs::write(&readable, "hello\n").unwrap();
    std::fs::write(&unreadable, "nope\n").unwrap();
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

    let files =
        discover_indexable_files(&test_security(&workspace), DiscoveryRules::default()).unwrap();

    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();
    let paths: Vec<_> = files
        .into_iter()
        .map(|entry| entry.file.relative_path)
        .collect();
    assert_eq!(paths, vec!["readable.txt"]);
}

#[test]
fn trigram_counts_are_deterministic() {
    let trigrams = trigram_counts(b"ababa");
    assert_eq!(trigrams.get(b"aba"), Some(&2));
    assert_eq!(trigrams.get(b"bab"), Some(&1));
}

#[test]
fn trigram_counts_keep_short_files_empty() {
    assert!(trigram_counts(b"").is_empty());
    assert!(trigram_counts(b"a").is_empty());
    assert!(trigram_counts(b"ab").is_empty());
}

#[test]
fn trigram_counts_use_raw_utf8_bytes_deterministically() {
    let trigrams = trigram_counts("éé".as_bytes());
    let expected = BTreeMap::from([([0xC3, 0xA9, 0xC3], 1_u32), ([0xA9, 0xC3, 0xA9], 1_u32)]);
    assert_eq!(trigrams, expected);
}

#[test]
fn persisted_file_paths_are_relative_only() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(workspace.path().join("src/main.rs"), "fn main() {}\n").unwrap();

    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(test_security(&workspace)).unwrap();

    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    let relative_paths: Vec<String> = conn
        .prepare("SELECT relative_path FROM files ORDER BY relative_path")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(relative_paths, vec!["src/main.rs"]);
}

#[test]
fn build_persists_metadata_and_trigram_rows() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(workspace.path().join("src/lib.rs"), "ababa\n").unwrap();

    let report = WorkspaceTrigramIndex::for_workspace(workspace.path())
        .build(test_security(&workspace))
        .unwrap();

    assert_eq!(report.files_indexed, 1);
    assert_eq!(metadata_value(&workspace, "format_version"), FORMAT_VERSION);
    assert_eq!(metadata_value(&workspace, "build_state"), BUILD_STATE_READY);

    let trigram_rows = trigram_rows(&workspace);
    assert!(trigram_rows.iter().any(|(trigram, path, occurrences)| {
        trigram == b"aba" && path == "src/lib.rs" && *occurrences == 2
    }));
}

#[test]
fn load_returns_existing_compatible_index() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "fn main() {}\n").unwrap();

    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(test_security(&workspace)).unwrap();
    let loaded = index.load().unwrap();

    let expected = workspace
        .path()
        .canonicalize()
        .unwrap()
        .join("state/code-search/index.db");

    assert_eq!(loaded.db_path, expected);
}

#[test]
fn compatible_index_refreshes_changed_and_deleted_files() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn old() {}\n").unwrap();
    std::fs::write(workspace.path().join("src/main.rs"), "fn main() {}\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();

    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn new() {}\n").unwrap();
    std::fs::remove_file(workspace.path().join("src/main.rs")).unwrap();
    index.refresh_or_rebuild(security).unwrap();

    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    let relative_paths: Vec<String> = conn
        .prepare("SELECT relative_path FROM files ORDER BY relative_path")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(relative_paths, vec!["src/lib.rs"]);

    let trigram_rows = trigram_rows(&workspace);
    assert!(trigram_rows.iter().all(|(_, path, _)| path == "src/lib.rs"));
    assert!(trigram_rows.iter().any(|(trigram, _, _)| trigram == b"new"));
}

#[test]
fn incompatible_format_version_forces_rebuild() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "fn main() {}\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();

    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    conn.execute(
        "UPDATE metadata SET value = '999' WHERE key = 'format_version'",
        [],
    )
    .unwrap();
    drop(conn);

    std::fs::write(workspace.path().join("main.rs"), "fn changed() {}\n").unwrap();
    index.refresh_or_rebuild(security).unwrap();

    assert_eq!(metadata_value(&workspace, "format_version"), FORMAT_VERSION);
    assert!(trigram_rows(&workspace)
        .iter()
        .any(|(trigram, path, _)| trigram == b"ang" && path == "main.rs"));
}

#[test]
fn incomplete_build_state_forces_rebuild() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "fn main() {}\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();

    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    conn.execute(
        "UPDATE metadata SET value = ?1 WHERE key = 'build_state'",
        [BUILD_STATE_BUILDING],
    )
    .unwrap();
    drop(conn);

    std::fs::write(workspace.path().join("main.rs"), "fn rebuilt() {}\n").unwrap();
    index.refresh_or_rebuild(security).unwrap();

    assert_eq!(metadata_value(&workspace, "build_state"), BUILD_STATE_READY);
    assert!(trigram_rows(&workspace)
        .iter()
        .any(|(trigram, path, _)| trigram == b"reb" && path == "main.rs"));
}

#[test]
fn foreign_workspace_index_forces_rebuild() {
    let source_workspace = TempDir::new().unwrap();
    std::fs::write(source_workspace.path().join("source.rs"), "source\n").unwrap();
    WorkspaceTrigramIndex::for_workspace(source_workspace.path())
        .build(test_security(&source_workspace))
        .unwrap();

    let target_workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(target_workspace.path().join("state/code-search")).unwrap();
    std::fs::write(target_workspace.path().join("target.rs"), "target\n").unwrap();
    std::fs::copy(
        source_workspace.path().join("state/code-search/index.db"),
        target_workspace.path().join("state/code-search/index.db"),
    )
    .unwrap();

    WorkspaceTrigramIndex::for_workspace(target_workspace.path())
        .refresh_or_rebuild(test_security(&target_workspace))
        .unwrap();

    let rows = file_rows(&target_workspace);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "target.rs");
}

#[test]
fn repeated_refresh_keeps_deterministic_membership() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(workspace.path().join("src/lib.rs"), "alpha\n").unwrap();
    std::fs::write(workspace.path().join("src/main.rs"), "beta\n").unwrap();
    std::fs::write(workspace.path().join("bad.txt"), [0xFF, 0xFE]).unwrap();
    std::fs::create_dir_all(workspace.path().join("state/code-search")).unwrap();
    std::fs::write(
        workspace.path().join("state/code-search/index.db-wal"),
        b"sidecar",
    )
    .unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.refresh_or_rebuild(security.clone()).unwrap();
    let first_rows = file_rows(&workspace);
    let first_trigrams = trigram_rows(&workspace);

    index.refresh_or_rebuild(security).unwrap();
    let second_rows = file_rows(&workspace);
    let second_trigrams = trigram_rows(&workspace);

    assert_eq!(first_rows, second_rows);
    assert_eq!(first_trigrams, second_trigrams);
}

#[test]
fn rebuild_cleans_up_temp_artifacts() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "fn main() {}\n").unwrap();

    WorkspaceTrigramIndex::for_workspace(workspace.path())
        .refresh_or_rebuild(test_security(&workspace))
        .unwrap();

    let state_dir = workspace.path().join("state/code-search");
    let entries: Vec<_> = std::fs::read_dir(&state_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(entries
        .iter()
        .all(|entry| !entry.starts_with(".index.db.tmp-")));
}

#[test]
fn refresh_fails_fast_when_index_db_is_locked() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "fn main() {}\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();
    std::fs::write(workspace.path().join("main.rs"), "fn changed() {}\n").unwrap();

    let lock_conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    lock_conn.execute_batch("BEGIN EXCLUSIVE;").unwrap();

    let started = Instant::now();
    let error = index.refresh_or_rebuild(security).unwrap_err();
    let elapsed = started.elapsed();
    let error_text = format!("{error:#}");

    lock_conn.execute_batch("ROLLBACK;").unwrap();

    assert!(
        elapsed < Duration::from_secs(2),
        "expected fail-fast lock handling, got {elapsed:?}"
    );
    assert!(
        error_text.contains("locked")
            || error_text.contains("busy")
            || error_text.contains("lock held")
            || error_text.contains("temporarily unavailable"),
        "expected lock-related error, got {error:?}"
    );
}

#[test]
fn searchable_discovery_respects_scope() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::create_dir_all(workspace.path().join("tests")).unwrap();
    std::fs::write(workspace.path().join("src/lib.rs"), "needle\n").unwrap();
    std::fs::write(workspace.path().join("tests/lib.rs"), "needle\n").unwrap();

    let files = discover_searchable_files(
        &test_security(&workspace),
        "src",
        &[],
        &[],
        DiscoveryRules::default(),
    )
    .unwrap();
    let paths: Vec<_> = files
        .iter()
        .map(|entry| entry.file.relative_path.as_str())
        .collect();
    assert_eq!(paths, vec!["src/lib.rs"]);
}

#[test]
fn sqlite_candidate_paths_intersect_trigrams_and_sort_lexically() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src/bin")).unwrap();
    std::fs::write(workspace.path().join("src/z.rs"), "needle here\n").unwrap();
    std::fs::write(workspace.path().join("src/a.rs"), "needle here\n").unwrap();
    std::fs::write(workspace.path().join("src/bin/tool.rs"), "needle here\n").unwrap();
    std::fs::write(workspace.path().join("notes.txt"), "needless\n").unwrap();

    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(test_security(&workspace)).unwrap();
    let loaded = index.load().unwrap();
    let conn = Connection::open(loaded.db_path).unwrap();

    let candidates = read_candidate_paths(
        &conn,
        &[[b'n', b'e', b'e'], [b'e', b'e', b'd'], [b'd', b'l', b'e']],
        Some("src"),
    )
    .unwrap();

    assert_eq!(candidates, vec!["src/a.rs", "src/bin/tool.rs", "src/z.rs"]);
    assert_eq!(read_index_file_count(&conn).unwrap(), 4);
}

#[test]
fn candidate_planner_returns_complete_for_safe_literal_query() {
    let workspace = TempDir::new().unwrap();
    std::fs::create_dir_all(workspace.path().join("src")).unwrap();
    std::fs::write(
        workspace.path().join("src/z.rs"),
        "const VALUE: &str = \"needle\";\n",
    )
    .unwrap();
    std::fs::write(workspace.path().join("src/a.rs"), "fn needle() {}\n").unwrap();
    std::fs::write(workspace.path().join("src/m.rs"), "fn other() {}\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();

    let plan = index
        .plan_candidates(
            &security,
            &CandidateRequest {
                relative_root: "src".to_string(),
                include: Vec::new(),
                exclude: Vec::new(),
                raw_pattern: "needle".to_string(),
                is_regex: false,
                case_sensitive: true,
                whole_word: false,
            },
            DiscoveryRules::default().max_file_size_bytes,
        )
        .unwrap();

    assert_eq!(plan.coverage, CandidateCoverage::Complete);
    assert_eq!(plan.ordered_paths, vec!["src/a.rs", "src/z.rs"]);
}

#[test]
fn candidate_planner_marks_stale_index_as_partial() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "needle\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();
    std::fs::write(workspace.path().join("later.rs"), "needle\n").unwrap();

    let plan = index
        .plan_candidates(
            &security,
            &CandidateRequest {
                relative_root: ".".to_string(),
                include: Vec::new(),
                exclude: Vec::new(),
                raw_pattern: "needle".to_string(),
                is_regex: false,
                case_sensitive: true,
                whole_word: false,
            },
            DiscoveryRules::default().max_file_size_bytes,
        )
        .unwrap();

    assert_eq!(plan.coverage, CandidateCoverage::Partial);
    assert!(plan.reason.contains("parity") || plan.reason.contains("stale"));
}

#[test]
fn candidate_planner_marks_deleted_indexed_path_as_partial() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("old.rs"), "needle\n").unwrap();
    std::fs::write(workspace.path().join("keep.rs"), "other\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();
    std::fs::remove_file(workspace.path().join("old.rs")).unwrap();

    let plan = plan_for_literal(&index, &security, "needle");

    assert_eq!(plan.coverage, CandidateCoverage::Partial);
    assert!(plan.reason.contains("parity") || plan.reason.contains("stale"));
}

#[test]
fn candidate_planner_marks_changed_indexed_content_as_partial() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(
        workspace.path().join("main.rs"),
        "const VALUE: &str = \"needle\";\n",
    )
    .unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();
    std::fs::write(
        workspace.path().join("main.rs"),
        "const VALUE: &str = \"other\";\n",
    )
    .unwrap();

    let plan = plan_for_literal(&index, &security, "needle");

    assert_eq!(plan.coverage, CandidateCoverage::Partial);
    assert!(plan.reason.contains("parity") || plan.reason.contains("stale"));
}

#[test]
fn candidate_planner_marks_renamed_path_drift_as_partial() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("legacy.rs"), "needle\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();
    std::fs::rename(
        workspace.path().join("legacy.rs"),
        workspace.path().join("current.rs"),
    )
    .unwrap();

    let plan = plan_for_literal(&index, &security, "needle");

    assert_eq!(plan.coverage, CandidateCoverage::Partial);
    assert!(plan.reason.contains("parity") || plan.reason.contains("stale"));
}

#[test]
fn candidate_planner_uses_hash_guard_when_size_and_mtime_match() {
    let workspace = TempDir::new().unwrap();
    let file_path = workspace.path().join("main.rs");
    std::fs::write(&file_path, "abcOLDxyz\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();

    std::fs::write(&file_path, "abcNEWxyz\n").unwrap();
    let modified_unix_ms = i64::try_from(
        std::fs::metadata(&file_path)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();

    let conn = Connection::open(workspace.path().join("state/code-search/index.db")).unwrap();
    conn.execute(
        "UPDATE files SET size_bytes = ?1, modified_unix_ms = ?2 WHERE relative_path = 'main.rs'",
        rusqlite::params![10_i64, modified_unix_ms],
    )
    .unwrap();
    drop(conn);

    let plan = plan_for_literal(&index, &security, "NEW");

    assert_eq!(plan.coverage, CandidateCoverage::Partial);
    assert!(plan.reason.contains("parity") || plan.reason.contains("stale"));
}

#[test]
fn candidate_planner_marks_regex_and_short_patterns_unavailable() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("main.rs"), "needle\n").unwrap();

    let security = test_security(&workspace);
    let index = WorkspaceTrigramIndex::for_workspace(workspace.path());
    index.build(security.clone()).unwrap();

    let regex_plan = index
        .plan_candidates(
            &security,
            &CandidateRequest {
                relative_root: ".".to_string(),
                include: Vec::new(),
                exclude: Vec::new(),
                raw_pattern: "need.*".to_string(),
                is_regex: true,
                case_sensitive: true,
                whole_word: false,
            },
            DiscoveryRules::default().max_file_size_bytes,
        )
        .unwrap();
    assert_eq!(regex_plan.coverage, CandidateCoverage::Unavailable);
    assert_eq!(regex_plan.reason, "query_regex_not_supported");

    let short_plan = index
        .plan_candidates(
            &security,
            &CandidateRequest {
                relative_root: ".".to_string(),
                include: Vec::new(),
                exclude: Vec::new(),
                raw_pattern: "ab".to_string(),
                is_regex: false,
                case_sensitive: true,
                whole_word: false,
            },
            DiscoveryRules::default().max_file_size_bytes,
        )
        .unwrap();
    assert_eq!(short_plan.coverage, CandidateCoverage::Unavailable);
}
