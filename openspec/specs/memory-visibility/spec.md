---
doc_id: memory-visibility
version: 1.3.0
created: 2026-03-28
status: active
owner: architecture
---

# Spec: Memory Visibility

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
- When `q` is provided, the implementation MAY cap the searched candidate set before pagination; if
  capped, `total` MUST reflect the capped match set rather than the full corpus.

#### Scenario: Admin browses all memory entries

```gherkin
Given 25 memory entries exist across categories (10 core, 8 conversation, 7 daily)
And the request has a valid admin bearer token
When GET /web/admin/memory is called with no filters
Then the response status MUST be 200
And the response MUST contain 25 memory entries
And each entry MUST include: id, key, content, category, timestamp, session_id
And "total" MUST be 25
```

#### Scenario: Admin filters by category

```gherkin
Given 10 core and 8 conversation memory entries exist
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
Given 50 memory entries exist (20 core, 15 conversation, 10 daily, 5 custom)
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

### MEM-3A: Cerebro Admin Capability Status Endpoint

The system MUST expose an admin-only Cerebro capability endpoint at `GET /web/admin/cerebro/status`.

The endpoint MUST:

- require the same bearer-token authentication, admin-role checks, and admin origin checks as the
  existing `/web/admin/memory*` endpoints,
- return a typed response instead of raw MCP JSON-RPC output,
- report a top-level `service_state` and per-tool readiness for the gateway-allowed Cerebro tools used
  by operator memory workflows,
- include the normalized states `available`, `unconfigured`, `unreachable`, `unsupported`, and
  `not_implemented`,
- treat `mem_save`, `mem_search`, `mem_delete`, `mem_get_observation`, `mem_update`,
  `mem_suggest_topic_key`, `mem_timeline`, and `mem_stats` as the currently available gateway-allowed
  tool inventory,
- treat `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and
  `mem_context` as deferred tools that MAY be reported only with unavailable states such as
  `not_implemented`,
- sanitize internal MCP/auth details so that Cerebro endpoint URLs, bearer tokens, and raw JSON-RPC
  envelopes are never returned to clients.

The status contract MUST map states as follows:

- `unconfigured` — the runtime is missing the Cerebro endpoint and/or auth token configuration.
- `unreachable` — configuration exists but the gateway cannot reach or successfully authenticate to
  Cerebro within the bounded request window.
- `unsupported` — Cerebro is reachable but the queried tool is absent from the discovered callable
  inventory or rejected as unsupported by the backend.
- `not_implemented` — Cerebro recognizes the tool but returns its structured NotImplemented outcome.
- `available` — the tool is currently implemented and ready for operator use.

The endpoint MUST NOT report `mem_context` as `available` unless Cerebro actually implements it.

#### Scenario: Status reports an unconfigured deployment

```gherkin
Given the runtime has no valid `memory.cerebro.endpoint` or `memory.cerebro.auth_token`
When an admin calls `GET /web/admin/cerebro/status`
Then the response status MUST be 200
And `service_state` MUST be `unconfigured`
And every allowlisted tool state MUST be `unconfigured`
And the response MUST NOT include raw MCP request or auth details.
```

#### Scenario: Status reports implemented and deferred split accurately

```gherkin
Given Cerebro is configured and reachable
And `mem_save`, `mem_search`, `mem_delete`, `mem_get_observation`, `mem_update`,
  `mem_suggest_topic_key`, `mem_timeline`, and `mem_stats` succeed
And `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and
  `mem_context` return Cerebro's structured NotImplemented error
When an admin calls `GET /web/admin/cerebro/status`
Then `service_state` MUST be `available`
And the 8 implemented tools MUST report `available`
And each deferred tool MUST report `not_implemented`.
```

#### Scenario: Status must not overstate mem_context availability

```gherkin
Given Cerebro is configured and reachable
And `mem_context` returns Cerebro's structured NotImplemented error
When an admin calls `GET /web/admin/cerebro/status`
Then the `mem_context` tool state MUST NOT be `available`
And the `mem_context` tool state MUST be `not_implemented` or another unavailable state consistent
  with the backend result.
```

#### Scenario: Status reports a reachable but older backend

```gherkin
Given Cerebro is configured and reachable
And the discovered tool inventory omits `mem_timeline`
When an admin calls `GET /web/admin/cerebro/status`
Then `service_state` MUST remain `available`
And the `mem_timeline` tool state MUST be `unsupported`
And other discovered tools MUST keep their own normalized states.
```

---

### MEM-3B: Allowlisted Typed Cerebro Proxy Endpoints

The system MUST expose admin-only, typed proxy endpoints under `/web/admin/cerebro/*` for approved
operator workflows and MUST NOT expose arbitrary MCP passthrough.

The gateway MUST provide typed wrappers for the currently implemented Cerebro workflows:

- `POST /web/admin/cerebro/search` → `mem_search`
- `GET /web/admin/cerebro/observations/:memory_id` → `mem_get_observation`
- `POST /web/admin/cerebro/timeline` → `mem_timeline`
- `GET /web/admin/cerebro/stats` → `mem_stats`
- `POST /web/admin/cerebro/memories` → `mem_save`
- `PATCH /web/admin/cerebro/memories/:memory_id` → `mem_update`
- `DELETE /web/admin/cerebro/memories/:memory_id` → `mem_delete`

The gateway MAY expose typed placeholders or disabled actions for deferred workflows mapped to:

- `POST /web/admin/cerebro/sessions/start` → `mem_session_start`
- `POST /web/admin/cerebro/sessions/:session_id/end` → `mem_session_end`
- `POST /web/admin/cerebro/sessions/:session_id/summary` → `mem_session_summary`
- `POST /web/admin/cerebro/context` → `mem_context`
- `POST /web/admin/cerebro/prompts` → `mem_save_prompt`

If deferred workflow endpoints are exposed, they MUST return normalized unavailable behavior and MUST
NOT be presented as successful operator workflows.

These endpoints MUST:

- reject request bodies that attempt to pass raw MCP envelopes or arbitrary tool names,
- validate only the typed fields documented for the corresponding operator workflow,
- return typed success/error bodies rather than raw `result` / `error` JSON-RPC objects,
- preserve existing admin security boundaries and MUST NOT be available on non-admin routes.

#### Scenario: Implemented typed proxy succeeds

```gherkin
Given Cerebro is configured and `mem_search` is available
And an admin submits `POST /web/admin/cerebro/search` with a typed search payload
When the gateway proxies the request
Then the response status MUST be 200
And the response body MUST include a typed search result payload
And the response body MUST NOT expose JSON-RPC fields such as `jsonrpc`, `method`, or raw MCP
  transport metadata.
```

#### Scenario: Deferred context workflow remains unavailable

```gherkin
Given Cerebro is configured and reachable
And `mem_context` returns Cerebro's structured NotImplemented error
When an admin calls `POST /web/admin/cerebro/context`
Then the response MUST use the normalized unavailable contract
And the workflow MUST NOT be presented as an available operator action.
```

#### Scenario: Raw tool passthrough is rejected

```gherkin
Given an admin sends `POST /web/admin/cerebro/search` with fields such as `tool`, `jsonrpc`,
  `method`, or `params`
When the gateway validates the request
Then the response status MUST be 400
And the gateway MUST reject the request as an invalid typed proxy payload
And no arbitrary Cerebro tool call MUST be executed.
```

#### Scenario: Non-admin access to Cerebro proxy is denied

```gherkin
Given a caller is missing admin authorization
When the caller requests any `/web/admin/cerebro/*` endpoint
Then the response MUST be rejected with the same 401/403 behavior used by existing admin memory
  endpoints
And no Cerebro tool call MUST be attempted.
```

---

### MEM-3C: Normalized Proxy Error Contract

Every `/web/admin/cerebro/*` proxy endpoint MUST normalize backend availability into a stable typed
contract that dashboard clients can branch on without parsing backend-specific error strings.

For allowlisted proxy endpoints:

- successful execution MUST return HTTP 200 with `state: "available"`,
- `unconfigured` and `unreachable` outcomes MUST return HTTP 503 with the normalized `state`,
- `unsupported` and `not_implemented` outcomes MUST return HTTP 501 with the normalized `state`,
- validation failures in the typed gateway payload MUST return HTTP 400,
- normalized error bodies MUST include the `state`, the target workflow/tool identity, and a
  user-safe message.

#### Scenario: Planned session summary returns normalized not_implemented

```gherkin
Given Cerebro is configured and reachable
And `mem_session_summary` returns Cerebro's structured NotImplemented error
When an admin calls `POST /web/admin/cerebro/sessions/abc-123/summary`
Then the response status MUST be 501
And the response body MUST include `state: "not_implemented"`
And the response body MUST identify `mem_session_summary` as the affected tool
And the response body MUST contain a user-safe message instead of a raw backend stack trace.
```

#### Scenario: Reachability failures return normalized unreachable

```gherkin
Given Cerebro configuration exists
And the gateway times out or cannot establish a working Cerebro connection
When an admin calls `GET /web/admin/cerebro/stats`
Then the response status MUST be 503
And the response body MUST include `state: "unreachable"`
And existing local `/web/admin/memory/stats` behavior MUST remain unaffected.
```

---

### MEM-3D: Local-First Independence for Admin Memory Visibility

The Cerebro enhancement layer MUST NOT replace or weaken the existing local-first admin memory and
session visibility contracts.

- `/web/admin/memory*` and `/web/admin/sessions*` MUST continue to operate when Cerebro is
  unconfigured, unreachable, unsupported, or partially implemented.
- The SQLite-backed admin endpoints MUST remain the source of truth for local session lifecycle and
  local memory visibility.
- Cerebro session/context proxy workflows MUST be additive operator actions; they MUST NOT redefine
  local session creation, update, or ending semantics.

#### Scenario: Local memory visibility still works without Cerebro

```gherkin
Given Cerebro is not configured
When an admin calls `GET /web/admin/memory` and `GET /web/admin/memory/stats`
Then both responses MUST continue to succeed according to the existing local memory specification
And the admin MUST still be able to browse and manage local memory entries.
```

#### Scenario: Local session detail remains authoritative during Cerebro outage

```gherkin
Given a local session `abc-123` exists in SQLite
And Cerebro is configured but currently unreachable
When an admin calls `GET /web/admin/sessions/abc-123`
Then the local session detail response MUST still succeed
And any Cerebro-specific operator action for that session MUST fail independently with a
  normalized `unreachable` outcome.
```

---

### MEM-4: Memory Visibility Access Control

The existing admin-only memory visibility boundary MUST extend to the Cerebro enhancement layer.

- All `/web/admin/memory*` endpoints MUST require admin role.
- All `/web/admin/cerebro*` endpoints MUST require the same admin role as `/web/admin/memory*`.
- End-user and non-admin surfaces MUST NOT expose Cerebro capability data, remote memory payloads,
  or session/context action results.
- Cerebro capability/status metadata MUST be treated as admin-only operational information.

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

#### Scenario: End-user routes do not expose Cerebro capability data

```gherkin
Given Cerebro is configured and reachable
When an authenticated end user calls a non-admin endpoint such as `GET /session/list`
Then the response MUST NOT contain Cerebro capability state, remote memory summaries, or tool
  readiness metadata.
```

---

### MEM-5: Memory Search Behavior

Full-text search via the `q` parameter on `GET /web/admin/memory` MUST use the SQLite FTS5 index.

- The search MUST match against the `content` field of memory entries.
- Search MUST be case-insensitive.
- Results MUST be ranked by relevance (BM25 score) when a search query is provided.
- When `q` is combined with other filters (`category`, `session_id`), all filters MUST be applied
  conjunctively (AND logic).
- The current implementation caps FTS-backed recall results at 200 entries before category filtering
  and pagination, and documented totals MUST be interpreted within that cap.

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

Admin memory visibility contracts MUST include typed Cerebro enhancement responses alongside the
existing local memory response types.

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

#### Additional Cerebro response contracts

```typescript
type CerebroGatewayState =
  | "available"
  | "unconfigured"
  | "unreachable"
  | "unsupported"
  | "not_implemented";

interface AdminCerebroToolStatus {
  state: CerebroGatewayState;
  message?: string;
}

interface AdminCerebroStatusResponse {
  service_state: CerebroGatewayState;
  tools: Record<string, AdminCerebroToolStatus>;
}

interface AdminCerebroSearchResponse {
  state: "available";
  results: Array<{ memory_id: string; summary: string; score?: number; topic_key?: string }>;
  truncated: boolean;
  results_count: number;
}

interface AdminCerebroObservationResponse {
  state: "available";
  observation: Record<string, unknown>;
}

interface AdminCerebroTimelineResponse {
  state: "available";
  items: Array<Record<string, unknown>>;
  items_count: number;
}

interface AdminCerebroStatsResponse {
  state: "available";
  stats: {
    memory_count: number;
    session_count: number;
    prompt_count: number;
    worker_enabled: boolean;
    worker_queue_depth: number;
  };
}

interface AdminCerebroActionError {
  state: Exclude<CerebroGatewayState, "available">;
  tool: string;
  message: string;
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

#### Scenario: Cerebro status response matches typed contract

```gherkin
Given an admin requests `GET /web/admin/cerebro/status`
When the gateway returns a status response
Then the response MUST contain `service_state`
And the response MUST contain a `tools` object keyed by allowlisted tool names
And every tool entry MUST expose a normalized `state`
And no raw JSON-RPC envelope fields MUST be present.
```

---

### MEM-7: Local Visualization Data Boundary

The local memory visibility contract MUST support dashboard visualization v1 using existing local
memory signals and MUST preserve a clear boundary from remote Cerebro semantics.

- MUST treat `GET /web/admin/memory` and `GET /web/admin/memory/stats` as the authoritative
  sources for local memory visualization input.
- MUST rely on returned `session_id`, `category`, `timestamp`, and category totals as the only
  required structural signals for v1 local relationship inference.
- MUST allow the dashboard to derive session-to-entry, category-to-entry, and session-to-category
  views without requiring a new explicit edge-storage contract.
- MUST keep Cerebro capability and proxy endpoints as remote-only workflows that MUST NOT be
  required to render v1 local memory visualization.

#### Scenario: Existing local admin responses are sufficient for v1 visualization

```gherkin
Given the dashboard receives local memory entries from `GET /web/admin/memory`
And the dashboard receives category totals from `GET /web/admin/memory/stats`
When the dashboard shapes data for the local memory visualization
Then it MUST be able to derive timeline groupings from `session_id`
And it MUST be able to derive category distribution from `by_category`
And it MUST NOT require explicit relationship-edge fields in either response.
```

#### Scenario: Local visualization does not depend on Cerebro semantics

```gherkin
Given local memory endpoints are available
And Cerebro is unconfigured, unreachable, unsupported, or not implemented for related workflows
When an operator opens the local memory visualization
Then the local visualization MAY still render from local memory data alone
And the absence of Cerebro semantics MUST NOT block v1 local timeline, category, or inferred
relationship views.
```

## Change History

| Version | Date       | Changes                                                     |
|---------|------------|-------------------------------------------------------------|
| 1.3.0   | 2026-04-28 | Updated Cerebro tool inventory to 8 implemented + 5 deferred, split implemented from planned workflows, added mem_context availability restriction from cerebro-align-mcp-tool-contract-with-implemented-surface-691 change |
| 1.2.0   | 2026-04-09 | Added local visualization data-boundary requirements from dashboard-memory-graph-explorer change |
| 1.1.0   | 2026-04-09 | Added Cerebro admin capability/proxy contracts, normalized error states, local-first independence, and typed Cerebro responses from cerebro-memory-enhancement-layer change |
| 1.0.0   | 2026-03-28 | Initial specification from session-memory-visibility change |
