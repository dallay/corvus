# code_search Result Format Specification

## Purpose

Defines the output contract for the `code_search` tool, including the human-readable `output`
field, the machine-readable `structured` JSON payload, match object shape, stats object shape,
content truncation, summary lines, and truncation warnings.

## Requirements

### REQ-RESULT-001: Grep-like Output Field

The `output` field MUST contain a human-readable grep-like format with one match per line:

```
file:line:column: content
```

Where:

- `file` is the workspace-relative file path
- `line` is the 1-based line number
- `column` is the 1-based column (byte offset within the line)
- `content` is the matched line content

#### Scenario: Output field contains grep-like format

- GIVEN a workspace file `src/main.rs` with `fn main()` on line 5, column 1
- WHEN `code_search` is invoked with `{ "pattern": "fn main" }`
- THEN the `output` field MUST contain a line matching `src/main.rs:5:1: fn main()`

### REQ-RESULT-002: Structured JSON Field

The `structured` field MUST contain a JSON object with two top-level keys:

- `matches` — an array of match objects
- `stats` — a statistics object

#### Scenario: Structured field has correct top-level shape

- GIVEN a workspace with files matching the search pattern
- WHEN `code_search` is invoked with valid parameters
- THEN the `structured` field MUST be a JSON object
- AND it MUST contain a `matches` key with an array value
- AND it MUST contain a `stats` key with an object value

### REQ-RESULT-003: Match Object Schema

Each element in the `matches` array MUST be an object with the following fields:

| Field            | Type     | Description                                             |
|------------------|----------|---------------------------------------------------------|
| `file`           | string   | Workspace-relative file path                            |
| `line`           | integer  | 1-based line number                                     |
| `column`         | integer  | 1-based column (byte offset within line)                |
| `content`        | string   | The full matched line (truncated to 500 chars max)      |
| `context_before` | string[] | Lines before match (length = `context_lines` parameter) |
| `context_after`  | string[] | Lines after match (length = `context_lines` parameter)  |

#### Scenario: Match object contains all required fields

- GIVEN a workspace file with a matching line
- WHEN `code_search` is invoked with valid parameters
- THEN each match object MUST contain `file`, `line`, `column`, `content`, `context_before`, and
  `context_after`
- AND `line` MUST be a positive integer (1-based)
- AND `column` MUST be a positive integer (1-based)

### REQ-RESULT-004: Stats Object Schema

The `stats` object MUST contain the following fields:

| Field            | Type    | Description                                                |
|------------------|---------|------------------------------------------------------------|
| `files_searched` | integer | Total number of files visited during the walk              |
| `files_matched`  | integer | Number of files with at least one match                    |
| `total_matches`  | integer | Total match count (may exceed returned count if truncated) |
| `truncated`      | boolean | Whether results were capped by `max_results` or file limit |
| `duration_ms`    | integer | Wall-clock search time in milliseconds                     |

#### Scenario: Stats reflect actual search metrics

- GIVEN a workspace with 100 files, 3 of which contain matches totaling 7 hits
- WHEN `code_search` is invoked with a matching pattern
- THEN `stats.files_searched` MUST be approximately 100
- AND `stats.files_matched` MUST be 3
- AND `stats.total_matches` MUST be 7
- AND `stats.truncated` MUST be `false`
- AND `stats.duration_ms` MUST be a non-negative integer

### REQ-RESULT-005: Content Line Truncation

The `content` field in each match object MUST be truncated at 500 characters.

Lines exceeding 500 characters MUST be cut at the 500-character boundary.

This MUST prevent minified files from producing oversized match content.

#### Scenario: Long line is truncated at 500 characters

- GIVEN a workspace file with a single line of 1000 characters that matches the pattern
- WHEN `code_search` is invoked with a matching pattern
- THEN the `content` field MUST contain at most 500 characters

### REQ-RESULT-006: Summary Line in Output

The `output` field MUST append a summary line after all matches:

```
Found {total_matches} matches in {files_matched} files ({files_searched} files searched, {duration_ms}ms)
```

#### Scenario: Summary line is appended to output

- GIVEN a workspace where the search finds 5 matches in 2 files out of 50 searched
- WHEN `code_search` is invoked with a matching pattern
- THEN the last line of `output` MUST be a summary matching the format:
  `Found 5 matches in 2 files (50 files searched, {N}ms)`

### REQ-RESULT-007: Truncation Warning

When results are truncated (by `max_results` or the 10K file scan limit), the `output` field
MUST include a truncation warning line:

```
Results truncated at {N} matches. Narrow your search with 'path' or 'include' filters.
```

The `stats.truncated` field MUST be `true` when any truncation occurs.

#### Scenario: Truncated results include warning and truncated flag

- GIVEN a workspace with more than 10 matches for a pattern
- WHEN `code_search` is invoked with `{ "pattern": "common", "max_results": 10 }`
- THEN the `output` field MUST contain a truncation warning line
- AND `stats.truncated` MUST be `true`
- AND the `matches` array MUST contain at most 10 elements

### REQ-RESULT-008: Context Lines in Output

When `context_lines` is greater than 0, matches in the `output` field MUST include context
lines and be separated by `--` (standard grep group separator).

Context lines MUST use the format `file-linenum- content` (dash instead of colon).

Match lines MUST use the format `file:linenum:column: content` (colon).

#### Scenario: Search with context_lines=2 returns before/after context

- GIVEN a workspace file with 10 lines where line 5 matches the pattern
- WHEN `code_search` is invoked with `{ "pattern": "target", "context_lines": 2 }`
- THEN the match MUST include `context_before` with lines 3 and 4
- AND the match MUST include `context_after` with lines 6 and 7
- AND the `output` field MUST show context lines with `-` separator format
- AND match groups MUST be separated by `--`

### REQ-RESULT-009: Zero Matches Result

When no matches are found, the tool MUST return `success: true` with:

- An empty `matches` array
- `stats.total_matches` equal to 0
- `stats.files_matched` equal to 0
- `stats.truncated` equal to `false`

#### Scenario: Zero matches returns success with empty matches array

- GIVEN a workspace with no files containing the text `xyzzy_nonexistent_pattern_42`
- WHEN `code_search` is invoked with `{ "pattern": "xyzzy_nonexistent_pattern_42" }`
- THEN the result MUST have `success: true`
- AND `matches` MUST be an empty array
- AND `stats.total_matches` MUST be 0
- AND `stats.files_matched` MUST be 0
- AND `stats.truncated` MUST be `false`
- AND the summary line MUST indicate 0 matches found
