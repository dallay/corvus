# Spec: Memory Visibility

> **Change:** session-memory-visibility
> **Domain:** memory-visibility
> **Status:** Draft
> **Date:** 2026-03-28

---

## Overview

This specification defines how memory contents are exposed to operator and admin surfaces through
the gateway. Memory visibility is admin-only — end users MUST NOT have direct access to raw memory
entries. All endpoints operate against the local SQLite backend as the authoritative source of
truth.

---

## Requirements

### MEM-1: Memory Browse Endpoint — `GET /web/admin/memory`

The gateway MUST expose a paginated memory browsing endpoint for admin users.

- MUST require bearer token authentication with admin role.
- MUST return memory entries from the local SQLite backend.
- MUST support the following query parameters:
    - `category`: filter by `MemoryCategory` value (`core`, `daily`, `conversation`, `custom`) —
      optional, defaults to all
    - `session_id`: filter by session ID — optional
    - `q`: full-text search query against memory content — optional
    - `limit`: max results per page (default: 50, max: 200)
    - `offset`: pagination offset (default: 0)
    - `sort`: `timestamp` or `key` (default: `timestamp`)
    - `order`: `asc` or `desc` (default: `desc`)
- Each returned entry MUST include: `id`, `key`, `content`, `category`, `timestamp`, `session_id`.
- Response MUST include `total` count for pagination.

#### Scenario: Admin browses all memory entries

```gherkin
Given 25 memory entries exist across categories (10 Core, 8 Conversation, 7 Daily)
And the request has a valid admin bearer token
When GET /web/admin/memory is called with no filters
Then the response status MUST be 200
And the response MUST contain 25 memory entries
And each entry MUST include: id, key, content, category, timestamp, session_id
And "total" MUST be 25
```

#### Scenario: Admin filters by category

```gherkin
Given 10 Core and 8 Conversation memory entries exist
And the request has a valid admin bearer token
When GET /web/admin/memory?category=core is called
Then the response MUST contain exactly 10 entries
And all entries MUST have category "core"
```

#### Scenario: Admin filters by session ID

```gherkin
Given 5 memory entries with session_id "sess-A" and 3 with session_id "sess-B"
And the request has a valid admin bearer token
When GET /web/admin/memory?session_id=sess-A is called
Then the response MUST contain exactly 5 entries
And all entries MUST have session_id "sess-A"
```

#### Scenario: Admin searches memory content

```gherkin
Given memory entries exist, 3 of which contain the text "deployment"
And the request has a valid admin bearer token
When GET /web/admin/memory?q=deployment is called
Then the response MUST contain exactly 3 entries
And all returned entries MUST contain "deployment" in their content
```

#### Scenario: Pagination for memory entries

```gherkin
Given 120 memory entries exist
And the request has a valid admin bearer token
When GET /web/admin/memory?limit=50&offset=0 is called
Then the response MUST contain 50 entries
And "total" MUST be 120
When GET /web/admin/memory?limit=50&offset=100 is called
Then the response MUST contain 20 entries
```

#### Scenario: Combined filters

```gherkin
Given memory entries exist with various categories and sessions
And 4 entries match both category=conversation AND session_id="sess-A"
And the request has a valid admin bearer token
When GET /web/admin/memory?category=conversation&session_id=sess-A is called
Then the response MUST contain exactly 4 entries
```

#### Scenario: Empty result set

```gherkin
Given no memory entries exist
And the request has a valid admin bearer token
When GET /web/admin/memory is called
Then the response status MUST be 200
And the response MUST contain an empty list
And "total" MUST be 0
```

#### Scenario: Unauthenticated memory browse

```gherkin
Given memory entries exist
When GET /web/admin/memory is called without a bearer token
Then the response status MUST be 401
And no memory data MUST be returned
```

#### Scenario: Non-admin memory browse

```gherkin
Given memory entries exist
And the request has a valid bearer token without admin role
When GET /web/admin/memory is called
Then the response status MUST be 403
And no memory data MUST be returned
```

---

### MEM-2: Memory Statistics Endpoint — `GET /web/admin/memory/stats`

The gateway MUST expose an endpoint for aggregated memory statistics.

- MUST require bearer token authentication with admin role.
- Response MUST include:
    - `total_entries`: total memory entry count
    - `by_category`: object mapping category names to entry counts
    - `total_sessions`: total session count (from sessions table)
    - `active_sessions`: count of sessions where ended_at IS NULL
    - `backend`: current memory backend name (e.g., "sqlite", "lucid", "markdown")
    - `cerebro_configured`: boolean indicating if Cerebro MCP endpoint is set

#### Scenario: Admin views memory stats

```gherkin
Given 50 memory entries exist (20 Core, 15 Conversation, 10 Daily, 5 Custom)
And 8 sessions exist (3 active, 5 ended)
And the memory backend is "sqlite"
And Cerebro is not configured
And the request has a valid admin bearer token
When GET /web/admin/memory/stats is called
Then the response status MUST be 200
And total_entries MUST be 50
And by_category MUST be {"core": 20, "conversation": 15, "daily": 10, "custom": 5}
And total_sessions MUST be 8
And active_sessions MUST be 3
And backend MUST be "sqlite"
And cerebro_configured MUST be false
```

#### Scenario: Stats with Cerebro configured

```gherkin
Given memory entries exist
And the Cerebro MCP endpoint is configured
And the request has a valid admin bearer token
When GET /web/admin/memory/stats is called
Then cerebro_configured MUST be true
```

#### Scenario: Stats with empty database

```gherkin
Given no memory entries and no sessions exist
And the request has a valid admin bearer token
When GET /web/admin/memory/stats is called
Then the response status MUST be 200
And total_entries MUST be 0
And by_category MUST be an empty object or all zeros
And total_sessions MUST be 0
And active_sessions MUST be 0
```

#### Scenario: Unauthenticated stats request

```gherkin
When GET /web/admin/memory/stats is called without a bearer token
Then the response status MUST be 401
```

---

### MEM-3: Memory Deletion Endpoint — `DELETE /web/admin/memory/:key`

The gateway MUST expose an admin endpoint to delete individual memory entries.

- MUST require bearer token authentication with admin role.
- The `:key` path parameter identifies the memory entry to delete.
- On success, MUST return 200 with a confirmation body.
- If the key does not exist, MUST return 404.
- Deletion MUST call the existing `Memory::forget()` trait method.

#### Scenario: Admin deletes a memory entry

```gherkin
Given a memory entry with key "user-preference-theme" exists
And the request has a valid admin bearer token
When DELETE /web/admin/memory/user-preference-theme is called
Then the response status MUST be 200
And the memory entry MUST be removed from the backend
And subsequent GET /web/admin/memory requests MUST NOT include the deleted entry
```

#### Scenario: Delete nonexistent memory entry

```gherkin
Given no memory entry with key "nonexistent-key" exists
And the request has a valid admin bearer token
When DELETE /web/admin/memory/nonexistent-key is called
Then the response status MUST be 404
```

#### Scenario: Unauthenticated deletion attempt

```gherkin
Given a memory entry with key "important-data" exists
When DELETE /web/admin/memory/important-data is called without a bearer token
Then the response status MUST be 401
And the memory entry MUST NOT be deleted
```

#### Scenario: Non-admin deletion attempt

```gherkin
Given a memory entry with key "important-data" exists
And the request has a valid bearer token without admin role
When DELETE /web/admin/memory/important-data is called
Then the response status MUST be 403
And the memory entry MUST NOT be deleted
```

---

### MEM-4: Memory Visibility Access Control

Memory contents MUST be restricted to admin users only.

- All `/web/admin/memory*` endpoints MUST require admin role.
- End-user endpoints (e.g., `/session/list`) MUST NOT expose memory content, keys, or categories.
- Memory entries MUST NOT appear in any non-admin API response.

#### Scenario: End-user session list contains no memory data

```gherkin
Given session "abc-123" has 5 associated memory entries
When GET /session/list is called by an authenticated end user
Then session "abc-123" MUST appear in the list
And the session object MUST NOT contain memory entries, keys, or content
And the session object MUST only contain: id, started_at, ended_at, message_count, last_activity
```

#### Scenario: Admin session detail includes memory summary but not raw content

```gherkin
Given session "abc-123" has 5 memory entries
And the request has a valid admin bearer token
When GET /web/admin/sessions/abc-123 is called
Then the response MUST include a memory_summary with entry counts by category
And the response SHOULD NOT include full memory content inline
And the admin SHOULD use GET /web/admin/memory?session_id=abc-123 for full content
```

---

### MEM-5: Memory Search Behavior

Full-text search via the `q` parameter on `GET /web/admin/memory` MUST use the SQLite FTS5 index.

- The search MUST match against the `content` field of memory entries.
- Search MUST be case-insensitive.
- Results MUST be ranked by relevance (BM25 score) when a search query is provided.
- When `q` is combined with other filters (`category`, `session_id`), all filters MUST be applied
  conjunctively (AND logic).

#### Scenario: Full-text search uses FTS5

```gherkin
Given memory entries exist, indexed in FTS5
And 2 entries contain the word "kubernetes" in their content
And the request has a valid admin bearer token
When GET /web/admin/memory?q=kubernetes is called
Then the response MUST return 2 entries
And results SHOULD be ordered by BM25 relevance score
```

#### Scenario: Case-insensitive search

```gherkin
Given a memory entry with content "Deployed to Kubernetes cluster"
And the request has a valid admin bearer token
When GET /web/admin/memory?q=kubernetes is called
Then the response MUST include the entry (case-insensitive match)
```

#### Scenario: Search with no matches

```gherkin
Given no memory entries contain the text "xylophone"
And the request has a valid admin bearer token
When GET /web/admin/memory?q=xylophone is called
Then the response status MUST be 200
And the response MUST contain an empty list
And "total" MUST be 0
```

---

### MEM-6: Response Types

All memory and stats responses MUST conform to typed contracts.

#### `AdminMemoryEntry`

```typescript
interface AdminMemoryEntry {
  id: string;
  key: string;
  content: string;
  category: "core" | "daily" | "conversation" | "custom";
  timestamp: string; // ISO 8601
  session_id: string | null;
}
```

#### `AdminMemoryStats`

```typescript
interface AdminMemoryStats {
  total_entries: number;
  by_category: Record<string, number>;
  total_sessions: number;
  active_sessions: number;
  backend: string;
  cerebro_configured: boolean;
}
```

#### `AdminMemoryListResponse`

```typescript
interface AdminMemoryListResponse {
  entries: AdminMemoryEntry[];
  total: number;
  limit: number;
  offset: number;
}
```

#### Scenario: Memory list response matches AdminMemoryEntry shape

```gherkin
Given memory entries exist
And the request has a valid admin bearer token
When GET /web/admin/memory is called
Then every entry in the "entries" array MUST have all AdminMemoryEntry fields
And "category" MUST be one of: "core", "daily", "conversation", "custom"
And "timestamp" MUST be a valid ISO 8601 string
And "session_id" MUST be a string or null
```

#### Scenario: Stats response matches AdminMemoryStats shape

```gherkin
Given the request has a valid admin bearer token
When GET /web/admin/memory/stats is called
Then the response MUST have all AdminMemoryStats fields
And total_entries MUST be a non-negative integer
And active_sessions MUST be less than or equal to total_sessions
And backend MUST be a non-empty string
```
