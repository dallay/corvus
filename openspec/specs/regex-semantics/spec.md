# code_search Regex Semantics Specification

## Purpose

Defines the pattern matching behavior of the `code_search` tool, including regex engine
semantics, literal escaping, modifier flags, and the precise order of pattern construction.

## Requirements

### REQ-REGEX-001: RE2-like Semantics via Rust `regex` Crate

The tool MUST use the Rust `regex` crate (RE2-like syntax) as its regex engine.

Unicode MUST be enabled by default — `\w`, `\d`, and similar classes MUST be Unicode-aware.

The `.` metacharacter MUST NOT match `\n` (single-line mode MUST be off by default).

#### Scenario: Unicode-aware character classes

- GIVEN a workspace file containing the line `café = true`
- WHEN `code_search` is invoked with `{ "pattern": "\\w+ = true", "is_regex": true }`
- THEN the result MUST match the line `café = true`
- AND the `\w` class MUST have matched the accented character `é`

### REQ-REGEX-002: Unsupported Regex Features

The tool MUST NOT support the following regex features (the `regex` crate does not implement them):

- Backreferences (`\1`, `\2`, etc.)
- Lookahead / lookbehind (`(?=...)`, `(?!...)`, `(?<=...)`, `(?<!...)`)
- Possessive quantifiers (`x++`)
- Atomic groups (`(?>...)`)
- Conditional patterns (`(?(cond)yes|no)`)
- PCRE-specific syntax

If a pattern using unsupported features fails to compile, the tool MUST return `success: false`
with the compilation error message.

#### Scenario: Lookahead pattern returns compilation error

- GIVEN any workspace
- WHEN `code_search` is invoked with `{ "pattern": "foo(?=bar)", "is_regex": true }`
- THEN the result MUST have `success: false`
- AND the error message MUST contain the regex compilation error

### REQ-REGEX-003: Literal Mode Escaping

When `is_regex` is `false` (the default), the tool MUST escape the pattern via `regex::escape()`
before compilation.

This MUST ensure that regex metacharacters in the pattern are treated as literal characters.

#### Scenario: Literal search with regex metacharacters matches literally

- GIVEN a workspace file containing the line `assert(vec[0] == x.y)`
- WHEN `code_search` is invoked with `{ "pattern": "vec[0] == x.y" }`
- THEN the result MUST match the literal text `vec[0] == x.y`
- AND `[0]` MUST NOT be interpreted as a character class
- AND `.` MUST NOT be interpreted as a wildcard

### REQ-REGEX-004: Case Insensitive Mode

When `case_sensitive` is `false`, the tool MUST prepend `(?i)` to the pattern before compilation.

This MUST cause the regex engine to match regardless of case.

#### Scenario: Case insensitive search matches mixed case

- GIVEN a workspace file containing the lines `NEED: fix this` and `Need: refactor`
- WHEN `code_search` is invoked with `{ "pattern": "need", "case_sensitive": false }`
- THEN the result MUST match both lines

#### Scenario: Case sensitive search is the default

- GIVEN a workspace file containing the lines `Error` and `error`
- WHEN `code_search` is invoked with `{ "pattern": "Error" }`
- THEN the result MUST match only the line `Error`
- AND the line `error` MUST NOT be matched

### REQ-REGEX-005: Whole Word Mode

When `whole_word` is `true`, the tool MUST wrap the pattern with `\b` word boundary anchors
(`\b` + pattern + `\b`) before compilation.

This MUST ensure the pattern only matches at word boundaries and does not match substrings.

#### Scenario: Whole word search does not match substrings

- GIVEN a workspace file containing the lines `log("message")` and `logger.info("message")`
- WHEN `code_search` is invoked with `{ "pattern": "log", "whole_word": true }`
- THEN the result MUST match the line containing `log("message")`
- AND the result MUST NOT match the line containing `logger.info("message")`

#### Scenario: Whole word with regex mode

- GIVEN a workspace file containing the lines `fn test()` and `fn test_helper()`
- WHEN `code_search` is invoked with `{ "pattern": "test", "whole_word": true, "is_regex": true }`
- THEN the result MUST match the line containing `fn test()`
- AND the result MUST NOT match the line containing `fn test_helper()`

### REQ-REGEX-006: Pattern Construction Order

The tool MUST construct the final regex pattern in the following strict order:

1. Validate pattern length (MUST be ≤ 1000 characters)
2. If `is_regex` is `false`: apply `regex::escape()` to the pattern
3. If `case_sensitive` is `false`: prepend `(?i)` to the pattern
4. If `whole_word` is `true`: wrap the pattern with `\b...\b`
5. Compile the final pattern via `regex::Regex::new()`

If compilation fails at step 5, the tool MUST return `success: false` with the compilation
error message.

#### Scenario: Combined literal + case insensitive + whole word

- GIVEN a workspace file containing the lines `Foo`, `foo`, `foobar`, and `FOOBAR`
- WHEN `code_search` is invoked with
  `{ "pattern": "foo", "case_sensitive": false, "whole_word": true }`
- THEN the result MUST match `Foo` and `foo`
- AND the result MUST NOT match `foobar` or `FOOBAR`

#### Scenario: Regex mode bypasses escaping

- GIVEN a workspace file containing the lines `a1`, `b2`, and `cc`
- WHEN `code_search` is invoked with `{ "pattern": "[a-b]\\d", "is_regex": true }`
- THEN the result MUST match `a1` and `b2`
- AND the result MUST NOT match `cc`

### REQ-REGEX-007: Live Verification Is Authoritative

The system MUST treat final literal or regex verification against current file contents as the only
authoritative source of reported matches.

Index-based candidate extraction MAY over-include files, but it MUST NOT emit a match unless the
compiled search pattern matches the file's live contents under the request's active search
semantics.

#### Scenario: Candidate false positive is eliminated by live verification

- GIVEN an indexed candidate file whose persisted trigrams suggest it may contain the requested
  pattern
- AND the file's current contents do not satisfy the compiled literal or regex matcher
- WHEN `code_search` verifies the candidate against live file contents
- THEN the file MUST NOT contribute any reported match
- AND the result set MUST contain only matches proven by live verification

#### Scenario: Regex verification remains authoritative after candidate filtering

- GIVEN indexed candidate extraction returns a file for a regex search request
- AND the file contains the required trigrams but does not satisfy the compiled regex at runtime
- WHEN `code_search` verifies the file contents with the compiled regex
- THEN the file MUST be excluded from the reported matches
- AND the index-derived candidate alone MUST NOT make the file visible as a match
