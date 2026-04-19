# Delta for Sessions

## ADDED Requirements

### Requirement: Session Discoverability Root Help

The system MUST treat `/session` with empty raw arguments as a read-only discoverability entry point for session commands in this slice.

The response MUST describe `/session` root usage and `/session status` as the supported `/session` family forms for this slice. The response MAY mention adjacent slash-session lifecycle commands such as `/resume`, `/suspend`, `/compact`, and `/tldr`, but it MUST distinguish them from `/session` subcommands. `/session` root help MUST NOT create, update, suspend, resume, compact, summarize, or otherwise mutate session records or slash-session state.

#### Scenario: Root help returns discoverability guidance without mutation

- GIVEN a current session context exists
- WHEN the user runs `/session`
- THEN the system MUST return read-only help or usage guidance for the `/session` family
- AND the guidance MUST include `/session status`
- AND no session lifecycle or snapshot state MUST be modified.

### Requirement: Current Session Status Discoverability

The system MUST make `/session status` a read-only status view over the current session identified by the typed slash command execution context.

The `/session status` result MUST identify the current session id from the execution context and MUST derive status from authoritative session records. When a `sessions` table record exists for that current session id, the result MUST classify the session as `suspended` only when the dedicated slash-session state record marks the lifecycle as suspended; otherwise it MUST classify the session as `active`. The result MUST include indicators for whether a latest TLDR snapshot reference exists and whether a latest compact snapshot reference exists. When the current session is suspended, the result MUST include the recorded suspension timestamp. When no authoritative session record exists for the current session id, the result MUST report that the current session is unknown to slash-session state and MUST NOT invent lifecycle or snapshot data.

The `/session status` result MUST also include exactly one actionable recommendation derived from the current session state:
- it MUST recommend `/compact` when the current session is active and has no latest compact snapshot reference;
- it MUST recommend `/suspend` when the current session is active and already has a latest compact snapshot reference;
- it MUST recommend `/resume` when the current session is suspended and has a latest compact snapshot reference; and
- it MUST withhold lifecycle-command recommendations when no authoritative current session record exists.

`/session status` MUST NOT mutate session records, slash-session state, or snapshots.

#### Scenario: Active current session without a compact snapshot recommends compact

- GIVEN the current session id resolves to an existing active session record
- AND the current session has no latest compact snapshot reference
- WHEN the user runs `/session status`
- THEN the system MUST report the current session id and `active` lifecycle
- AND the system MUST report that no latest compact snapshot is available
- AND the actionable recommendation MUST be `/compact`
- AND the command MUST NOT mutate session state.

#### Scenario: Active current session with a compact snapshot recommends suspend

- GIVEN the current session id resolves to an existing active session record
- AND the current session already has a latest compact snapshot reference
- WHEN the user runs `/session status`
- THEN the system MUST report the current session id and `active` lifecycle
- AND the system MUST report that a latest compact snapshot is available
- AND the actionable recommendation MUST be `/suspend`.

#### Scenario: Suspended current session recommends resume

- GIVEN the current session id resolves to an existing suspended session record
- AND the current session has a latest compact snapshot reference
- WHEN the user runs `/session status`
- THEN the system MUST report the current session id and `suspended` lifecycle
- AND the system MUST include the recorded suspension timestamp
- AND the actionable recommendation MUST be `/resume`.

#### Scenario: Current session without authoritative state reports limited status

- GIVEN the typed slash command execution context identifies a current session id
- AND no authoritative session record exists for that current session id
- WHEN the user runs `/session status`
- THEN the system MUST report the current session id
- AND the system MUST report that the current session is unknown to slash-session state
- AND the result MUST NOT invent lifecycle or snapshot references
- AND the result MUST NOT recommend `/resume`, `/suspend`, or `/compact`.
