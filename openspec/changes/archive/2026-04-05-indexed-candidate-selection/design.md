# Design: Indexed Candidate Selection for `code_search`

## Technical Approach

This change keeps `clients/agent-runtime/src/tools/code_search.rs` as the boundary for request
validation, security checks, candidate-strategy selection, live-file verification, deterministic
output formatting, and result truncation. Indexed candidate extraction will be added under
`clients/agent-runtime/src/search/*`, primarily by extending the workspace trigram index query
surface in `search/index.rs` and `search/sqlite.rs`.

The implementation is intentionally conservative. The index is an optimization, not a source of
truth. `code_search` will only trust indexed candidate selection when it can prove the index
preserves correctness for the active request; otherwise it will fall back to the existing
discovery-driven scan path. This aligns with `openspec/specs/workspace-index/spec.md`,
`openspec/specs/result-format/spec.md`, `openspec/specs/regex-semantics/spec.md`, and
`openspec/specs/safety-model/spec.md`.

The first slice should favor safe eligibility over aggressive indexing:

- require candidate extraction to live under `src/search/*`,
- keep `code_search` responsible for orchestration and verification,
- make fallback mandatory when the index is unavailable, stale for the request semantics, or too
  weak to preserve correctness,
- make deterministic ordering explicit in SQL candidate reads and in verified match emission,
- extend structured results with explicit verified location metadata and preview fields.

## Architecture Decisions

### Decision: Keep candidate planning in `search/*` and verification in `code_search`

**Choice**: Add an indexed candidate-selection API in `clients/agent-runtime/src/search/index.rs`
backed by SQLite helpers in `clients/agent-runtime/src/search/sqlite.rs`, while
`clients/agent-runtime/src/tools/code_search.rs` remains responsible for security checks, pattern
compilation, fallback choice, live-file verification, limits, and output formatting.

**Alternatives considered**:

- Move verification into `search/index.rs`
- Let SQLite return final matches directly from indexed content

**Rationale**: The current tool already owns security/path validation, result formatting,
truncation, and live-content semantics. Keeping those concerns in `code_search` preserves the
existing boundary and avoids turning the persisted index into an externally visible source of truth.

### Decision: Use a conservative index-eligibility gate

**Choice**: Introduce a small request-analysis layer that derives required trigrams only for query
shapes that are provably safe for byte-preserving prefiltering. The initial safe set should be
case-sensitive literal queries with at least one trigram; queries that depend on regex semantics,
case folding, or other transformations fall back immediately unless the extractor can prove a
required trigram set.

**Alternatives considered**:

- Attempt trigram extraction for every regex
- Use the compiled regex string as the trigram source
- Force all requests through the index

**Rationale**: The index stores raw UTF-8 trigrams, while `code_search` semantics include regex
features, case-insensitive mode, whole-word wrappers, and lossy verification of non-UTF-8 bytes
today. A conservative gate avoids false negatives and gives a safe path to incremental expansion.

### Decision: Model candidate coverage explicitly

**Choice**: The `search/*` API should return a candidate plan with explicit coverage semantics, for
example `Complete`, `Partial`, or `Unavailable`, plus ordered relative paths and a reason code.

**Alternatives considered**:

- Return only `Vec<String>` and let the caller guess
- Treat every successful index lookup as complete

**Rationale**: The tool needs to know whether it may verify only indexed candidates or must combine
them with scan fallback. Making coverage explicit is the cleanest way to encode safety decisions
driven by corpus parity, path filters, and query shape.

### Decision: Make deterministic ordering contractual in both SQL and verification

**Choice**: SQL candidate queries MUST end with `ORDER BY f.relative_path ASC`, and verification
MUST emit matches ordered by `(file relative path, byte_start, byte_end)` with file traversal based
on that same sorted relative-path order.

**Alternatives considered**:

- Depend on SQLite row order
- Preserve whichever order the candidate source happened to produce
- Verify indexed candidates first and fallback files later without reordering

**Rationale**: The proposal requires stable results across runs. Determinism must be explicit at
both the candidate-selection stage and the verified-result stage, especially if the implementation
buffers indexed and fallback hits before formatting.

### Decision: Extend match objects compatibly instead of replacing them

**Choice**: Preserve the current top-level match fields (`file`, `line`, `column`, `content`,
`context_before`, `context_after`) and add explicit verified metadata fields for byte/offset and
preview information.

**Alternatives considered**:

- Replace the match schema with a nested location object
- Expose index-internal scores or postings metadata

**Rationale**: Existing consumers already rely on the current result format. Adding fields is safer
than reshaping the payload, and it satisfies the new requirement for line/column, byte/offset, and
preview metadata without making the index visible in the public contract.

## Data Flow

### Request Flow

```text
Caller
  │
  ▼
CodeSearchTool::execute
  │  validate args + compile regex + security chain
  ▼
search_workspace
  │
  ├─► Candidate planner (search/index.rs)
  │      │
  │      ├─ analyze query for safe trigram extraction
  │      ├─ load compatible workspace index if available
  │      ├─ run deterministic SQLite candidate query
  │      └─ return CandidatePlan { coverage, ordered_paths, reason }
  │
  ├─► if coverage == Complete
  │      verify only ordered indexed paths against live file contents
  │
  ├─► if coverage == Partial
  │      combine indexed paths with fallback-discovered remainder
  │      dedupe by relative path
  │      verify in final sorted file order
  │
  └─► if coverage == Unavailable
         use existing discovery scan path unchanged

Verified matches
  │  compute line/column + byte offsets + preview
  ▼
Deterministic formatting + truncation
  ▼
ToolResult { output, structured }
```

### Safe Combination Algorithm

Recommended hybrid algorithm:

1. **Analyze request safety**
    - Derive required trigrams only when the request is byte-preserving.
    - If no required trigram set can be proven, return `Unavailable` and use full scan.
2. **Load and validate index**
    - Use existing compatibility checks in `WorkspaceTrigramIndex::load()`.
    - On any load/query error, return `Unavailable` and use full scan.
3. **Query deterministic candidates**
    - Query SQLite for files containing all required trigrams.
    - Restrict by path prefix in SQL when possible.
    - Always sort by `relative_path ASC` in SQL.
4. **Assess coverage**
    - `Complete`: request semantics and corpus parity are proven compatible.
    - `Partial`: the index can help, but cannot prove completeness because of parity risks or
      request filters.
    - `Unavailable`: no safe indexed value.
5. **Verify against live files**
    - For `Complete`, verify only indexed paths.
    - For `Partial`, build a **stable union** of `indexed_paths ∪ fallback_discovered_paths`, dedupe
      by relative path, then verify in lexical file order.
    - For `Unavailable`, use the existing discovery list.
6. **Emit deterministic results**
    - Within each file, compute verified matches in ascending byte-start order.
    - Apply `max_results` only after live verification, never at raw-candidate time.

This algorithm preserves correctness even when the index is only a helpful subset.

### Sequence Diagram

```text
CodeSearchTool        SearchIndex          SQLite              Discovery          Live File Verify
     |                    |                  |                     |                     |
     | execute()          |                  |                     |                     |
     |------------------->| analyze query    |                     |                     |
     |                    | load index       |                     |                     |
     |                    |----------------->| open/read          |                     |
     |                    |<-----------------| ordered candidates |                     |
     |<-------------------| CandidatePlan    |                     |                     |
     |    coverage=Complete/Partial/Unavailable                  |                     |
     |                    |                  |                     |                     |
     | if Partial/Unavailable                                       discover files      |
     |------------------------------------------------------------>| sorted paths        |
     |<------------------------------------------------------------|                     |
     |                                                                                    |
     | verify final ordered file stream ------------------------------------------------->|
     |<-----------------------------------------------------------------------------------|
     | format output + structured payload                                                  |
```

## File Changes

| File                                                           | Action | Description                                                                                                                                   |
|----------------------------------------------------------------|--------|-----------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/search/index.rs`                    | Modify | Add candidate-planning API, request analysis, coverage classification, and index-backed candidate retrieval surface.                          |
| `clients/agent-runtime/src/search/sqlite.rs`                   | Modify | Add deterministic read queries for candidate paths and supporting counts/metadata needed for selectivity and coverage decisions.              |
| `clients/agent-runtime/src/search/mod.rs`                      | Modify | Export the candidate-selection types/functions used by `code_search`.                                                                         |
| `clients/agent-runtime/src/tools/code_search.rs`               | Modify | Keep orchestration here; integrate candidate planning, safe fallback, deterministic verification order, and richer structured match metadata. |
| `clients/agent-runtime/src/search/tests.rs`                    | Modify | Add index-selection, SQL ordering, coverage-classification, and parity/fallback regression tests.                                             |
| `clients/agent-runtime/src/tools/code_search.rs` (test module) | Modify | Add end-to-end tests for indexed-vs-fallback parity, verified-result ordering, `max_results` semantics, and enriched match output.            |
| `openspec/changes/indexed-candidate-selection/design.md`       | Create | Technical design for the change.                                                                                                              |

## Interfaces / Contracts

### Candidate selection contract

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateCoverage {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRequest {
    pub relative_root: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub raw_pattern: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePlan {
    pub coverage: CandidateCoverage,
    pub ordered_paths: Vec<String>,
    pub reason: String,
}
```

Notes:

- `CandidatePlan` lives under `search/*`.
- `code_search` remains the only layer that turns candidate paths into verified user-visible
  matches.
- `ordered_paths` MUST be workspace-relative and already sorted.

### SQLite query surface

```rust
pub fn read_candidate_paths(
    conn: &rusqlite::Connection,
    required_trigrams: &[[u8; 3]],
    relative_root_prefix: Option<&str>,
) -> anyhow::Result<Vec<String>>;

pub fn read_index_file_count(conn: &rusqlite::Connection) -> anyhow::Result<usize>;
```

Recommended SQL shape:

```sql
SELECT f.relative_path
FROM files f
JOIN trigram_postings p ON p.file_id = f.file_id
WHERE p.trigram IN (?, ?, ...)
  AND (?root IS NULL OR f.relative_path = ?root OR f.relative_path LIKE ?root_prefix)
GROUP BY f.file_id, f.relative_path
HAVING COUNT(DISTINCT p.trigram) = ?required_count
ORDER BY f.relative_path ASC;
```

Key points:

- `COUNT(DISTINCT ...)` enforces trigram intersection.
- `ORDER BY f.relative_path ASC` makes query ordering explicit.
- Path prefix restriction happens in SQL only when it is semantically safe.
- Additional include/exclude glob filtering may still happen in Rust before coverage is declared
  `Complete`.

### Structured match contract extension

```rust
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SearchMatch {
    file: String,
    line: usize,
    column: usize,
    content: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
    line_end: usize,
    column_end: usize,
    byte_start: usize,
    byte_end: usize,
    preview: String,
}
```

Notes:

- `line` / `column` remain backward-compatible start positions.
- `line_end` / `column_end` make range metadata explicit.
- `byte_start` / `byte_end` expose verified byte offsets within the file.
- `preview` is the normalized user-facing snippet for downstream tools; it should align with the
  truncated line content already used for `content`.
- The index MUST NOT leak postings counts or ranking signals into this public match schema.

## Testing Strategy

| Layer       | What to Test                            | Approach                                                                                                                                                                                |
|-------------|-----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Unit        | Query analysis and trigram eligibility  | Add focused tests for literal/case-sensitive eligibility, regex fallback, short-pattern fallback, and whole-word handling in `search/tests.rs`.                                         |
| Unit        | SQLite candidate reads                  | Build temporary indexes and assert candidate intersection, root-prefix restriction, and `ORDER BY relative_path` determinism.                                                           |
| Unit        | Coverage classification                 | Assert `Complete`, `Partial`, and `Unavailable` decisions for index-missing, incompatible-index, parity-risk, and unselective-query cases.                                              |
| Integration | `code_search` indexed verification path | Add tool tests that build an index, run search, and verify identical visible matches to the scan path for safe literal queries.                                                         |
| Integration | Hybrid fallback parity                  | Add tests where indexed corpus cannot prove completeness (for example request filters or non-UTF-8/parity fixtures) and assert stable union behavior plus result parity with full scan. |
| Integration | Deterministic ordering                  | Assert final match ordering by file path and in-file byte location across repeated runs.                                                                                                |
| Integration | Structured result schema                | Assert presence and correctness of `line_end`, `column_end`, `byte_start`, `byte_end`, and `preview`, while preserving existing fields.                                                 |
| Integration | Verified truncation semantics           | Assert `max_results` truncates after verification, not after candidate enumeration, and remains deterministic with indexed + fallback flows.                                            |

## Migration / Rollout

No migration required.

The SQLite schema can stay unchanged if candidate reads use existing `files` and `trigram_postings`
tables. If additional metadata is needed for coverage decisions, it should be additive and versioned
through the existing compatibility mechanism. Rollout should be conservative:

1. ship the candidate planner behind the existing safe fallback rules,
2. treat unsupported or ambiguous requests as scan-only,
3. expand eligibility only after parity tests prove correctness.

## Open Questions

- [ ] The searchable corpus currently tolerates invalid UTF-8 via `String::from_utf8_lossy`, while
  the indexed corpus excludes invalid UTF-8 entirely. Should the long-term contract align on
  UTF-8-only search, or should hybrid fallback remain mandatory whenever parity cannot be proven?
- [ ] Do we want the first implementation to support only case-sensitive literal prefiltering, or do
  we want a minimal regex trigram extractor in this change?
- [ ] A follow-up delta spec is still needed to formalize the new structured match fields and
  indexed-fallback behavior under `openspec/changes/indexed-candidate-selection/specs/`.

## Parity Risks

1. **Ignore-rule parity**: index discovery currently uses workspace-local ignore rules only, while
   `code_search` search discovery uses the broader ignore stack. This can change corpus membership.
2. **Encoding parity**: the index excludes invalid UTF-8, but the current search path can still
   match such files after lossy decoding.
3. **Scope/filter parity**: `path`, `include`, and `exclude` are request-time filters, not persisted
   corpus rules.
4. **Freshness window**: live files can change after index load and before verification.

Because of these risks, the safe rule is simple: **if the index cannot prove completeness for the
current request, it must downgrade to `Partial` or `Unavailable`, and `code_search` must complete
correctness with scan fallback or full fallback.**
