# code_search Safety Model Specification

## Purpose

Defines the security constraints, workspace scoping, symlink handling, binary file behavior,
rate limiting, autonomy mode support, and resource limits for the `code_search` tool.

## Requirements

### REQ-SAFE-001: Workspace-Scoped Search

All search paths MUST be resolved relative to `SecurityPolicy::workspace_dir`.

Absolute paths in the `path` parameter MUST be rejected outright with an error.

Path traversal attempts (e.g., `../`) MUST be rejected via the same `is_path_allowed` check
used by `file_read`.

Null bytes (`\0`) in any path component MUST be rejected.

URL-encoded traversal sequences (`%2f`, `%2e`) MUST be rejected via `is_path_allowed`.

#### Scenario: Path traversal attempt is rejected

- GIVEN a workspace at `/workspace`
- WHEN `code_search` is invoked with `{ "pattern": "secret", "path": "../etc" }`
- THEN the result MUST have `success: false`
- AND the error MUST indicate the path is not allowed

#### Scenario: Absolute path is rejected

- GIVEN a workspace at `/workspace`
- WHEN `code_search` is invoked with `{ "pattern": "secret", "path": "/etc/passwd" }`
- THEN the result MUST have `success: false`
- AND the error MUST indicate the path is not allowed

### REQ-SAFE-002: Security Chain Parity with file_read

The `path` parameter MUST be validated using the same security chain as `file_read`:

1. `is_path_allowed(path)` — validate the raw relative path
2. `record_action(...)` — consume budget after the raw-path check and before path resolution
3. `workspace_dir.join(path)` — resolve to absolute
4. `canonicalize()` — resolve symlinks and normalize
5. `is_resolved_path_allowed(...)` — validate the resolved absolute path

This chain MUST be applied to the search root path at invocation start, matching the placement
of `record_action()` in `file_read.rs` so pre-canonicalization rejections still consume budget.

#### Scenario: Security chain is applied to search root

- GIVEN a workspace with a valid `src/` directory
- WHEN `code_search` is invoked with `{ "pattern": "fn", "path": "src" }`
- THEN the path MUST pass through `is_path_allowed`, `record_action`, `canonicalize`, and
  `is_resolved_path_allowed`
- AND if any check fails, the result MUST have `success: false`

### REQ-SAFE-003: Symlink Escape Prevention

Each matched file's path MUST be resolved via `canonicalize()` and checked against
`is_resolved_path_allowed()`.

Files whose resolved paths escape the workspace boundary MUST be silently skipped.

Symlink escapes SHOULD be logged at `debug` level but MUST NOT appear in results.

#### Scenario: Symlink to outside workspace is skipped

- GIVEN a workspace containing a symlink `link.rs` pointing to `/outside/secret.rs`
- WHEN `code_search` is invoked with `{ "pattern": "secret" }`
- THEN the symlinked file MUST NOT appear in the results
- AND the search MUST complete with `success: true`

### REQ-SAFE-004: Binary File Skipping

Binary files MUST be detected and skipped. They MUST NOT appear in results.

Binary detection MUST use the `ignore` crate's built-in null-byte detection as the primary
mechanism.

As a fallback, if a manual file-reading path is used, the first 8KB MUST be checked for null
bytes.

#### Scenario: Binary file is skipped

- GIVEN a workspace containing a binary file `image.png` and a text file `readme.md`
- WHEN `code_search` is invoked with a pattern that would match content in both files
- THEN only `readme.md` MUST appear in results
- AND `image.png` MUST NOT appear in results

### REQ-SAFE-005: Rate Limiting

A single `code_search` invocation MUST count as ONE action via `record_action()`.

The `is_rate_limited()` check MUST run before any file I/O.

Individual file reads within the search MUST NOT increment the action counter.

#### Scenario: Rate limited invocation is rejected

- GIVEN an agent that has exhausted its rate limit budget
- WHEN `code_search` is invoked with any valid parameters
- THEN the result MUST have `success: false`
- AND the error MUST indicate rate limiting

#### Scenario: Single invocation counts as one action

- GIVEN an agent with rate limit budget remaining
- WHEN `code_search` is invoked and scans 500 files
- THEN only ONE action MUST be recorded via `record_action()`
- AND the rate limit budget MUST decrease by exactly 1

### REQ-SAFE-006: ReadOnly Autonomy Mode Support

`code_search` MUST be a read-only operation.

It MUST NOT require `can_act()` — it MUST work in `ReadOnly` autonomy mode.

Only `is_rate_limited()` / `record_action()` gates MUST apply.

#### Scenario: Search works in ReadOnly mode

- GIVEN an agent running in `AutonomyLevel::ReadOnly`
- WHEN `code_search` is invoked with valid parameters
- THEN the search MUST execute successfully
- AND the result MUST have `success: true` (assuming matches exist)

### REQ-SAFE-007: Resource Limits

The tool MUST enforce the following resource limits:

| Limit                            | Value                            |
|----------------------------------|----------------------------------|
| Max files scanned per invocation | 10,000                           |
| Max file size scanned            | 10 MB                            |
| Max total output size            | 100 KB                           |
| Max matches per file             | 50                               |
| Max total matches returned       | 500 (via `max_results` hard cap) |
| Execution timeout                | 30 seconds                       |

Files exceeding 10 MB MUST be silently skipped.

When the 10,000-file scan limit is reached, the search MUST stop and the response MUST include
a truncation warning suggesting narrower `path` or `include` filters.

When the execution timeout (30s) is reached, the tool MUST return partial results collected
so far with a timeout warning.

#### Scenario: Search exceeding 10K files returns truncation warning

- GIVEN a workspace containing more than 10,000 scannable files
- WHEN `code_search` is invoked with `{ "pattern": "import" }`
- THEN the search MUST stop after scanning 10,000 files
- AND the result MUST include a truncation warning
- AND the `stats.truncated` field MUST be `true`
- AND the warning MUST suggest narrowing scope with `path` or `include` filters

#### Scenario: File exceeding 10MB is skipped

- GIVEN a workspace containing a 15MB text file and a 1KB text file, both containing the pattern
- WHEN `code_search` is invoked with a matching pattern
- THEN only the 1KB file MUST appear in results
- AND the 15MB file MUST be silently skipped

#### Scenario: Per-file match cap at 50

- GIVEN a workspace file with 100 lines all matching the pattern
- WHEN `code_search` is invoked with a matching pattern
- THEN at most 50 matches MUST be returned from that single file

### REQ-SAFE-008: .gitignore Respect

The tool MUST respect `.gitignore` rules when walking directories.

Files matched by `.gitignore`, `.git/info/exclude`, global gitignore, and nested override files
MUST be excluded from the search.

Hidden directories (`.git`, `.hg`, `.svn`, etc.) MUST be excluded by default.

#### Scenario: Gitignored files are excluded

- GIVEN a workspace with a `.gitignore` containing `target/`
- AND a file `target/debug/output.rs` containing the search pattern
- WHEN `code_search` is invoked with a matching pattern
- THEN `target/debug/output.rs` MUST NOT appear in results

### REQ-SAFE-009: Permission Denied Handling

Files that cannot be read due to permission errors MUST be silently skipped.

Permission errors SHOULD be logged at `debug` level but MUST NOT cause the search to fail.

#### Scenario: Unreadable file is skipped gracefully

- GIVEN a workspace file with no read permission
- WHEN `code_search` is invoked with a pattern
- THEN the unreadable file MUST be skipped
- AND the search MUST complete with `success: true`
