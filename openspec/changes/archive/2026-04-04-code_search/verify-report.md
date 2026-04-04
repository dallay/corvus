## Verification Report

**Change**: code_search
**Version**: N/A
**Date**: 2026-04-04

---

### Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 30 |
| Tasks checked off in tasks.md | 28 |
| Tasks actually verified complete | 28 |
| Tasks incomplete / not passing | 2 |

**Incomplete tasks**
- `7.1 Run cargo fmt --all -- --check in clients/agent-runtime` — still not complete; blocked by unrelated repository formatting debt outside this change.
- `7.2 Run cargo clippy --all-targets -- -D warnings in clients/agent-runtime` — still not complete; blocked by unrelated repository lint debt outside this change.

**Accurate task updates after second apply pass**
- `7.3 Run cargo test full suite in clients/agent-runtime` — now accurate; verified green.
- `7.4 Verify code_search appears in tool name list via existing all_tools test pattern` — still accurate.

---

### Build & Tests Execution

**Targeted tests**: `cargo test code_search` ✅ Passed
- 41 passed / 0 failed / 0 ignored / 0 measured

**Full runtime tests**: `cargo test` ✅ Passed
- Exit code 0
- Full suite completed successfully, including unit tests, integration tests, and doc tests
- Previously failing change-local regressions are resolved:
  1. bootstrap/profile classification now includes `code_search`
  2. default-tools count test now expects 4 tools

**Formatting**: `cargo fmt --all -- --check` ❌ Failed
- Blocked by unrelated pre-existing formatting debt outside `code_search`:
  - `src/channels/audio_media.rs`
  - `src/channels/cli.rs`
  - `src/gateway/mod.rs`

**Clippy**: `cargo clippy --all-targets -- -D warnings` ❌ Failed
- Blocked by unrelated pre-existing issues outside `code_search`:
  - `src/gateway/mod.rs:6896` unused import
  - `src/channels/cli.rs:884` `clippy::single-char-pattern`

**Coverage**: ➖ Not configured in `openspec/config.yaml`

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| REQ-API-001 | Tool is discoverable by name | `src/tools/mod.rs > default_tools_names` | ✅ COMPLIANT |
| REQ-API-002 | Valid literal search returns matches | `src/tools/code_search.rs > code_search_literal_search_finds_matches` | ✅ COMPLIANT |
| REQ-API-002 | Valid regex search returns matches | `src/tools/code_search.rs > code_search_regex_search_finds_matches` | ✅ COMPLIANT |
| REQ-API-002 | Empty pattern returns error | `src/tools/code_search.rs > code_search_empty_pattern_returns_error` | ✅ COMPLIANT |
| REQ-API-002 | Pattern exceeding 1000 characters returns error | `src/tools/code_search.rs > code_search_pattern_too_long_returns_error` | ✅ COMPLIANT |
| REQ-API-002 | Invalid regex returns compilation error | `src/tools/code_search.rs > code_search_invalid_regex_returns_error` | ✅ COMPLIANT |
| REQ-API-003 | Scoped search respects path parameter | `src/tools/code_search.rs > code_search_scopes_by_subdirectory` | ✅ COMPLIANT |
| REQ-API-003 | Nonexistent path returns error | `src/tools/code_search.rs > code_search_nonexistent_path_returns_error` | ✅ COMPLIANT |
| REQ-API-003 | Path pointing to a file returns error | `src/tools/code_search.rs > code_search_path_must_be_directory` | ✅ COMPLIANT |
| REQ-API-004 | Include filter restricts file types | `src/tools/code_search.rs > code_search_include_filters_files` | ✅ COMPLIANT |
| REQ-API-005 | Exclude filter removes matching files | `src/tools/code_search.rs > code_search_exclude_filters_files` | ✅ COMPLIANT |
| REQ-API-006 | Default literal mode does not interpret regex metacharacters | `src/tools/code_search.rs > code_search_escapes_literal_patterns` | ✅ COMPLIANT |
| REQ-API-007 | Case insensitive search matches mixed case | `src/tools/code_search.rs > code_search_case_insensitive_matches_mixed_case` | ✅ COMPLIANT |
| REQ-API-008 | Results truncated at max_results | `src/tools/code_search.rs > code_search_max_results_truncates` | ✅ COMPLIANT |
| REQ-API-009 | Context lines included in results | `src/tools/code_search.rs > code_search_context_lines_and_group_separator` | ✅ COMPLIANT |
| REQ-API-010 | Whole word search does not match substrings | `src/tools/code_search.rs > code_search_whole_word_skips_substrings` | ✅ COMPLIANT |
| REQ-REGEX-001 | Unicode-aware character classes | `src/tools/code_search.rs > code_search_regex_supports_unicode_character_classes` | ✅ COMPLIANT |
| REQ-REGEX-002 | Lookahead pattern returns compilation error | `src/tools/code_search.rs > code_search_rejects_unsupported_lookahead_regex` | ✅ COMPLIANT |
| REQ-REGEX-003 | Literal search with regex metacharacters matches literally | `src/tools/code_search.rs > code_search_escapes_literal_patterns` | ✅ COMPLIANT |
| REQ-REGEX-004 | Case insensitive search matches mixed case | `src/tools/code_search.rs > code_search_case_insensitive_matches_mixed_case` | ✅ COMPLIANT |
| REQ-REGEX-004 | Case sensitive search is the default | `src/tools/code_search.rs > code_search_case_sensitive_is_default` | ✅ COMPLIANT |
| REQ-REGEX-005 | Whole word search does not match substrings | `src/tools/code_search.rs > code_search_whole_word_skips_substrings` | ✅ COMPLIANT |
| REQ-REGEX-005 | Whole word with regex mode | `src/tools/code_search.rs > code_search_whole_word_regex_mode_skips_substrings` | ✅ COMPLIANT |
| REQ-REGEX-006 | Combined literal + case insensitive + whole word | `src/tools/code_search.rs > code_search_preserves_combined_pattern_order` + `code_search_case_insensitive_matches_mixed_case` + `code_search_whole_word_skips_substrings` | ✅ COMPLIANT |
| REQ-REGEX-006 | Regex mode bypasses escaping | `src/tools/code_search.rs > code_search_regex_search_finds_matches` | ✅ COMPLIANT |
| REQ-SAFE-001 | Path traversal attempt is rejected | `src/tools/code_search.rs > code_search_blocks_path_traversal` | ✅ COMPLIANT |
| REQ-SAFE-001 | Absolute path is rejected | `src/tools/code_search.rs > code_search_blocks_absolute_paths` | ✅ COMPLIANT |
| REQ-SAFE-002 | Security chain is applied to search root | `src/tools/code_search.rs > code_search_scopes_by_subdirectory` plus static evidence in `execute()` | ✅ COMPLIANT |
| REQ-SAFE-003 | Symlink to outside workspace is skipped | `src/tools/code_search.rs > code_search_skips_symlink_escape` | ✅ COMPLIANT |
| REQ-SAFE-004 | Binary file is skipped | `src/tools/code_search.rs > code_search_skips_binary_files` | ✅ COMPLIANT |
| REQ-SAFE-005 | Rate limited invocation is rejected | `src/tools/code_search.rs > code_search_rate_limited_invocation_is_rejected` | ✅ COMPLIANT |
| REQ-SAFE-005 | Single invocation counts as one action | `src/tools/code_search.rs > code_search_counts_one_action_per_invocation` | ✅ COMPLIANT |
| REQ-SAFE-006 | Search works in ReadOnly mode | `src/tools/code_search.rs > code_search_allows_readonly_mode` | ✅ COMPLIANT |
| REQ-SAFE-007 | Search exceeding 10K files returns truncation warning | `src/tools/code_search.rs > code_search_file_scan_cap_sets_truncation` | ✅ COMPLIANT |
| REQ-SAFE-007 | File exceeding 10MB is skipped | `src/tools/code_search.rs > code_search_skips_large_files` | ✅ COMPLIANT |
| REQ-SAFE-007 | Per-file match cap at 50 | `src/tools/code_search.rs > code_search_caps_matches_per_file_at_fifty` | ✅ COMPLIANT |
| REQ-SAFE-007 | Timeout returns partial results with warning | `src/tools/code_search.rs > code_search_timeout_sets_truncation_warning` | ✅ COMPLIANT |
| REQ-SAFE-008 | Gitignored files are excluded | `src/tools/code_search.rs > code_search_respects_gitignore` | ✅ COMPLIANT |
| REQ-SAFE-008 | Hidden directories are excluded by default | `src/tools/code_search.rs > code_search_skips_hidden_directories_by_default` | ✅ COMPLIANT |
| REQ-SAFE-009 | Unreadable file is skipped gracefully | `src/tools/code_search.rs > code_search_skips_unreadable_files` | ✅ COMPLIANT |
| REQ-RESULT-001 | Output field contains grep-like format | `src/tools/code_search.rs > code_search_output_and_structured_format` | ✅ COMPLIANT |
| REQ-RESULT-002 | Structured field has correct top-level shape | `src/tools/code_search.rs > code_search_output_and_structured_format` | ✅ COMPLIANT |
| REQ-RESULT-003 | Match object contains all required fields | `src/tools/code_search.rs > code_search_output_and_structured_format` + `code_search_context_lines_and_group_separator` | ✅ COMPLIANT |
| REQ-RESULT-004 | Stats reflect actual search metrics | `src/tools/code_search.rs > code_search_zero_matches_returns_success` + `code_search_max_results_truncates` + `code_search_output_and_structured_format` | ✅ COMPLIANT |
| REQ-RESULT-005 | Long line is truncated at 500 characters | `src/tools/code_search.rs > code_search_truncates_long_content` | ✅ COMPLIANT |
| REQ-RESULT-006 | Summary line is appended to output | `src/tools/code_search.rs > code_search_zero_matches_returns_success` + `code_search_output_and_structured_format` | ✅ COMPLIANT |
| REQ-RESULT-007 | Truncated results include warning and truncated flag | `src/tools/code_search.rs > code_search_max_results_truncates` | ✅ COMPLIANT |
| REQ-RESULT-008 | Search with context_lines returns before/after context | `src/tools/code_search.rs > code_search_context_lines_and_group_separator` | ✅ COMPLIANT |
| REQ-RESULT-009 | Zero matches returns success with empty matches array | `src/tools/code_search.rs > code_search_zero_matches_returns_success` | ✅ COMPLIANT |

**Compliance summary**: 47/47 scenarios compliant

---

### Correctness (Static — Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| API/schema contract | ✅ Implemented | Name, description, and 9-parameter schema match the documented contract in `src/tools/code_search.rs`. |
| Regex semantics | ✅ Implemented | Construction order is correct: escape → case flag → whole-word wrapping → compile. |
| Safety model | ✅ Implemented | Reuses `SecurityPolicy` path validation and resolved-path checks like `file_read`; rate limiting and ReadOnly support behave correctly. |
| Structured result format | ✅ Implemented | `matches[]` and `stats{}` are returned in `structured`, with grep-like `output` plus warnings and summary line. |
| Integration coherence | ✅ Implemented | `code_search` is now classified in bootstrap code profile allowlists and default-tools expectations are updated. |

---

### Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| New tool file `src/tools/code_search.rs` | ✅ Yes | Implemented with logic and tests. |
| Register in `src/tools/mod.rs` | ✅ Yes | Tool is registered in default tools. |
| Add `ignore = "0.4"` dependency | ✅ Yes | Present in `clients/agent-runtime/Cargo.toml`. |
| Canonical design doc under `docs/design/code-search-tool.md` | ✅ Yes | File exists. |
| Reuse `SecurityPolicy` / `file_read` path model | ✅ Yes | Same raw-path + canonicalization + resolved-path guard chain. |
| Single action per search | ✅ Yes | Verified by test. |
| Hidden directories excluded by default | ✅ Yes | Regression fixed; runtime test proves hidden dir exclusion. |
| Integration stays coherent with existing registry/profile tests | ✅ Yes | Previous regressions are fixed and full suite is green. |

---

### Issues Found

**CRITICAL** (must fix before archive):
- None

**WARNING** (should fix):
1. `cargo fmt --all -- --check` still fails, but only because of unrelated repository formatting debt in `src/channels/audio_media.rs`, `src/channels/cli.rs`, and `src/gateway/mod.rs`.
2. `cargo clippy --all-targets -- -D warnings` still fails, but only because of unrelated repository lint debt in `src/gateway/mod.rs:6896` and `src/channels/cli.rs:884`.

**SUGGESTION** (nice to have):
1. If desired, add a dedicated assertion for exact summary-line placement (`last line`) and more explicit per-field stats assertions, though current behavior is already adequately covered.

---

### Verdict
PASS WITH WARNINGS

Change-local implementation is now correct: prior regressions are fixed, added tests cover the previously identified spec gaps, and `cargo test` passes. The only remaining verification issues are unrelated repo-wide `fmt`/`clippy` debt outside the `code_search` change.
