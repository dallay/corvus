# Delta for Regex Semantics

## ADDED Requirements

### Requirement: REQ-REGEX-007 Live Verification Is Authoritative

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
