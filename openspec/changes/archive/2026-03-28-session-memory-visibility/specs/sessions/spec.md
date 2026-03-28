# Spec: Session Lifecycle

> **Change:** session-memory-visibility
> **Domain:** sessions
> **Status:** Draft
> **Date:** 2026-03-28

---

## Overview

This specification defines session lifecycle management for the Corvus runtime. Sessions transition from implicit memory filters to explicit, tracked entities with creation timestamps, activity tracking, and deterministic closure. The SQLite backend is the authoritative source of truth.

---

## Requirements

### SESS-1: Session Table Schema

The runtime MUST maintain a `sessions` table in the SQLite `brain.db` with the following columns:

| Column | Type | Constraints |
|--------|------|-------------|
| `id` | TEXT | PRIMARY KEY, NOT NULL |
| `started_at` | TEXT (ISO 8601) | NOT NULL |
| `ended_at` | TEXT (ISO 8601) | NULL (active sessions) |
| `status` | TEXT | NOT NULL, DEFAULT 'active' |
| `message_count` | INTEGER | NOT NULL, DEFAULT 0 |
| `last_activity` | TEXT (ISO 8601) | NOT NULL |
| `token_hash` | TEXT | NULL |
| `metadata` | TEXT (JSON) | NULL |

The table MUST have indexes on `status`, `started_at`, `last_activity`, and `token_hash` for efficient listing, filtering, and end-user scoping.

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

The runtime MUST explicitly create a session record when a new `session_id` is first used in the agent loop.

- Session creation MUST be idempotent — if a session with the given ID already exists, the existing record MUST be returned without modification (UPSERT semantics on insert).
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

### SESS-4: Session State Transitions

Sessions MUST follow this state model:

```text
[active] ──(explicit close / auto-close)──▶ [ended]
```

- An **active** session has `ended_at` IS NULL.
- An **ended** session has `ended_at` set to a valid ISO 8601 timestamp.
- State transitions MUST be one-way: `active → ended`. There is no reactivation.
- Ending a session MUST set `ended_at` to the current UTC timestamp.

#### Scenario: Explicit session close

```gherkin
Given an active session "abc-123"
When the runtime explicitly ends session "abc-123"
Then ended_at MUST be set to the current UTC timestamp
And the session MUST no longer appear in active session queries
And the session MUST appear in ended session queries
```

#### Scenario: Close an already-ended session

```gherkin
Given session "abc-123" with ended_at "2026-03-28T09:00:00Z"
When the runtime attempts to end session "abc-123" again
Then the operation MUST be idempotent (no error, no change)
And ended_at MUST remain "2026-03-28T09:00:00Z"
```

---

### SESS-5: Stale Session Auto-Close

The memory hygiene pass MUST auto-close stale sessions.

- A session is **stale** when `ended_at` IS NULL AND `last_activity` is older than the configured threshold.
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

The `Memory` trait MUST be extended with session lifecycle methods:

- `upsert_session(session_id: &str, token_hash: Option<&str>) -> Result<()>` — create session or touch existing.
- `end_session(session_id: &str) -> Result<()>` — mark session as ended.
- `update_session_activity(session_id: &str) -> Result<()>` — increment message count and update last_activity.
- `list_sessions(status, limit, offset, sort, order) -> Result<(Vec<SessionEntry>, u64)>` — list sessions with filtering and pagination.
- `get_session(session_id: &str) -> Result<Option<SessionEntry>>` — get a single session by ID.
- `list_sessions_for_token(token_hash: &str, limit, offset) -> Result<(Vec<SessionEntry>, u64)>` — list sessions scoped to a bearer token.
- `memory_stats() -> Result<MemoryStats>` — return aggregated memory and session statistics.

These methods MUST have default implementations that return `Ok` with empty/default values, so that non-SQLite backends (Markdown, None) do not break.

#### Scenario: Non-SQLite backend handles session methods gracefully

```gherkin
Given the memory backend is "markdown"
When upsert_session("abc-123", None) is called
Then the method MUST return Ok
And no error MUST be raised
```

#### Scenario: SQLite backend creates session via trait method

```gherkin
Given the memory backend is "sqlite"
When upsert_session("new-session", None) is called
Then a session record MUST be inserted into the sessions table
And the stored session MUST have id "new-session" and ended_at as None
```
