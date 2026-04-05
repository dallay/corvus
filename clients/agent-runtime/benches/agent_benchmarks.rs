//! Criterion microbenchmarks for hot-loop `code_search` behavior.
//!
//! These benches are intentionally scoped to low-level timing only. For rollout evidence
//! (shell baseline, no-index/cold-build/warm-index comparisons, parity checks, and docs-ready
//! reporting), run `cargo run --example code_search_rollout_benchmark --manifest-path clients/agent-runtime/Cargo.toml`.

use corvus::security::{AutonomyLevel, SecurityPolicy};
use corvus::tools::traits::Tool;
use corvus::tools::CodeSearchTool;
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_workspace(file_count: usize, lines_per_file: usize) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    for i in 0..file_count {
        let mut content = String::new();
        for j in 0..lines_per_file {
            content.push_str(&format!(
                "fn function_{i}_{j}() {{ let value_{j} = {j}; }}\n"
            ));
        }
        std::fs::write(src.join(format!("mod_{i}.rs")), &content).unwrap();
    }
    dir
}

fn make_security(dir: &TempDir) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: dir.path().to_path_buf(),
        max_actions_per_hour: 100_000,
        ..SecurityPolicy::default()
    })
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Literal search across 20 files × 50 lines each — typical small workspace.
fn bench_literal_search_small_workspace(c: &mut Criterion) {
    let dir = make_workspace(20, 50);
    let security = make_security(&dir);
    let tool = CodeSearchTool::new(security);
    let rt = rt();

    c.bench_function("code_search/literal/20_files_50_lines", |b| {
        b.to_async(&rt).iter(|| async {
            let result = tool
                .execute(serde_json::json!({
                    "pattern": "fn function_10_25",
                }))
                .await
                .unwrap();
            assert!(result.success, "expected successful search");
            let matches = result.structured.as_ref().unwrap()["matches"]
                .as_array()
                .unwrap()
                .len();
            assert!(matches > 0, "expected at least one match");
            result
        });
    });
}

/// Regex search across the same 20-file workspace.
fn bench_regex_search_small_workspace(c: &mut Criterion) {
    let dir = make_workspace(20, 50);
    let security = make_security(&dir);
    let tool = CodeSearchTool::new(security);
    let rt = rt();

    c.bench_function("code_search/regex/20_files_50_lines", |b| {
        b.to_async(&rt).iter(|| async {
            let result = tool
                .execute(serde_json::json!({
                    "pattern": r"fn function_\d+_\d+",
                    "is_regex": true,
                }))
                .await
                .unwrap();
            assert!(result.success, "expected successful regex search");
            let matches = result.structured.as_ref().unwrap()["matches"]
                .as_array()
                .unwrap()
                .len();
            assert!(matches > 0, "expected regex to match generated functions");
            result
        });
    });
}

/// Pattern that produces zero matches — measures walk + miss cost.
fn bench_no_match_search(c: &mut Criterion) {
    let dir = make_workspace(20, 50);
    let security = make_security(&dir);
    let tool = CodeSearchTool::new(security);
    let rt = rt();

    c.bench_function("code_search/no_match/20_files_50_lines", |b| {
        b.to_async(&rt).iter(|| async {
            let result = tool
                .execute(serde_json::json!({
                    "pattern": "xyzzy_nonexistent_symbol_42",
                }))
                .await
                .unwrap();
            assert!(result.success, "expected successful search with no matches");
            let matches = result.structured.as_ref().unwrap()["matches"]
                .as_array()
                .unwrap()
                .len();
            assert_eq!(matches, 0, "expected zero matches for absent pattern");
            result
        });
    });
}

/// Scoped search confined to a subdirectory — measures path-filter effectiveness.
fn bench_scoped_path_search(c: &mut Criterion) {
    let dir = make_workspace(40, 50);
    let security = make_security(&dir);
    let tool = CodeSearchTool::new(security);
    let rt = rt();

    c.bench_function("code_search/literal/scoped_to_src", |b| {
        b.to_async(&rt).iter(|| async {
            let result = tool
                .execute(serde_json::json!({
                    "pattern": "fn function_5_",
                    "path": "src",
                }))
                .await
                .unwrap();
            assert!(result.success, "expected successful scoped search");
            let matches = result.structured.as_ref().unwrap()["matches"]
                .as_array()
                .unwrap();
            assert!(
                !matches.is_empty(),
                "expected matches within scoped src directory"
            );
            // All returned matches must be from the src subtree.
            for m in matches {
                let file = m["file"].as_str().unwrap_or_default();
                assert!(
                    file.starts_with("src/"),
                    "match '{file}' escaped the 'src' scope"
                );
            }
            result
        });
    });
}

/// Extension-filtered search (`include: ["*.rs"]`) across a mixed-extension workspace.
fn bench_include_glob_search(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    for i in 0..20 {
        std::fs::write(
            src.join(format!("mod_{i}.rs")),
            format!("pub fn target_fn_{i}() {{}}\n"),
        )
        .unwrap();
        std::fs::write(
            src.join(format!("mod_{i}.txt")),
            format!("not a target fn {i}\n"),
        )
        .unwrap();
    }

    let security = make_security(&dir);
    let tool = CodeSearchTool::new(security);
    let rt = rt();

    c.bench_function("code_search/include_glob/rs_only", |b| {
        b.to_async(&rt).iter(|| async {
            let result = tool
                .execute(serde_json::json!({
                    "pattern": "target_fn_",
                    "include": ["*.rs"],
                }))
                .await
                .unwrap();
            assert!(result.success, "expected successful include-glob search");
            let matches = result.structured.as_ref().unwrap()["matches"]
                .as_array()
                .unwrap();
            assert!(!matches.is_empty(), "expected matches in .rs files");
            // All returned matches must be from .rs files only.
            for m in matches {
                let file = m["file"].as_str().unwrap_or_default();
                assert!(
                    file.ends_with(".rs"),
                    "match '{file}' is not a .rs file; include glob leaked"
                );
            }
            result
        });
    });
}

criterion_group!(
    benches,
    bench_literal_search_small_workspace,
    bench_regex_search_small_workspace,
    bench_no_match_search,
    bench_scoped_path_search,
    bench_include_glob_search,
);
criterion_main!(benches);
