## Exploration: Search index freshness across writes and local workspace changes

### Current State

`WorkspaceTrigramIndex` lives in `clients/agent-runtime/src/search/index.rs` and persists a SQLite
trigram corpus under `state/code-search/index.db`. It supports `build()`, `load()`,
`refresh_or_rebuild()`, and `plan_candidates()`. The index stores metadata plus per-file rows keyed
by workspace-relative path, including `size_bytes`, `modified_unix_ms`, and `content_sha256` (
`clients/agent-runtime/src/search/sqlite.rs`).

`code_search` uses the index only as an advisory candidate source. In `search_workspace()`, it calls
`WorkspaceTrigramIndex::plan_candidates()` and then verifies live file contents before reporting
matches (`clients/agent-runtime/src/tools/code_search.rs:397-555`). If candidate coverage is
`Partial` or `Unavailable`, it falls back to full discovery scan (
`clients/agent-runtime/src/tools/code_search.rs:446-515`).

`file_write` writes directly with `tokio::fs::write` after sandbox checks, but it does not call any
index invalidation or refresh API (`clients/agent-runtime/src/tools/file_write.rs:45-171`). In
production code, `refresh_or_rebuild()` is not called anywhere outside tests, so the persisted index
is effectively static once built.

### Affected Areas

- `clients/agent-runtime/src/search/index.rs` — current freshness logic, compatibility checks, and
  refresh/rebuild flow.
- `clients/agent-runtime/src/search/sqlite.rs` — persisted file metadata contract (`relative_path`,
  `size_bytes`, `modified_unix_ms`, `content_sha256`).
- `clients/agent-runtime/src/search/discovery.rs` — workspace discovery and file metadata
  collection; currently provides bytes and `modified_unix_ms`.
- `clients/agent-runtime/src/tools/code_search.rs` — candidate planning, fallback behavior, and live
  verification boundary.
- `clients/agent-runtime/src/tools/file_write.rs` — write tool with no index hook today.
- `clients/agent-runtime/src/search/tests.rs` — good lifecycle coverage for manual refresh/rebuild,
  but no regression proving agent writes are immediately searchable.
- `openspec/specs/workspace-index/spec.md` — current spec covers build/load/refresh/delete
  lifecycle, but not explicit tool-write freshness guarantees.

### Approaches

1. **Eager write-through refresh hook** — after successful `file_write`, update the index
   immediately.
    - Pros: solves “agent reads its own writes” deterministically; easy user model; keeps index
      fresh for tool-originated writes.
    - Cons: full `refresh_or_rebuild()` after every write is simple but expensive; targeted per-path
      refresh needs a small new API.
    - Effort: Medium.

2. **Search-time stale guard only** — do not update on write; make `plan_candidates()` refuse to
   trust stale rows and always fall back when anything changed.
    - Pros: smallest correctness fix for external changes; no tool coupling required.
    - Cons: does not keep persisted index fresh; own writes may still pay fallback cost every time;
      weaker than acceptance wording.
    - Effort: Low.

3. **Hybrid v1: eager per-write invalidation/refresh + search-time freshness guard** — update
   tool-written paths immediately and use a stronger stale check before `Complete` coverage.
    - Pros: meets acceptance with smallest safe scope; own writes are reflected immediately; local
      out-of-band edits/deletes/renames stop being silently trusted.
    - Cons: still not a true filesystem watcher; some searches may trigger fallback or refresh work.
    - Effort: Medium.

### Recommendation

Use the **hybrid v1**.

Concretely:

- Add an index-side API in `search/index.rs` for path-scoped mutation, e.g. `refresh_paths()` /
  `remove_paths()` or a single `sync_paths_after_write()` that recomputes rows for specific relative
  paths under the existing lock/transaction model.
- Call that API from `FileWriteTool` only after a successful write. If the index does not exist yet,
  do nothing; the next search can still fall back safely.
- Strengthen `plan_candidates()` so `CandidateCoverage::Complete` requires exact freshness for the
  current searchable set, not just count/mtime parity. Because discovery already reads file bytes,
  v1 can compare persisted `content_sha256` too, not just `size_bytes` and `modified_unix_ms`.
- When `plan_candidates()` sees missing files, extra indexed files, or metadata/hash mismatch, it
  should either (a) opportunistically `refresh_or_rebuild()` before querying candidates, or (b)
  downgrade to `Partial`/`Unavailable` so `code_search` falls back instead of silently trusting
  stale rows. For smallest safe v1, downgrade/fallback is enough; eager refresh can be added for
  clearer freshness.
- Treat rename detection as `deleted old path + added new path`. The index identity is already
  workspace-relative path, so explicit rename tracking is unnecessary in v1.
- Document v1 metadata rules as: workspace-relative path is identity; `size_bytes` and
  `modified_unix_ms` are fast change hints; `content_sha256` is the correctness guard when deciding
  whether an indexed entry is still trustworthy; git state is diagnostic/optional and MUST NOT be
  the sole freshness source because untracked and uncommitted writes matter.

### Risks

- Same-size, same-mtime rewrites are the main silent-staleness gap today. Evidence:
  `plan_candidates()` declares `Complete` using only `size_bytes` and `modified_unix_ms` parity (
  `clients/agent-runtime/src/search/index.rs:239-257`), even though the DB already stores
  `content_sha256`.
- `file_write` currently has no index hook at all (
  `clients/agent-runtime/src/tools/file_write.rs:45-171`), so tool-originated writes depend on
  incidental fallback rather than deterministic invalidation.
- The persisted index can remain stale indefinitely because `refresh_or_rebuild()` has no production
  caller outside tests.
- Deletions are masked for user-visible search results because candidate paths are intersected with
  fresh discovery, but stale rows remain in SQLite until an explicit refresh. That is acceptable for
  correctness but not for “fresh index” semantics.
- `plan_candidates()` already performs full discovery (including reading file bytes) before deciding
  coverage, so adding hash comparison is safe for correctness but highlights that current “index
  optimization” is not yet cheap.
- Filesystem timestamp resolution differs across platforms. v1 should document that mtimes are hints
  only; trust decisions must not rely on mtime alone.

### Ready for Proposal

Yes — the codebase is clear enough to propose a small, safe change focused on:

1. write-path invalidation/refresh in `file_write`,
2. stronger freshness checks in `plan_candidates`,
3. regression tests for own-write visibility, changed/deleted/renamed workspace files, and
   stale-entry fallback behavior,
4. spec/design text documenting metadata usage and v1 limits.
