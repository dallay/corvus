# Tasks: Search Index Freshness v1

## Phase 1: Regression Tests First

- [x] 1.1 In `clients/agent-runtime/src/search/tests.rs`, add RED regressions proving `plan_candidates()` downgrades from `Complete` when indexed rows are stale because of content change, delete, or rename-shaped path drift.
- [x] 1.2 In `clients/agent-runtime/src/search/tests.rs`, add a regression for same-size / same-mtime content rewrites so hash mismatch, not hints alone, controls freshness.
- [x] 1.3 In `clients/agent-runtime/src/tools/file_write.rs`, add an integration-style regression showing a successful `file_write` against an already indexed path is searchable afterward without a manual rebuild.
- [x] 1.4 In `clients/agent-runtime/src/tools/code_search.rs`, add or extend a regression proving stale indexed coverage falls back to safe live verification instead of silently trusting SQLite candidates.

## Phase 2: Index Freshness Core

- [x] 2.1 In `clients/agent-runtime/src/search/discovery.rs`, add a small helper that rediscoveries one workspace-relative path with the same index admission rules as full discovery.
- [x] 2.2 In `clients/agent-runtime/src/search/sqlite.rs`, add or expose the minimal scoped row read/replace/delete helpers needed for single-path sync and stale-row comparison without widening responsibilities.
- [x] 2.3 In `clients/agent-runtime/src/search/index.rs`, add a path-scoped sync API (for example `sync_written_path`) that updates or removes one persisted entry in one transaction and returns an explicit outcome.
- [x] 2.4 In `clients/agent-runtime/src/search/index.rs`, tighten `plan_candidates()` so `CandidateCoverage::Complete` requires exact scoped path parity plus matching `content_sha256`; otherwise return `Partial`.

## Phase 3: Tool Wiring

- [x] 3.1 In `clients/agent-runtime/src/tools/file_write.rs`, call the new index sync only after a successful write and keep sync failures best-effort so the write result still succeeds.
- [x] 3.2 In `clients/agent-runtime/src/tools/code_search.rs`, keep current fallback behavior intact for `Partial` / `Unavailable` coverage and adjust only what the new regressions require.

## Phase 4: Documentation and Focused Validation

- [x] 4.1 Update `openspec/changes/2026-04-05-search-index-freshness/design.md` and this change’s spec notes only if implementation details drift, keeping v1 guarantees and non-goals aligned.
- [x] 4.2 Validate with focused commands: `cargo test --manifest-path clients/agent-runtime/Cargo.toml search::`, `cargo test --manifest-path clients/agent-runtime/Cargo.toml tools::file_write`, and `cargo test --manifest-path clients/agent-runtime/Cargo.toml tools::code_search`.
