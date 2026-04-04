# Tasks: Native `code_search` Tool

## Phase 1: Foundation

- [x] 1.1 Add `ignore = "0.4"` to `[dependencies]` in `clients/agent-runtime/Cargo.toml`
- [x] 1.2 Create `src/tools/code_search.rs` with `CodeSearchTool` struct holding
  `security: Arc<SecurityPolicy>` and `pub fn new(security: Arc<SecurityPolicy>) -> Self`
- [x] 1.3 Add `pub mod code_search;` and `pub use code_search::CodeSearchTool;` in
  `src/tools/mod.rs` (alphabetical order)
- [x] 1.4 Implement `Tool` trait skeleton: `name()` → `"code_search"`, `description()` per design,
  `parameters_schema()` with all 9 params, `execute()` returning a temporary stub
- [x] 1.5 Register `Box::new(CodeSearchTool::new(security.clone()))` in `all_tools_with_runtime()`
  alongside `FileReadTool`
- [x] 1.6 Verify: `cargo check --manifest-path clients/agent-runtime/Cargo.toml`

## Phase 2: Parameter Validation (TDD)

- [x] 2.1 RED: Write failing tests — empty pattern error, pattern >1000 chars error, invalid regex
  error (`[invalid(`), nonexistent path error, absolute path rejected, path-is-file error
- [x] 2.2 GREEN: Implement param parsing in `execute()` — extract `pattern`/`path`/`is_regex`/
  `case_sensitive`/`max_results`/`context_lines`/`whole_word`/`include`/`exclude` with defaults per
  schema
- [x] 2.3 GREEN: Add validation — empty check, length ≤1000, regex compilation test, path relativity
  check, path existence + is_dir check
- [x] 2.4 Verify: `cargo test --manifest-path clients/agent-runtime/Cargo.toml code_search`

## Phase 3: Pattern Construction (TDD)

- [x] 3.1 RED: Write failing tests — literal `vec[0]` escaped correctly, `(?i)` prepended when
  `case_sensitive: false`, `\b...\b` wrapping when `whole_word: true`, combined literal+case+word
  order
- [x] 3.2 GREEN: Implement pipeline — validate → `regex::escape()` if literal → prepend `(?i)` if
  case insensitive → wrap `\b` if whole_word → `Regex::new()`
- [x] 3.3 Verify: `cargo test --manifest-path clients/agent-runtime/Cargo.toml code_search`

## Phase 4: Search Engine (TDD)

- [x] 4.1 RED: Write failing tests — literal match found, regex match found, include glob `["*.rs"]`
  filters, exclude glob filters, `.gitignore` respected, binary file skipped, >10MB file skipped,
  symlink escape skipped, subdirectory path scoping
- [x] 4.2 GREEN: Build `ignore::WalkBuilder` rooted at `workspace_dir.join(path)` with
  include/exclude globs
- [x] 4.3 GREEN: Implement per-file scanning — read lines, `regex.find()` per line, collect
  `(file, line, column, content)` with 50-per-file cap
- [x] 4.4 GREEN: Add security checks per file — `canonicalize()` + `is_resolved_path_allowed()`,
  skip binary, skip >10MB
- [x] 4.5 Verify: `cargo test --manifest-path clients/agent-runtime/Cargo.toml code_search`

## Phase 5: Result Formatting (TDD)

- [x] 5.1 RED: Write failing tests — grep-like `file:line:col: content` output, structured JSON
  shape (`matches[]` + `stats{}`), context lines with `--` separator, content truncated at 500
  chars, summary line format, zero matches returns `success: true` with empty array
- [x] 5.2 GREEN: Build `ToolResult` with `output` (grep lines + summary) and `structured` (JSON with
  matches/stats including `duration_ms`)
- [x] 5.3 GREEN: Add context line collection (`context_before`/`context_after`) and `--` group
  separators in output
- [x] 5.4 Verify: `cargo test --manifest-path clients/agent-runtime/Cargo.toml code_search`

## Phase 6: Resource Limits (TDD)

- [x] 6.1 RED: Write failing tests — `max_results` cap with `truncated: true` + warning, per-file
  50-match cap, 10K file scan cap, rate limiting rejected, ReadOnly mode works
- [x] 6.2 GREEN: Enforce limits in search loop — total match cap, file scan counter at 10K,
  truncation warning in output
- [x] 6.3 GREEN: Add `is_rate_limited()` check before I/O and `record_action()` after — single
  action per invocation
- [x] 6.4 Verify: `cargo test --manifest-path clients/agent-runtime/Cargo.toml code_search`

## Phase 7: Integration & Validation

- [ ] 7.1 Run `cargo fmt --all -- --check` in `clients/agent-runtime`
- [ ] 7.2 Run `cargo clippy --all-targets -- -D warnings` in `clients/agent-runtime`
- [x] 7.3 Run `cargo test` full suite in `clients/agent-runtime`
- [x] 7.4 Verify `code_search` appears in tool name list via existing `all_tools` test pattern
