# Delta for Workspace Index

## ADDED Requirements

### Requirement: REQ-WIDX-009 Indexed Candidate Files Are Advisory and Deterministic

The system MUST use the local workspace trigram index only to derive workspace-relative candidate
files for `code_search`.

Indexed candidate extraction MUST NOT create externally visible matches on its own. It MUST return
candidate file identities in deterministic workspace-relative path order so downstream verification
can preserve stable processing.

#### Scenario: Index returns advisory candidate files only

- GIVEN a compatible workspace trigram index
- AND a search request whose pattern can safely produce required trigrams
- WHEN the system derives candidate files from the index
- THEN it MUST return only workspace-relative file identities as candidate inputs
- AND it MUST NOT treat any indexed candidate as a reported match before live file-content
  verification occurs

#### Scenario: Candidate files are ordered deterministically

- GIVEN a compatible workspace trigram index containing candidate files `src/z.rs`, `src/a.rs`, and
  `src/m.rs`
- WHEN the system derives candidate files for the same request on repeated runs
- THEN the candidate file list MUST be returned in the order `src/a.rs`, `src/m.rs`, `src/z.rs`
- AND that ordering MUST remain stable across repeated runs on unchanged workspace contents

### Requirement: REQ-WIDX-010 Candidate Extraction Must Signal When It Cannot Safely Narrow Search

The index query surface MUST report when candidate extraction cannot safely narrow the verification
set for the active request.

When the active request cannot produce a trustworthy candidate reduction, the system MUST NOT claim
that the derived candidate set is complete.

#### Scenario: Query without trustworthy trigram reduction is not treated as complete

- GIVEN a search request whose semantics do not yield a trustworthy required trigram set
- WHEN the system evaluates whether to derive indexed candidates
- THEN it MUST signal that indexed candidate extraction cannot safely narrow the request
- AND it MUST leave final correctness to the safe fallback search path
