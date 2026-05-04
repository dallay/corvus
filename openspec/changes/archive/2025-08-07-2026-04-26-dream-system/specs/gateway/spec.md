# Dream Specification

## Purpose

This specification defines Dream as Corvus's runtime-level long-term memory consolidation capability.
Dream converts eligible completed session history into durable long-term memory artifacts while
keeping session transcripts and gateway transport concerns separate from the Dream behavioral
source-of-truth.

## Requirements

### Requirement: Dream Eligibility for Completed Sessions

The runtime MUST treat Dream as a post-session consolidation capability that evaluates completed
sessions for Dream eligibility.

A Dream run MUST target a specific completed session identity. Dream MUST NOT consolidate sessions
that have not been recorded as completed.

The eligibility decision MUST be deterministic for the same completed session inputs and runtime
configuration.

#### Scenario: Completed session becomes a Dream candidate

- GIVEN a session `sess-123` has been recorded as completed
- AND the runtime can access the relevant session history for `sess-123`
- WHEN Dream eligibility is evaluated for `sess-123`
- THEN the runtime MUST treat `sess-123` as a Dream candidate
- AND the eligibility result MUST be derived from the completed session inputs rather than gateway transport details.

#### Scenario: Active session is not Dream-eligible

- GIVEN a session `sess-123` is still active and has not been recorded as completed
- WHEN Dream eligibility is evaluated for `sess-123`
- THEN the runtime MUST reject Dream consolidation for that session
- AND no Dream artifact MUST be persisted.

### Requirement: Dream Consolidation Output Contract

For an eligible completed session, Dream MUST synthesize durable long-term memory artifacts that
capture stable high-value knowledge from the completed session.

The Dream output MUST be additive. Dream MUST NOT require preserving the full session transcript as
the durable long-term memory artifact itself.

The runtime SHOULD favor stable summaries, facts, or other high-value distilled outputs over
verbatim transcript retention.

#### Scenario: Eligible completed session produces durable distilled memory

- GIVEN a completed session `sess-123` is Dream-eligible
- WHEN Dream consolidates the session
- THEN the runtime MUST produce one or more durable long-term memory artifacts for `sess-123`
- AND those artifacts MUST represent distilled high-value information from the session
- AND the artifacts MUST be persisted independently of the original request/response transport flow.

#### Scenario: Dream does not require verbatim transcript persistence as output

- GIVEN a completed session `sess-123` contains a multi-turn transcript
- WHEN Dream completes consolidation for `sess-123`
- THEN the durable Dream output MUST be allowed to omit verbatim transcript reproduction
- AND the Dream contract MUST remain satisfied so long as stable high-value information is preserved.

### Requirement: Dream Persistence Across Supported Backends

Dream artifacts MUST survive restart, export, and reload flows across supported memory backends.

For this change, supported backends are the runtime backends that already participate in Corvus
memory persistence and snapshot hydration/export behavior, including SQLite, markdown, and runtime
snapshot flows where supported.

A backend that claims Dream support MUST persist both Dream artifacts and enough Dream state to
avoid ambiguous replay for the same completed session.

#### Scenario: Dream artifacts survive runtime restart and reload

- GIVEN Dream has successfully consolidated completed session `sess-123`
- AND the runtime persists Dream through a supported backend
- WHEN the runtime is restarted and its persisted state is reloaded
- THEN the Dream artifacts for `sess-123` MUST still be available
- AND the runtime MUST preserve enough Dream state to avoid treating `sess-123` as unconsolidated solely because of the restart.

#### Scenario: Snapshot export and hydration preserve Dream state

- GIVEN Dream artifacts and Dream replay state exist for completed session `sess-123`
- WHEN the runtime exports its persisted state and later hydrates from that exported state
- THEN the hydrated runtime MUST restore the Dream artifacts for `sess-123`
- AND it MUST restore enough Dream state to keep consolidation behavior unambiguous for that session.

### Requirement: Dream Idempotency per Completed Session

Dream consolidation for a completed session MUST be idempotent.

The runtime MUST record enough Dream state to determine whether a completed session has already
been consolidated or is otherwise no longer eligible for duplicate consolidation.

Repeated Dream triggering for the same completed session MUST NOT create duplicate or ambiguous
consolidation results.

#### Scenario: Duplicate Dream trigger for completed session is suppressed

- GIVEN completed session `sess-123` has already been successfully consolidated by Dream
- WHEN Dream is triggered again for `sess-123`
- THEN the runtime MUST detect that `sess-123` has already been consolidated
- AND it MUST NOT create a second ambiguous consolidation result for the same completion event.

#### Scenario: Repeated trigger after restore remains idempotent

- GIVEN completed session `sess-123` was consolidated before runtime export or shutdown
- AND the persisted Dream state is later restored
- WHEN Dream is triggered again for `sess-123` after restore
- THEN the runtime MUST preserve idempotent behavior for that session
- AND it MUST NOT treat the restored runtime as permission to reconsolidate the same completion ambiguously.
