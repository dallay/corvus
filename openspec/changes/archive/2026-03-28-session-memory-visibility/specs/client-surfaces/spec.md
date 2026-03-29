# Spec: Client Surface Requirements

> **Change:** session-memory-visibility
> **Domain:** client-surfaces
> **Status:** Draft
> **Date:** 2026-03-28

---

## Overview

This specification defines the client-side requirements for session and memory visibility across Corvus surfaces: dashboard (operator/admin), chat (end-user), and KMP/mobile (deferred). Each surface has distinct visibility rules based on user role.

---

## Requirements

### CS-1: Dashboard — Session List View

The dashboard MUST include a session monitoring page that displays a paginated table of sessions.

- The page MUST be accessible from the dashboard navigation.
- The table MUST display columns: Session ID, Started, Last Activity, Messages, Status (Active/Ended).
- The table MUST support filtering by status (`active`, `ended`, `all`).
- The table MUST support sorting by `started_at` and `last_activity`.
- The table MUST support pagination (page size selector, next/previous navigation).
- Active sessions SHOULD be visually distinguished from ended sessions (e.g., badge or row highlight).
- The view MUST consume `GET /web/admin/sessions` from the gateway.

#### Scenario: Dashboard displays session list

```gherkin
Given the admin user is authenticated in the dashboard
And 5 active and 3 ended sessions exist
When the admin navigates to the session monitoring page
Then a table MUST display 8 rows
And each row MUST show: session ID, started timestamp, last activity, message count, status
And active sessions MUST be visually distinct from ended sessions
```

#### Scenario: Filter sessions by active status

```gherkin
Given the admin is on the session monitoring page
And 5 active and 3 ended sessions exist
When the admin selects the "Active" status filter
Then the table MUST display only the 5 active sessions
And ended sessions MUST NOT appear
```

#### Scenario: Paginate through sessions

```gherkin
Given 60 sessions exist
And page size is set to 25
When the admin views page 1 of the session list
Then 25 sessions MUST be displayed
And a pagination control MUST indicate 3 total pages
When the admin navigates to page 3
Then 10 sessions MUST be displayed
```

#### Scenario: Empty session list

```gherkin
Given no sessions exist
When the admin navigates to the session monitoring page
Then the table MUST display an empty state message
And the message SHOULD indicate "No sessions found"
```

---

### CS-2: Dashboard — Session Detail View

The dashboard MUST provide a session detail panel accessible by clicking a session row.

- The detail view MUST display: session ID, started_at, ended_at (or "Active"), message_count, last_activity, metadata (if present).
- The detail view MUST display a memory summary: count of memory entries by category for that session.
- The detail view SHOULD provide a link/button to view the session's memory entries in the memory browser (pre-filtered by session_id).
- The detail view MUST consume `GET /web/admin/sessions/:id` from the gateway.

#### Scenario: Admin views session detail

```gherkin
Given session "abc-123" is active with 15 messages and 6 memory entries (4 Conversation, 2 Core)
When the admin clicks session "abc-123" in the session list
Then the session detail panel MUST open
And it MUST show: id "abc-123", message count 15, status "Active"
And it MUST show memory summary: Conversation 4, Core 2
```

#### Scenario: Session detail for ended session

```gherkin
Given session "old-session" ended at "2026-03-27T18:00:00Z" with 30 messages
When the admin clicks session "old-session" in the session list
Then the detail panel MUST display ended_at as "2026-03-27T18:00:00Z"
And status MUST show as "Ended"
```

#### Scenario: Session detail with no memory entries

```gherkin
Given session "empty-session" has 0 memory entries
When the admin views the detail for "empty-session"
Then the memory summary MUST indicate 0 entries
And no memory categories MUST be listed
```

---

### CS-3: Dashboard — Memory Browser

The dashboard MUST include a memory administration page with a searchable, filterable list of memory entries.

- The page MUST be accessible from the dashboard navigation.
- The page MUST display a table/list of memory entries with columns: Key, Category, Timestamp, Session ID, Content (truncated preview).
- The page MUST support filtering by category (Core, Daily, Conversation, Custom).
- The page MUST support filtering by session ID (dropdown or text input).
- The page MUST support full-text search via a search input field.
- The page MUST support pagination.
- Each entry MUST have a "Delete" action (with confirmation dialog).
- The page MUST consume `GET /web/admin/memory` and `DELETE /web/admin/memory/:key`.

#### Scenario: Admin browses memory entries

```gherkin
Given 30 memory entries exist
When the admin navigates to the memory browser page
Then a list MUST display memory entries with key, category, timestamp, content preview
And the list MUST be paginated
```

#### Scenario: Admin searches memory

```gherkin
Given 30 memory entries exist, 3 of which contain "API key rotation"
When the admin enters "API key rotation" in the search field
Then the list MUST display exactly 3 entries
And all entries MUST contain "API key rotation" in their content
```

#### Scenario: Admin filters memory by category

```gherkin
Given 10 Core and 20 Conversation entries exist
When the admin selects "Core" from the category filter
Then the list MUST display exactly 10 entries
```

#### Scenario: Admin deletes a memory entry

```gherkin
Given a memory entry with key "outdated-fact" exists in the browser
When the admin clicks "Delete" on that entry
Then a confirmation dialog MUST appear
When the admin confirms deletion
Then the entry MUST be removed from the list
And a DELETE request MUST be sent to /web/admin/memory/outdated-fact
```

#### Scenario: Admin cancels memory deletion

```gherkin
Given a memory entry with key "important-fact" exists in the browser
When the admin clicks "Delete" on that entry
And a confirmation dialog appears
When the admin cancels the dialog
Then the entry MUST remain in the list
And no DELETE request MUST be sent
```

---

### CS-4: Dashboard — Memory Stats Summary

The dashboard memory browser page MUST display a stats summary panel.

- The panel MUST show: total entry count, entries by category, total sessions, active sessions, backend name, Cerebro status.
- The panel MUST consume `GET /web/admin/memory/stats`.
- The panel SHOULD be displayed above or alongside the memory entry list.

#### Scenario: Memory stats panel displays correctly

```gherkin
Given 50 memory entries (20 Core, 15 Conversation, 10 Daily, 5 Custom)
And 8 total sessions, 3 active
And backend is "sqlite", Cerebro is not configured
When the admin views the memory browser page
Then the stats panel MUST show: 50 total entries
And the panel MUST show category breakdown
And the panel MUST show 8 total sessions, 3 active
And the panel MUST show backend "sqlite"
And the panel MUST show Cerebro as "Not configured"
```

#### Scenario: Stats panel with Cerebro configured

```gherkin
Given Cerebro MCP endpoint is configured
When the admin views the memory stats panel
Then Cerebro status MUST show as "Configured" or equivalent positive indicator
```

---

### CS-5: Chat — Session History Sidebar

The chat app MUST include a collapsible session history sidebar.

- The sidebar MUST list past sessions from `GET /session/list`.
- Each session entry MUST display: session start time (relative or absolute) and message count.
- The current active session MUST be visually highlighted.
- Clicking a past session MUST switch the chat to that session's context.
- The sidebar MUST include a "New Chat" action that creates a new session.
- The sidebar MUST be collapsible to preserve chat viewport space.
- The sidebar MUST NOT display memory contents, keys, or categories.

#### Scenario: Chat sidebar lists past sessions

```gherkin
Given the user is authenticated in the chat app
And the user has 4 past sessions and 1 current active session
When the chat app loads
Then the sidebar MUST display 5 session entries
And the current session MUST be visually highlighted
And each entry MUST show start time and message count
```

#### Scenario: User switches to a past session

```gherkin
Given the sidebar shows sessions including "sess-old" with 12 messages
When the user clicks "sess-old" in the sidebar
Then the chat MUST load the context for session "sess-old"
And the X-Session-Id header MUST be set to "sess-old" for subsequent requests
And "sess-old" MUST become the highlighted session
```

#### Scenario: User starts a new chat

```gherkin
Given the user is in session "current-session"
When the user clicks "New Chat" in the sidebar
Then a new session ID MUST be generated
And the chat messages MUST be cleared
And the new session MUST become the highlighted session in the sidebar
And the previous session MUST appear as a past session in the list
```

#### Scenario: Sidebar with no past sessions

```gherkin
Given the user has no past sessions (first-time use)
When the chat app loads
Then the sidebar MUST display only the current active session
And a "New Chat" button MUST still be available
```

#### Scenario: Collapsed sidebar

```gherkin
Given the sidebar is expanded showing session history
When the user clicks the collapse toggle
Then the sidebar MUST collapse to a minimal width or be hidden
And the chat viewport MUST expand to fill the available space
When the user clicks the expand toggle
Then the sidebar MUST re-expand showing the session list
```

---

### CS-6: Chat — Session Data Persistence

Chat session context MUST be persisted across page reloads.

- The current session ID MUST be stored in `sessionStorage` (existing behavior).
- The session list MUST be fetched from the server via `GET /session/list` on load.
- Messages for the current session MUST continue to use `sessionStorage` persistence (existing behavior).
- When switching sessions, the current session's messages MUST be saved to `sessionStorage` before loading the new session.

#### Scenario: Session list loads from server on mount

```gherkin
Given the user is authenticated
And the user has 3 past sessions on the server
When the chat app mounts
Then useChat MUST call GET /session/list
And the sidebar MUST populate with the 3 sessions plus the current session
```

#### Scenario: Page reload preserves current session

```gherkin
Given the user is in session "my-session" with 5 messages
When the page is reloaded
Then the session ID "my-session" MUST be restored from sessionStorage
And the 5 messages MUST be restored from sessionStorage
And the session list MUST be re-fetched from the server
```

---

### CS-7: Chat — No Memory Visibility

The chat app MUST NOT expose raw memory contents to end users.

- The chat MUST NOT display memory keys, categories, or raw content.
- The chat MAY display subtle "context used" indicators (e.g., "The agent recalled context from a previous session") as a future enhancement — this is NOT required for Phase 1.
- The chat MUST NOT call any `/web/admin/memory*` endpoint.

#### Scenario: Chat does not expose memory data

```gherkin
Given session "abc-123" has 10 associated memory entries
When the user is chatting in session "abc-123"
Then no memory entries, keys, or categories MUST be visible in the chat UI
And no requests to /web/admin/memory endpoints MUST be made
```

---

### CS-8: Dashboard — Admin TypeScript Types

The dashboard MUST define TypeScript types for all new API responses.

- `AdminSessionView`: session list item (id, started_at, ended_at, status, message_count, last_activity).
- `AdminSessionDetail`: extends `AdminSessionView` with metadata and memory_summary.
- `AdminMemoryEntry`: memory entry (id, key, content, category, timestamp, session_id).
- `AdminMemoryStats`: stats response (total_entries, by_category, total_sessions, active_sessions, backend, cerebro_configured).
- Types MUST be defined in the existing `admin-config.ts` or a co-located types file.

#### Scenario: TypeScript types match API response shape

```gherkin
Given the dashboard makes a request to GET /web/admin/sessions
When the response is received
Then the response MUST be parseable as PaginatedResponse<AdminSessionView>
And all fields defined in AdminSessionView MUST be present
```

#### Scenario: Chat types match end-user API shape

```gherkin
Given the chat app makes a request to GET /session/list
When the response is received
Then the response MUST be parseable into the chat SessionListItem type
And the type MUST NOT include admin-only fields (metadata, memory_summary)
```

---

### CS-9: KMP/Mobile — Deferred

KMP and mobile clients (composeApp, androidApp) are OUT OF SCOPE for Phase 1.

- Session history and memory visibility for KMP clients MUST NOT be implemented in this change.
- The KMP `CoreContracts.kt` MAY be updated with session history type stubs if convenient, but this is NOT required.
- The mobile bridge is not wired — session history depends on bridge completion (tracked separately).

#### Scenario: KMP contracts remain unchanged

```gherkin
Given the KMP module CoreContracts.kt
When this change is implemented
Then CoreContracts.kt MUST NOT be modified unless adding optional type stubs
And existing KMP functionality MUST NOT be affected
```

---

### CS-10: Visibility Rules Summary

| Capability | Dashboard (Admin) | Chat (End-User) | KMP/Mobile |
|------------|-------------------|------------------|------------|
| Session list (all) | MUST | - | Deferred |
| Session list (own) | - | MUST | Deferred |
| Session detail | MUST | - | Deferred |
| Memory browser | MUST | MUST NOT | Deferred |
| Memory stats | MUST | MUST NOT | Deferred |
| Memory delete | MUST | MUST NOT | Deferred |
| Memory search | MUST | MUST NOT | Deferred |
| Session switching | - | MUST | Deferred |
| New chat / session | - | MUST | Deferred |

#### Scenario: Admin has full visibility

```gherkin
Given an admin user authenticated in the dashboard
Then the user MUST have access to: session list, session detail, memory browser, memory stats, memory delete
```

#### Scenario: End-user has scoped visibility

```gherkin
Given an end user authenticated in the chat app
Then the user MUST have access to: own session list, session switching, new chat
And the user MUST NOT have access to: memory browser, memory stats, memory delete, all-sessions list
```
