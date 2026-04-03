# Delta for Agent Loop

## ADDED Requirements

### Requirement: Specialized Session Reuse

The canonical agent loop MUST support specialized runtime sessions that reuse the same bootstrap,
dispatcher, approval, and security boundaries as generic sessions. A specialized session MUST add
mode-specific behavior without creating a parallel loop contract.

#### Scenario: Code-specialist session uses canonical loop

- GIVEN a caller starts a code-specialist session from a canonical runtime entry point
- WHEN the session enters execution
- THEN the system MUST run that session through the same canonical loop lifecycle used by other
  dispatcher-backed sessions
- AND any specialized prompt or output behavior MUST remain inside the canonical loop contract.

### Requirement: Delegated Specialized Sessions

The canonical agent loop MUST permit a parent session to launch a bounded delegated specialized
session when configuration allows it. Delegated specialized sessions MUST inherit the same policy
and approval semantics as direct canonical sessions and MUST terminate within their configured
bounds.

#### Scenario: Delegated code session inherits canonical protections

- GIVEN a parent canonical session delegates work to a code-specialist session
- WHEN the delegated session executes tool calls
- THEN the system MUST apply the same dispatcher policy, approval checks, and security invariants
  used for direct canonical sessions
- AND the delegated session MUST return a structured completion result to the parent session.

#### Scenario: Delegated specialized session hits configured limit

- GIVEN a delegated specialized session with explicit iteration or timeout limits
- WHEN execution reaches a configured limit before task completion
- THEN the system MUST stop the delegated session within the same safety model used by the
  canonical loop
- AND the session MUST return a structured non-success result that identifies the enforced limit.
