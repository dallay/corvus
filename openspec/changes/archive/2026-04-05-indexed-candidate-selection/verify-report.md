# Verification Report

**Change**: indexed-candidate-selection
**Version**: N/A

---

### Completeness

| Metric           | Value |
|------------------|-------|
| Tasks total      | 11    |
| Tasks complete   | 11    |
| Tasks incomplete | 0     |

All tasks in `openspec/changes/indexed-candidate-selection/tasks.md` are checked complete.

---

### Build & Tests Execution

**Format**: ✅ Passed

```text
Command: cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check
Result: passed
```

**Build / Type Check**: ✅ Passed

```text
Command: cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings
Result: passed
```

**Tests**: ✅ Targeted Rust validation passed

```text
Command: cargo test --manifest-path "clients/agent-runtime/Cargo.toml" --lib search::tests::
Result: 69 passed; 0 failed; 0 ignored; 0 measured; 3327 filtered out

Command: cargo test --manifest-path "clients/agent-runtime/Cargo.toml" --lib tools::code_search::tests::
Result: 47 passed; 0 failed; 0 ignored; 0 measured; 3349 filtered out

Behavioral evidence covered by passing tests includes:
- search::tests::sqlite_candidate_paths_intersect_trigrams_and_sort_lexically
- search::tests::candidate_planner_returns_complete_for_safe_literal_query
- search::tests::candidate_planner_marks_stale_index_as_partial
- search::tests::candidate_planner_marks_regex_and_short_patterns_unavailable
- tools::code_search::tests::code_search_eliminates_index_false_positives_with_live_verification
- tools::code_search::tests::code_search_falls_back_when_index_is_unavailable
- tools::code_search::tests::code_search_falls_back_for_unsupported_regex_query_shape
- tools::code_search::tests::code_search_applies_max_results_to_verified_matches_after_filtering_candidates
- tools::code_search::tests::code_search_returns_deterministic_verified_order_across_repeated_runs
- tools::code_search::tests::code_search_structured_matches_include_verified_offsets_and_preview
- tools::code_search::tests::code_search_regex_search_finds_matches
```

**Coverage**: ➖ Not configured

---

### Spec Compliance Matrix

| Requirement                                                                       | Scenario                                                               | Test                                                                                                                                                                                     | Result      |
|-----------------------------------------------------------------------------------|------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------|
| REQ-WIDX-009 Indexed Candidate Files Are Advisory and Deterministic               | Index returns advisory candidate files only                            | `src/search/tests.rs > candidate_planner_returns_complete_for_safe_literal_query`; `src/tools/code_search.rs > code_search_eliminates_index_false_positives_with_live_verification`      | ✅ COMPLIANT |
| REQ-WIDX-009 Indexed Candidate Files Are Advisory and Deterministic               | Candidate files are ordered deterministically                          | `src/search/tests.rs > sqlite_candidate_paths_intersect_trigrams_and_sort_lexically`; `src/tools/code_search.rs > code_search_returns_deterministic_verified_order_across_repeated_runs` | ✅ COMPLIANT |
| REQ-WIDX-010 Candidate Extraction Must Signal When It Cannot Safely Narrow Search | Query without trustworthy trigram reduction is not treated as complete | `src/search/tests.rs > candidate_planner_marks_regex_and_short_patterns_unavailable`; `src/tools/code_search.rs > code_search_falls_back_for_unsupported_regex_query_shape`              | ✅ COMPLIANT |
| REQ-RESULT-003 Match Object Schema                                                | Match object includes verified range and preview fields                | `src/tools/code_search.rs > code_search_structured_matches_include_verified_offsets_and_preview`                                                                                         | ✅ COMPLIANT |
| REQ-RESULT-007 Truncation Warning                                                 | Verified match cap applies after candidate verification                | `src/tools/code_search.rs > code_search_applies_max_results_to_verified_matches_after_filtering_candidates`                                                                              | ✅ COMPLIANT |
| REQ-RESULT-010 Deterministic Verified Match Ordering                              | Verified matches are stable across repeated runs                       | `src/tools/code_search.rs > code_search_returns_deterministic_verified_order_across_repeated_runs`                                                                                       | ✅ COMPLIANT |
| REQ-REGEX-007 Live Verification Is Authoritative                                  | Candidate false positive is eliminated by live verification            | `src/tools/code_search.rs > code_search_eliminates_index_false_positives_with_live_verification`                                                                                         | ✅ COMPLIANT |
| REQ-REGEX-007 Live Verification Is Authoritative                                  | Regex verification remains authoritative after candidate filtering     | `src/tools/code_search.rs > code_search_regex_search_finds_matches`; `src/tools/code_search.rs > code_search_falls_back_for_unsupported_regex_query_shape`                               | ⚠️ PARTIAL  |
| REQ-SAFE-012 Safe Fallback Preserves Search Correctness                           | Request without safe indexed reduction falls back to discovery scan    | `src/search/tests.rs > candidate_planner_marks_regex_and_short_patterns_unavailable`; `src/tools/code_search.rs > code_search_falls_back_for_unsupported_regex_query_shape`              | ✅ COMPLIANT |
| REQ-SAFE-012 Safe Fallback Preserves Search Correctness                           | Index unavailability does not reduce correctness                       | `src/tools/code_search.rs > code_search_falls_back_when_index_is_unavailable`                                                                                                            | ✅ COMPLIANT |

**Compliance summary**: 9/10 scenarios compliant, 1/10 partial, 0/10 failing, 0/10 fully untested

---

### Correctness (Static — Structural Evidence)

| Requirement    | Status        | Notes                                                                                                                                                                                                                                                                                              |
|----------------|---------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| REQ-WIDX-009   | ✅ Implemented | `WorkspaceTrigramIndex::plan_candidates` returns ordered workspace-relative paths in `clients/agent-runtime/src/search/index.rs:166-264`; SQLite candidate reads enforce `ORDER BY f.relative_path ASC` in `clients/agent-runtime/src/search/sqlite.rs:177-197`.                                   |
| REQ-WIDX-010   | ✅ Implemented | Unsupported regex, case-insensitive, whole-word, and short queries are downgraded by `extract_required_trigrams` in `clients/agent-runtime/src/search/index.rs:537-561`.                                                                                                                           |
| REQ-RESULT-003 | ✅ Implemented | `SearchMatch` adds `line_end`, `column_end`, `byte_start`, `byte_end`, and `preview` in `clients/agent-runtime/src/tools/code_search.rs:77-90`, and `verify_file_matches` populates them in `clients/agent-runtime/src/tools/code_search.rs:687-701`.                                              |
| REQ-RESULT-007 | ✅ Implemented | `verify_inputs` enforces `max_results` after live verification and emits truncation warnings in `clients/agent-runtime/src/tools/code_search.rs:601-643`.                                                                                                                                          |
| REQ-RESULT-010 | ✅ Implemented | Candidate order is lexical from SQLite and in-file matches are sorted by `(byte_start, byte_end)` in `clients/agent-runtime/src/tools/code_search.rs:705-709`.                                                                                                                                     |
| REQ-REGEX-007  | ⚠️ Partial    | Live verification is authoritative in `verify_file_matches`, and unsupported regex shapes now have explicit fallback coverage, but indexed regex candidate filtering is still not implemented because regex planning returns `Unavailable` in `clients/agent-runtime/src/search/index.rs:537-545`. |
| REQ-SAFE-012   | ✅ Implemented | `build_verification_inputs` falls back to discovery for `Partial` and `Unavailable` plans and on indexed-read failure in `clients/agent-runtime/src/tools/code_search.rs:446-475`.                                                                                                                 |

---

### Coherence (Design)

| Decision                                                                | Followed?   | Notes                                                                                                                                                                                                                                           |
|-------------------------------------------------------------------------|-------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Keep candidate planning in `search/*` and verification in `code_search` | ✅ Yes       | Planner lives in `search/index.rs`; verification and result shaping remain in `tools/code_search.rs`.                                                                                                                                           |
| Use a conservative index-eligibility gate                               | ✅ Yes       | Regex, case-insensitive, whole-word, and short patterns downgrade to `Unavailable`.                                                                                                                                                             |
| Model candidate coverage explicitly                                     | ✅ Yes       | `CandidateCoverage`, `CandidateRequest`, and `CandidatePlan` are present and used.                                                                                                                                                              |
| Make deterministic ordering contractual in SQL and verification         | ✅ Yes       | SQLite orders by relative path and verified matches are sorted by byte position.                                                                                                                                                                |
| Extend match objects compatibly                                         | ✅ Yes       | Existing fields are preserved and the new metadata fields are additive.                                                                                                                                                                         |
| Partial coverage should use stable union+dedupe before verification     | ⚠️ Deviated | `build_verification_inputs` currently downgrades `Partial` directly to full discovery fallback instead of combining indexed and fallback inputs. This preserves correctness but not the optimization path described in the design and task 3.3. |

---

### Issues Found

**CRITICAL** (must fix before archive):

- None

**WARNING** (should fix):

- No runtime evidence proves the exact scenario where a regex request is first narrowed by indexed
  candidates and then rejected by live regex verification; current implementation conservatively
  avoids indexed regex planning.
- Partial candidate plans still use full discovery fallback instead of the design/task-level stable
  union+dedupe optimization path.

**SUGGESTION** (nice to have):

- Add a focused integration test if indexed regex prefiltering is introduced later, proving live
  regex verification rejects an indexed false positive.
- Implement and test the planned stable union+dedupe behavior for `CandidateCoverage::Partial` if
  the optimization remains desired.

---

### Verdict

PASS WITH WARNINGS

The follow-up fixes cleared the previous formatting and clippy blockers, and targeted Rust
verification now passes with explicit unsupported-regex fallback coverage; remaining gaps are
limited to a conservative design deviation and one partially evidenced regex-index scenario.
