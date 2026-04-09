# Delta for memory-visibility

## ADDED Requirements

### Requirement: Cerebro Admin Capability Status Endpoint

The system MUST expose an admin-only Cerebro capability endpoint at `GET /web/admin/cerebro/status`.

The endpoint MUST:

- require the same bearer-token authentication, admin-role checks, and admin origin checks as the
  existing `/web/admin/memory*` endpoints,
- return a typed response instead of raw MCP JSON-RPC output,
- report a top-level `service_state` and per-tool readiness for the allowlisted Cerebro tools used by
  operator memory workflows,
- include the normalized states `available`, `unconfigured`, `unreachable`, `unsupported`, and
  `not_implemented`,
- treat `mem_search`, `mem_get_observation`, `mem_timeline`, `mem_stats`, `mem_save`,
  `mem_update`, `mem_delete`, `mem_session_start`, `mem_session_end`, `mem_session_summary`,
  `mem_context`, and `mem_save_prompt` as the allowlisted gateway-facing tool inventory,
- sanitize internal MCP/auth details so that Cerebro endpoint URLs, bearer tokens, and raw JSON-RPC
  envelopes are never returned to clients.

The status contract MUST map states as follows:

- `unconfigured` — the runtime is missing the Cerebro endpoint and/or auth token configuration.
- `unreachable` — configuration exists but the gateway cannot reach or successfully authenticate to
  Cerebro within the bounded request window.
- `unsupported` — Cerebro is reachable but the queried tool is absent from the discovered tool
  inventory or rejected as unsupported by the backend.
- `not_implemented` — Cerebro recognizes the tool but returns its structured NotImplemented outcome.
- `available` — the tool is ready for operator use.

#### Scenario: Status reports an unconfigured deployment

- GIVEN the runtime has no valid `memory.cerebro.endpoint` or `memory.cerebro.auth_token`
- WHEN an admin calls `GET /web/admin/cerebro/status`
- THEN the response status MUST be 200
- AND `service_state` MUST be `unconfigured`
- AND every allowlisted tool state MUST be `unconfigured`
- AND the response MUST NOT include raw MCP request or auth details.

#### Scenario: Status reports mixed ready and planned tools

- GIVEN Cerebro is configured and reachable
- AND `mem_search`, `mem_get_observation`, `mem_timeline`, `mem_stats`, `mem_save`, `mem_update`,
  and `mem_delete` succeed
- AND `mem_session_start`, `mem_session_end`, `mem_session_summary`, `mem_context`, and
  `mem_save_prompt` return Cerebro's structured NotImplemented error
- WHEN an admin calls `GET /web/admin/cerebro/status`
- THEN `service_state` MUST be `available`
- AND the implemented tools MUST report `available`
- AND the planned tools MUST report `not_implemented`.

#### Scenario: Status reports a reachable but older backend

- GIVEN Cerebro is configured and reachable
- AND the discovered tool inventory omits `mem_timeline`
- WHEN an admin calls `GET /web/admin/cerebro/status`
- THEN `service_state` MUST remain `available`
- AND the `mem_timeline` tool state MUST be `unsupported`
- AND other discovered tools MUST keep their own normalized states.

### Requirement: Allowlisted Typed Cerebro Proxy Endpoints

The system MUST expose admin-only, typed proxy endpoints under `/web/admin/cerebro/*` for approved
operator workflows and MUST NOT expose arbitrary MCP passthrough.

The gateway MUST provide typed wrappers for at least the following workflows:

- `POST /web/admin/cerebro/search` → `mem_search`
- `GET /web/admin/cerebro/observations/:memory_id` → `mem_get_observation`
- `POST /web/admin/cerebro/timeline` → `mem_timeline`
- `GET /web/admin/cerebro/stats` → `mem_stats`
- `POST /web/admin/cerebro/memories` → `mem_save`
- `PATCH /web/admin/cerebro/memories/:memory_id` → `mem_update`
- `DELETE /web/admin/cerebro/memories/:memory_id` → `mem_delete`
- `POST /web/admin/cerebro/sessions/start` → `mem_session_start`
- `POST /web/admin/cerebro/sessions/:session_id/end` → `mem_session_end`
- `POST /web/admin/cerebro/sessions/:session_id/summary` → `mem_session_summary`
- `POST /web/admin/cerebro/context` → `mem_context`
- `POST /web/admin/cerebro/prompts` → `mem_save_prompt`

These endpoints MUST:

- reject request bodies that attempt to pass raw MCP envelopes or arbitrary tool names,
- validate only the typed fields documented for the corresponding operator workflow,
- return typed success/error bodies rather than raw `result` / `error` JSON-RPC objects,
- preserve existing admin security boundaries and MUST NOT be available on non-admin routes.

#### Scenario: Typed semantic search succeeds without raw MCP passthrough

- GIVEN Cerebro is configured and `mem_search` is available
- AND an admin submits `POST /web/admin/cerebro/search` with a typed search payload
- WHEN the gateway proxies the request
- THEN the response status MUST be 200
- AND the response body MUST include a typed search result payload
- AND the response body MUST NOT expose JSON-RPC fields such as `jsonrpc`, `method`, or raw MCP
  transport metadata.

#### Scenario: Raw tool passthrough is rejected

- GIVEN an admin sends `POST /web/admin/cerebro/search` with fields such as `tool`, `jsonrpc`,
  `method`, or `params`
- WHEN the gateway validates the request
- THEN the response status MUST be 400
- AND the gateway MUST reject the request as an invalid typed proxy payload
- AND no arbitrary Cerebro tool call MUST be executed.

#### Scenario: Non-admin access to Cerebro proxy is denied

- GIVEN a caller is missing admin authorization
- WHEN the caller requests any `/web/admin/cerebro/*` endpoint
- THEN the response MUST be rejected with the same 401/403 behavior used by existing admin memory
  endpoints
- AND no Cerebro tool call MUST be attempted.

### Requirement: Normalized Proxy Error Contract

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

- GIVEN Cerebro is configured and reachable
- AND `mem_session_summary` returns Cerebro's structured NotImplemented error
- WHEN an admin calls `POST /web/admin/cerebro/sessions/abc-123/summary`
- THEN the response status MUST be 501
- AND the response body MUST include `state: "not_implemented"`
- AND the response body MUST identify `mem_session_summary` as the affected tool
- AND the response body MUST contain a user-safe message instead of a raw backend stack trace.

#### Scenario: Reachability failures return normalized unreachable

- GIVEN Cerebro configuration exists
- AND the gateway times out or cannot establish a working Cerebro connection
- WHEN an admin calls `GET /web/admin/cerebro/stats`
- THEN the response status MUST be 503
- AND the response body MUST include `state: "unreachable"`
- AND existing local `/web/admin/memory/stats` behavior MUST remain unaffected.

### Requirement: Local-First Independence for Admin Memory Visibility

The Cerebro enhancement layer MUST NOT replace or weaken the existing local-first admin memory and
session visibility contracts.

- `/web/admin/memory*` and `/web/admin/sessions*` MUST continue to operate when Cerebro is
  unconfigured, unreachable, unsupported, or partially implemented.
- The SQLite-backed admin endpoints MUST remain the source of truth for local session lifecycle and
  local memory visibility.
- Cerebro session/context proxy workflows MUST be additive operator actions; they MUST NOT redefine
  local session creation, update, or ending semantics.

#### Scenario: Local memory visibility still works without Cerebro

- GIVEN Cerebro is not configured
- WHEN an admin calls `GET /web/admin/memory` and `GET /web/admin/memory/stats`
- THEN both responses MUST continue to succeed according to the existing local memory specification
- AND the admin MUST still be able to browse and manage local memory entries.

#### Scenario: Local session detail remains authoritative during Cerebro outage

- GIVEN a local session `abc-123` exists in SQLite
- AND Cerebro is configured but currently unreachable
- WHEN an admin calls `GET /web/admin/sessions/abc-123`
- THEN the local session detail response MUST still succeed
- AND any Cerebro-specific operator action for that session MUST fail independently with a
  normalized `unreachable` outcome.

## MODIFIED Requirements

### Requirement: Memory Visibility Access Control

The existing admin-only memory visibility boundary MUST extend to the Cerebro enhancement layer.

(Previously: Access control covered `/web/admin/memory*` and prevented memory data from appearing on
non-admin endpoints.)

- All `/web/admin/cerebro*` endpoints MUST require the same admin role as `/web/admin/memory*`.
- End-user and non-admin surfaces MUST NOT expose Cerebro capability data, remote memory payloads,
  or session/context action results.
- Cerebro capability/status metadata MUST be treated as admin-only operational information.

#### Scenario: End-user routes do not expose Cerebro capability data

- GIVEN Cerebro is configured and reachable
- WHEN an authenticated end user calls a non-admin endpoint such as `GET /session/list`
- THEN the response MUST NOT contain Cerebro capability state, remote memory summaries, or tool
  readiness metadata.

### Requirement: Response Types

Admin memory visibility contracts MUST include typed Cerebro enhancement responses alongside the
existing local memory response types.

(Previously: Typed contracts covered only local memory entry, local memory stats, and local memory
list responses.)

The system MUST define typed contracts equivalent to the following shapes:

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

#### Scenario: Cerebro status response matches typed contract

- GIVEN an admin requests `GET /web/admin/cerebro/status`
- WHEN the gateway returns a status response
- THEN the response MUST contain `service_state`
- AND the response MUST contain a `tools` object keyed by allowlisted tool names
- AND every tool entry MUST expose a normalized `state`
- AND no raw JSON-RPC envelope fields MUST be present.
