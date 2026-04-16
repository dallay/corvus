# Delta for sessions

## MODIFIED Requirements

### Requirement: Resume List, Select, and Load Behavior

The `/resume` command MUST support deterministic list and load behavior for suspended sessions using the typed slash command execution context as the authorization input.

- When invoked without a target, `/resume` MUST return a list of resumable suspended sessions visible to the caller scope represented in the execution context.
- When invoked with a target session identifier, `/resume {session_id}` MUST validate that the target exists, is suspended, has a valid resume-capable snapshot, AND is visible to that same caller scope.
- The runtime MUST preserve per-surface identity semantics when deriving caller scope for `/resume`. Authenticated gateway caller identity, derived channel caller identity, derived CLI caller identity, and unavailable caller identity MUST remain distinguishable inputs to authorization-sensitive behavior.
- The runtime MUST enforce the same caller-scoped visibility or ownership rules for `/resume {session_id}` that it uses for the no-target `/resume` listing. A caller MUST NOT be allowed to resume a session that would not appear in that caller's authorized resumable-session set.
- If no verifiable or derivable caller scope can be established for an authorization-sensitive `/resume` operation, the runtime MUST return an explicit authorization-denied or unsupported outcome instead of broadening visibility.
- On successful resume, the runtime MUST load the authoritative resume snapshot for that session, reactivate the session, and return a user-visible result that identifies the resumed session.

(Previously: the spec required `/resume` listing and target validation to use caller-scoped visibility rules, but it did not require those rules to consume a typed execution context or explicitly preserve distinct authenticated, derived, and unavailable caller-scope semantics.)

#### Scenario: Resume listing uses the caller scope represented in typed context

- GIVEN suspended sessions `abc-123` and `xyz-789` both have valid resume-capable snapshots
- AND the typed execution context represents a caller scope that is authorized to view only `abc-123`
- WHEN the user runs `/resume` without a target
- THEN the system MUST return a deterministic list containing `abc-123`
- AND the system MUST NOT include `xyz-789` in that caller's result.

#### Scenario: Resume target is denied when target session falls outside caller scope

- GIVEN session `xyz-789` is suspended and resume-capable
- AND the typed execution context represents a caller scope that is not authorized to view or resume `xyz-789`
- WHEN the user runs `/resume xyz-789`
- THEN the system MUST return an explicit authorization-denied outcome
- AND no session state for `xyz-789` MUST be modified.

#### Scenario: Derived channel or CLI scope remains distinct from authenticated gateway scope

- GIVEN `/resume` is invoked from a non-gateway surface with a derived caller scope
- WHEN the runtime evaluates authorization for list or target behavior
- THEN the system MUST apply that surface's existing derived-scope semantics
- AND it MUST NOT treat the request as if it came from an authenticated gateway bearer caller
- AND any denial or unsupported result MUST preserve that distinction internally.

#### Scenario: Resume denied when no verifiable caller scope is available

- GIVEN `/resume xyz-789` is invoked in a context where no verifiable caller identity or derivable scope can be established (e.g., absent token claims, tampered token, or unable to derive scope from the transport context)
- WHEN the runtime evaluates authorization for the target resume
- THEN the system MUST return an explicit authorization-denied or unsupported outcome
- AND no session state MUST be modified
- AND the system MUST NOT return the broader visibility that an authenticated caller would receive.

#### Scenario: Resume target loads snapshot and reactivates session within authorized scope

- GIVEN session `abc-123` is suspended and has a valid authoritative compact snapshot
- AND the typed execution context represents a caller scope authorized to resume `abc-123`
- WHEN the user runs `/resume abc-123`
- THEN the runtime MUST load the referenced resume snapshot for session `abc-123`
- AND the session state MUST transition from `suspended` to `active`
- AND the user-visible result MUST identify that session `abc-123` was resumed from persisted state.
