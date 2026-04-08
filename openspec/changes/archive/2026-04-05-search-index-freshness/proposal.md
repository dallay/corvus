# Proposal: Search Index Freshness v1

## Intent

Issue #358 needs the workspace search index to stop behaving like a static snapshot after writes.
This change delivers the minimal safe v1 so agent-authored file writes become searchable
immediately,
stale index rows are not silently trusted, and local changed, deleted, or renamed files are handled
without claiming complete indexed coverage when the persisted corpus is out of date.

## Scope

### In Scope

- Add a write-path freshness hook so successful `file_write` operations update or invalidate the
  affected workspace-relative index entry when a compatible index already exists.
- Strengthen indexed candidate planning so `CandidateCoverage::Complete` is returned only when the
  current searchable workspace state exactly matches persisted entries, including hash-based trust
  checks instead of relying on mtime/size hints alone.
- Handle changed, deleted, and renamed files safely by treating stale paths as refresh/remove
  events or by downgrading indexed coverage so `code_search` falls back to live verification.
- Add regression tests proving own-write visibility plus changed/deleted/renamed file behavior and
  stale-entry fallback semantics.
- Document v1 freshness signals, trust boundaries, and non-goals in OpenSpec artifacts.

### Out of Scope

- Filesystem watchers, background auto-refresh daemons, or always-on incremental indexing.
- Perfect rename detection beyond path-based delete + add semantics.
- Broad search-performance optimization beyond the minimal correctness-preserving v1 behavior.
- Any change to the external `code_search` result contract beyond freshness/candidate safety.

## Approach

Use the hybrid v1 identified in exploration: combine targeted write-through freshness for
agent-originated writes with stricter search-time stale guards.

Concretely, add a small index API in `clients/agent-runtime/src/search/index.rs` that can refresh or
remove specific workspace-relative paths under the existing index lock/transaction model. Call that
API from `clients/agent-runtime/src/tools/file_write.rs` only after a successful write. If no
compatible on-disk index exists yet, skip the hook and rely on existing safe fallback behavior.

Then tighten `plan_candidates()` so it only reports complete indexed coverage when the discovered
workspace corpus exactly matches persisted rows for the searchable set. `size_bytes` and
`modified_unix_ms` remain fast change hints, but `content_sha256` becomes the correctness guard for
trusting an indexed entry. When discovery finds a changed file, missing file, extra stale row, or a
rename-shaped old/new path pair, candidate planning MUST NOT silently trust the old corpus; it
should refresh/remove the affected rows when safe to do so, or downgrade coverage so `code_search`
falls back to live scanning.

Documentation for v1 should explicitly state that workspace-relative path is the persisted identity,
hash comparison is the trust boundary, mtimes are hints only, and git state is optional/diagnostic
but never authoritative for freshness.

## Affected Areas

| Area                                                  | Impact                       | Description                                                                                                         |
|-------------------------------------------------------|------------------------------|---------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/search/index.rs`           | Modified                     | Add path-scoped refresh/remove API and tighten `plan_candidates()` freshness rules.                                 |
| `clients/agent-runtime/src/search/sqlite.rs`          | Modified                     | Reuse or extend persisted row operations needed for targeted refresh/removal using existing metadata contract.      |
| `clients/agent-runtime/src/search/discovery.rs`       | Modified                     | Ensure discovery data used for freshness decisions includes the metadata/hash inputs needed for exact trust checks. |
| `clients/agent-runtime/src/tools/file_write.rs`       | Modified                     | Invoke the index freshness hook after successful writes so the agent can read its own writes.                       |
| `clients/agent-runtime/src/tools/code_search.rs`      | Modified                     | Preserve safe fallback behavior when indexed candidate coverage is downgraded for stale state.                      |
| `clients/agent-runtime/src/search/tests.rs`           | Modified                     | Add regression coverage for own-write visibility and changed/deleted/renamed file freshness behavior.               |
| `clients/agent-runtime/src/tools/file_write.rs` tests | Modified                     | Add regression coverage that successful writes update index freshness semantics.                                    |
| `openspec/specs/workspace-index/spec.md`              | Modified later in spec phase | Extend normative requirements for freshness trust, own-write behavior, and v1 limits.                               |

## Risks

| Risk                                                                                      | Likelihood | Mitigation                                                                                                                                                                    |
|-------------------------------------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Path-scoped refresh logic leaves stale trigram rows behind for deleted or renamed files   | Medium     | Implement removal and refresh inside the existing transaction boundary and cover delete/rename regressions with persisted-state assertions.                                   |
| Search-time freshness checks increase overhead because discovery already reads file bytes | Medium     | Keep v1 scope correctness-first, preserve fallback behavior, and defer performance tuning or watcher-based optimization to future work.                                       |
| Same-size, same-mtime rewrites remain invisible if hash checks are skipped in any path    | Medium     | Require `content_sha256` as the trust guard for `Complete` coverage and add a regression for unchanged mtime/size hints with changed contents when practical.                 |
| Write hook failures could block otherwise valid file writes                               | Low        | Make index update best-effort after the successful write, surface/log refresh failures appropriately, and allow safe fallback search behavior instead of rejecting the write. |

## Rollback Plan

Revert the path-scoped write hook and freshness-guard changes in `search/index.rs`, `file_write.rs`,
and any supporting SQLite/discovery helpers, returning the runtime to the current fallback-driven
behavior. Because the v1 change only tightens trust and updates persisted rows using the existing
index location, rollback is limited to code reversion; if needed, deleting
`state/code-search/index.db`
forces a clean rebuild under the prior behavior.

## Dependencies

- Existing workspace index SQLite metadata contract in `clients/agent-runtime/src/search/sqlite.rs`
- Existing `code_search` fallback behavior for `CandidateCoverage::Partial` and `Unavailable`
- Exploration findings in `openspec/changes/2026-04-05-search-index-freshness/exploration.md`

## Success Criteria

- [ ] A successful `file_write` makes the written file discoverable by subsequent indexed search in
  the same workspace without requiring manual rebuild.
- [ ] Indexed candidate planning MUST NOT report complete coverage when searchable files have been
  changed, deleted, or renamed outside the persisted corpus state.
- [ ] Changed, deleted, and renamed files are handled safely via targeted refresh/removal or
  fallback to live scanning, with no silently trusted stale rows.
- [ ] Regression tests cover own-write visibility and stale-entry handling for changed/deleted/
  renamed paths.
- [ ] Follow-on spec/design artifacts document v1 freshness signals (`relative_path`, `size_bytes`,
  `modified_unix_ms`, `content_sha256`) and clearly state v1 limits.
