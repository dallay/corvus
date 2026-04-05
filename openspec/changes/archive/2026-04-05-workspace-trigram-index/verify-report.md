## Verification Report

**Change**: workspace-trigram-index
**Version**: N/A

---

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 16 |
| Tasks complete | 16 |
| Tasks incomplete | 0 |

All tasks in `tasks.md` are now marked complete. Task `4.4` correctly records the local validation status:

- `cargo test` → passed
- `cargo fmt --all -- --check` → passed
- `cargo clippy --all-targets -- -D warnings` → still blocked by unrelated pre-existing warning in `src/channels/telegram.rs:3479`

---

### Build & Tests Execution

**Build / format / lint evidence**

- `cargo fmt --all -- --check` → ✅ Passed
- `cargo clippy --all-targets -- -D warnings` → ⚠️ Failed due to pre-existing unrelated warning in `src/channels/telegram.rs:3479` (`clippy::unreadable_literal` on `-100123456`)
- `cargo test` → ✅ Passed

```text
cargo clippy --all-targets -- -D warnings
error: long literal lacking separators
--> src/channels/telegram.rs:3479:24
3479 |                 "id": -100123456,
     |                        ^^^^^^^^^ help: consider: `100_123_456`
```

**Tests**: ✅ focused workspace-index suite passed and full runtime test suite passed

```text
cargo test search::tests:: -- --nocapture
59 passed; 0 failed; 0 ignored

cargo test
3384 passed; 0 failed; 0 ignored
```

**Coverage**: ➖ Not configured

Notes:

- `openspec/config.yaml` lists repo-wide web verification commands too (`make web-test-all`, `pnpm check`), but this change is Rust-runtime scoped and verification evidence was taken from the relevant runtime commands.
- Behavioral DB evidence came from passing tests that query SQLite directly (`SELECT ... FROM metadata/files/trigram_postings`) in `clients/agent-runtime/src/search/tests.rs`.

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| REQ-SAFE-010 | Discovery indexes only files inside the active workspace | `search/tests.rs > searchable_discovery_respects_scope`; `search/tests.rs > discovery_excludes_symlink_escape` | ⚠️ PARTIAL |
| REQ-SAFE-010 | Symlink escape is excluded from corpus discovery | `search/tests.rs > discovery_excludes_symlink_escape` | ✅ COMPLIANT |
| REQ-SAFE-011 | Invalid UTF-8 file is excluded from the corpus | `search/tests.rs > discovery_excludes_invalid_utf8_and_binary_content` | ✅ COMPLIANT |
| REQ-SAFE-011 | Binary file is excluded from the corpus | `search/tests.rs > discovery_excludes_invalid_utf8_and_binary_content` | ✅ COMPLIANT |
| REQ-SAFE-011 | Index database files are excluded from the corpus | `search/tests.rs > discovery_excludes_hidden_ignored_oversized_and_index_artifacts` | ✅ COMPLIANT |
| REQ-WIDX-001 | Initial build persists required logical record types | `search/tests.rs > build_persists_metadata_and_trigram_rows` | ✅ COMPLIANT |
| REQ-WIDX-002 | Persisted file entries are stored as relative paths only | `search/tests.rs > persisted_file_paths_are_relative_only` | ✅ COMPLIANT |
| REQ-WIDX-003 | Compatible metadata allows existing index load | `search/tests.rs > load_returns_existing_compatible_index` | ✅ COMPLIANT |
| REQ-WIDX-004 | Missing index triggers first build | `search/tests.rs > build_persists_metadata_and_trigram_rows` | ⚠️ PARTIAL |
| REQ-WIDX-005 | Existing compatible index is loaded and unchanged files are reused | `search/tests.rs > repeated_refresh_keeps_deterministic_membership` | ✅ COMPLIANT |
| REQ-WIDX-005 | Changed file is refreshed in place | `search/tests.rs > compatible_index_refreshes_changed_and_deleted_files` | ✅ COMPLIANT |
| REQ-WIDX-006 | Deleted file is removed during refresh | `search/tests.rs > compatible_index_refreshes_changed_and_deleted_files` | ✅ COMPLIANT |
| REQ-WIDX-007 | Version mismatch forces rebuild | `search/tests.rs > incompatible_format_version_forces_rebuild` | ✅ COMPLIANT |
| REQ-WIDX-007 | Workspace mismatch forces rebuild | `search/tests.rs > foreign_workspace_index_forces_rebuild` | ✅ COMPLIANT |
| REQ-WIDX-007 | Incomplete prior build forces rebuild | `search/tests.rs > incomplete_build_state_forces_rebuild` | ✅ COMPLIANT |
| REQ-WIDX-008 | Automated tests prove lifecycle behavior | `cargo test search::tests:: -- --nocapture` + named lifecycle tests above | ✅ COMPLIANT |
| REQ-WIDX-008 | Automated tests prove deterministic exclusions | `search/tests.rs > repeated_refresh_keeps_deterministic_membership`; exclusion tests above | ✅ COMPLIANT |

**Compliance summary**: 14/17 scenarios compliant, 3/17 partial, 0 failing, 0 untested.

---

### Correctness (Static — Structural Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| Shared workspace corpus discovery | ✅ Implemented | Discovery root is canonicalized and policy-checked before walking, resolved entries are revalidated, normalized to workspace-relative paths, and sorted deterministically in `search/discovery.rs:42-245`. |
| Deterministic non-text and self-index exclusion | ✅ Implemented | Binary detection, UTF-8-only admission, unreadable skipping, and self-index exclusion live in `search/discovery.rs:100-105, 202-245`. |
| SQLite logical contract | ✅ Implemented | `metadata`, `files`, and `trigram_postings` schema and helpers exist in `search/sqlite.rs:9-212`. |
| Workspace-relative file identity only | ✅ Implemented | Only `relative_path` is persisted in file rows; `workspace_fingerprint` is hashed metadata, not raw path (`search/index.rs:343-357`, `search/sqlite.rs:29-35, 163-203`). |
| Build/load/refresh/rebuild lifecycle | ✅ Implemented | Full build uses temp DB + rename (`search/index.rs:71-106, 193-235, 363-389`); compatible indexes refresh changed/deleted rows in place (`search/index.rs:108-191`); incompatible or incomplete indexes rebuild (`search/index.rs:237-289`). |
| Discovery alignment with `code_search` | ✅ Implemented | `code_search` now consumes shared discovery helpers in `tools/code_search.rs:2-4, 387-396`. |

---

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Add reusable `search` module | ✅ Yes | `src/search/{mod,discovery,trigram,sqlite,index}.rs` added and exported from `src/lib.rs`. |
| Extract shared discovery from `code_search` | ✅ Yes | `code_search` delegates discovery to `search::discovery`. |
| Dedicated SQLite DB at `workspace/state/code-search/index.db` | ✅ Yes | `WorkspaceTrigramIndex::for_workspace` pins that path. |
| Store file identity as workspace-relative only | ✅ Yes | Verified structurally and behaviorally. |
| Strict UTF-8 admission | ✅ Yes | `discover_indexable_files` filters to valid UTF-8 before indexing. |
| Incrementally refresh compatible indexes; rebuild incompatible/incomplete indexes | ✅ Yes | Implementation matches the resolved lifecycle contract in `search/index.rs:108-191, 237-289`. |
| Design text consistency | ✅ Yes | `design.md` now consistently describes the resolved lifecycle contract: load unchanged compatible indexes, refresh changed/deleted rows for compatible stale indexes, and fully rebuild only when missing/incompatible/incomplete. |

---

### Issues Found

**CRITICAL** (must fix before archive):
None

**WARNING** (should fix):

- `cargo clippy --all-targets -- -D warnings` still fails on pre-existing unrelated code in `clients/agent-runtime/src/channels/telegram.rs:3479`.
- Scenario coverage is slightly indirect for “active workspace only” and “missing index triggers first build”; runtime behavior looks correct, but the tests are broader than the scenario wording.

**SUGGESTION** (nice to have):

- Add a small explicit test that creates a sibling non-workspace directory with regular files (not symlinks) and proves `discover_indexable_files` never admits them.
- Add a focused `refresh_or_rebuild` missing-DB test that explicitly asserts the initial request path goes through first-build behavior.

---

### Verdict
PASS WITH WARNINGS

Change-local implementation matches the approved workspace-index contract and the local artifact issues are resolved; the only remaining blocking signal is unrelated repo-wide clippy debt outside this change.
