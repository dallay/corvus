# Design: Search Index Freshness v1

## Technical Approach

Implement a correctness-first freshness layer around the existing SQLite trigram index without changing the external `code_search` contract.

V1 adds two localized behaviors:

1. **Post-write path sync**: after a successful `file_write`, attempt a best-effort single-path index sync against an already compatible on-disk index.
2. **Search-time freshness gate**: before `plan_candidates()` can return `CandidateCoverage::Complete`, the runtime must prove that the current searchable workspace set for the request is still consistent with persisted rows.

This keeps the current architecture intact:

- `WorkspaceTrigramIndex` remains the owner of index mutation and compatibility decisions.
- `file_write` remains responsible for the filesystem write and only triggers a follow-up sync after success.
- `code_search` continues to trust only live verification for matches; freshness work only decides whether indexed candidates are safe enough to narrow the scan.

This design maps directly to the proposal by preferring small changes in `search/index.rs`, a small discovery helper for a single path, and a post-write hook in `tools/file_write.rs`.

## Architecture Decisions

### Decision: Keep freshness logic inside `WorkspaceTrigramIndex`

**Choice**: Add small path-scoped mutation APIs on `WorkspaceTrigramIndex` instead of letting tools manipulate SQLite rows directly.

**Alternatives considered**:
- Call SQLite helpers directly from `file_write`
- Re-run `refresh_or_rebuild()` after every successful write

**Rationale**: `WorkspaceTrigramIndex` already owns compatibility checks, lock handling, workspace validation, and transaction boundaries. Reusing that owner keeps tool code small and avoids duplicating security and SQLite rules. A full refresh after every write is safer than direct SQL from the tool, but unnecessarily expensive for the minimal v1.

### Decision: Treat `content_sha256` as the trust boundary for `Complete`

**Choice**: `plan_candidates()` may return `Complete` only when each discovered file in scope has a persisted row with matching `relative_path`, `size_bytes`, `modified_unix_ms`, and `content_sha256`, and there are no extra persisted rows considered stale for that scope.

**Alternatives considered**:
- Keep current mtime+size parity check only
- Refresh automatically during `plan_candidates()` when any mismatch is detected

**Rationale**: the database already stores `content_sha256`, and discovery already reads file bytes. Using the hash closes the same-size/same-mtime rewrite gap with minimal new surface area. Auto-refresh during search would mix query planning with write-side mutation and make failure handling more complex; conservative downgrade to `Partial` is smaller and safer for v1.

### Decision: Rename handling is `delete old path + add new path`

**Choice**: V1 does not introduce explicit rename identity or history. Path identity remains `relative_path`.

**Alternatives considered**:
- Persist inode/device-style rename hints
- Use git rename detection as an authoritative source

**Rationale**: the current persisted identity is already workspace-relative path, and that is enough for safe correctness. Rename tracking would expand scope and portability risk. For freshness, a rename is simply an indexed path missing from discovery plus a discovered path missing from the index.

### Decision: Make post-write sync best-effort and non-blocking to correctness

**Choice**: successful file writes remain successful even if index sync fails, skips, or finds no compatible index.

**Alternatives considered**:
- Fail the write if index sync cannot complete
- Ignore sync errors completely with no logging/diagnostic path

**Rationale**: the write tool is the primary user action. Search correctness is still protected by the search-time freshness gate and existing fallback behavior. The index is an optimization, not the source of truth. V1 should log/debug sync failures and preserve a safe fallback rather than reject valid writes.

### Decision: Be conservative for scoped searches

**Choice**: when the runtime cannot prove that persisted rows exactly match the current searchable scope, it downgrades to `Partial`.

**Alternatives considered**:
- Ignore extra stale rows that are not current trigram candidates
- Ignore rows outside the discovered set if they might be excluded by globs

**Rationale**: v1 is correctness-first. If scope equivalence is uncertain, the safe outcome is fallback scanning. This may reduce index usage for some filtered searches, but it prevents silently trusting stale corpus state.

## Data Flow

### Sequence: successful `file_write`

```text
FileWriteTool
  -> validate sandbox/path/rate limits
  -> tokio::fs::write(resolved_target, content)
  -> on success, trigger WorkspaceTrigramIndex::sync_written_path(...)
       -> load compatible DB if present
       -> acquire existing build/index lock
       -> rediscover/admit exactly that relative path under index rules
       -> if file is indexable: replace row + trigram postings in one tx
       -> if file is no longer indexable: delete existing row in one tx
       -> update metadata built_at/build_state=ready
  -> return successful ToolResult regardless of sync outcome
```

### Sequence: `code_search` candidate planning

```text
code_search::search_workspace
  -> WorkspaceTrigramIndex::plan_candidates(request)
       -> load compatible DB or return Unavailable
       -> read trigram candidate paths from SQLite
       -> discover current searchable files for the request scope
       -> build current-file fingerprints from bytes already read
       -> compare discovered scope vs persisted rows
            - missing discovered row? stale
            - persisted row hash mismatch? stale
            - modified_unix_ms == 0 on either side? stale
            - extra persisted row in scoped view? stale
       -> if all checks pass: Complete + ordered candidate paths
       -> else: Partial + stale reason
  -> Complete => verify only indexed candidate files live
  -> Partial/Unavailable => full discovery fallback + live verification
```

### Freshness comparison model

```text
Current workspace bytes/metadata
        │
        ├─ relative_path      -> identity
        ├─ size_bytes         -> fast hint
        ├─ modified_unix_ms   -> fast hint, 0 means unknown/stale
        └─ content_sha256     -> correctness guard

Persisted SQLite row
        │
        ├─ relative_path      -> identity key
        ├─ size_bytes         -> hint parity
        ├─ modified_unix_ms   -> hint parity
        └─ content_sha256     -> trust / no-trust decision
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/search/index.rs` | Modify | Add path-scoped sync/remove logic, freshness comparison helpers, and stricter `plan_candidates()` gating based on scoped row equivalence plus hash checks. |
| `clients/agent-runtime/src/search/discovery.rs` | Modify | Add a small helper for rediscovering/admitting one workspace-relative file with the same indexing rules used by full discovery, so post-write sync can refresh or remove one path safely. |
| `clients/agent-runtime/src/search/sqlite.rs` | Modify | Reuse current row replacement/deletion helpers and add any tiny scoped-read helper needed for comparing persisted rows without widening SQL responsibilities. |
| `clients/agent-runtime/src/tools/file_write.rs` | Modify | After successful writes, invoke best-effort single-path index sync without changing the tool’s external contract. |
| `clients/agent-runtime/src/search/tests.rs` | Modify | Add index-level regressions for hash-based stale detection, deleted/renamed path handling, and targeted path sync semantics. |
| `clients/agent-runtime/src/tools/code_search.rs` | Modify (tests only or minimal logic) | Keep fallback semantics intact and add regression coverage proving stale indexes downgrade to safe live verification instead of silently trusting SQLite rows. |
| `openspec/changes/2026-04-05-search-index-freshness/design.md` | Create | Document the v1 implementation approach, metadata semantics, and failure handling. |

## Interfaces / Contracts

### Index-side mutation API

```rust
impl WorkspaceTrigramIndex {
    pub fn sync_written_path(
        &self,
        security: Arc<SecurityPolicy>,
        relative_path: &str,
    ) -> anyhow::Result<PathSyncOutcome>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSyncOutcome {
    Updated,
    Removed,
    SkippedIndexUnavailable,
    SkippedIncompatible,
}
```

Design intent:
- `Updated`: file exists and is indexable under current rules; file row and trigram postings were replaced in one transaction.
- `Removed`: file no longer belongs in the indexable corpus for that exact path; stale row was deleted if present.
- `Skipped*`: no compatible DB was available to mutate; the write still succeeded and later search will rely on freshness gating.

### Discovery helper for a single relative path

```rust
pub fn discover_indexable_path(
    security: &SecurityPolicy,
    relative_path: &str,
    rules: DiscoveryRules,
) -> anyhow::Result<Option<DiscoveredFileContent>>;
```

Contract:
- Returns `Some(...)` only when the exact path is inside the workspace, admissible by the same text/size/ignore rules used for indexing, and readable as text.
- Returns `None` when the path is absent or excluded from the indexable corpus.
- Treats index artifacts and unsafe paths as non-indexable.

### Freshness assessment shape inside `plan_candidates()`

```rust
struct ScopeFreshnessReport {
    coverage: CandidateCoverage,
    reason: &'static str,
    stale_paths: Vec<String>,
}
```

This can remain private to `search/index.rs`. The important contract is behavioral:
- `Complete` only when the current discovered scope and persisted scoped view are provably aligned.
- `Partial` when any path mismatch, deletion, rename-shaped delta, unknown mtime, or hash mismatch is detected.
- `Unavailable` for unsupported query shapes or index load/query failures, as today.

## Metadata and Freshness Signals in v1

### `relative_path`
- Primary persisted identity for indexed files.
- Used for row replacement, removal, and stale comparison.
- Renames are represented as `old relative_path removed` + `new relative_path added`.

### `size_bytes`
- Fast change hint only.
- Still useful for cheap mismatch detection and diagnostics.
- Never sufficient on its own to declare freshness.

### `modified_unix_ms`
- Fast change hint only.
- `0` remains the sentinel for unknown mtime and MUST force stale treatment.
- Different filesystem timestamp precision is explicitly tolerated by treating mtime as advisory, not authoritative.

### `content_sha256`
- Correctness guard for trusting persisted content.
- Required for `CandidateCoverage::Complete`.
- Prevents silent trust of same-size/same-mtime rewrites.

### Workspace metadata
- Existing SQLite metadata (`workspace_fingerprint`, `schema_version`, `format_version`, `discovery_rules_version`, `builder_version`, `build_state`, `built_at`) continues to decide whether the DB is compatible enough to load or mutate.
- `workspace_fingerprint` remains the ownership check; a foreign DB is rebuilt or skipped, never trusted.
- `built_at` is diagnostic only.
- `build_state` must be `ready` before load or path-scoped sync is attempted.

### Git/workspace metadata in v1
- Workspace-local ignore rules continue to shape what is indexable/searchable through discovery.
- Git state is **not** used as an authoritative freshness source.
- No git rename/status heuristic is required for v1 correctness; git information may be logged later for diagnosis, but freshness decisions are made from current workspace content plus persisted SQLite metadata.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Single-path sync updates a row after a successful write | Add a temp-workspace test that builds an index, writes new content, runs `sync_written_path`, and asserts updated SHA/trigram rows in SQLite. |
| Unit | Single-path sync removes a row when the path no longer belongs in the indexable corpus | Use a path that becomes non-indexable for the index (for example index artifact path or size-gated case if practical) and assert row removal is committed cleanly. |
| Unit | `plan_candidates()` downgrades on hash mismatch even when mtime/size hints are unchanged or unhelpful | Seed/build index, mutate file contents, normalize mtime when practical, and assert `CandidateCoverage::Partial`. |
| Unit | `plan_candidates()` downgrades on deleted and rename-shaped changes | Build with `old.rs`, then delete or rename to `new.rs`, and assert stale/partial coverage plus no silent `Complete`. |
| Integration | `file_write` makes own writes visible to indexed search when a compatible DB already exists | Build index, execute `FileWriteTool`, then run `CodeSearchTool` and assert the new content is found without requiring manual refresh. |
| Integration | Sync failure does not fail the write and stale search still falls back safely | Force index unavailability/lock after a write, assert `file_write` still succeeds, and confirm `code_search` finds the file through fallback behavior. |
| Integration | Existing `code_search` live verification still suppresses false positives from stale candidate rows | Extend current regression coverage to assert stale planning downgrades before unsafe trust, while final match correctness remains live-verified. |

Focused validation command for implementation phase:

```bash
cargo test --manifest-path clients/agent-runtime/Cargo.toml search::
cargo test --manifest-path clients/agent-runtime/Cargo.toml tools::file_write
cargo test --manifest-path clients/agent-runtime/Cargo.toml tools::code_search
```

## Failure-Mode Handling

- **Write succeeds, index DB missing**: return success from `file_write`; `sync_written_path` returns `SkippedIndexUnavailable`; later search uses fallback or rebuilt index.
- **Write succeeds, index DB incompatible/incomplete**: do not mutate the DB; treat as skipped and rely on normal rebuild/fallback paths.
- **Write succeeds, index sync hits SQLite lock/busy error**: do not fail the write; emit diagnostic context and rely on freshness gate.
- **File disappears between write completion and sync**: treat as `Removed`; delete any stale row for that path.
- **Path is no longer indexable under corpus rules**: remove persisted row instead of inserting a misleading one.
- **Search discovery cannot prove freshness**: return `Partial`, never `Complete`.
- **Search query shape unsupported for trustworthy trigram narrowing**: keep returning `Unavailable` and use full live discovery.

## Migration / Rollout

No migration required.

The SQLite schema does not need a new table for v1. Existing file rows and metadata keys are sufficient because freshness correctness comes from stricter use of already persisted `content_sha256` plus localized row replacement/removal.

Rollout is code-only and reversible:
- revert the post-write sync hook,
- revert the stricter completeness gate,
- optionally delete `state/code-search/index.db` to force a clean rebuild under previous behavior.

## Open Questions

- [ ] None blocking for v1. The main follow-up after this design is whether future work should add background/watcher-driven refresh to reduce fallback frequency without widening the current change.
