# Design: Tooling Parity for Search, Fetch, and Task Tools

## Technical Approach

This first slice adds three additive parity-facing native tools in `clients/agent-runtime/src/tools/`: `Glob`, `Grep`, and `WebFetch`. The implementation stays additive and keeps `code_search` and `http_request` intact, but it extracts small shared internals where drift risk is otherwise high.

The main architecture split is:

- `Glob` gets its own tool implementation, backed by a safer metadata-only discovery path in `clients/agent-runtime/src/search/discovery.rs`.
- `Grep` becomes the public parity contract, but both `Grep` and `code_search` will share one extracted content-search backend under `clients/agent-runtime/src/search/` so validation, candidate planning, workspace traversal, verification ordering, and truncation stay aligned.
- `WebFetch` becomes a new read-only fetch-and-extract tool that reuses `http_request` URL-policy and transport helpers without inheriting its mutation-capable API surface.
- Tool naming parity is surfaced in two places only for this slice: runtime inventory (`/tools`) and canonical docs. Broad renaming/deprecation is explicitly deferred.

This maps directly to the delta spec requirements for additive parity tools, stable workspace-relative outputs, preserved security boundaries, and published mapping/documentation.

## Architecture Decisions

### Decision: Place parity tools as first-class native tools in `src/tools/`

**Choice**: Create `clients/agent-runtime/src/tools/glob.rs`, `clients/agent-runtime/src/tools/grep.rs`, and `clients/agent-runtime/src/tools/web_fetch.rs`, and register them from `clients/agent-runtime/src/tools/mod.rs`.

**Alternatives considered**:
- Rename `code_search` and `http_request` in place.
- Hide parity behavior behind aliases outside the tool registry.

**Rationale**: Corvus tool discovery, capability registration, provider tool specs, and `/tools` all derive from registered `Tool` implementations. First-class native tools are the smallest change that makes parity names real everywhere they need to be real, without breaking existing names or widening the blast radius into approvals, prompts, and provider fixtures.

### Decision: `Grep` should share extracted internals, not wrap `code_search.execute()`

**Choice**: Extract a reusable content-search backend from `clients/agent-runtime/src/tools/code_search.rs` into a new search-layer module, recommended as `clients/agent-runtime/src/search/content.rs`, then have both `code_search` and `Grep` adapt their own public schemas into that backend.

**Alternatives considered**:
- Thin wrapper around `CodeSearchTool::execute()` and post-process its `ToolResult`.
- Duplicate the search engine in `grep.rs`.

**Rationale**: A wrapper over `execute()` would lock `Grep` to the current `code_search` request/response contract, which does not match the slice spec (`glob`, `output_mode`, offset/limit/count/files-only modes). Duplicating the engine would create immediate drift risk in the most security-sensitive and correctness-sensitive logic: path validation, candidate planning, binary filtering, deterministic verification ordering, truncation, and warnings. Extracting internals keeps one search engine and two public adapters.

### Decision: `Glob` should reuse discovery through metadata-only helpers

**Choice**: Extend `clients/agent-runtime/src/search/discovery.rs` with a metadata-only discovery path for file listing, then implement `Glob` on top of that path plus stable deterministic sorting.

**Alternatives considered**:
- Reuse `discover_searchable_files_with_stats()` directly.
- Shell out to the `glob` crate from the tool layer without discovery reuse.

**Rationale**: `discover_searchable_files_with_stats()` currently opens files, reads bytes, and filters binary content because it is optimized for content search. `Glob` only needs safe path discovery and metadata ordering, so reusing it as-is would be wasteful and would couple file-name search to content-read costs. A metadata-only helper preserves the same workspace-root validation, ignore behavior, symlink handling, and relative-path normalization, but does not read file contents unnecessarily.

### Decision: `WebFetch` should reuse `http_request` security/transport helpers but remain a separate read-only tool

**Choice**: Extract shared outbound HTTP policy and GET transport helpers from `clients/agent-runtime/src/tools/http_request.rs` into a small shared support module, recommended as `clients/agent-runtime/src/tools/http_common.rs`, and use that from both `http_request` and `web_fetch`.

**Alternatives considered**:
- Call `HttpRequestTool::execute()` internally.
- Reimplement URL validation and transport in `web_fetch.rs`.

**Rationale**: `HttpRequestTool::execute()` bundles method parsing, headers, bodies, and `can_act()` gating for an action-bearing API tool. `WebFetch` needs the same scheme checks, allowlist enforcement, private-host blocking, redirect policy, timeout, and size cap, but only for read-only GET-style retrieval. Extracting helpers keeps one SSRF/security posture while letting `WebFetch` present a different contract and autonomy classification.

### Decision: Surface parity mapping in descriptions and docs, not by renaming legacy tools

**Choice**: Keep existing native names registered, add parity tools with parity-oriented descriptions, and update `/tools` plus docs to explicitly show backing relationships.

**Alternatives considered**:
- Suppress legacy tools from `/tools`.
- Add a separate alias registry or a new inventory schema in this slice.

**Rationale**: The current `/tools` inventory is a thin projection of registered tool descriptors (`name` + `description`). Reusing that path keeps this slice small. Clear descriptions such as “Claude-style parity search backed by code_search internals” are enough to distinguish additive parity tools from legacy/native surfaces now. A richer alias metadata model can come later if the team wants consolidation.

### Decision: `WebFetch` is extraction-first in slice 1

**Choice**: Treat `prompt` as a required caller intent field, but keep the implementation deterministic and extraction-oriented: fetch, normalize content by media type, cap output, and return extracted content in `result`.

**Alternatives considered**:
- Invoke a model from inside the tool to summarize content.
- Ignore `prompt` entirely.

**Rationale**: Embedding provider/model calls inside a runtime tool is a larger architecture change than this slice needs and would create recursion, cost, and permission questions. Ignoring `prompt` would break parity expectations. The compromise is to accept and validate `prompt`, keep it in the public contract, and return extraction-oriented normalized content now. That preserves the tool shape and security posture without inventing a second orchestration stack inside the tool.

## Data Flow

### `Glob`

```text
LLM/tool call
   │
   ▼
GlobTool::execute
   │ validate pattern + optional path
   │ validate workspace root via SecurityPolicy/discovery
   ▼
search::discovery::discover_matching_paths_with_stats (new metadata-only helper)
   │ walk workspace with ignore rules
   │ canonicalize + block workspace escapes
   │ normalize relative paths
   ▼
GlobTool result shaping
   │ deterministic ordering
   │ cap/truncation metadata
   ▼
ToolResult { output, structured }
```

Sequence:

```text
Caller -> GlobTool: { pattern, path? }
GlobTool -> SecurityPolicy: is_path_allowed(path)
GlobTool -> discovery: validate_search_root(path)
discovery -> filesystem: walk/search within resolved root
filesystem --> discovery: candidate entries
.discovery -> GlobTool: relative paths + stats
GlobTool --> Caller: filenames[], numFiles, truncated, durationMs
```

### `Grep`

```text
LLM/tool call
   │
   ▼
GrepTool::execute
   │ parse parity schema
   │ normalize output_mode/context/include filters
   ▼
search::content backend (new shared engine)
   │ validate root + compile pattern
   │ use WorkspaceTrigramIndex candidate planning when applicable
   │ fall back to discovery when needed
   │ verify live file contents deterministically
   ▼
GrepTool result adapter
   │ content/files_with_matches/count shaping
   │ offset/limit application at public contract layer
   ▼
ToolResult { output, structured }
```

Sequence:

```text
Caller -> GrepTool: parity args
GrepTool -> content backend: SharedSearchRequest
content backend -> trigram index: plan_candidates(...)
content backend -> discovery: fallback inputs if needed
content backend -> verifier: ordered live-match verification
verifier --> content backend: verified matches + stats + warnings
content backend --> GrepTool: SharedSearchOutcome
GrepTool --> Caller: Grep structured contract
```

### `WebFetch`

```text
LLM/tool call
   │
   ▼
WebFetchTool::execute
   │ validate url + prompt
   │ rate-limit accounting (read-only, no can_act gate)
   ▼
http_common policy helpers (extracted from http_request)
   │ scheme check
   │ allowlist match
   │ private/local host rejection
   ▼
http_common GET transport
   │ timeout
   │ redirects disabled
   │ bounded body streaming
   ▼
WebFetch extraction pipeline
   │ inspect status + content-type
   │ normalize html/json/text into bounded text/markdown-like output
   ▼
ToolResult { bytes, code, codeText, result, durationMs, url }
```

Sequence:

```text
Caller -> WebFetchTool: { url, prompt }
WebFetchTool -> http_common: validate_url(url)
http_common -> allowlist/private-host checks: permit?
http_common --> WebFetchTool: validated url
WebFetchTool -> http_common: execute_get(validated url)
http_common --> WebFetchTool: response(status, headers, body)
WebFetchTool -> extractor: normalize by content-type
extractor --> WebFetchTool: extracted result text
WebFetchTool --> Caller: bytes/code/codeText/result/durationMs/url
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/tools/glob.rs` | Create | New parity `Glob` tool with schema validation, deterministic ordering, truncation behavior, and tests. |
| `clients/agent-runtime/src/tools/grep.rs` | Create | New parity `Grep` tool exposing Claude-style request/response shape while delegating to shared search internals. |
| `clients/agent-runtime/src/tools/web_fetch.rs` | Create | New read-only `WebFetch` tool with extracted-content contract and tests. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Export/register new tools in the native registry. |
| `clients/agent-runtime/src/search/content.rs` | Create | Shared content-search backend extracted from `code_search.rs` for reuse by `code_search` and `Grep`. |
| `clients/agent-runtime/src/search/mod.rs` | Modify | Export the new shared search backend module. |
| `clients/agent-runtime/src/tools/code_search.rs` | Modify | Reduce to the `code_search` public adapter plus existing result-format behavior, now backed by shared internals. |
| `clients/agent-runtime/src/search/discovery.rs` | Modify | Add metadata-only path discovery helper(s) for `Glob` and retain existing content-search discovery behavior. |
| `clients/agent-runtime/src/tools/http_common.rs` | Create | Shared URL validation, host policy, redirect/timeout/body-cap helpers extracted from `http_request`. |
| `clients/agent-runtime/src/tools/http_request.rs` | Modify | Reuse shared HTTP helpers without changing `http_request` public behavior. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Register parity tools into profile allowlists and keep slash-tool snapshots aligned. |
| `clients/agent-runtime/src/security/policy.rs` | Modify | Add `Glob`, `Grep`, and `WebFetch` to plan-safe read-only tool allowlist; keep legacy names intact. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modify | No schema change required; optionally refine rendered descriptions if formatting tests need updates. |
| `clients/agent-runtime/src/skills/frontmatter.rs` | Modify (tests/docs only if needed) | Keep skill-facing parity names consistent with runtime availability; no parser change required. |
| `clients/web/apps/docs/src/content/docs/clients/agent-runtime/tools/index.mdx` | Modify | Add parity mapping section and note additive/canonical/deferred status. |
| `clients/web/apps/docs/src/content/docs/clients/agent-runtime/tools/core.md` | Modify | Document `Glob` and `Grep`, and note `code_search` as retained native tool. |
| `clients/web/apps/docs/src/content/docs/clients/agent-runtime/tools/web.md` | Modify | Document `WebFetch` and distinguish it from `http_request` and `web_search_tool`. |
| `clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/tools/index.mdx` | Modify | Spanish parity mapping mirror of canonical docs. |
| `clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/tools/core.md` | Modify | Spanish parity docs for `Glob`/`Grep`. |
| `clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/tools/web.md` | Modify | Spanish parity docs for `WebFetch`. |

## Interfaces / Contracts

### Shared search backend

```rust
pub struct SharedSearchRequest {
    pub pattern: String,
    pub relative_root: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub context_lines: usize,
    pub max_results: usize,
}

pub struct SharedSearchOutcome {
    pub matches: Vec<VerifiedSearchMatch>,
    pub stats: SharedSearchStats,
    pub warnings: Vec<String>,
    pub fatal_error: Option<String>,
}
```

`code_search` will keep its current public schema and formatting. `Grep` will adapt its parity schema into `SharedSearchRequest`, then reshape `SharedSearchOutcome` into the parity response modes.

### `Grep` request normalization

```rust
pub enum GrepOutputMode {
    Content,
    FilesWithMatches,
    Count,
}

pub struct GrepRequest {
    pub pattern: String,
    pub path: String,
    pub glob: Option<String>,
    pub output_mode: GrepOutputMode,
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub context: Option<usize>,
    pub case_insensitive: bool,
    pub head_limit: usize,
    pub offset: usize,
    pub multiline: bool,
}
```

Normalization rules:

- `glob` becomes a single include override passed into shared search.
- `-A`, `-B`, `-C`, and `context` collapse to one validated `context_lines` value.
- `files_with_matches` and `count` mode derive from verified matches, not raw candidate files.
- `offset`/`head_limit` apply after deterministic file/match ordering so repeated calls remain stable.

### `Glob` discovery output

```rust
pub struct GlobMatchSet {
    pub filenames: Vec<String>,
    pub duration_ms: u64,
    pub num_files: usize,
    pub truncated: bool,
}
```

Ordering rule for slice 1:

- Sort by `modified_unix_ms` descending.
- Tie-break on `relative_path` ascending.

That gives deterministic output and matches the parity expectation that modification-time ordering is acceptable when stable.

### `WebFetch` extracted result

```rust
pub struct WebFetchResponse {
    pub bytes: usize,
    pub code: u16,
    pub code_text: String,
    pub result: String,
    pub duration_ms: u64,
    pub url: String,
}
```

Normalization rules:

- `text/html`: extract main readable body into markdown-like/plain text.
- `application/json` and `text/*`: return bounded text directly.
- Unsupported but textual types: best-effort UTF-8 lossless-ish conversion with bounds.
- Binary/non-textual responses: fail clearly instead of returning opaque bytes in this slice.

### Naming and inventory contract

For this slice, canonical parity-facing names are additive:

| Parity name | Backing runtime piece | Slice status | Notes |
|---|---|---|---|
| `Glob` | discovery metadata helper | Additive + canonical for parity docs | New native tool |
| `Grep` | shared search backend extracted from `code_search` | Additive + canonical for parity docs | `code_search` remains available |
| `WebFetch` | shared HTTP policy/transport extracted from `http_request` | Additive + canonical for parity docs | `http_request` remains action-bearing |
| `Task*` | none in this slice | Deferred | Explicitly out of scope |

`/tools` should list both additive parity tools and retained native tools, but descriptions must state the relationship, for example:

- `Grep — Claude-style parity content search backed by Corvus native search internals.`
- `code_search — Corvus native workspace search with advanced match payloads.`
- `WebFetch — Read-only fetch-and-extract tool for allowlisted web content.`
- `http_request — Structured API request tool for explicit HTTP operations.`

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `Glob` argument validation | Add `glob.rs` tests for empty pattern, invalid/escaping path, deterministic ordering, and truncation metadata. |
| Unit | Discovery reuse for `Glob` | Extend `src/search/tests.rs` for metadata-only discovery: ignore rules, symlink escape rejection, hidden/oversized filtering, relative-path normalization, stable ordering inputs. |
| Unit | Shared search backend | Move or mirror core `code_search` engine tests to the new backend for candidate planning fallback, verification ordering, truncation, and zero-match behavior. |
| Unit | `Grep` parity request normalization | Add `grep.rs` tests for `glob` mapping, context flag combinations, negative values, `output_mode` validation, and offset/head-limit behavior. |
| Unit | `Grep`/`code_search` alignment | Add parity regression tests asserting the same workspace/pattern yields the same verified file ordering and same match counts between shared backend consumers. |
| Unit | Shared HTTP helpers | Add tests in `http_common.rs` or `http_request.rs` for scheme validation, allowlist matching, private-host blocking, redirect policy, and bounded reads. |
| Unit | `WebFetch` extraction pipeline | Add tests for html/text/json extraction, unsupported scheme rejection, binary-response rejection, and read-only autonomy semantics. |
| Integration | Bootstrap/tool inventory | Extend `bootstrap/mod.rs` tests so code/full profiles include `Glob`, `Grep`, and `WebFetch` when enabled, lite remains unchanged unless explicitly widened, and plan mode reflects the new read-only tools. |
| Integration | `/tools` rendered listing | Extend `session_commands/service.rs` tests to assert parity tool names appear and descriptions distinguish parity vs native names. |
| Documentation | Parity mapping consistency | Review docs pages and examples so canonical parity names and additive/native status match runtime descriptions. |

## Validation Strategy

Implementation-phase validation should stay targeted to the Rust runtime and docs touched by this slice:

1. `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`
2. `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`
3. `cargo test --manifest-path clients/agent-runtime/Cargo.toml glob`
4. `cargo test --manifest-path clients/agent-runtime/Cargo.toml grep`
5. `cargo test --manifest-path clients/agent-runtime/Cargo.toml web_fetch`
6. `cargo test --manifest-path clients/agent-runtime/Cargo.toml search`
7. Targeted bootstrap/session-command tests covering `/tools` inventory and profile gating
8. Docs checks only if the docs workspace requires them for touched pages

The most important verification invariant is behavioral alignment:

- `Grep` must not widen filesystem scope beyond `code_search`.
- `WebFetch` must not weaken URL policy versus `http_request`.
- `Glob` must not read file contents just to list names.

## Migration / Rollout

No migration required.

Rollout plan:

1. Add parity tools additively and register them in code/full profiles.
2. Keep `code_search`, `http_request`, and `web_search_tool` unchanged and still listed.
3. Update canonical docs and `/tools` descriptions in the same slice so users immediately see the mapping.
4. Defer any removal, renaming, or alias collapsing to a follow-up change after usage and confusion are measured.

## Risks

- **Search drift risk**: if `Grep` re-implements search behavior instead of sharing internals, results will diverge from `code_search`. Mitigation: extract one backend and add alignment tests.
- **Discovery cost risk**: if `Glob` reuses byte-reading discovery directly, large trees pay unnecessary I/O cost. Mitigation: add metadata-only discovery helpers.
- **Security regression risk**: if `WebFetch` forks URL validation logic, allowlist/private-host behavior can drift. Mitigation: extract shared HTTP policy helpers and test denial-before-request behavior.
- **Naming confusion risk**: exposing both parity and native names can confuse users. Mitigation: parity-first descriptions in `/tools`, explicit mapping tables in docs, and no silent hiding of native tools.
- **Dependency/HTML parsing risk**: HTML extraction may tempt a heavy dependency. Mitigation: isolate extraction behind a helper boundary so dependency choice stays reversible and can be swapped without changing the tool contract.
- **Plan-mode mismatch risk**: new read-only tools could be forgotten in plan-safe gating. Mitigation: update `PLAN_MODE_SAFE_TOOLS` and bootstrap tests in the same implementation slice.

## Rollback

Rollback is additive and low-risk:

1. Remove `Glob`, `Grep`, and `WebFetch` registrations from `tools/mod.rs` and bootstrap allowlists.
2. Revert `/tools` description/doc updates.
3. Keep extracted internal helpers only if still used by `code_search`/`http_request`; otherwise remove them in the same revert.

Because this slice introduces no persistent state, no config migration, and no task lifecycle storage, rollback restores the pre-slice surface without data recovery work.

## Open Questions

- [ ] None blocking. The implementation may choose the concrete HTML normalization helper during coding, but the boundary is intentionally isolated so that decision stays local and reversible.
