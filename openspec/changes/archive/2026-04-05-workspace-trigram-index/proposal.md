# Proposal: Workspace Trigram Index

## Intent

`clients/agent-runtime` already has a safe `code_search` tool for walking the workspace, filtering ignored files, and excluding unsafe or binary content. What it does not have is a persistent local corpus index, so every future code-search improvement starts from a cold filesystem scan.

This change creates the first persistent index layer for fast local code search by introducing deterministic workspace corpus discovery and storing an initial trigram-backed SQLite index. The goal is to make indexed corpus state durable, reproducible, and safe so later search/query features can build on a stable foundation instead of reimplementing discovery and persistence rules.

## Scope

### In Scope

- Add a reusable workspace search/index module in `clients/agent-runtime` that can discover indexable files using the same safety and filtering rules already proven in `code_search`.
- Persist an initial workspace corpus index in SQLite, including workspace-relative file entries, trigram index data, and refresh/build metadata needed to load or rebuild the corpus deterministically.
- Define and test index build, load, and rebuild behavior, including deterministic exclusion of unsafe paths, symlink escapes, unreadable paths, ignored files, oversized files, and non-text files.
- Ensure persisted file identities are workspace-relative only and never store absolute workspace paths in index rows.

### Out of Scope

- Adding the final query-serving path that replaces or augments `code_search` results with trigram-backed retrieval.
- Incremental watch mode, background refresh daemons, or partial per-file updates.
- Semantic/vector indexing, ranking, snippets, or hybrid retrieval.
- Cross-workspace or remote corpus storage.
- Expanding filesystem access beyond the current `SecurityPolicy` and safe discovery model.

## Approach

Create a new reusable indexing layer under `clients/agent-runtime/src/` that separates three concerns clearly:

1. **Discovery** — enumerate candidate files with the same deterministic workspace walk, ignore handling, path validation, symlink resolution, size limits, and binary/text filtering already enforced by `code_search`.
2. **Persistence** — store corpus membership, file metadata, refresh state, and trigram postings in a SQLite schema designed for reliable build/load/rebuild cycles.
3. **Index lifecycle** — provide explicit operations for initial build, loading an existing index, and full rebuild when corpus metadata no longer matches the current workspace snapshot.

The proposal assumes SQLite because it is already used in the runtime, keeps the first index layer local/offline, and gives us transactional rebuild semantics. The build path should favor deterministic inputs and outputs:

- canonicalize and validate real filesystem paths before indexing;
- convert accepted files to workspace-relative keys before persistence;
- skip unsafe, unreadable, ignored, oversized, and binary files with stable rules;
- record refresh metadata so tests can prove when a build is fresh, stale, or rebuilt.

Where practical, shared discovery helpers should be extracted from `code_search` so the runtime has one source of truth for safe corpus enumeration instead of duplicating workspace-walk behavior.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/tools/code_search.rs` | Modified | Extract or reuse safe workspace discovery/filtering logic so indexing and search share deterministic corpus rules. |
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Wire any shared search/index module exports needed by runtime code paths and tests. |
| `clients/agent-runtime/src/memory/sqlite.rs` or new SQLite-backed index module | Modified/New | Reuse existing SQLite patterns or add a dedicated index persistence module for schema creation, transactions, and rebuild behavior. |
| `clients/agent-runtime/src/security/policy.rs` | Referenced/Possibly Modified | Reuse existing workspace path and resolved-path safety checks without broadening access scope. |
| `clients/agent-runtime/Cargo.toml` | Possibly Modified | Only if small supporting crates are required; prefer existing dependencies and current SQLite stack. |
| `clients/agent-runtime/src/**/tests` or inline `#[cfg(test)]` blocks | New/Modified | Add regression coverage for build/load/rebuild behavior and deterministic exclusion rules. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Discovery logic diverges from `code_search`, causing inconsistent corpora | Medium | Extract shared helpers or centralize discovery rules so one implementation drives both scan and index behavior. |
| SQLite schema locks in a poor shape for later query features | Medium | Keep the first schema minimal and explicit: corpus entries, trigram postings, and refresh metadata only; document non-goals for later evolution. |
| Large workspaces make initial builds expensive | Medium | Preserve current safety/resource filters, use transactional bulk writes, and treat this as an initial build path rather than a background always-on feature. |
| Absolute or unsafe paths leak into persisted state | Low | Canonicalize before acceptance, store workspace-relative keys only, and add tests that inspect persisted rows directly. |
| Binary/text detection mismatch produces unstable indexing | Low | Reuse the same null-byte sampling strategy and file-size gates already specified for `code_search`. |

## Rollback Plan

If this change causes correctness or performance regressions, revert the index-building entrypoints and schema wiring in `clients/agent-runtime`, remove the new SQLite index tables/module, and fall back to the current scan-only behavior. Because this proposal stores only derived workspace corpus data, rollback does not require content migration; the runtime can simply stop reading the index and rebuild later from the filesystem when a revised design lands.

## Dependencies

- Existing `code_search` workspace discovery and filtering behavior in `clients/agent-runtime/src/tools/code_search.rs`
- Existing workspace safety checks in `clients/agent-runtime/src/security/policy.rs`
- Existing SQLite support via `rusqlite` in `clients/agent-runtime/Cargo.toml`

## Success Criteria

- [ ] The runtime can build an initial local SQLite-backed index for a workspace corpus.
- [ ] Persisted corpus entries use workspace-relative file identities only.
- [ ] Tests cover index build, load, and rebuild behavior.
- [ ] Unsafe paths, symlink escapes, ignored files, oversized files, unreadable paths, and non-text files are excluded deterministically.
- [ ] The proposal leaves a clear path for a later trigram query layer without requiring a discovery/persistence redesign.

## Rollout Notes

This should roll out as an internal runtime capability first, not as a broad user-facing behavior change. The safest sequence is: land the shared discovery layer, land SQLite persistence with strong tests, then add a later change for query integration once build and rebuild semantics are proven stable.