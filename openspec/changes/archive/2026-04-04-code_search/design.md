# Design: `code_search` Tool

> Canonical location: `clients/agent-runtime/docs/design/code-search-tool.md`
>
> This file mirrors the design document stored alongside the runtime source code.
> See the canonical file for the full, authoritative version.

| Field  | Value              |
|--------|--------------------|
| Status | DRAFT              |
| Author | AI-assisted design |
| Date   | 2026-04-04         |
| Linear | DALLAY-200         |

## Technical Approach

Add a native `code_search` tool to the Corvus agent runtime that performs workspace-scoped text
and regex search across source files. v1 uses brute-force directory walking via the `ignore`
crate combined with the existing `regex` crate. No search index.

## Architecture Decisions

1. **`ignore` crate for directory walking** — `.gitignore`-aware, parallel traversal, same
   ecosystem as `regex`. Preferred over `walkdir` + manual parsing or shelling out to `rg`.
2. **No search index in v1** — brute-force is fast enough for <10K file repos. Index adds
   complexity without proven need. Same API contract means v2 can add indexing transparently.
3. **Single action per search for rate limiting** — one `record_action()` per invocation,
   matching `file_read` semantics.
4. **500-char line content cap** — prevents minified files from blowing up output.
5. **`code_search` name** — clear intent, avoids collision with `web_search_tool` or generic
   `search`.

## File Changes

| File                              | Action | Description                                            |
|-----------------------------------|--------|--------------------------------------------------------|
| `src/tools/code_search.rs`        | Create | `CodeSearchTool` struct + `Tool` trait impl + tests    |
| `src/tools/mod.rs`                | Modify | Register module and tool in `all_tools_with_runtime()` |
| `Cargo.toml`                      | Modify | Add `ignore = "0.4"` dependency                        |
| `docs/design/code-search-tool.md` | Create | Full design document                                   |

## Testing Strategy

| Layer | What to Test                                          | Approach                          |
|-------|-------------------------------------------------------|-----------------------------------|
| Unit  | Pattern matching (literal, regex, case, whole-word)   | Temp dirs with known files        |
| Unit  | Security (path traversal, symlinks, rate limiting)    | Same helpers as `file_read` tests |
| Unit  | Resource limits (max results, file size, binary skip) | Synthetic large/binary files      |
| Unit  | Edge cases (empty pattern, bad regex, missing path)   | Error path validation             |

## Open Questions

- [ ] Should `include`/`exclude` support `**` recursive globs? (Recommend: yes)
- [ ] Custom `.searchignore` file? (Recommend: defer to v2)
