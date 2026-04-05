# Design: Workspace Trigram Index

## Technical Approach

Implement a reusable workspace indexing capability under `clients/agent-runtime/src/search/` and keep
`code_search` as the existing tool surface. The new module owns three concerns that are currently mixed
inside `code_search`: deterministic workspace discovery, trigram extraction, and SQLite-backed index
lifecycle management.

The design reuses the same safety model already enforced by `clients/agent-runtime/src/tools/code_search.rs`
and `clients/agent-runtime/src/security/policy.rs`: relative path validation, canonicalized resolved-path
checks, `.gitignore`/standard ignore handling, hidden-directory skipping, symlink-escape rejection,
large-file rejection, unreadable-path rejection, and binary-content rejection. v1 adds durable local
storage at `workspace/state/code-search/index.db`, stores only workspace-relative file identities in file
rows, incrementally refreshes changed/deleted entries when the on-disk index is still compatible, and
uses full rebuilds with temp-DB + atomic rename only for missing, incompatible, or incomplete indexes.

## Architecture Decisions

### Decision: Add a reusable `search` runtime module instead of tool-local indexing code

**Choice**: Create a new top-level runtime module at `clients/agent-runtime/src/search/` and expose a
small public API for discovery and index lifecycle operations.

**Alternatives considered**:

- Add all index code directly inside `tools/code_search.rs`
- Hide indexing under `memory/sqlite.rs`
- Create a separate tool for indexing

**Rationale**: The proposal is about a reusable runtime capability, not tool-only glue. Keeping index
logic under `search/` lets `code_search` reuse the same deterministic discovery rules now while leaving a
clean path for later query-serving integration without coupling persistence to a single tool or to memory
storage.

### Decision: Extract discovery rules from `code_search` into shared search discovery helpers

**Choice**: Move the filesystem walk/filter logic into `search/discovery.rs`, and make `code_search`
consume that shared discovery API for scan-time enumeration.

**Alternatives considered**:

- Duplicate the walk/filter logic in the new indexer
- Keep `code_search` unchanged and reimplement equivalent rules in the indexer

**Rationale**: Discovery drift is the highest correctness risk called out in the proposal. One shared
implementation is the safest way to guarantee that scan-only search and indexed corpus builds accept and
reject the same files.

### Decision: Use a dedicated SQLite index file at `workspace/state/code-search/index.db`

**Choice**: Store the persistent index in its own SQLite database rooted at
`<workspace>/state/code-search/index.db`.

**Alternatives considered**:

- Reuse `memory/brain.db`
- Store JSON files or flat posting lists
- Store the index outside the workspace state tree

**Rationale**: A dedicated DB keeps code-search derived state isolated from memory data, aligns with the
existing workspace-local persistence style, and allows transactional build/load/rebuild behavior without
mixing unrelated schemas.

### Decision: Store file identity as workspace-relative paths only

**Choice**: Persist file rows with normalized workspace-relative paths (`/` separators), never absolute
workspace paths.

**Alternatives considered**:

- Persist canonical absolute paths
- Persist both absolute and relative paths

**Rationale**: This is a hard proposal requirement and avoids leaking host-specific paths into durable
state. Relative keys also make the index more portable across workspace relocations when the corpus is
otherwise unchanged.

### Decision: Prefer strict UTF-8 admission for indexed corpus contents

**Choice**: v1 indexes only files that both pass binary detection and decode as valid UTF-8 without
lossy conversion.

**Alternatives considered**:

- Continue using `String::from_utf8_lossy` as in `code_search`
- Index arbitrary bytes directly

**Rationale**: The index becomes durable corpus state, so silent replacement characters would make the
persisted corpus nondeterministic and harder to reason about. Strict exclusion is simpler, safer, and
matches the exploration recommendation.

### Decision: Incrementally refresh compatible indexes; rebuild incompatible ones with temp DB + atomic swap

**Choice**: Reuse a compatible `index.db` by refreshing only changed/deleted files in transactional
updates. When the DB is missing, incompatible, corrupted, or marked incomplete, build into a temp DB in
the same directory and atomically rename it over `index.db`.

**Alternatives considered**:

- Rebuild from scratch on every open
- Delete and recreate `index.db` directly
- Write postings to sidecar files

**Rationale**: Acceptance requires enough metadata to detect when entries need rebuild or refresh. A
small, transactional in-place refresh keeps unchanged rows reusable and still remains maintainable for
v1. Atomic temp-DB swap is still the safest path for forced rebuilds because it preserves the last
known-good index on failure.

## Module and File Placement

### New module layout

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/search/mod.rs` | Create | Public exports for search discovery and index lifecycle APIs. |
| `clients/agent-runtime/src/search/discovery.rs` | Create | Shared workspace walk/filter pipeline extracted from `code_search`. |
| `clients/agent-runtime/src/search/trigram.rs` | Create | Trigram extraction helpers and deterministic normalization rules. |
| `clients/agent-runtime/src/search/index.rs` | Create | Public `WorkspaceTrigramIndex` orchestration API (`open`, `build`, `load`, `refresh_or_rebuild`). |
| `clients/agent-runtime/src/search/sqlite.rs` | Create | SQLite schema creation, metadata reads/writes, bulk insert, temp-DB swap. |
| `clients/agent-runtime/src/search/tests.rs` or inline `#[cfg(test)]` blocks | Create | Shared unit/integration tests for discovery and index lifecycle. |

### Existing files likely modified

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/lib.rs` | Modify | Export the new `search` module. |
| `clients/agent-runtime/src/tools/code_search.rs` | Modify | Replace tool-local filesystem enumeration with shared discovery helpers. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Only if shared exports or construction wiring are needed by runtime tests. |
| `clients/agent-runtime/Cargo.toml` | Modify (only if needed) | Prefer no new dependency; use existing `rusqlite`, `ignore`, `regex`, `sha2`. |

## Data Model / SQLite Schema

The index schema stays minimal for v1: workspace metadata, indexed file membership, and trigram
postings. Query-specific ranking/snippet tables are deferred.

### Metadata

Use a dedicated key/value metadata table so lifecycle/version fields are explicit and extensible.

```sql
CREATE TABLE metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
) WITHOUT ROWID;
```

Required metadata keys for v1:

- `schema_version`
- `format_version`
- `workspace_fingerprint`
- `discovery_rules_version`
- `built_at`
- `build_state`
- `builder_version`

`build_state` values:

- `building` - temp DB still being constructed
- `ready` - last completed build is usable
- `failed` - optional terminal state recorded only inside temp DB before cleanup/logging

### Files table

```sql
CREATE TABLE files (
  file_id INTEGER PRIMARY KEY,
  relative_path TEXT NOT NULL UNIQUE,
  size_bytes INTEGER NOT NULL,
  modified_unix_ms INTEGER NOT NULL,
  trigram_count INTEGER NOT NULL,
  content_sha256 TEXT NOT NULL
);
CREATE INDEX idx_files_relative_path ON files(relative_path);
```

Notes:

- `relative_path` is always workspace-relative and normalized with `/` separators.
- `content_sha256` is derived from admitted file bytes; it supports deterministic inspection and future
  incremental refresh without storing contents.
- `modified_unix_ms` and `size_bytes` are refresh hints, not security checks.

### Trigram postings table

```sql
CREATE TABLE trigram_postings (
  trigram BLOB NOT NULL,
  file_id INTEGER NOT NULL,
  occurrences INTEGER NOT NULL,
  PRIMARY KEY (trigram, file_id),
  FOREIGN KEY (file_id) REFERENCES files(file_id) ON DELETE CASCADE
) WITHOUT ROWID;
CREATE INDEX idx_trigram_postings_file ON trigram_postings(file_id);
```

Notes:

- `trigram` is stored as a 3-byte blob derived from sliding windows over the file's raw UTF-8 bytes.
- v1 stores per-file occurrence counts, not positions. Positions/snippets are deferred.
- A normalized `trigrams` lookup table is intentionally skipped in v1 to keep writes simpler and reduce
  schema surface.

### Trigram extraction rules

- Admission happens only after the file passes safety/discovery checks and strict UTF-8 validation.
- Trigrams are generated from raw UTF-8 bytes, preserving exact corpus bytes.
- Files shorter than 3 bytes are admitted into `files` with `trigram_count = 0` and no posting rows.

## Interfaces / Contracts

```rust
pub struct DiscoveryRules {
    pub max_file_size_bytes: u64,
    pub follow_links: bool,
    pub include_hidden: bool,
}

pub struct DiscoveredFile {
    pub resolved_path: PathBuf,
    pub relative_path: String,
    pub metadata: std::fs::Metadata,
}

pub struct IndexBuildReport {
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub trigrams_written: usize,
    pub built_at: String,
}

pub struct IndexRefreshDecision {
    pub action: RefreshAction,
    pub reasons: Vec<String>,
}

pub enum RefreshAction {
    LoadExisting,
    Rebuild,
}

pub struct WorkspaceTrigramIndex {
    db_path: PathBuf,
}

impl WorkspaceTrigramIndex {
    pub fn for_workspace(workspace_dir: &Path) -> Self;
    pub fn load(&self) -> anyhow::Result<LoadedIndex>;
    pub fn build(&self, security: Arc<SecurityPolicy>) -> anyhow::Result<IndexBuildReport>;
    pub fn refresh_or_rebuild(
        &self,
        security: Arc<SecurityPolicy>,
    ) -> anyhow::Result<IndexBuildReport>;
}
```

Contract notes:

- Discovery returns both resolved and relative paths; only `relative_path` crosses into persistence.
- `WorkspaceTrigramIndex` is runtime capability code, not a `Tool` implementation.
- `code_search` continues exposing the user-facing tool API and may later decide whether to use the
  index or scan directly.
- `refresh_or_rebuild()` performs incremental per-file replace and delete in place (not a separate
  `RefreshExisting` action); the enum only exposes `LoadExisting` and `Rebuild` to represent the
  decision outcome.

## Discovery Pipeline

The discovery pipeline is shared between scan-only search and index builds.

1. Validate requested scope with `SecurityPolicy::is_path_allowed` when a caller provides a relative
   root.
2. Resolve the workspace root (or scoped subdirectory) with `canonicalize`.
3. Reject roots whose canonical path fails `SecurityPolicy::is_resolved_path_allowed`.
4. Walk files with `ignore::WalkBuilder` using the same defaults already used in `code_search`:
   standard filters, `.gitignore`, global gitignore, parent ignores, hidden-directory skipping,
   `follow_links(false)`.
5. For each file candidate:
   - canonicalize the entry path;
   - reject unresolved or escaping paths;
   - read metadata and reject unreadable entries;
   - reject non-files and files larger than `MAX_FILE_SIZE_BYTES`;
   - read bytes and reject unreadable files;
   - reject binary files using the same null-byte sample heuristic;
   - reject non-UTF-8 payloads in v1;
   - compute normalized workspace-relative path.
6. Yield `DiscoveredFile + admitted bytes` to either:
   - `code_search` for brute-force regex scanning, or
   - the index builder for trigram extraction and persistence.

This keeps one deterministic source of truth for workspace corpus admission.

## Data Flow

### Load / refresh / rebuild flow

```text
caller
  |
  v
WorkspaceTrigramIndex::refresh_or_rebuild
  |
  +--> load metadata from index.db
  |       |
  |       +--> compatible + fresh ----------> return loaded index
  |       |
  |       +--> compatible + stale ----------> refresh changed/deleted rows in place
  |       |
  |       +--> missing/incompatible/incomplete --> build temp DB
  |
  v
shared discovery pipeline
  |
  v
strict UTF-8 admission + trigram extraction
  |
  v
SQLite temp DB writer
  |
  v
mark metadata ready -> fsync -> atomic rename -> index.db
```

### Shared discovery reuse with `code_search`

```text
SecurityPolicy
      |
      v
search::discovery::walk_workspace()
      |                    |
      |                    +--> code_search regex scan
      |
      +--> WorkspaceTrigramIndex build pipeline
```

## Refresh / Rebuild Algorithm

v1 supports three lifecycle outcomes:

- load an unchanged compatible index,
- refresh changed/deleted rows inside a compatible index,
- perform a full rebuild when the on-disk index is missing, incompatible, or incomplete.

### Freshness inputs

The refresh decision reads `metadata` and compares:

- `schema_version`
- `format_version`
- `discovery_rules_version`
- `builder_version`
- `workspace_fingerprint`
- presence of required tables/indexes
- `build_state == ready`

### Workspace fingerprint

`workspace_fingerprint` is a stable digest of the canonical workspace root identity plus the current
indexing scope/version inputs. It is used for stale detection only and MUST NOT persist the raw absolute
workspace path.

### Refresh triggers

Refresh the existing compatible DB when metadata/schema are valid but one or more admitted files have:

- a new relative path not present in `files`,
- changed `size_bytes`, `modified_unix_ms`, or `content_sha256`,
- disappeared from the admitted corpus and therefore need row/posting deletion.

Refresh updates happen transactionally against the published DB and touch only changed/deleted rows.

### Rebuild triggers

Rebuild if any of the following are true:

- `index.db` does not exist
- schema or required metadata keys are missing
- `build_state != ready`
- schema/format/discovery/builder versions differ from the runtime
- workspace fingerprint differs from the current workspace
- SQLite open/schema validation fails

### Rebuild steps

1. Create `workspace/state/code-search/` if missing.
2. Create a temp DB in the same directory, e.g. `.index.db.tmp-<uuid>`.
3. Open the temp DB and apply SQLite pragmas tuned for bulk local writes (`WAL`, `NORMAL`, bounded
   cache/temp settings following existing sqlite patterns).
4. Create schema and write `metadata(build_state=building, ...)`.
5. Run shared discovery over the workspace root.
6. For each admitted file, insert the `files` row and bulk insert its aggregated trigram postings inside
   a transaction.
7. Finalize metadata (`built_at`, `build_state=ready`) and commit.
8. Sync/close the temp DB and atomically rename it to `index.db`.
9. Optionally sync the parent directory so the rename is durable.
10. On any failure, remove the temp DB and leave the previous `index.db` untouched.

### Refresh steps

1. Open the existing compatible `index.db`.
2. Run shared discovery over the workspace root.
3. Compare discovered file metadata against persisted `files` rows.
4. Insert rows/postings for new files, replace rows/postings for changed files, and delete rows/postings
   for files no longer present in the admitted corpus.
5. Commit the refresh transaction and update metadata such as `built_at` as needed.

## Concurrency / Atomicity Approach

### Process safety

v1 treats builds as serialized write operations.

- Reads open `index.db` normally.
- Rebuilds write only to a temp DB and do not mutate the active DB in place.
- The final publish step is a single rename on the same filesystem.

### Concurrent readers

Because the active DB is never modified in place during rebuild, concurrent readers either:

- keep using the old last-known-good `index.db`, or
- open the newly swapped `index.db` after rename.

No reader should ever observe a partially written published DB.

### Concurrent writers

If multiple rebuild attempts happen concurrently, v1 SHOULD fail one builder fast with a lock file or
best-effort exclusive create in `workspace/state/code-search/` rather than racing two swaps. The exact
mechanism can be a simple local build lock file because incremental multi-writer coordination is out of
scope.

## Safety Model Alignment

The index MUST NOT expand filesystem reach beyond the current runtime policy.

- Relative scopes are validated with `SecurityPolicy::is_path_allowed`.
- Resolved paths are checked with `SecurityPolicy::is_resolved_path_allowed` after canonicalization.
- Symlink escapes are excluded because resolved paths must remain under the canonical workspace root.
- `.gitignore` and standard hidden/ignored directory rules stay aligned with `code_search` by sharing
  the same `WalkBuilder` configuration.
- Unreadable files, unreadable metadata, oversized files, and binary files are skipped deterministically.
- Indexed rows persist only relative file identifiers; absolute paths stay in transient runtime memory
  only long enough to validate and read files.

## What Is Deferred From v1

The following are explicitly deferred to keep the first index layer small and reversible:

- Serving `code_search` queries from the trigram index
- Incremental per-file refresh/update/delete
- File watch mode or background rebuild daemons
- Position storage for snippets/highlighting
- Case-folded/normalized alternate trigram representations
- Semantic/vector indexing or hybrid ranking
- Multi-workspace or remote/shared index storage
- Complex query planner/ranking heuristics
- Storing lossy-decoded or arbitrary-byte documents in the indexed corpus

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Discovery parity with `code_search` defaults | Factor shared discovery tests from existing `code_search` cases: ignored files, hidden dirs, large files, binary files, symlink escapes, unreadable files, scoped subdirectories. |
| Unit | Strict UTF-8 admission | Temp files with invalid UTF-8 bytes must be skipped by indexing even if they are not binary by null-byte heuristic. |
| Unit | Relative path persistence | Inspect SQLite rows directly and assert `files.relative_path` never contains absolute workspace prefixes. |
| Unit | Trigram extraction determinism | Known byte sequences should generate stable posting counts and zero-posting behavior for files shorter than 3 bytes. |
| Integration | Build path | Create a temp workspace, build `index.db`, then verify metadata, file count, posting count, and build state. |
| Integration | Load existing vs rebuild decision | Seed compatible and stale DB fixtures and assert `refresh_or_rebuild` chooses the correct path. |
| Integration | Atomic rebuild safety | Start with a good index, inject a failing rebuild, and verify the original `index.db` remains readable. |
| Integration | Concurrency guard | Simulate two builders and assert one gets a deterministic busy/fail-fast result without corrupting the published DB. |
| Regression | Discovery alignment between scan and index | For the same temp workspace, compare files accepted by `code_search` shared discovery and the index build input set. |

## Migration / Rollout

No external migration is required. This is new derived workspace state under `workspace/state/code-search/`.

Rollout sequence:

1. Land shared discovery extraction.
2. Land SQLite schema + build/load/rebuild lifecycle.
3. Keep `code_search` behavior unchanged externally.
4. Add later query integration only after the index lifecycle proves stable.

## Open Questions

- [ ] Should the workspace fingerprint include a cheap workspace marker beyond canonical root identity
      (for example repo root metadata) to better detect moved/copied workspaces without storing raw
      absolute paths?
- [ ] Should v1 publish a lightweight health/status API for the index builder now, or wait until query
      integration needs it?
- [ ] Is a dedicated build lock file sufficient, or does the team want a stricter SQLite/open-file lock
      contract for cross-process coordination in v1?