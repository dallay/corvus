# Delta for Sessions

## ADDED Requirements

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

The `/resume` command MUST support deterministic list and load behavior for suspended sessions.

- When invoked without a target, `/resume` MUST return a list of resumable suspended sessions visible to the authenticated caller based on the existing `sessions` identity/listing records combined with authoritative suspended state.
- When invoked with a target session identifier, `/resume {session_id}` MUST validate that the target
  exists, is suspended, and has a valid resume-capable snapshot.
- On successful resume, the runtime MUST load the authoritative resume snapshot for that session,
  reactivate the session, and return a user-visible result that identifies the resumed session.

#### Scenario: Resume without target lists suspended sessions

- GIVEN suspended sessions `abc-123` and `xyz-789` both have valid resume-capable snapshots and are visible to the authenticated caller
- WHEN the user runs `/resume`
- THEN the system MUST return a deterministic list of resumable suspended sessions limited to sessions owned by or otherwise accessible to that caller
- AND each listed item MUST identify the session and expose enough information for explicit selection.

#### Scenario: Resume target loads snapshot and reactivates session

- GIVEN session `abc-123` is suspended and has a valid authoritative compact snapshot
- WHEN the user runs `/resume abc-123`
- THEN the runtime MUST load the referenced resume snapshot for session `abc-123`
- AND the session state MUST transition from `suspended` to `active`
- AND the user-visible result MUST identify that session `abc-123` was resumed from persisted state.

#### Scenario: Resume target is invalid

- GIVEN no suspended resumable session exists for identifier `missing-session`
- WHEN the user runs `/resume missing-session`
- THEN the system MUST return an invalid-resume-target error result
- AND no other session state MUST be modified.

## MODIFIED Requirements

### Requirement: SESS-4: Session State Transitions

The runtime MUST support slash-session suspension as an additive lifecycle state while preserving the
existing ended-session contract.

The lifecycle model becomes:

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

(Previously: sessions only transitioned one-way from `active` to `ended`, and there was no
suspended/resumed lifecycle.)

#### Scenario: Ended session cannot be resumed

- GIVEN session `abc-123` is ended
- WHEN the user runs `/resume abc-123`
- THEN the system MUST reject the request as an invalid resume target
- AND the ended session MUST remain ended.

### Requirement: SESS-9: Memory Trait Session Methods

The session persistence contract MUST be extended so SQLite-backed runtimes can manage authoritative
slash-session snapshot and state operations.

The runtime MUST provide deterministic methods for:
- persisting `tldr` and `compact` snapshots;
- reading the latest authoritative summary and resume snapshot references;
- suspending a session;
- listing suspended resumable sessions;
- resuming a session from persisted state.

For this slice, non-SQLite backends MUST fail these slash-session operations explicitly as
unsupported rather than silently succeeding.

(Previously: session lifecycle methods defaulted to successful no-op behavior so non-SQLite backends
would not break, and there were no authoritative slash-session snapshot/state operations.)

#### Scenario: SQLite backend exposes slash-session persistence operations

- GIVEN the runtime is backed by SQLite
- WHEN `/compact` or `/resume` requires snapshot/state persistence APIs
- THEN the runtime MUST provide deterministic session snapshot/state operations that succeed when the underlying data is valid.

#### Scenario: Non-SQLite backend rejects slash-session persistence operations

- GIVEN the runtime is backed by a non-SQLite backend
- WHEN slash-session persistence operations are requested
- THEN the runtime MUST return an explicit unsupported result
- AND the operation MUST NOT report success.
