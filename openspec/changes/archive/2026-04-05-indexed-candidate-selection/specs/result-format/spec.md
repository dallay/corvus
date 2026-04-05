# Delta for Result Format

## MODIFIED Requirements

### Requirement: REQ-RESULT-003 Match Object Schema

Each element in the `matches` array MUST be an object describing a match verified against live file
contents with the following fields:

| Field            | Type     | Description                                                  |
|------------------|----------|--------------------------------------------------------------|
| `file`           | string   | Workspace-relative file path                                 |
| `line`           | integer  | 1-based starting line number                                 |
| `column`         | integer  | 1-based starting column (byte offset within the line)        |
| `content`        | string   | The full matched line (truncated to 500 chars max)           |
| `context_before` | string[] | Lines before match (length = `context_lines` parameter)      |
| `context_after`  | string[] | Lines after match (length = `context_lines` parameter)       |
| `line_end`       | integer  | 1-based ending line number for the verified match range      |
| `column_end`     | integer  | 1-based ending column for the verified match range           |
| `byte_start`     | integer  | 0-based starting byte offset within the file                 |
| `byte_end`       | integer  | 0-based ending byte offset within the file                   |
| `preview`        | string   | Machine-readable preview text for the verified match snippet |

The structured match payload MUST expose verified location and preview data without making indexed
candidate extraction observable as a source of truth.

(Previously: Each match object only required `file`, `line`, `column`, `content`, `context_before`,
and `context_after`.)

#### Scenario: Match object includes verified range and preview fields

- GIVEN a workspace file with a verified match
- WHEN `code_search` returns structured results
- THEN each match object MUST contain `file`, `line`, `column`, `content`, `context_before`, and
  `context_after`
- AND it MUST also contain `line_end`, `column_end`, `byte_start`, `byte_end`, and `preview`
- AND the added fields MUST describe the same verified match range returned by live verification

### Requirement: REQ-RESULT-007 Truncation Warning

When results are truncated by `max_results` or another result cap, the `output` field MUST include
a truncation warning line:

```
Results truncated at {N} matches. Narrow your search with 'path' or 'include' filters.
```

The `stats.truncated` field MUST be `true` when any truncation occurs.

Result caps MUST apply to verified matches after deterministic ordering and live verification, not
merely to raw indexed candidates.

(Previously: Truncation behavior did not specify whether `max_results` applied before or after
verification.)

#### Scenario: Verified match cap applies after candidate verification

- GIVEN indexed candidate extraction returns more candidate files than the requested `max_results`
- AND live verification finds matches in those files
- WHEN `code_search` applies `max_results`
- THEN the cap MUST be applied to the ordered verified matches
- AND the `matches` array MUST contain at most `max_results` verified matches
- AND candidate files without verified matches MUST NOT count toward the cap

## ADDED Requirements

### Requirement: REQ-RESULT-010 Deterministic Verified Match Ordering

The system MUST return verified matches in deterministic order.

Verified matches MUST be ordered first by workspace-relative file path, then by verified match
location within the file.

#### Scenario: Verified matches are stable across repeated runs

- GIVEN an unchanged workspace and the same `code_search` request
- WHEN the search is executed repeatedly
- THEN the returned `matches` array MUST appear in the same order on every run
- AND matches from the same file MUST appear in ascending verified location order
