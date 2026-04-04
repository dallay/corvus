# Proposal: Native `code_search` Tool

**Linear**: DALLAY-200

## Intent

Agents running on the Corvus runtime currently rely on shell-based `grep`/`rg` invocations to search
workspace code. This approach is fragile (unstructured text output the LLM must parse), insecure (
bypasses `SecurityPolicy` path checks and requires `can_act()` autonomy), and unavailable in
`ReadOnly` mode.

Adding a native `code_search` tool provides structured, workspace-scoped text and regex search that
works at all autonomy levels, respects `.gitignore`, enforces the same security model as
`file_read`, and returns both human-readable and machine-readable results.

## Scope

### In Scope

- `CodeSearchTool` struct implementing the `Tool` trait in `src/tools/code_search.rs`
- Brute-force directory walk via the `ignore` crate with `.gitignore` awareness
- Regex and literal pattern matching using the existing `regex` crate
- 9 input parameters: `pattern`, `path`, `include`, `exclude`, `is_regex`, `case_sensitive`,
  `max_results`, `context_lines`, `whole_word`
- Dual output: grep-like text (`output`) + structured JSON (`structured`) with matches and stats
- Full security model: workspace scoping, symlink escape prevention, binary file skipping, path
  traversal rejection, rate limiting (one action per search)
- Resource limits: 10K file scan cap, 10MB per-file cap, 100KB output cap, 50 matches per file, 500
  max total matches, 30s timeout
- Unit tests in `code_search.rs` following the `file_read.rs` test pattern
- Module registration in `src/tools/mod.rs`
- New dependency: `ignore = "0.4"`

### Out of Scope

- Search index / persistent index (v2)
- Multi-line pattern matching (v2)
- AST-aware / symbol-kind search (v2)
- `.searchignore` support (v2)
- `mmap`-based file reading (v2)
- Search history or caching layer (v2)
- Configurable file-scan limits via agent config (v2)

## Approach

Implement a single new tool following the established `Tool` trait pattern:

1. **Parameter validation** — Validate pattern length (≤1000 chars), path relativity, and regex
   compilation. Construct the final regex following the order: escape (if literal) → case flag →
   word boundary wrapping.
2. **Security checks** — `is_rate_limited()` / `record_action()` before any I/O.
   `is_path_allowed()` → `canonicalize()` → `is_resolved_path_allowed()` on the search root path,
   same chain as `file_read`.
3. **Directory walking** — `ignore::WalkBuilder` rooted at `workspace_dir.join(path)` with
   include/exclude globs. The `ignore` crate handles `.gitignore`, nested overrides, hidden
   directory exclusion, and binary detection natively.
4. **Per-file scanning** — Read each file, apply regex per line, collect matches with optional
   context lines. Skip files that are binary, >10MB, or resolve outside the workspace via symlinks.
5. **Result construction** — Build `ToolResult` with grep-like `output` text and structured JSON
   containing `matches[]` and `stats{}`. Append truncation warnings when limits are hit.

No feature flag or config toggle — the tool is always registered, like `file_read`.

## Affected Areas

| Area                       | Impact   | Description                                                               |
|----------------------------|----------|---------------------------------------------------------------------------|
| `src/tools/code_search.rs` | New      | Tool implementation, search logic, and unit tests                         |
| `src/tools/mod.rs`         | Modified | Module declaration, re-export, registration in `all_tools_with_runtime()` |
| `Cargo.toml`               | Modified | Add `ignore = "0.4"` dependency                                           |

## Risks

| Risk                                                  | Likelihood | Mitigation                                                                                                    |
|-------------------------------------------------------|------------|---------------------------------------------------------------------------------------------------------------|
| New dependency (`ignore` crate) increases binary size | Low        | `ignore` is from the BurntSushi/ripgrep ecosystem, ~15KB addition, minimal transitive deps, well-maintained   |
| Performance degradation on large repos (>10K files)   | Medium     | 10K file scan limit with truncation warning, 30s execution timeout, parallel walk via `ignore` crate          |
| Oversized output blowing up LLM context               | Low        | 100KB output cap, 500 max matches, 50 per-file cap, 500-char line truncation                                  |
| Regex denial-of-service (pathological patterns)       | Low        | Delegated to `regex` crate's built-in compile-time size/nesting limits + 30s timeout                          |
| Symlink escape leaking files outside workspace        | Low        | Same `canonicalize()` + `is_resolved_path_allowed()` chain as `file_read`; escaping symlinks silently skipped |

## Rollback Plan

Revert three file changes:

1. Remove `src/tools/code_search.rs`
2. Remove module declaration, re-export, and registration from `src/tools/mod.rs`
3. Remove `ignore = "0.4"` from `Cargo.toml`

The change is purely additive — no existing tool behavior is modified. Rollback has zero impact on
other tools or the runtime.

## Dependencies

- `ignore` crate v0.4 (new) — `.gitignore`-aware parallel directory walker from the ripgrep
  ecosystem
- `regex` crate (existing) — already in `Cargo.toml`, no version change needed
- `SecurityPolicy` (existing) — reuses workspace scoping, path validation, and rate limiting
  infrastructure

## Success Criteria

- [ ] `CodeSearchTool` passes all unit tests covering: literal search, regex search, case
  insensitivity, whole word, context lines, include/exclude globs, path scoping, binary skip, file
  size limit, empty pattern error, invalid regex error, path traversal rejection, symlink escape
  blocking, max results truncation, per-file cap, structured output shape, ReadOnly mode,
  `.gitignore` respect, and zero-match case
- [ ] `cargo clippy --all-targets -- -D warnings` passes with no new warnings
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo test` passes with all existing tests still green
- [ ] Tool is registered and available at all autonomy levels (ReadOnly, Supervised, Full)
- [ ] Binary size increase from `ignore` crate is ≤50KB in release profile
