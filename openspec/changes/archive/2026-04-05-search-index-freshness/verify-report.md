## Verification Report

**Change**: 2026-04-05-search-index-freshness
**Version**: N/A

---

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 10 |
| Tasks complete | 10 |
| Tasks incomplete | 0 |

All listed tasks in `openspec/changes/2026-04-05-search-index-freshness/tasks.md` are marked complete.

---

### Build & Tests Execution

**Format**: ✅ Passed
```text
Command: cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check
Result: success
```

**Clippy**: ✅ Passed
```text
Command: cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings
Result: success
```

**Focused tests**: ✅ Passed
```text
Command: cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib search::
Result: 74 passed, 0 failed, 0 ignored

Command: cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib tools::file_write
Result: 16 passed, 0 failed, 0 ignored

Command: cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib tools::code_search
Result: 48 passed, 0 failed, 0 ignored
```

**Coverage**: ➖ Not configured in `openspec/config.yaml`

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| REQ-WIDX-003 Compatibility and Freshness Metadata | Freshness metadata records the v1 trust signals | `search/tests.rs > persisted_file_paths_are_relative_only`; `search/tests.rs > candidate_planner_uses_hash_guard_when_size_and_mtime_match` | ⚠️ PARTIAL |
| REQ-WIDX-003 Compatibility and Freshness Metadata | Optional git state is never treated as the sole freshness source | `search/tests.rs > candidate_planner_marks_stale_index_as_partial`; `candidate_planner_marks_changed_indexed_content_as_partial`; `candidate_planner_marks_deleted_indexed_path_as_partial`; `candidate_planner_marks_renamed_path_drift_as_partial` | ✅ COMPLIANT |
| REQ-WIDX-005 Compatible Load and Refresh Behavior | Existing compatible index is loaded and unchanged files are reused | `search/tests.rs > load_returns_existing_compatible_index` | ✅ COMPLIANT |
| REQ-WIDX-005 Compatible Load and Refresh Behavior | Changed file is refreshed in place | `search/tests.rs > compatible_index_refreshes_changed_and_deleted_files` | ✅ COMPLIANT |
| REQ-WIDX-005 Compatible Load and Refresh Behavior | Successful agent write is reflected without manual rebuild | `tools/file_write.rs > file_write_keeps_indexed_path_searchable_without_manual_rebuild` | ✅ COMPLIANT |
| REQ-WIDX-006 Deleted File Removal | Deleted file is removed during refresh | `search/tests.rs > compatible_index_refreshes_changed_and_deleted_files` | ✅ COMPLIANT |
| REQ-WIDX-006 Deleted File Removal | Renamed file is handled as delete plus add | `search/tests.rs > candidate_planner_marks_renamed_path_drift_as_partial` | ⚠️ PARTIAL |
| REQ-WIDX-008 Verification Coverage | Automated tests prove lifecycle behavior | `search/tests.rs` lifecycle suite including `build_persists_metadata_and_trigram_rows`, `load_returns_existing_compatible_index`, `compatible_index_refreshes_changed_and_deleted_files`, `incompatible_format_version_forces_rebuild`, `incomplete_build_state_forces_rebuild` | ✅ COMPLIANT |
| REQ-WIDX-008 Verification Coverage | Automated tests prove deterministic exclusions | `search/tests.rs > discovery_excludes_invalid_utf8_and_binary_content`; `discovery_excludes_hidden_ignored_oversized_and_index_artifacts`; `discovery_excludes_symlink_escape` | ✅ COMPLIANT |
| REQ-WIDX-008 Verification Coverage | Regression tests prove stale-state safety | `search/tests.rs > candidate_planner_marks_changed_indexed_content_as_partial`; `candidate_planner_marks_deleted_indexed_path_as_partial`; `candidate_planner_marks_renamed_path_drift_as_partial`; `candidate_planner_uses_hash_guard_when_size_and_mtime_match`; `tools/file_write.rs > file_write_keeps_indexed_path_searchable_without_manual_rebuild`; `tools/code_search.rs > code_search_falls_back_when_indexed_entry_is_hash_stale` | ✅ COMPLIANT |
| REQ-WIDX-011 Indexed Candidate Freshness Guard | Changed file prevents silently trusting complete indexed coverage | `search/tests.rs > candidate_planner_marks_changed_indexed_content_as_partial`; `tools/code_search.rs > code_search_falls_back_when_indexed_entry_is_hash_stale` | ✅ COMPLIANT |
| REQ-WIDX-011 Indexed Candidate Freshness Guard | Missing or extra indexed path prevents silently trusting complete indexed coverage | `search/tests.rs > candidate_planner_marks_stale_index_as_partial`; `candidate_planner_marks_deleted_indexed_path_as_partial`; `candidate_planner_marks_renamed_path_drift_as_partial` | ✅ COMPLIANT |
| REQ-WIDX-012 V1 Freshness Documentation | V1 freshness model is documented with guarantees and limits | (documentation inspected in change artifacts; no runtime test) | ⚠️ PARTIAL |

**Compliance summary**: 10/13 scenarios compliant, 3 partial, 0 untested.

---

### Correctness (Static — Structural Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| REQ-WIDX-003 | ✅ Implemented | SQLite schema stores `relative_path`, `size_bytes`, `modified_unix_ms`, and `content_sha256` (`clients/agent-runtime/src/search/sqlite.rs:63-80`), and candidate freshness requires hash + hint parity (`clients/agent-runtime/src/search/index.rs:599-625`). |
| REQ-WIDX-005 | ✅ Implemented | `sync_written_path()` performs best-effort single-path update/removal (`clients/agent-runtime/src/search/index.rs:429-497`), and `file_write` triggers it only after successful writes (`clients/agent-runtime/src/tools/file_write.rs:161-214`). |
| REQ-WIDX-006 | ✅ Implemented | Refresh removes missing paths with `delete_file_tx()` and reindexes changed paths in one transaction (`clients/agent-runtime/src/search/index.rs:370-426`). Rename drift is treated safely as stale scope mismatch, and explicit refresh logic would remove old + add new based on path parity. |
| REQ-WIDX-008 | ✅ Implemented | Regression coverage exists in `clients/agent-runtime/src/search/tests.rs`, `clients/agent-runtime/src/tools/file_write.rs`, and `clients/agent-runtime/src/tools/code_search.rs`. |
| REQ-WIDX-011 | ✅ Implemented | `plan_candidates()` discovers current searchable files, computes current hashes, loads scoped persisted rows, and downgrades to `Partial` unless exact scoped parity holds (`clients/agent-runtime/src/search/index.rs:228-264`). |
| REQ-WIDX-012 | ✅ Implemented | Proposal, design, and delta spec document identity, trust signals, hash guard, write-through behavior, and v1 non-goals/limits. |

---

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Keep freshness logic inside `WorkspaceTrigramIndex` | ✅ Yes | Tools call `sync_written_path()` rather than direct SQLite mutation. |
| Treat `content_sha256` as trust boundary for `Complete` | ✅ Yes | `persisted_record_matches_current()` requires matching hash and non-zero mtimes. |
| Rename handling is delete + add semantics | ✅ Yes | No explicit rename tracking exists; stale scope mismatch downgrades coverage and refresh logic is path-based. |
| Post-write sync is best-effort and non-blocking | ✅ Yes | `file_write` logs sync failures and still returns success after the write. |
| Conservative downgrade for uncertain scope | ✅ Yes | `plan_candidates()` returns `Partial` when scoped rows do not match exactly. |
| File changes table | ⚠️ Minor deviation | `clients/agent-runtime/src/tools/code_search.rs` behavior change is minimal; most of the change in that area is regression coverage rather than notable production logic drift. |

---

### Issues Found

**CRITICAL** (must fix before archive):
None.

**WARNING** (should fix):
- Rename safety is runtime-tested via stale-plan downgrade/fallback, but there is still no dedicated refresh-cycle test that asserts SQLite removes the old path and inserts the new path after a rename.
- The metadata-trust scenario is only partially runtime-proven; tests demonstrate path identity and hash-guard behavior, but do not directly assert persisted `size_bytes` and `modified_unix_ms` values for a row.

**SUGGESTION** (nice to have):
- Add a focused refresh-after-rename persistence assertion and a direct persisted-metadata assertion test to close the remaining partial scenarios.

---

### Verdict
PASS WITH WARNINGS

The implementation meets the requested acceptance behavior and all executed verification commands now pass, but a couple of spec scenarios still have only partial runtime proof rather than dedicated end-to-end assertions.
