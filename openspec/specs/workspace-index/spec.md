# Workspace Index Specification

## Purpose

Defines the first-version persistent workspace corpus index for `clients/agent-runtime`, including
the SQLite-backed storage contract, compatibility metadata, and build/load/refresh/rebuild
behavior for a deterministic trigram index.

## Requirements

### Requirement: REQ-WIDX-001 SQLite Index Logical Contract

The system MUST persist the workspace trigram index in SQLite as a logical contract with enough
structure to recover corpus membership and trigram data without re-scanning the full workspace on
every load.

The persisted index MUST contain behaviorally distinct records for:

- index metadata,
- corpus file entries keyed by workspace-relative path,
- trigram data associated with indexed files,
- build lifecycle state sufficient to distinguish complete indexes from interrupted ones.

The contract MAY evolve internally, but a compatible index MUST preserve these logical record types
and their meanings.

#### Scenario: Initial build persists required logical record types

- GIVEN a workspace with indexable text files
- WHEN the workspace trigram index is built for the first time
- THEN the SQLite database MUST contain metadata describing the index build
- AND it MUST contain corpus file entries keyed by workspace-relative path
- AND it MUST contain trigram data for the admitted files
- AND it MUST record that the build completed successfully

### Requirement: REQ-WIDX-002 Workspace-Relative File Identity

Every persisted corpus entry MUST use a workspace-relative file path as its stable identity.

Absolute workspace paths MUST NOT be stored as file identities in corpus-entry records, trigram
records, or externally visible index metadata. The index MAY retain non-file metadata needed to
identify the owning workspace, but stored file entries themselves MUST remain relative.

#### Scenario: Persisted file entries are stored as relative paths only

- GIVEN a workspace rooted at `/workspace`
- AND a file `/workspace/src/main.rs` is admitted to the corpus
- WHEN the workspace trigram index is persisted
- THEN the stored file identity MUST be `src/main.rs`
- AND `/workspace/src/main.rs` MUST NOT be stored as the file identity in index rows

### Requirement: REQ-WIDX-003 Compatibility and Freshness Metadata

The index MUST persist metadata sufficient to determine whether an on-disk index is compatible with
the current runtime and current workspace.

At minimum, that metadata MUST allow the runtime to detect:

- index format version compatibility,
- the workspace identity the index belongs to,
- whether the last build completed successfully,
- whether persisted corpus entries are stale relative to current file state.

#### Scenario: Compatible metadata allows existing index load

- GIVEN an existing SQLite index whose format version is supported
- AND the index metadata identifies the current workspace
- AND the previous build state is complete
- WHEN the runtime opens the workspace trigram index
- THEN the runtime MUST treat the on-disk index as loadable
- AND it MUST proceed to freshness checks instead of forcing a full rebuild immediately

### Requirement: REQ-WIDX-004 Initial Build Behavior

When no compatible completed index exists for the active workspace, the runtime MUST perform an
initial build from the discovered workspace corpus.

An initial build MUST populate metadata, corpus entries, and trigram data within a lifecycle that
can later be recognized as complete or incomplete.

#### Scenario: Missing index triggers first build

- GIVEN a workspace with no existing trigram index database
- WHEN the runtime requests the workspace trigram index
- THEN the runtime MUST build a new SQLite-backed index from the admitted corpus
- AND the resulting index MUST be marked complete only after build success

### Requirement: REQ-WIDX-005 Compatible Load and Refresh Behavior

When a compatible completed index already exists for the active workspace, the runtime MUST load it
and refresh persisted state for files whose current workspace state has changed.

Refresh behavior MUST:

- reuse persisted entries for unchanged files,
- update corpus metadata and trigram data for changed files,
- leave unrelated unchanged files intact.

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

### Requirement: REQ-WIDX-006 Deleted File Removal

Refresh behavior MUST remove files from the persisted corpus when they are no longer present in the
admitted workspace corpus.

Removing a deleted file MUST also remove its associated trigram data so the index no longer reports
that file as indexed content.

#### Scenario: Deleted file is removed during refresh

- GIVEN a completed SQLite index containing entries for `src/old.rs` and `src/new.rs`
- AND `src/old.rs` has been deleted from the workspace
- WHEN the runtime refreshes the workspace trigram index
- THEN the corpus entry for `src/old.rs` MUST be removed from the database
- AND trigram data associated with `src/old.rs` MUST also be removed
- AND `src/new.rs` MUST remain indexed

### Requirement: REQ-WIDX-007 Forced Rebuild on Incompatible or Incomplete State

The runtime MUST force a full rebuild instead of trusting an existing index when the persisted state
is incompatible or incomplete.

A full rebuild MUST be triggered when at least one of the following is true:

- the stored index format version is unsupported,
- the stored workspace identity does not match the active workspace,
- the stored build lifecycle state is incomplete or interrupted.

During a forced rebuild, previously persisted corpus and trigram data from the incompatible or
incomplete index MUST NOT be treated as authoritative.

#### Scenario: Version mismatch forces rebuild

- GIVEN an existing SQLite index whose stored format version is not supported by the runtime
- WHEN the runtime opens the workspace trigram index
- THEN the runtime MUST discard that index as incompatible
- AND it MUST rebuild the index from the current admitted workspace corpus

#### Scenario: Workspace mismatch forces rebuild

- GIVEN an existing SQLite index file copied from a different workspace
- WHEN the runtime opens the workspace trigram index for the current workspace
- THEN the runtime MUST detect the workspace mismatch from persisted metadata
- AND it MUST perform a full rebuild for the current workspace instead of loading foreign entries

#### Scenario: Incomplete prior build forces rebuild

- GIVEN an existing SQLite index whose persisted lifecycle state indicates an interrupted or
  incomplete build
- WHEN the runtime opens the workspace trigram index
- THEN the runtime MUST treat that index as unusable
- AND it MUST perform a full rebuild before the index is considered available

### Requirement: REQ-WIDX-008 Verification Coverage

The workspace trigram index MUST have automated verification that proves build, load, refresh,
rebuild, and deterministic exclusion behavior.

At minimum, automated tests MUST cover:

- initial build from an empty on-disk state,
- loading an existing compatible index,
- refreshing changed files,
- removing deleted files,
- forcing rebuild for version mismatch, workspace mismatch, and incomplete build state,
- deterministic exclusion of unsafe paths, symlink escapes, binary files, invalid UTF-8 or other
  non-text files, and self-index files.

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
