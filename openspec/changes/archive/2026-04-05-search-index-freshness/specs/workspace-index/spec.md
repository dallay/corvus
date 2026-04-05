# Delta for Workspace Index

## MODIFIED Requirements

### Requirement: REQ-WIDX-003 Compatibility and Freshness Metadata

The index MUST persist metadata sufficient to determine whether an on-disk index is compatible with
the current runtime and current workspace.

At minimum, that metadata MUST allow the runtime to detect:

- index format version compatibility,
- the workspace identity the index belongs to,
- whether the last build completed successfully,
- whether persisted corpus entries are stale relative to current file state.

For v1 freshness decisions, each persisted corpus entry MUST use workspace-relative path as the
entry identity and MUST retain `size_bytes`, `modified_unix_ms`, and `content_sha256` for the
current indexed contents. `size_bytes` and `modified_unix_ms` SHALL be treated as change hints, but
`content_sha256` MUST be the correctness guard when the runtime decides whether an indexed entry is
still trustworthy enough to support complete indexed candidate coverage. Diagnostic signals such as
git state MAY be recorded or consulted, but they MUST NOT be the sole authority for freshness.

Known v1 limits MUST be documented: freshness is path-based rather than watcher-based, rename
handling is modeled as delete plus add, and timestamp values alone are insufficient to prove that a
file is unchanged.

#### Scenario: Freshness metadata records the v1 trust signals

- GIVEN a completed workspace index for the active workspace
- WHEN the runtime persists corpus-entry metadata for an indexed file
- THEN the entry MUST be keyed by workspace-relative path
- AND it MUST retain `size_bytes`, `modified_unix_ms`, and `content_sha256` for that path
- AND any freshness decision that claims complete indexed coverage MUST treat `content_sha256` as
  the trust boundary instead of relying only on timestamp or size hints

#### Scenario: Optional git state is never treated as the sole freshness source

- GIVEN a workspace whose tracked and untracked files may change without a commit
- WHEN the runtime evaluates whether persisted corpus entries remain fresh
- THEN it MAY use git-derived information as a diagnostic signal
- BUT it MUST NOT trust persisted entries solely because git state appears unchanged

### Requirement: REQ-WIDX-005 Compatible Load and Refresh Behavior

When a compatible completed index already exists for the active workspace, the runtime MUST load it
and refresh persisted state for files whose current workspace state has changed.

Refresh behavior MUST:

- reuse persisted entries for unchanged files,
- update corpus metadata and trigram data for changed files,
- leave unrelated unchanged files intact.

When an agent-originated file write succeeds for a path inside the active workspace and a
compatible index already exists, the runtime MUST update or invalidate the affected persisted entry
as part of the same write-through freshness flow so subsequent search requests in that workspace do
not depend on a manual rebuild to observe the new contents. If no compatible index exists yet, the
runtime MAY skip the write-through update and rely on safe fallback behavior until the index is
built.

#### Scenario: Existing compatible index is loaded and unchanged files are reused

- GIVEN a completed SQLite index for the current workspace
- AND none of the admitted corpus files have changed since the prior build
- WHEN the runtime opens the workspace trigram index
- THEN the existing index MUST be loaded without a full rebuild
- AND previously persisted corpus and trigram data for unchanged files MUST remain reusable

#### Scenario: Changed file is refreshed in place

- GIVEN a completed SQLite index for the current workspace
- AND `src/lib.rs` has changed since the prior build
- AND `src/main.rs` has not changed
- WHEN the runtime refreshes the workspace trigram index
- THEN the persisted corpus metadata and trigram data for `src/lib.rs` MUST be replaced with data
  derived from the new file contents
- AND the persisted data for `src/main.rs` MUST remain intact without rebuild of unrelated files

#### Scenario: Successful agent write is reflected without manual rebuild

- GIVEN a completed compatible SQLite index for the current workspace
- AND `src/query.rs` is already indexed
- WHEN an agent file-write operation successfully writes new contents to `src/query.rs`
- THEN the runtime MUST update or invalidate the persisted entry for `src/query.rs` in the same
  freshness flow
- AND a subsequent search in that workspace MUST NOT require a manual rebuild to observe the new
  contents

### Requirement: REQ-WIDX-006 Deleted File Removal

Refresh behavior MUST remove files from the persisted corpus when they are no longer present in the
admitted workspace corpus.

Removing a deleted file MUST also remove its associated trigram data so the index no longer reports
that file as indexed content.

When a path disappears because a file was renamed, the v1 index MUST treat the old path as deleted
and the new path as added content. The runtime MUST NOT continue trusting the old path as a current
indexed file after the rename.

#### Scenario: Deleted file is removed during refresh

- GIVEN a completed SQLite index containing entries for `src/old.rs` and `src/new.rs`
- AND `src/old.rs` has been deleted from the workspace
- WHEN the runtime refreshes the workspace trigram index
- THEN the corpus entry for `src/old.rs` MUST be removed from the database
- AND trigram data associated with `src/old.rs` MUST also be removed
- AND `src/new.rs` MUST remain indexed

#### Scenario: Renamed file is handled as delete plus add

- GIVEN a completed SQLite index containing an entry for `src/legacy.rs`
- AND the workspace now contains the same file contents at `src/current.rs` instead of
  `src/legacy.rs`
- WHEN the runtime refreshes or validates the workspace trigram index
- THEN it MUST stop treating `src/legacy.rs` as a current indexed file
- AND it MUST treat `src/current.rs` as newly added content to refresh or index
- AND it MUST NOT require explicit rename tracking to stay correct

### Requirement: REQ-WIDX-008 Verification Coverage

The workspace trigram index MUST have automated verification that proves build, load, refresh,
rebuild, deterministic exclusion behavior, and freshness-guard correctness.

At minimum, automated tests MUST cover:

- initial build from an empty on-disk state,
- loading an existing compatible index,
- refreshing changed files,
- removing deleted files,
- forcing rebuild for version mismatch, workspace mismatch, and incomplete build state,
- deterministic exclusion of unsafe paths, symlink escapes, binary files, invalid UTF-8 or other
  non-text files, and self-index files,
- successful agent-write freshness for already indexed files,
- changed, deleted, and renamed workspace paths that would otherwise leave stale indexed rows,
- stale-entry protection when size or mtime hints alone could incorrectly appear unchanged.

#### Scenario: Automated tests prove lifecycle behavior

- GIVEN the workspace trigram index implementation
- WHEN its automated test suite is executed
- THEN the suite MUST include scenarios for build, compatible load, refresh, deleted-file removal,
  and forced rebuild states
- AND those tests MUST assert observable persisted-index outcomes rather than only in-memory success

#### Scenario: Automated tests prove deterministic exclusions

- GIVEN workspace fixtures that include safe text files, unsafe paths, symlink escapes, binary
  files, invalid UTF-8 files, and index database files
- WHEN the workspace trigram index tests are executed repeatedly against the same fixtures
- THEN the admitted corpus membership MUST be identical across runs
- AND excluded files MUST remain absent from persisted corpus entries and trigram data

#### Scenario: Regression tests prove stale-state safety

- GIVEN a compatible completed index and a workspace whose files have been changed, deleted, or
  renamed since the persisted rows were written
- WHEN the runtime verification suite exercises indexed search planning and file-write freshness
  flows
- THEN the tests MUST prove the runtime does not silently trust stale indexed candidates as
  complete coverage
- AND they MUST prove successful agent writes and safe fallback or refresh behavior for stale paths

## ADDED Requirements

### Requirement: REQ-WIDX-011 Indexed Candidate Freshness Guard

The runtime MUST NOT report complete indexed candidate coverage for `code_search` unless the current
searchable workspace state exactly matches the persisted corpus entries that would be trusted for
that request.

If discovery or validation finds a changed file, missing file, extra stale indexed row, or a
rename-shaped old/new path mismatch, the runtime MUST either refresh/remove the affected persisted
rows before using them or downgrade indexed coverage so downstream search falls back to live
verification instead of silently trusting stale candidates.

#### Scenario: Changed file prevents silently trusting complete indexed coverage

- GIVEN a completed compatible index whose persisted entry for `src/filter.rs` no longer matches the
  file's current contents
- WHEN the runtime plans indexed candidates for a search request that could match `src/filter.rs`
- THEN it MUST NOT report complete indexed coverage based on the stale persisted entry
- AND it MUST refresh the entry or downgrade coverage so live verification remains authoritative

#### Scenario: Missing or extra indexed path prevents silently trusting complete indexed coverage

- GIVEN a completed compatible index whose persisted corpus does not exactly match the current
  searchable workspace paths
- WHEN the runtime plans indexed candidates for a search request
- THEN it MUST NOT treat the persisted candidate set as complete
- AND it MUST refresh or remove stale rows, or downgrade coverage so the safe fallback path is used

### Requirement: REQ-WIDX-012 V1 Freshness Documentation

The workspace-index specification and related change artifacts MUST document the v1 freshness model
clearly enough for implementers and reviewers to understand what the index guarantees and what it
does not.

That documentation MUST state that workspace-relative path is the persisted identity,
`size_bytes` and `modified_unix_ms` are fast hints, `content_sha256` is the trust boundary for
freshness, agent writes use write-through freshness only after successful writes when a compatible
index exists, and v1 does not include filesystem watchers or perfect rename tracking.

#### Scenario: V1 freshness model is documented with guarantees and limits

- GIVEN the workspace-index OpenSpec artifacts for this change
- WHEN a reviewer reads the freshness requirements
- THEN the artifacts MUST describe the v1 metadata and trust signals
- AND they MUST state the known limits and non-goals that remain outside this change
