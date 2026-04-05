# Tasks: Indexed Candidate Selection for `code_search`

## Phase 1: Foundation / Contract-First TDD

- [x] 1.1 In `clients/agent-runtime/src/search/tests.rs`, add RED tests for safe query analysis: case-sensitive literals with required trigrams, regex/whole-word/short patterns downgrading to non-complete coverage.
- [x] 1.2 In `clients/agent-runtime/src/search/index.rs`, add `CandidateCoverage`, `CandidateRequest`, and `CandidatePlan`; in `clients/agent-runtime/src/search/mod.rs`, export the new planner API used by `code_search`.
- [x] 1.3 In `clients/agent-runtime/src/search/index.rs`, implement planner reason codes and coverage classification so unsupported or parity-risk requests return `Partial` or `Unavailable`, never a false `Complete`.

## Phase 2: Deterministic Indexed Candidate Retrieval

- [x] 2.1 In `clients/agent-runtime/src/search/tests.rs`, add RED tests for SQLite candidate intersection, root-prefix restriction, empty/weak-query handling, and lexical candidate ordering.
- [x] 2.2 In `clients/agent-runtime/src/search/sqlite.rs`, implement deterministic helpers for `read_candidate_paths` and `read_index_file_count`, with `ORDER BY f.relative_path ASC` and trigram intersection semantics.
- [x] 2.3 In `clients/agent-runtime/src/search/index.rs`, wire `WorkspaceTrigramIndex` loading plus SQLite helpers into a planner that returns ordered workspace-relative paths and explicit coverage.

## Phase 3: `code_search` Orchestration and Result Shape

- [x] 3.1 In `clients/agent-runtime/src/tools/code_search.rs` test module, add RED tests for indexed false positives being removed by live verification and for index-unavailable requests falling back to scan-only parity.
- [x] 3.2 In `clients/agent-runtime/src/tools/code_search.rs`, extend the structured `SearchMatch` payload with `line_end`, `column_end`, `byte_start`, `byte_end`, and `preview` while preserving existing fields.
- [x] 3.3 In `clients/agent-runtime/src/tools/code_search.rs`, integrate indexed prefilter planning, authoritative live verification, stable union+dedeupe for `Partial`, and full discovery fallback for `Unavailable`.
- [x] 3.4 In `clients/agent-runtime/src/tools/code_search.rs`, enforce deterministic verified ordering by `(file, byte_start, byte_end)` and apply `max_results` plus truncation stats only after verification.

## Phase 4: Regression Coverage / Safe Completion

- [x] 4.1 In `clients/agent-runtime/src/tools/code_search.rs` test module, add regression tests for deterministic ordering across repeated runs and verified-result limits with more candidates than final matches.
- [x] 4.2 In `clients/agent-runtime/src/tools/code_search.rs` test module, add regression tests asserting offsets and `preview` fields match the verified range and preserve existing schema fields.
- [x] 4.3 In `clients/agent-runtime/src/search/tests.rs`, add regression tests for `Complete`/`Partial`/`Unavailable` planner outcomes, including fallback-trigger reasons tied to false-positive and corpus-parity risks.
