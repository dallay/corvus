# Tasks: Workspace Trigram Index

## Phase 1: Foundation and shared discovery

- [x] 1.1 Add `clients/agent-runtime/src/search/mod.rs` and export `pub mod search;` from
  `clients/agent-runtime/src/lib.rs` for the new runtime indexing surface.
- [x] 1.2 RED: add discovery-focused tests in `clients/agent-runtime/src/search/tests.rs` covering
  workspace-only admission, symlink escapes, ignored/hidden paths, oversized files, unreadable
  files, binary files, invalid UTF-8, and self-index DB exclusions.
- [x] 1.3 Extract the reusable workspace walk/filter pipeline from
  `clients/agent-runtime/src/tools/code_search.rs` into
  `clients/agent-runtime/src/search/discovery.rs`, keeping the existing `code_search` behavior
  unchanged.
- [x] 1.4 Add shared helpers in `search/discovery.rs` for canonical root validation,
  workspace-relative path normalization (`/` separators), and deterministic exclusion of
  `workspace/state/code-search/index.db*` artifacts.

## Phase 2: Trigram and SQLite primitives

- [x] 2.1 RED: extend `clients/agent-runtime/src/search/tests.rs` with cases for trigram byte-window
  determinism, files shorter than 3 bytes, and direct SQLite assertions that persisted file
  identities are relative only.
- [x] 2.2 Create `clients/agent-runtime/src/search/trigram.rs` with strict UTF-8 admission and
  trigram aggregation helpers that return stable counts from raw UTF-8 bytes.
- [x] 2.3 Create `clients/agent-runtime/src/search/sqlite.rs` to initialize
  `workspace/state/code-search/index.db`, create `metadata`, `files`, and `trigram_postings`, and
  expose metadata/file/posting read-write helpers.
- [x] 2.4 Implement metadata/version constants and workspace fingerprint generation in
  `search/sqlite.rs` or `search/index.rs` without persisting raw absolute workspace paths.

## Phase 3: Index lifecycle and wiring

- [x] 3.1 RED: add integration tests for first build, compatible load, incompatible/incomplete-state
  rebuild, atomic temp-DB swap, and fail-fast lock behavior using temp workspaces plus direct DB
  inspection.
- [x] 3.2 Create `clients/agent-runtime/src/search/index.rs` with `WorkspaceTrigramIndex`,
  `IndexBuildReport`, `RefreshAction`, and `refresh_or_rebuild` orchestration APIs from the design.
- [x] 3.3 Implement build/rebuild flow: create state dir, build temp DB, write
  `build_state=building`, persist files/postings transactionally, finalize `ready`, fsync/rename
  atomically, and remove temp artifacts on failure.
- [x] 3.4 Wire `clients/agent-runtime/src/tools/code_search.rs` to consume `search::discovery` and
  update `clients/agent-runtime/src/tools/mod.rs` only if test or export wiring requires it.

## Phase 4: Lifecycle coverage and polish

- [x] 4.1 RED: add repeated-run tests proving deterministic corpus membership and persisted outcomes
  for excluded paths, changed files, and deleted files.
- [x] 4.2 GREEN: implement load-vs-rebuild decision logic from persisted metadata and ensure
  changed/deleted corpus state is reflected in the published DB according to the approved lifecycle
  contract.
- [x] 4.3 REFACTOR: consolidate duplicated limits/constants between `code_search.rs` and `search/*`,
  keeping one source of truth for admission rules and no new crate unless
  `clients/agent-runtime/Cargo.toml` truly requires it.
- [x] 4.4 Validate in `clients/agent-runtime/` with `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --all -- --check`; if a narrower test
  target is used first, finish with the full runtime checks before handoff. (`cargo test` and
  `cargo fmt --all -- --check` passed; `cargo clippy --all-targets -- -D warnings` remains blocked
  by unrelated pre-existing `src/channels/telegram.rs:3479`.)
