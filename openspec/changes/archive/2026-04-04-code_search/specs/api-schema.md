# code_search Tool API Contract Specification

## Purpose

Defines the tool registration contract for the `code_search` tool: its name, description,
parameter schema, types, defaults, constraints, and required/optional semantics.

## Requirements

### REQ-API-001: Tool Identity

The tool MUST be registered with the name `code_search`.

The tool MUST expose the following description for LLM registration:
> Search for text or regex patterns across files in the workspace. Returns matching lines with
> file paths, line numbers, and optional context. Respects .gitignore. Use 'path' and 'include'
> to narrow scope for faster results.

#### Scenario: Tool is discoverable by name

- GIVEN an agent runtime with all tools registered
- WHEN the tool registry is queried for `code_search`
- THEN the tool MUST be present with the correct name and description

### REQ-API-002: Parameter Schema — `pattern` (required)

The tool MUST accept a `pattern` parameter of type `string`.

`pattern` MUST be the only required parameter. All other parameters MUST be optional.

`pattern` MUST NOT be empty. The tool MUST return an error if an empty string is provided.

`pattern` MUST NOT exceed 1000 characters. The tool MUST return an error if the limit is exceeded.

#### Scenario: Valid literal search returns matches

- GIVEN a workspace containing a file `src/main.rs` with the line `fn main() {`
- WHEN `code_search` is invoked with `{ "pattern": "fn main" }`
- THEN the result MUST have `success: true`
- AND the result MUST contain at least one match in `src/main.rs`

#### Scenario: Valid regex search returns matches

- GIVEN a workspace containing a file with lines `let x = 42;` and `let y = 99;`
- WHEN `code_search` is invoked with `{ "pattern": "let \\w+ = \\d+", "is_regex": true }`
- THEN the result MUST have `success: true`
- AND the result MUST contain matches for both lines

#### Scenario: Empty pattern returns error

- GIVEN any workspace
- WHEN `code_search` is invoked with `{ "pattern": "" }`
- THEN the result MUST have `success: false`
- AND the error message MUST indicate that the pattern must not be empty

#### Scenario: Pattern exceeding 1000 characters returns error

- GIVEN any workspace
- WHEN `code_search` is invoked with a `pattern` of 1001 characters
- THEN the result MUST have `success: false`
- AND the error message MUST indicate that the pattern exceeds the maximum length of 1000 characters

#### Scenario: Invalid regex returns error with compilation message

- GIVEN any workspace
- WHEN `code_search` is invoked with `{ "pattern": "[invalid(", "is_regex": true }`
- THEN the result MUST have `success: false`
- AND the error message MUST contain the regex compilation error from the `regex` crate

### REQ-API-003: Parameter Schema — `path` (optional)

The tool MUST accept an optional `path` parameter of type `string`.

`path` MUST be interpreted as a subdirectory relative to the workspace root.

When `path` is omitted, the search MUST default to the workspace root.

`path` MUST be a relative path. Absolute paths MUST be rejected.

#### Scenario: Scoped search respects path parameter

- GIVEN a workspace with files in `src/` and `tests/`
- WHEN `code_search` is invoked with `{ "pattern": "fn", "path": "src" }`
- THEN matches MUST only include files under `src/`
- AND files under `tests/` MUST NOT appear in results

#### Scenario: Nonexistent path returns error

- GIVEN a workspace with no `nonexistent/` directory
- WHEN `code_search` is invoked with `{ "pattern": "foo", "path": "nonexistent" }`
- THEN the result MUST have `success: false`
- AND the error message MUST indicate the search path was not found

#### Scenario: Path pointing to a file returns error

- GIVEN a workspace with a file `src/main.rs`
- WHEN `code_search` is invoked with `{ "pattern": "foo", "path": "src/main.rs" }`
- THEN the result MUST have `success: false`
- AND the error message MUST indicate the search path is not a directory

### REQ-API-004: Parameter Schema — `include` (optional)

The tool MUST accept an optional `include` parameter of type `array` of `string`.

Each element MUST be a glob pattern for files to include (e.g., `["*.rs", "*.toml"]`).

When `include` is omitted, all non-ignored files MUST be searched.

The `include` parameter MUST support `**` recursive globs.

#### Scenario: Include filter restricts file types

- GIVEN a workspace with `main.rs` and `main.py` both containing `hello`
- WHEN `code_search` is invoked with `{ "pattern": "hello", "include": ["*.rs"] }`
- THEN matches MUST only include `main.rs`
- AND `main.py` MUST NOT appear in results

### REQ-API-005: Parameter Schema — `exclude` (optional)

The tool MUST accept an optional `exclude` parameter of type `array` of `string`.

Each element MUST be a glob pattern for additional files to exclude beyond `.gitignore` rules.

The `exclude` parameter MUST support `**` recursive globs.

#### Scenario: Exclude filter removes matching files

- GIVEN a workspace with `src/app.rs` and `src/app.generated.rs` both containing `struct`
- WHEN `code_search` is invoked with `{ "pattern": "struct", "exclude": ["*.generated.rs"] }`
- THEN matches MUST include `src/app.rs`
- AND `src/app.generated.rs` MUST NOT appear in results

### REQ-API-006: Parameter Schema — `is_regex` (optional, default false)

The tool MUST accept an optional `is_regex` parameter of type `boolean`, defaulting to `false`.

When `false`, the `pattern` MUST be treated as a literal string.

When `true`, the `pattern` MUST be interpreted as a Rust regex with RE2-like semantics.

#### Scenario: Default literal mode does not interpret regex metacharacters

- GIVEN a workspace containing a file with the line `vec[0]`
- WHEN `code_search` is invoked with `{ "pattern": "vec[0]" }`
- THEN the result MUST match the literal text `vec[0]`
- AND the `[0]` MUST NOT be interpreted as a character class

### REQ-API-007: Parameter Schema — `case_sensitive` (optional, default true)

The tool MUST accept an optional `case_sensitive` parameter of type `boolean`, defaulting to `true`.

When `true`, the search MUST be case-sensitive.

When `false`, the search MUST match regardless of case.

#### Scenario: Case insensitive search matches mixed case

- GIVEN a workspace containing a file with the line `Hello World`
- WHEN `code_search` is invoked with `{ "pattern": "hello world", "case_sensitive": false }`
- THEN the result MUST contain a match for that line

### REQ-API-008: Parameter Schema — `max_results` (optional, default 100)

The tool MUST accept an optional `max_results` parameter of type `integer`, defaulting to `100`.

The minimum value MUST be `1`. The maximum value MUST be `500`.

When the number of matches exceeds `max_results`, results MUST be truncated with a warning.

#### Scenario: Results truncated at max_results

- GIVEN a workspace with files containing more than 5 matches for a pattern
- WHEN `code_search` is invoked with `{ "pattern": "match_me", "max_results": 5 }`
- THEN the result MUST contain at most 5 matches
- AND the result MUST include a truncation warning

### REQ-API-009: Parameter Schema — `context_lines` (optional, default 0)

The tool MUST accept an optional `context_lines` parameter of type `integer`, defaulting to `0`.

The minimum value MUST be `0`. The maximum value MUST be `5`.

When greater than 0, the result MUST include the specified number of lines before and after
each match.

#### Scenario: Context lines included in results

- GIVEN a workspace file with lines `[A, B, C, D, E]` where `C` matches the pattern
- WHEN `code_search` is invoked with `{ "pattern": "C", "context_lines": 1 }`
- THEN the match MUST include `context_before: ["B"]` and `context_after: ["D"]`

### REQ-API-010: Parameter Schema — `whole_word` (optional, default false)

The tool MUST accept an optional `whole_word` parameter of type `boolean`, defaulting to `false`.

When `true`, the pattern MUST only match whole words (wrapped in word boundary anchors `\b`).

#### Scenario: Whole word search does not match substrings

- GIVEN a workspace containing a file with the lines `foo` and `foobar`
- WHEN `code_search` is invoked with `{ "pattern": "foo", "whole_word": true }`
- THEN the result MUST match the line containing `foo`
- AND the result MUST NOT match the line containing `foobar`
