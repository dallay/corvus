# Delta for Safety Model

## ADDED Requirements

### Requirement: REQ-SAFE-012 Safe Fallback Preserves Search Correctness

When indexed candidate extraction is unavailable, cannot safely narrow the request, or would risk
missing correct matches, the system MUST fall back to the existing safe discovery-based scan path
before reporting results.

The system MAY combine indexed candidates with fallback-discovered files, but only when the final
verification input set remains complete for the active request and deterministic in ordering.

#### Scenario: Request without safe indexed reduction falls back to discovery scan

- GIVEN a `code_search` request whose semantics do not allow trustworthy indexed candidate
  extraction
- WHEN the system plans the search
- THEN it MUST use the existing discovery-based scan path for correctness
- AND it MUST still verify reported matches against live file contents before returning them

#### Scenario: Index unavailability does not reduce correctness

- GIVEN the local workspace trigram index is unavailable, incompatible, or fails during candidate
  extraction
- WHEN `code_search` executes a request
- THEN the search MUST fall back to the safe discovery-based scan path
- AND the final reported matches MUST preserve the same correctness guarantees as a scan-only
  execution
