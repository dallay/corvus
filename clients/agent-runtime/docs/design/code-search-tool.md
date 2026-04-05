# Design: `code_search` Tool

| Field  | Value              |
|--------|--------------------|
| Status | DRAFT              |
| Author | AI-assisted design |
| Date   | 2026-04-04         |
| Linear | DALLAY-200         |

## Technical Approach

Add a native `code_search` tool to the Corvus agent runtime that performs workspace-scoped text
and regex search across source files. The tool follows the same `Tool` trait pattern as
`file_read` and reuses the existing `SecurityPolicy` for path validation, rate limiting, and
workspace sandboxing. v1 uses brute-force directory walking via the `ignore` crate (for
`.gitignore` awareness) combined with the `regex` crate for pattern matching. No index is built.

The tool returns both a human-readable grep-like `output` string and a machine-readable
`structured` JSON payload, consistent with the `ToolResult` contract.

## 1. Tool Schema (API Shape)

Tool name: `code_search`

Description for LLM registration:
> Search for text or regex patterns across files in the workspace. Returns matching lines with
> file paths, line numbers, and optional context. Respects .gitignore. Use 'path' and 'include'
> to narrow scope for faster results.

### Parameters JSON Schema

```json
{
  "type": "object",
  "properties": {
    "pattern": {
      "type": "string",
      "description": "The search pattern. Treated as a literal string by default; set 'is_regex' to true for regular expression matching. Max 1000 characters."
    },
    "path": {
      "type": "string",
      "description": "Subdirectory to scope the search, relative to workspace root. Defaults to workspace root if omitted. Must be a relative path."
    },
    "include": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Glob patterns for files to include (e.g., ['*.rs', '*.toml']). When omitted, all non-ignored files are searched."
    },
    "exclude": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Additional glob patterns to exclude beyond .gitignore rules (e.g., ['*.generated.rs', 'vendor/*'])."
    },
    "is_regex": {
      "type": "boolean",
      "description": "When true, 'pattern' is interpreted as a Rust regex (RE2-like semantics). When false (default), 'pattern' is matched as a literal string.",
      "default": false
    },
    "case_sensitive": {
      "type": "boolean",
      "description": "Whether the search is case-sensitive. Defaults to true.",
      "default": true
    },
    "max_results": {
      "type": "integer",
      "description": "Maximum number of matches to return. Defaults to 100, maximum 500. Results are truncated with a warning when the cap is hit.",
      "default": 100,
      "minimum": 1,
      "maximum": 500
    },
    "context_lines": {
      "type": "integer",
      "description": "Number of lines of context to include before and after each match. Defaults to 0, maximum 5.",
      "default": 0,
      "minimum": 0,
      "maximum": 5
    },
    "whole_word": {
      "type": "boolean",
      "description": "When true, only match whole words (the pattern is wrapped in word boundary anchors \\b). Defaults to false.",
      "default": false
    }
  },
  "required": [
    "pattern"
  ]
}
```

The schema is defined inline as `serde_json::json!({...})`, matching the convention in
`file_read.rs` and all other tools.

## 2. Regex Semantics (v1)

| Property           | Behavior                                                                                |
|--------------------|-----------------------------------------------------------------------------------------|
| Engine             | Rust `regex` crate v1.12.3 (RE2-like syntax)                                            |
| Unicode            | Enabled by default (`\w`, `\d`, etc. are Unicode-aware)                                 |
| Dot-newline        | `.` does NOT match `\n` (single-line mode is off)                                       |
| Literal mode       | When `is_regex: false`, the pattern is escaped via `regex::escape()` before compilation |
| Case insensitivity | When `case_sensitive: false`, the pattern is prefixed with `(?i)`                       |
| Whole word         | When `whole_word: true`, the pattern is wrapped with `\b...\b`                          |
| Max pattern length | 1000 characters (validated before compilation)                                          |
| Complexity limits  | Delegated to `regex` crate's built-in compile-time size and nesting limits              |

### Unsupported features (explicitly)

These features are NOT supported — the `regex` crate does not implement them:

- Backreferences (`\1`, `\2`, etc.)
- Lookahead / lookbehind (`(?=...)`, `(?!...)`, `(?<=...)`, `(?<!...)`)
- Possessive quantifiers (`x++`)
- Atomic groups (`(?>...)`)
- Conditional patterns (`(?(cond)yes|no)`)
- PCRE-specific syntax

If a pattern fails to compile, the tool returns `success: false` with the compilation error
message from the `regex` crate.

### Pattern construction order

```
1. Validate length ≤ 1000 chars
2. If !is_regex → regex::escape(pattern)
3. If !case_sensitive → prepend "(?i)"
4. If whole_word → wrap with "\b" + pattern + "\b"
5. regex::Regex::new(final_pattern)
```

## 3. Safety Model

### Workspace scoping

All paths are resolved relative to `SecurityPolicy::workspace_dir`. Absolute paths in the `path`
parameter are rejected outright. The same `is_path_allowed` → `join` → `canonicalize` →
`is_resolved_path_allowed` chain from `file_read` is applied to the `path` parameter at
invocation start.

### Symlink handling

Each matched file's path is resolved via `canonicalize()` and checked against
`is_resolved_path_allowed()`. Files whose resolved paths escape the workspace boundary are
silently skipped and logged at `tracing::debug!` level. This prevents symlink-based information
exfiltration while keeping search results clean.

### Binary file handling

Binary files are detected and skipped. Detection uses a two-layer approach:

1. **Primary**: The `ignore` crate's built-in binary detection (which checks for null bytes in a
   small prefix buffer).
2. **Fallback**: If using a manual file-reading path, check the first 8KB for null bytes.

Binary files never appear in results.

### Rate limiting

- A single `code_search` invocation counts as ONE action via `record_action()`.
- The search root follows the same ordering as `file_read`: `is_rate_limited()` →
  `is_path_allowed(path)` → `record_action()` → `canonicalize()` →
  `is_resolved_path_allowed()`.
- This ordering is intentional: `record_action()` happens after raw-path validation but before
  `canonicalize()` so pre-canonicalization rejections still consume budget and do not create a
  timing side channel for path probing.
- Individual file reads within the search do NOT increment the action counter.

### Resource limits

| Limit                            | Value                                | Rationale                                               |
|----------------------------------|--------------------------------------|---------------------------------------------------------|
| Max files scanned per invocation | 10,000                               | Prevents runaway walks on huge repos                    |
| Max file size scanned            | 10 MB                                | Matches `file_read` MAX_FILE_SIZE_BYTES                 |
| Max total output size            | 100 KB                               | Prevents oversized responses to LLM context             |
| Max matches per file             | 50                                   | Prevents single-file flooding                           |
| Max total matches returned       | 500 (configurable via `max_results`) | Hard cap                                                |
| Execution timeout                | 30 seconds                           | Prevents hanging on pathological patterns or huge trees |

When the 10,000-file scan limit is reached, the search stops and the response includes a
truncation warning.

### Path traversal

Same protections as `file_read`:

- Null byte (`\0`) in any path component → reject
- `..` component detection → reject via `is_path_allowed`
- URL-encoded traversal (`%2f`, `%2e`) → reject via `is_path_allowed`

### Autonomy mode

`code_search` is a read-only operation. It does NOT require `can_act()` — it works in
`ReadOnly` mode, same as `file_read`. Only `is_rate_limited()` / `record_action()` gates apply.

## 4. Fallback Behavior

| Scenario                                | Behavior                                                                                    |
|-----------------------------------------|---------------------------------------------------------------------------------------------|
| No index                                | v1 is brute-force walk + scan. No pre-built index.                                          |
| Scope too large (>10,000 files)         | Return partial results + truncation warning suggesting narrower `path` or `include` filters |
| No `.gitignore`                         | All non-hidden files are scanned (the `ignore` crate falls back gracefully)                 |
| `path` points to nonexistent directory  | Return `success: false`, error: `"Search path not found: {path}"`                           |
| `path` points to a file (not directory) | Return `success: false`, error: `"Search path is not a directory: {path}"`                  |
| Empty `pattern`                         | Return `success: false`, error: `"Pattern must not be empty"`                               |
| Pattern too long (>1000 chars)          | Return `success: false`, error: `"Pattern exceeds maximum length of 1000 characters"`       |
| Invalid regex                           | Return `success: false`, error: regex compilation error message                             |
| Hidden directories                      | `.git`, `.hg`, `.svn`, etc. are excluded by default via `ignore` crate                      |
| Permission denied on file               | Skip file, log at debug level, continue walking                                             |
| Zero matches                            | Return `success: true` with empty `matches` array and stats                                 |

## 5. Structured Result Format

### `structured` field (machine-readable)

```json
{
  "matches": [
    {
      "file": "src/tools/file_read.rs",
      "line": 42,
      "column": 12,
      "content": "    pub fn execute(...)",
      "context_before": [
        "    /// Execute the tool"
      ],
      "context_after": [
        "        let path = args..."
      ]
    }
  ],
  "stats": {
    "files_searched": 1234,
    "files_matched": 5,
    "total_matches": 23,
    "truncated": false,
    "duration_ms": 145
  }
}
```

**Field definitions:**

| Field                      | Type     | Description                                                |
|----------------------------|----------|------------------------------------------------------------|
| `matches[].file`           | string   | Workspace-relative file path                               |
| `matches[].line`           | integer  | 1-based line number                                        |
| `matches[].column`         | integer  | 1-based column (byte offset within line)                   |
| `matches[].content`        | string   | The full matched line (trimmed to 500 chars max)           |
| `matches[].context_before` | string[] | Lines before match (length = `context_lines`)              |
| `matches[].context_after`  | string[] | Lines after match (length = `context_lines`)               |
| `stats.files_searched`     | integer  | Total files visited                                        |
| `stats.files_matched`      | integer  | Files with at least one match                              |
| `stats.total_matches`      | integer  | Total match count (may exceed returned if truncated)       |
| `stats.truncated`          | boolean  | Whether results were capped by `max_results` or file limit |
| `stats.duration_ms`        | integer  | Wall-clock search time in milliseconds                     |

### `output` field (human-readable)

Grep-like format:

```
src/tools/file_read.rs:42:12:    pub fn execute(...)
src/tools/file_read.rs:87:5:    fn parameters_schema(...)
```

Summary line appended:

```
Found 23 matches in 5 files (1234 files searched, 145ms)
```

When truncated:

```
Results truncated at 100 matches. Narrow your search with 'path' or 'include' filters.
```

When context lines are requested, matches are separated by `--` (standard grep group separator):

```
src/tools/file_read.rs-41-    /// Execute the tool
src/tools/file_read.rs:42:12:    pub fn execute(...)
src/tools/file_read.rs-43-        let path = args...
--
```

## 6. Architecture Decisions

### Decision: Use `ignore` crate for directory walking

**Choice**: Add the `ignore` crate (~15KB addition) for `.gitignore`-aware walking.
**Alternatives considered**:

- `walkdir` + manual `.gitignore` parsing — more code, error-prone, misses nested `.gitignore`
  files and `.git/info/exclude`.
- `glob` crate (already in deps) — no `.gitignore` support, no parallel walking.
- Shell out to `rg` or `grep` — breaks sandboxing model, non-portable, external dependency.
  **Rationale**: `ignore` is the same walker used by `ripgrep`. It handles `.gitignore`,
  `.git/info/exclude`, global gitignore, and nested override files correctly. It supports parallel
  directory traversal. Minimal dependency footprint. Maintained by the BurntSushi ecosystem
  (same author as the `regex` crate already in use).

### Decision: No search index in v1

**Choice**: Brute-force walk + regex scan on every invocation.
**Alternatives considered**:

- n-gram index with persistent storage — significant complexity, storage overhead, stale index
  risk, build-time cost.
- In-memory trie/suffix array — high memory usage, cold-start penalty.
  **Rationale**: Most agent workspaces are small-to-medium repos (<10K files). A brute-force walk
  with the `ignore` crate's parallel traversal completes in <500ms for typical workspaces.
  Indexing adds complexity that isn't justified until we see real latency problems. The 10K file
  limit and 30s timeout bound the worst case. v2 can add indexing transparently (same API
  contract, faster backend).

### Decision: Single action per search for rate limiting

**Choice**: One `record_action()` call per `code_search` invocation.
**Alternatives considered**:

- Count per file scanned — would exhaust rate limit budget extremely fast, making the tool
  unusable.
- Count per match returned — unpredictable budget consumption.
  **Rationale**: From the agent's perspective, a search is one logical action. The security
  boundary is "how many operations can the agent trigger per hour," not "how many files were
  touched internally." This matches `file_read` (one action per read, regardless of file size).

### Decision: Max line content at 500 chars

**Choice**: Truncate `content` field at 500 characters per matched line.
**Alternatives considered**:

- No truncation — risk of minified files producing massive single-line matches that blow up
  output.
- Smaller cap (200 chars) — loses context for long lines in legitimate code.
  **Rationale**: 500 chars covers the vast majority of code lines while preventing minified
  JS/CSS from dominating output. Column offset lets consumers locate the match precisely if the
  full line is needed (via `file_read`).

### Decision: `code_search` as the tool name (not `grep` or `search`)

**Choice**: `code_search`
**Alternatives considered**:

- `grep` — too Unix-specific, implies exact grep semantics we don't fully replicate.
- `search` — too generic, could conflict with web search or memory search.
- `file_search` — ambiguous (search for files vs search in files).
  **Rationale**: `code_search` clearly communicates "search code content" and follows the
  `snake_case` naming convention of existing tools (`file_read`, `file_write`, `web_search_tool`).

## Data Flow

```
LLM function call
       │
       ▼
CodeSearchTool::execute(args)
       │
       ├─ 1. Validate params (pattern, path, limits)
       ├─ 2. is_rate_limited() → is_path_allowed(path) → record_action()
       ├─ 3. canonicalize → is_resolved_path_allowed
       │
       ▼
 Build ignore::WalkBuilder
       │
       ├─ root = workspace_dir.join(path)
       ├─ apply include/exclude globs
       ├─ .gitignore rules loaded automatically
       │
       ▼
 Walk files (parallel via ignore crate)
       │
       ├─ Per file:
       │    ├─ Skip if binary
       │    ├─ Skip if > 10MB
       │    ├─ canonicalize → is_resolved_path_allowed (symlink check)
       │    ├─ Read file content
       │    ├─ regex.find_iter() on each line
       │    ├─ Collect matches (up to 50 per file)
       │    └─ Stop if total matches ≥ max_results or files ≥ 10,000
       │
       ▼
 Build ToolResult
       ├─ output: grep-like text + summary
       ├─ structured: JSON with matches[] and stats{}
       └─ success: true (or false on error)
```

## File Changes

| File                              | Action | Description                                                                                                |
|-----------------------------------|--------|------------------------------------------------------------------------------------------------------------|
| `src/tools/code_search.rs`        | Create | New tool implementation: `CodeSearchTool` struct, `Tool` trait impl, search logic, unit tests              |
| `src/tools/mod.rs`                | Modify | Add `pub mod code_search;`, `pub use code_search::CodeSearchTool;`, register in `all_tools_with_runtime()` |
| `Cargo.toml`                      | Modify | Add `ignore = "0.4"` to `[dependencies]`                                                                   |
| `docs/design/code-search-tool.md` | Create | This design document                                                                                       |

## Interfaces / Contracts

### Struct

```rust
pub struct CodeSearchTool {
    security: Arc<SecurityPolicy>,
}

impl CodeSearchTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}
```

### Tool trait implementation

```rust
#[async_trait]
impl Tool for CodeSearchTool {
    fn name(&self) -> &str { "code_search" }

    fn description(&self) -> &str {
        "Search for text or regex patterns across files in the workspace. \
         Returns matching lines with file paths, line numbers, and optional context. \
         Respects .gitignore. Use 'path' and 'include' to narrow scope for faster results."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        // JSON Schema as defined in Section 1
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // Implementation following the data flow in this design
    }
}
```

### Registration in `mod.rs`

```rust
// In all_tools_with_runtime():
Box::new(CodeSearchTool::new(security.clone())),
```

The tool is always registered (no feature flag, no config toggle) — it's a core read-only
capability like `file_read`.

## Testing Strategy

| Layer | What to Test             | Approach                                                |
|-------|--------------------------|---------------------------------------------------------|
| Unit  | Literal pattern matching | Search temp dir with known files, verify exact matches  |
| Unit  | Regex pattern matching   | Patterns with `\d+`, `\w+`, character classes           |
| Unit  | Case insensitive search  | Same content, verify match with `case_sensitive: false` |
| Unit  | Whole word matching      | Verify `foo` matches word but not `foobar`              |
| Unit  | Context lines            | Verify `context_before` / `context_after` correctness   |
| Unit  | Include/exclude globs    | Verify file filtering by extension                      |
| Unit  | Path scoping             | Subdirectory restriction                                |
| Unit  | Binary file skipping     | Create file with null bytes, verify it's skipped        |
| Unit  | File size limit          | Create >10MB file, verify it's skipped                  |
| Unit  | Empty pattern            | Returns error                                           |
| Unit  | Pattern too long         | Returns error                                           |
| Unit  | Invalid regex            | Returns error with compilation message                  |
| Unit  | Nonexistent path         | Returns `success: false`                                |
| Unit  | Path traversal blocked   | `../` and absolute paths rejected                       |
| Unit  | Symlink escape blocked   | Symlink to outside workspace is skipped                 |
| Unit  | Rate limiting            | Same pattern as `file_read` rate limit tests            |
| Unit  | Max results truncation   | Generate >100 matches, verify cap and `truncated: true` |
| Unit  | Max per-file cap         | File with >50 matches, verify cap at 50                 |
| Unit  | Structured output shape  | Validate JSON schema of `structured` field              |
| Unit  | ReadOnly mode works      | Verify search works under `AutonomyLevel::ReadOnly`     |
| Unit  | .gitignore respect       | Create `.gitignore`, verify ignored files are skipped   |
| Unit  | Zero matches             | Returns `success: true` with empty matches array        |

All unit tests use `#[cfg(test)]` in `code_search.rs`, following the `file_read.rs` pattern
with temp directories and `test_security()` / `test_security_with()` helpers.

## 7. Freshness Strategy

v1 has no index, no cache, and no in-memory result store. Every `code_search` invocation walks
the workspace directory from scratch and reads each file from disk at the moment of execution.

### Guarantee: reads reflect the latest writes

Because there is no intermediate data store that could become stale, the agent can always rely
on the following ordering:

1. `file_write` completes and flushes the file to disk.
2. The next `code_search` invocation opens that same file from the OS filesystem.
3. The match result reflects the content written in step 1.

This guarantee is scoped to files that the subsequent `code_search` is allowed to scan (i.e.,
within the invoked `path` and `include` filters, and not excluded by `exclude` patterns,
`.gitignore` rules, binary detection, or resource limits). Binary detection, ignore rules, and
resource limits can prevent the search from seeing the fresh write even under v1's "always read
from disk" model. No warm-up, index rebuild, or explicit invalidation step is needed between a
write and a subsequent search for files within the search scope.

### Implications for agent workflows

- An agent that writes a file and immediately searches for a symbol it just added **will find
  it** if the file falls within the `code_search` scope (matching `path`/`include` filters, not
  ignored, not detected as binary, and within size limits) — there is no propagation delay for
  eligible files.
- Concurrent writes from other processes may or may not be visible depending on OS buffering,
  but this is outside the scope of the agent's execution model (agents are single-threaded in
  their tool-call loop).
- The 30-second execution timeout is a per-invocation bound, not a freshness window.

### Why v2 requires an explicit freshness strategy

If a future version adds a persistent trigram index (v2+), the index will become a second source
of truth that can diverge from the filesystem. That version must define:

- **Write-through**: every `file_write` call triggers an index update for the affected file.
- **Invalidation horizon**: maximum age a cached index entry may have before re-reading the file.
- **Rebuild trigger**: conditions under which the full index is discarded and rebuilt.

Until then, v1's "always read from disk" model is the simplest possible freshness guarantee.

## Migration / Rollout

No migration required. The tool is additive:

- New file (`code_search.rs`) with no changes to existing tool behavior.
- Registration in `all_tools_with_runtime()` adds it to all agent instances automatically.
- No config changes needed — the tool is always available.
- One new dependency (`ignore`) with no transitive conflicts.
- Rollback: revert the 3 file changes (remove module, remove registration, remove dependency).

## v1 vs Future Scope

### v1 (this design)

- Brute-force directory walk + regex/literal scan
- `.gitignore`-aware via `ignore` crate
- Structured results with context lines
- All safety constraints defined above
- Single-line matching only (pattern matches within one line)

### v2+ (future — explicitly NOT in v1)

- Sparse n-gram index for sub-100ms searches on large repos
- Probabilistic bloom/mask filters for fast rejection
- `mmap`-based file reading for reduced memory pressure
- Incremental index updates on file watch events
- Multi-line pattern matching (spanning line boundaries)
- Search history / caching layer
- AST-aware search (search by symbol kind: function, class, etc.)
- Configurable max-file-scan limit via agent config

## Alternatives Considered

### 1. Shell out to `ripgrep`

**Rejected**. Breaks the sandbox model — `code_search` would bypass `SecurityPolicy` path
checks by delegating to an external binary. Also creates a hard dependency on `rg` being
installed, which isn't guaranteed on all deployment targets.

### 2. Reuse the existing `shell` tool with a grep command

**Rejected**. The `shell` tool requires `can_act()` (Supervised or Full autonomy), so it
wouldn't work in ReadOnly mode. It also produces unstructured text output, making the LLM parse
unreliable. A native tool provides structured results, consistent security guarantees, and works
at all autonomy levels.

### 3. Use `grep` crate instead of `regex` + `ignore`

**Rejected**. The `grep` family of crates (`grep-regex`, `grep-searcher`, `grep-matcher`) is
the full ripgrep engine decomposed into libraries. While powerful, it's a significantly larger
dependency surface. The `regex` + `ignore` combination gives us what we need with smaller
footprint and simpler code.

### 4. Add search to `file_read` as a mode

**Rejected**. `file_read` has a single clear responsibility (read one file). Adding search
would bloat its schema and confuse LLM tool selection. Separate tools with focused schemas
produce better LLM function-calling accuracy.

## Resolved Questions

- [x] **`**` recursive globs in `include`/`exclude`**: Yes — supported in v1. The `ignore` crate
  handles `**` natively, so patterns like `src/**/*.rs` and `**/test_*.py` work out of the box.
- [x] **Custom `.searchignore` file**: Deferred to v2. `.gitignore` coverage is sufficient for v1.