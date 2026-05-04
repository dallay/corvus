# Delta for Sessions

## MODIFIED Requirements

### Requirement: SESS-5: Stale Session Auto-Close

The memory hygiene pass MUST auto-close stale sessions.

- A session is **stale** when `ended_at` IS NULL AND `last_activity` is older than the configured
  threshold.
- The default stale threshold MUST be 24 hours.
- The threshold SHOULD be configurable via runtime config.
- Auto-close MUST set `ended_at` to the current UTC timestamp, not the `last_activity` time.
- When a session is auto-closed or otherwise recorded as completed, that completed state MUST be a
  valid Dream trigger input for downstream runtime Dream evaluation.

(Previously: SESS-5 defined stale-session detection and auto-close timing, but it did not define
how a recorded completion participates in downstream Dream triggering.)

#### Scenario: Hygiene pass closes stale session and produces a Dream trigger input

- GIVEN an active session `old-session` with last_activity `2026-03-27T08:00:00Z`
- AND the stale session threshold is 24 hours
- WHEN the hygiene pass runs at `2026-03-28T10:00:00Z`
- THEN session `old-session` MUST have ended_at set to `2026-03-28T10:00:00Z`
- AND the completed state MUST be available for downstream Dream evaluation as a recorded completion.

## ADDED Requirements

### Requirement: Session Completion Must Produce a Deterministic Dream Trigger Input

The runtime MUST record session completion before Dream is evaluated for that session.

Dream evaluation MUST use the recorded completed-session state as its trigger input rather than an
in-flight or inferred gateway-only notion of completion.

The runtime MUST ensure the completion-to-Dream ordering is deterministic for the same session.

#### Scenario: Dream runs only after completion is recorded

- GIVEN session `sess-123` is reaching the end of its lifecycle
- WHEN the runtime records session completion for `sess-123`
- THEN the session MUST first exist in a completed recorded state
- AND only after that recorded completion MAY Dream be evaluated for `sess-123`.

#### Scenario: Failed or missing completion record blocks Dream evaluation

- GIVEN no recorded completed-session state exists for `sess-123`
- WHEN Dream would otherwise be evaluated for `sess-123`
- THEN the runtime MUST NOT consolidate that session
- AND it MUST treat the missing completion record as a failed trigger precondition.

### Requirement: Session Completion and Dream Triggering Must Be Idempotent Together

The runtime MUST coordinate session completion recording and Dream triggering so repeated completion
handling for the same session does not create duplicate Dream work.

If the same completion event is observed more than once for a session, the runtime MUST preserve a
single logical completed session outcome for Dream purposes.

#### Scenario: Repeated completion handling does not duplicate Dream work

- GIVEN session `sess-123` has already been recorded as completed
- AND Dream has already evaluated or consolidated that completion
- WHEN the same completion handling path runs again for `sess-123`
- THEN the runtime MUST preserve a single logical completion outcome
- AND it MUST NOT create duplicate Dream consolidation for that session.

#### Scenario: Duplicate completion record without prior Dream result remains safe

- GIVEN session `sess-123` has already been recorded as completed
- AND Dream has not yet produced a durable result for that session
- WHEN the completion handling path is retried for `sess-123`
- THEN the runtime MUST keep completion recording idempotent
- AND any subsequent Dream evaluation MUST remain unambiguous for that same session.
