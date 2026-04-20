# Delta for Sessions

## MODIFIED Requirements

### Requirement: Session Discoverability Root Help

The system MUST treat `/session` with empty raw arguments as a read-only discoverability entry point for session commands in this slice.

The response MUST describe `/session` root usage and the supported `/session` family forms for this slice. It MUST identify `/session status` as the compact current-session summary view, and it MUST identify `/session inspect` as the richer current-session inspection view. The response MAY mention adjacent slash-session lifecycle commands such as `/resume`, `/suspend`, `/compact`, and `/tldr`, but it MUST distinguish them from `/session` subcommands. `/session` root help MUST remain the family help or usage hub and MUST NOT create, update, suspend, resume, compact, summarize, inspect, or otherwise mutate session records or slash-session state.

(Previously: The response described `/session` root usage and `/session status` as the supported `/session` family forms for the slice. The spec did not require `/session inspect` to appear as a supported richer inspection view.)

#### Scenario: Root help returns discoverability guidance without mutation

- GIVEN a current session context exists
- WHEN the user runs `/session`
- THEN the system MUST return read-only help or usage guidance for the `/session` family
- AND the guidance MUST include `/session status` as the compact summary view
- AND the guidance MUST include `/session inspect` as the richer inspection view
- AND no session lifecycle or snapshot state MUST be modified.

### Requirement: Current Session Status Discoverability

The system MUST make `/session status` a read-only compact summary view over the current session identified by the typed slash command execution context.

The `/session status` result MUST identify the current session id from the execution context and MUST derive status from authoritative session records. When a `sessions` table record exists for that current session id, the result MUST classify the session as `suspended` only when the dedicated slash-session state record marks the lifecycle as suspended; otherwise it MUST classify the session as `active`. The result MUST include indicators for whether a latest TLDR snapshot reference exists and whether a latest compact snapshot reference exists. When the current session is suspended, the result MUST include the recorded suspension timestamp. When no authoritative session record exists for the current session id, the result MUST report that the current session is unknown to slash-session state and MUST NOT invent lifecycle or snapshot data.

The `/session status` result MUST also include exactly one actionable recommendation derived from the current session state:
- it MUST recommend `/compact` when the current session is active and has no latest compact snapshot reference;
- it MUST recommend `/suspend` when the current session is active and already has a latest compact snapshot reference;
- it MUST recommend `/resume` when the current session is suspended and has a latest compact snapshot reference;
- it MUST recommend `/compact` when the current session is suspended and has no latest compact snapshot reference; and
- it MUST withhold lifecycle-command recommendations when no authoritative current session record exists.

`/session status` SHOULD remain concise enough to act as the compact summary view for the `/session` family, and it MAY direct callers to `/session inspect` when a richer inspection view is needed. `/session status` MUST NOT mutate session records, slash-session state, or snapshots.

(Previously: The system made `/session status` a read-only status view over the current session identified by the typed slash command execution context. The spec did not explicitly define `/session status` as the compact summary view relative to a richer `/session inspect` view.)

#### Scenario: Status remains the compact summary view

- GIVEN the current session id resolves to an existing active session record
- AND the current session already has authoritative slash-session state available
- WHEN the user runs `/session status`
- THEN the system MUST return a concise current-session summary
- AND the result MAY direct the user to `/session inspect` for richer inspection details
- AND the command MUST NOT mutate session state.

## ADDED Requirements

### Requirement: Current Session Inspection Discoverability

The system MUST make `/session inspect` a read-only richer inspection view over the current session identified by the typed slash command execution context.

The `/session inspect` result MUST be current-session-only and MUST NOT accept or require target-session arguments. The result MUST combine authoritative data from the current session record, the dedicated slash-session state record, and any referenced authoritative snapshot rows that are available for that current session. The result MUST return balanced output consisting of a human-readable summary plus a structured inspect payload, and both views MUST be derived from the same authoritative inspect model so they remain consistent. `/session inspect` MUST NOT become a standalone canonical command or alias, and it MUST NOT mutate session records, slash-session state, or snapshots.

When a current session record exists but slash-session state or referenced snapshot rows are missing or incomplete, `/session inspect` MUST return partial data for the current session, MUST explicitly identify each missing or incomplete data area as a gap, and MUST NOT invent lifecycle, snapshot, or hydration facts that are not present in authoritative storage. When no authoritative current session record exists, the result MUST report that the current session is unknown to slash-session state and MUST NOT invent state or snapshot details.

#### Scenario: Inspect returns a richer current-session view when authoritative data is complete

- GIVEN the typed slash command execution context identifies current session `abc-123`
- AND an authoritative `sessions` row exists for `abc-123`
- AND an authoritative slash-session state row exists for `abc-123`
- AND the referenced authoritative snapshot rows exist for that state
- WHEN the user runs `/session inspect`
- THEN the system MUST return a human-readable inspection summary for `abc-123`
- AND the system MUST return a structured inspect payload for `abc-123`
- AND the structured payload MUST include session record details, slash-session state details, and referenced snapshot details derived from authoritative storage
- AND the command MUST NOT mutate session state.

#### Scenario: Inspect returns partial data when slash-session state is missing

- GIVEN the typed slash command execution context identifies current session `abc-123`
- AND an authoritative `sessions` row exists for `abc-123`
- AND no authoritative slash-session state row exists for `abc-123`
- WHEN the user runs `/session inspect`
- THEN the system MUST return the known current session record details for `abc-123`
- AND the result MUST explicitly mark slash-session state as missing
- AND the result MUST explicitly mark snapshot-derived details as unavailable when they depend on missing state
- AND the result MUST NOT invent lifecycle or snapshot facts.

#### Scenario: Inspect returns partial data when a referenced snapshot is missing or incomplete

- GIVEN the typed slash command execution context identifies current session `abc-123`
- AND an authoritative `sessions` row exists for `abc-123`
- AND an authoritative slash-session state row exists for `abc-123`
- AND that state references a snapshot row that is missing or incomplete
- WHEN the user runs `/session inspect`
- THEN the system MUST return the known current session record details and slash-session state details for `abc-123`
- AND the result MUST explicitly identify the referenced snapshot gap
- AND the result MUST preserve any authoritative snapshot fields that are available
- AND the result MUST NOT synthesize the missing snapshot details.

#### Scenario: Inspect reports an unknown current session without inventing state

- GIVEN the typed slash command execution context identifies a current session id
- AND no authoritative session record exists for that current session id
- WHEN the user runs `/session inspect`
- THEN the system MUST report the current session id
- AND the system MUST report that the current session is unknown to slash-session state
- AND the result MUST NOT invent lifecycle, state, or snapshot details.
