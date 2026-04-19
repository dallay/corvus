---
doc_id: session-lifecycle
version: 1.0.0
created: 2026-03-28
status: active
owner: architecture
---

# Spec: Session Lifecycle

## Overview

This specification defines session lifecycle management for the Corvus runtime. Sessions transition
from implicit memory filters to explicit, tracked entities with creation timestamps, activity
tracking, and deterministic closure. The SQLite backend is the authoritative source of truth.

---

## Requirements

### SESS-1: Session Table Schema

The runtime MUST maintain a `sessions` table in the SQLite `brain.db` with the following columns:

| Column          | Type            | Constraints                |
|-----------------|-----------------|----------------------------|
| `id`            | TEXT            | PRIMARY KEY, NOT NULL      |
| `started_at`    | TEXT (ISO 8601) | NOT NULL                   |
| `ended_at`      | TEXT (ISO 8601) | NULL (active sessions)     |
| `status`        | TEXT            | NOT NULL, DEFAULT 'active' |
| `message_count` | INTEGER         | NOT NULL, DEFAULT 0        |
| `last_activity` | TEXT (ISO 8601) | NOT NULL                   |
| `token_hash`    | TEXT            | NULL                       |
| `metadata`      | TEXT (JSON)     | NULL                       |

The table MUST have indexes on `status`, `started_at`, `last_activity`, and `token_hash` for
efficient listing, filtering, and end-user scoping.

#### Scenario: Session table migration on existing brain.db

```gherkin
Given a brain.db file without a "sessions" table
When the runtime starts
Then the "sessions" table MUST be created via safe migration (CREATE TABLE IF NOT EXISTS)
And existing memory entries with session_id values MUST NOT be affected
And no existing tables or columns MUST be modified
```

#### Scenario: Session table already exists

```gherkin
Given a brain.db file that already has a "sessions" table
When the runtime starts
Then the migration MUST succeed silently (idempotent)
And no data in the existing sessions table MUST be modified
```

---

### SESS-2: Session Creation

The runtime MUST explicitly create a session record when a new `session_id` is first used in the
agent loop.

- Session creation MUST be idempotent — if a session with the given ID already exists, the existing
  record MUST be returned without modification (UPSERT semantics on insert).
- The `started_at` and `last_activity` fields MUST be set to the current UTC timestamp on creation.
- The `message_count` MUST be initialized to `0`.

#### Scenario: New session created on first message

```gherkin
Given no session record exists for session_id "abc-123"
When the gateway receives a request with X-Session-Id header "abc-123"
And the agent loop processes the message
Then a new row MUST be inserted into the sessions table with id "abc-123"
And started_at MUST be set to the current UTC timestamp
And message_count MUST be 0
And ended_at MUST be NULL
```

#### Scenario: Duplicate session creation is idempotent

```gherkin
Given a session record already exists for session_id "abc-123" with started_at "2026-03-28T10:00:00Z"
When a session creation is attempted for session_id "abc-123"
Then the existing session record MUST be returned unchanged
And started_at MUST remain "2026-03-28T10:00:00Z"
```

#### Scenario: Auto-generated session ID

```gherkin
Given a request arrives at the gateway without an X-Session-Id header
When resolve_session_id() generates a "webhook-{uuid}" session ID
Then a session record MUST be created for the generated ID
And the session MUST behave identically to explicitly-provided session IDs
```

---

### SESS-3: Session Activity Updates

The runtime MUST update session activity on each message processed within the agent loop.

- `message_count` MUST be incremented by 1 for each user message processed.
- `last_activity` MUST be updated to the current UTC timestamp.
- Updates MUST only apply to sessions where `ended_at` IS NULL (active sessions).

#### Scenario: Message increments session counters

```gherkin
Given an active session "abc-123" with message_count 5 and last_activity "2026-03-28T10:00:00Z"
When a new message is processed in session "abc-123" at "2026-03-28T10:05:00Z"
Then message_count MUST be 6
And last_activity MUST be "2026-03-28T10:05:00Z"
```

#### Scenario: Activity update on ended session is rejected

```gherkin
Given a session "abc-123" with ended_at "2026-03-28T09:00:00Z"
When a new message attempts to update session "abc-123"
Then a new session SHOULD be created (or the request rejected)
And the ended session record MUST NOT be modified
```

---

### Requirement: Authoritative Session Snapshot and State Persistence

The runtime MUST keep the existing `sessions` table as the identity and listing source for session
records, and it MUST persist slash-session resumability state in dedicated SQLite tables that are
separate from generic memory entries.

For this slice, SQLite MUST be the only authoritative persistence backend for `/resume`, `/suspend`,
`/tldr`, and `/compact`. Generic memory entries MUST NOT be treated as the source of truth for
resume, suspension, or snapshot state.

#### Scenario: Slash session state is stored outside generic memory

- GIVEN a SQLite-backed runtime with an existing session `abc-123`
- WHEN the user runs `/compact`
- THEN the runtime MUST persist the authoritative compact snapshot in the dedicated session snapshot/state tables
- AND the existing `sessions` table MUST remain the source for session identity and listing
- AND generic memory records MUST NOT be required to reconstruct the resumable state.

#### Scenario: Non-SQLite backend is rejected for slash session persistence

- GIVEN the runtime is configured with a non-SQLite memory backend
- WHEN the user runs `/tldr`, `/compact`, `/suspend`, or `/resume`
- THEN the system MUST return an explicit unsupported result for the command
- AND the system MUST NOT pretend the command succeeded with in-memory or no-op behavior.

### Requirement: SQLite Session Snapshot Schema and Migration

The SQLite backend MUST add additive, idempotent schema support for dedicated session snapshot/state
persistence.

The runtime MUST maintain:
- a `session_snapshots` table for persisted slash-command snapshots;
- a `session_state` table for the current authoritative slash-session lifecycle state.

The `session_snapshots` table MUST record, at minimum:
- a stable snapshot identifier;
- the owning `session_id` referencing `sessions.id`;
- a snapshot kind that distinguishes `tldr` and `compact` snapshots;
- a created-at timestamp;
- the persisted snapshot payload;
- whether the snapshot is resume-capable.

The `session_state` table MUST record, at minimum:
- the owning `session_id` referencing `sessions.id`;
- the current slash-session state;
- updated-at timestamp information;
- suspension timestamp information when applicable;
- the latest authoritative snapshot references needed for summary display and resume.

These migrations MUST be additive and idempotent, and they MUST NOT alter or repurpose existing
`memories` rows as authoritative slash-session state.

#### Scenario: Existing SQLite database receives additive slash-session migration

- GIVEN a `brain.db` file that already contains `sessions` and existing memory tables
- WHEN the runtime starts with slash-session persistence enabled
- THEN the runtime MUST create `session_snapshots` and `session_state` if they do not already exist
- AND the migration MUST NOT remove or rewrite existing `sessions` or `memories` data.

#### Scenario: Repeated startup keeps slash-session migration idempotent

- GIVEN a `brain.db` file that already contains the slash-session tables
- WHEN the runtime starts again
- THEN the migration MUST complete successfully without duplicating schema objects
- AND existing snapshot and state rows MUST remain unchanged unless a command explicitly updates them.

### Requirement: TLDR Snapshot Persistence and Result

When a user runs `/tldr` for a valid active session, the runtime MUST create a persisted `tldr`
snapshot for that session and MUST return the generated summary as the user-visible command result.

A `/tldr` snapshot MUST be stored as a dedicated snapshot record and MUST update the authoritative
session-state record so the latest summary can be retrieved deterministically later.

#### Scenario: TLDR persists summary and returns it to the user

- GIVEN an active session `abc-123` in a SQLite-backed runtime
- WHEN the user runs `/tldr`
- THEN the system MUST persist a new `tldr` snapshot linked to session `abc-123`
- AND the session state MUST reference that snapshot as the latest summary snapshot
- AND the user-visible result MUST include the summary generated for that command.

#### Scenario: TLDR on unknown session fails clearly

- GIVEN no session record exists for the current session identity
- WHEN the user runs `/tldr`
- THEN the system MUST return an unknown-session error result
- AND no snapshot row or state row MUST be created.

### Requirement: Compact Snapshot Persistence and Resume-Friendly Behavior

When a user runs `/compact` for a valid active session, the runtime MUST create a persisted compact
snapshot that is marked as resume-capable and suitable for later resume loading.

The runtime MUST update the authoritative session-state record so the latest compact snapshot becomes
the default resume snapshot for that session.

#### Scenario: Compact creates a resume-capable snapshot

- GIVEN an active session `abc-123` in a SQLite-backed runtime
- WHEN the user runs `/compact`
- THEN the system MUST persist a new `compact` snapshot linked to session `abc-123`
- AND that snapshot MUST be marked as resume-capable
- AND the session state MUST reference it as the latest authoritative resume snapshot
- AND the user-visible result MUST confirm that the session is compacted and ready for resume.

#### Scenario: Missing resume-capable snapshot is detectable

- GIVEN a suspended session whose state row references no valid resume-capable snapshot
- WHEN the user attempts to resume that session
- THEN the system MUST return a missing-snapshot error result
- AND the session MUST remain suspended until a valid resume-capable snapshot exists.

### Requirement: Session Suspension Semantics and Listability

When a user runs `/suspend` for a valid active session, the runtime MUST transition the session into
a suspended state only if an authoritative resume-capable snapshot is available for that session.

Suspension MUST be reflected in the dedicated session-state record and MUST remain listable through
the existing session identity/listing model.

Sessions with state "suspended" (as recorded in the session-state table by `/suspend`) MUST be excluded
from normal activity-counting updates (SESS-3) and from the stale auto-close rule (SESS-5) that closes
sessions where `ended_at` IS NULL and `last_activity` is older than the configured threshold. Suspended
sessions MUST NOT be auto-closed by the hygiene pass until they are explicitly resumed via `/resume` or
explicitly ended by another mechanism. The runtime MUST ensure that the suspended lifecycle state takes
precedence over activity-based auto-close logic, so only active sessions are subject to stale detection.

#### Scenario: Suspend marks a session as suspended and listable

- GIVEN an active session `abc-123` with a latest authoritative resume-capable snapshot
- WHEN the user runs `/suspend`
- THEN the session state MUST transition to `suspended`
- AND the state row MUST record the suspension timestamp
- AND session `abc-123` MUST remain discoverable in suspended-session listings.

#### Scenario: Suspend without a resume-capable snapshot is rejected

- GIVEN an active session `abc-123` with no authoritative resume-capable snapshot
- WHEN the user runs `/suspend`
- THEN the system MUST return a missing-snapshot error result
- AND the session MUST remain active
- AND the system MUST NOT create a suspended state for that session.

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

#### Scenario: Resume target loads snapshot and reactivates session within authorized scope

- GIVEN session `abc-123` is suspended and has a valid authoritative compact snapshot
- AND the typed execution context represents a caller scope authorized to resume `abc-123`
- WHEN the user runs `/resume abc-123`
- THEN the runtime MUST load the referenced resume snapshot for session `abc-123`
- AND the session state MUST transition from `suspended` to `active`
- AND the user-visible result MUST identify that session `abc-123` was resumed from persisted state.

### Requirement: Session Discoverability Root Help

The system MUST treat `/session` with empty raw arguments as a read-only discoverability entry
point for session commands in this slice.

The response MUST describe `/session` root usage and the supported `/session` family forms for this
slice. It MUST identify `/session status` as the compact current-session summary view, it MUST
identify `/session inspect` as the richer current-session inspection view, and it MUST identify
`/session list` as the caller-scoped accessible-session listing view. The response MAY mention
adjacent slash-session lifecycle commands such as `/resume`, `/suspend`, `/compact`, and `/tldr`,
but it MUST distinguish them from `/session` subcommands. `/session` root help MUST remain the
family help or usage hub and MUST NOT create, update, suspend, resume, compact, summarize,
inspect, list beyond the current caller scope, or otherwise mutate session records or slash-session
state.

#### Scenario: Root help includes `/session list` without mutation

- GIVEN a current session context exists
- WHEN the user runs `/session`
- THEN the system MUST return read-only help or usage guidance for the `/session` family
- AND the guidance MUST include `/session status` as the compact summary view
- AND the guidance MUST include `/session inspect` as the richer inspection view
- AND the guidance MUST include `/session list` as the accessible-session listing view
- AND no session lifecycle or snapshot state MUST be modified.

### Requirement: Caller-Scoped Session List Discoverability

The system MUST make `/session list` a read-only discoverability view over sessions accessible to
the current caller scope represented by the typed slash command execution context.

The `/session` handler and service boundary MUST preserve sufficient caller-scope context for
`/session list` so the visibility contract can be enforced explicitly rather than inferred from
only the current session identifier. `/session list` MUST list only sessions accessible to that
current caller scope, and it MUST NOT broaden visibility to admin, global, or cross-scope session
inventory. If the runtime cannot establish or preserve sufficient caller-scope facts for this
authorization-sensitive listing operation, it MUST return an explicit denial or unsupported outcome
instead of broadening visibility.

The `/session list` result MUST be ordered by `last_activity DESC`. When two or more visible
sessions share the same `last_activity`, the system MUST apply a stable secondary ordering rule so
repeated executions over unchanged authoritative data return the same row order.

The structured row contract for `/session list` MUST contain only these fields: `id`,
`last_activity`, `lifecycle`, and `resumable`. In that contract, `id` MUST identify the listed
session, `last_activity` MUST reflect the authoritative last-activity timestamp used for ordering,
`lifecycle` MUST reflect the authoritative slash-session lifecycle classification for that session,
and `resumable` MUST indicate whether that session currently has authoritative resume-capable state
available for resume. The command MUST return balanced output consisting of concise human-readable
summary text plus structured row data derived from the same authoritative listing model so both
views remain consistent. `/session list` MUST remain read-only and MUST NOT require or accept
target-session arguments, filters, search, pagination, attach, switch, delete, resume, suspend, or
any other mutation behavior. It MUST NOT expose rich row metadata beyond the minimal row contract.

#### Scenario: Session list returns only caller-visible rows in deterministic order

- GIVEN authoritative session records include `sess-a`, `sess-b`, and `sess-c`
- AND the typed execution context represents a caller scope authorized to view only `sess-a` and
  `sess-c`
- AND `sess-c` has more recent `last_activity` than `sess-a`
- WHEN the user runs `/session list`
- THEN the system MUST return only `sess-c` and `sess-a`
- AND the rows MUST be ordered by `last_activity DESC`
- AND each structured row MUST include only `id`, `last_activity`, `lifecycle`, and `resumable`.

#### Scenario: Stable tiebreaker preserves repeated ordering for equal activity timestamps

- GIVEN the current caller scope is authorized to view sessions `sess-a` and `sess-b`
- AND both sessions have the same authoritative `last_activity` value
- WHEN the user runs `/session list` multiple times without any underlying session changes
- THEN the system MUST return `sess-a` and `sess-b` in the same relative order on each execution
- AND that ordering MUST be produced by a stable secondary ordering rule.

#### Scenario: Missing caller-scope context does not broaden visibility

- GIVEN `/session list` is invoked on a surface where sufficient caller-scope facts are unavailable
  at the `/session` handler or service boundary
- WHEN the runtime evaluates the authorization-sensitive listing request
- THEN the system MUST return an explicit denial or unsupported outcome
- AND the system MUST NOT fall back to listing all sessions or an implementation-defined wider
  scope.

#### Scenario: Empty caller-visible set still returns balanced read-only output

- GIVEN authoritative session records exist
- AND the typed execution context represents a caller scope that is authorized to view none of
  them
- WHEN the user runs `/session list`
- THEN the system MUST return a read-only success result with a human-readable empty-state summary
- AND the structured result MUST contain zero rows
- AND no session lifecycle or snapshot state MUST be modified.

### Requirement: Current Session Status Discoverability

The system MUST make `/session status` a read-only compact summary view over the current session
identified by the typed slash command execution context.

The `/session status` result MUST identify the current session id from the execution context and
MUST derive status from authoritative session records. When a `sessions` table record exists for
that current session id, the result MUST classify the session as `suspended` only when the
dedicated slash-session state record marks the lifecycle as suspended; otherwise it MUST classify
the session as `active`. The result MUST include indicators for whether a latest TLDR snapshot
reference exists and whether a latest compact snapshot reference exists. When the current session
is suspended, the result MUST include the recorded suspension timestamp. When no authoritative
session record exists for the current session id, the result MUST report that the current session
is unknown to slash-session state and MUST NOT invent lifecycle or snapshot data.

The `/session status` result MUST also include exactly one actionable recommendation derived from
the current session state:
- it MUST recommend `/compact` when the current session is active and has no latest compact
  snapshot reference;
- it MUST recommend `/suspend` when the current session is active and already has a latest compact
  snapshot reference;
- it MUST recommend `/resume` when the current session is suspended and has a latest compact
  snapshot reference; and
- it MUST withhold lifecycle-command recommendations when no authoritative current session record
  exists.

`/session status` SHOULD remain concise enough to act as the compact summary view for the
`/session` family, and it MAY direct callers to `/session inspect` when a richer inspection view is
needed. `/session status` MUST NOT mutate session records, slash-session state, or snapshots.

#### Scenario: Status remains the compact summary view

- GIVEN the current session id resolves to an existing active session record
- AND the current session already has authoritative slash-session state available
- WHEN the user runs `/session status`
- THEN the system MUST return a concise current-session summary
- AND the result MAY direct the user to `/session inspect` for richer inspection details
- AND the command MUST NOT mutate session state.

### Requirement: Current Session Inspection Discoverability

The system MUST make `/session inspect` a read-only richer inspection view over the current session
identified by the typed slash command execution context.

The `/session inspect` result MUST be current-session-only and MUST NOT accept or require
target-session arguments. The result MUST combine authoritative data from the current session
record, the dedicated slash-session state record, and any referenced authoritative snapshot rows
that are available for that current session. The result MUST return balanced output consisting of a
human-readable summary plus a structured inspect payload, and both views MUST be derived from the
same authoritative inspect model so they remain consistent. `/session inspect` MUST NOT become a
standalone canonical command or alias, and it MUST NOT mutate session records, slash-session
state, or snapshots.

When a current session record exists but slash-session state or referenced snapshot rows are
missing or incomplete, `/session inspect` MUST return partial data for the current session, MUST
explicitly identify each missing or incomplete data area as a gap, and MUST NOT invent lifecycle,
snapshot, or hydration facts that are not present in authoritative storage. When no authoritative
current session record exists, the result MUST report that the current session is unknown to
slash-session state and MUST NOT invent state or snapshot details.

#### Scenario: Inspect returns a richer current-session view when authoritative data is complete

- GIVEN the typed slash command execution context identifies current session `abc-123`
- AND an authoritative `sessions` row exists for `abc-123`
- AND an authoritative slash-session state row exists for `abc-123`
- AND the referenced authoritative snapshot rows exist for that state
- WHEN the user runs `/session inspect`
- THEN the system MUST return a human-readable inspection summary for `abc-123`
- AND the system MUST return a structured inspect payload for `abc-123`
- AND the structured payload MUST include session record details, slash-session state details, and
  referenced snapshot details derived from authoritative storage
- AND the command MUST NOT mutate session state.

#### Scenario: Inspect returns partial data when slash-session state is missing

- GIVEN the typed slash command execution context identifies current session `abc-123`
- AND an authoritative `sessions` row exists for `abc-123`
- AND no authoritative slash-session state row exists for `abc-123`
- WHEN the user runs `/session inspect`
- THEN the system MUST return the known current session record details for `abc-123`
- AND the result MUST explicitly mark slash-session state as missing
- AND the result MUST explicitly mark snapshot-derived details as unavailable when they depend on
  missing state
- AND the result MUST NOT invent lifecycle or snapshot facts.

#### Scenario: Inspect returns partial data when a referenced snapshot is missing or incomplete

- GIVEN the typed slash command execution context identifies current session `abc-123`
- AND an authoritative `sessions` row exists for `abc-123`
- AND an authoritative slash-session state row exists for `abc-123`
- AND that state references a snapshot row that is missing or incomplete
- WHEN the user runs `/session inspect`
- THEN the system MUST return the known current session record details and slash-session state
  details for `abc-123`
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

---

### SESS-4: Session State Transitions

The runtime MUST support slash-session suspension as an additive lifecycle state while preserving the
existing ended-session contract.

```text
[active] ──(/suspend)──▶ [suspended] ──(/resume)──▶ [active]
   │
   └──(explicit close / auto-close)──▶ [ended]
```

- An **active** session accepts normal runtime turns.
- A **suspended** session remains an existing session identity that is listable and resumable only
  through authoritative suspended state plus a valid resume-capable snapshot.
- An **ended** session remains terminal and MUST NOT be resumed.
- `/resume` MUST reactivate only suspended sessions; it MUST NOT reactivate ended sessions.
- The dedicated slash-session state tables MUST be authoritative for suspended-versus-active resume
  semantics, while `sessions` remains the identity/listing source.

#### Scenario: Ended session cannot be resumed

- GIVEN session `abc-123` is ended
- WHEN the user runs `/resume abc-123`
- THEN the system MUST reject the request as an invalid resume target
- AND the ended session MUST remain ended.

---

### SESS-5: Stale Session Auto-Close

The memory hygiene pass MUST auto-close stale sessions.

- A session is **stale** when `ended_at` IS NULL AND `last_activity` is older than the configured
  threshold.
- The default stale threshold MUST be 24 hours.
- The threshold SHOULD be configurable via runtime config.
- Auto-close MUST set `ended_at` to the current UTC timestamp, not the `last_activity` time.

#### Scenario: Hygiene pass closes stale session

```gherkin
Given an active session "old-session" with last_activity "2026-03-27T08:00:00Z"
And the stale session threshold is 24 hours
When the hygiene pass runs at "2026-03-28T10:00:00Z"
Then session "old-session" MUST have ended_at set to "2026-03-28T10:00:00Z"
```

#### Scenario: Active session within threshold is not closed

```gherkin
Given an active session "recent-session" with last_activity "2026-03-28T09:30:00Z"
And the stale session threshold is 24 hours
When the hygiene pass runs at "2026-03-28T10:00:00Z"
Then session "recent-session" MUST remain active (ended_at IS NULL)
```

#### Scenario: Hygiene pass with no stale sessions

```gherkin
Given all active sessions have last_activity within the stale threshold
When the hygiene pass runs
Then no sessions MUST be modified
And the hygiene pass MUST complete without error
```

---

### SESS-6: Session ID Validation

Session IDs MUST conform to the existing `resolve_session_id()` validation rules:

- Length: 1–64 characters.
- Allowed characters: alphanumeric, `-`, `_`.
- IDs that fail validation MUST be rejected with an appropriate error.

#### Scenario: Valid session ID accepted

```gherkin
Given a request with X-Session-Id header "my-session_01"
When resolve_session_id() processes the header
Then the session ID "my-session_01" MUST be accepted
```

#### Scenario: Session ID exceeding max length rejected

```gherkin
Given a request with X-Session-Id header containing 65 characters
When resolve_session_id() processes the header
Then the request MUST be rejected
And the response MUST indicate an invalid session ID
```

#### Scenario: Session ID with invalid characters rejected

```gherkin
Given a request with X-Session-Id header "session id with spaces!"
When resolve_session_id() processes the header
Then the request MUST be rejected
And the response MUST indicate an invalid session ID
```

---

### SESS-7: Gateway Admin Session Endpoints

The gateway MUST expose admin endpoints for session management.

#### SESS-7.1: List Sessions — `GET /web/admin/sessions`

- MUST require bearer token authentication with admin role.
- MUST return a paginated list of sessions.
- MUST support query parameters:
    - `status`: filter by `active`, `ended`, or `all` (default: `all`)
    - `limit`: max results per page (default: 50, max: 200)
    - `offset`: pagination offset (default: 0)
    - `sort`: `started_at` or `last_activity` (default: `last_activity`)
    - `order`: `asc` or `desc` (default: `desc`)
- Response MUST include `total` count for pagination.

##### Scenario: Admin lists all sessions

```gherkin
Given 3 active sessions and 2 ended sessions exist
And the request has a valid admin bearer token
When GET /web/admin/sessions is called with no filters
Then the response status MUST be 200
And the response MUST contain 5 session objects
And each session object MUST include: id, started_at, ended_at, status, message_count, last_activity
And the response MUST include a "total" field with value 5
```

##### Scenario: Admin filters active sessions only

```gherkin
Given 3 active sessions and 2 ended sessions exist
And the request has a valid admin bearer token
When GET /web/admin/sessions?status=active is called
Then the response MUST contain exactly 3 session objects
And all returned sessions MUST have ended_at as null
```

##### Scenario: Pagination with limit and offset

```gherkin
Given 10 sessions exist
And the request has a valid admin bearer token
When GET /web/admin/sessions?limit=3&offset=0 is called
Then the response MUST contain 3 session objects
And "total" MUST be 10
When GET /web/admin/sessions?limit=3&offset=9 is called
Then the response MUST contain 1 session object
```

##### Scenario: Unauthenticated session list request

```gherkin
Given sessions exist in the database
When GET /web/admin/sessions is called without a bearer token
Then the response status MUST be 401
And no session data MUST be returned
```

##### Scenario: Non-admin session list request

```gherkin
Given sessions exist in the database
And the request has a valid bearer token without admin role
When GET /web/admin/sessions is called
Then the response status MUST be 403
And no session data MUST be returned
```

#### SESS-7.2: Session Detail — `GET /web/admin/sessions/:id`

- MUST require bearer token authentication with admin role.
- MUST return the full session record including metadata.
- MUST include a summary of memory entries associated with the session.
- The memory entry summary MUST include count per category.

##### Scenario: Admin views session detail

```gherkin
Given a session "abc-123" exists with message_count 10
And 5 memory entries are associated with session "abc-123" (3 Conversation, 2 Core)
And the request has a valid admin bearer token
When GET /web/admin/sessions/abc-123 is called
Then the response status MUST be 200
And the response MUST include: id, started_at, ended_at, status, message_count, last_activity, metadata
And the response MUST include a memory_summary with conversation: 3, core: 2
```

##### Scenario: Session detail for nonexistent session

```gherkin
Given no session with id "nonexistent" exists
And the request has a valid admin bearer token
When GET /web/admin/sessions/nonexistent is called
Then the response status MUST be 404
```

---

### SESS-8: End-User Session List Endpoint

The gateway MUST expose a scoped endpoint for end-user session history.

#### `GET /session/list`

- MUST require bearer token authentication (any authenticated user).
- MUST return only sessions belonging to the authenticated user's scope.
- Scoping MUST use the session IDs associated with the user's auth token.
- MUST NOT include memory contents or metadata in the response.
- Response fields per session: `id`, `started_at`, `ended_at`, `message_count`, `last_activity`.
- MUST support `limit` (default: 20, max: 100) and `offset` (default: 0) query parameters.

#### Scenario: End user lists own sessions

```gherkin
Given the authenticated user has 3 sessions
And 10 other sessions exist from other users/sources
When GET /session/list is called with a valid bearer token
Then the response status MUST be 200
And the response MUST contain exactly 3 session objects
And no sessions from other users MUST be included
And no memory content MUST be present in the response
```

#### Scenario: End user with no sessions

```gherkin
Given the authenticated user has no sessions
When GET /session/list is called with a valid bearer token
Then the response status MUST be 200
And the response MUST contain an empty list
And "total" MUST be 0
```

#### Scenario: Unauthenticated end-user session list

```gherkin
Given sessions exist in the database
When GET /session/list is called without a bearer token
Then the response status MUST be 401
```

---

### SESS-9: Memory Trait Session Methods

The session persistence contract MUST be extended so SQLite-backed runtimes can manage authoritative
slash-session snapshot and state operations.

- The runtime MUST provide deterministic methods for:
  - persisting `tldr` and `compact` snapshots;
  - reading the latest authoritative summary and resume snapshot references;
  - suspending a session;
  - listing suspended resumable sessions;
  - resuming a session from persisted state.

For this slice, non-SQLite backends MUST fail these slash-session operations explicitly as
unsupported rather than silently succeeding.

#### Scenario: SQLite backend exposes slash-session persistence operations

- GIVEN the runtime is backed by SQLite
- WHEN `/compact` or `/resume` needs snapshot/state persistence operations
- THEN the runtime MUST provide deterministic session snapshot/state operations that succeed when the underlying data is valid.

#### Scenario: Non-SQLite backend rejects slash-session persistence operations

- GIVEN the runtime is backed by a non-SQLite backend
- WHEN slash-session persistence operations are requested
- THEN the runtime MUST return an explicit unsupported result
- AND the operation MUST NOT report success.

## Change History

| Version | Date       | Changes                                                     |
|---------|------------|-------------------------------------------------------------|
| 1.0.0   | 2026-03-28 | Initial specification from session-memory-visibility change |
