# Delta for Memory Visibility

## MODIFIED Requirements

### Requirement: Cerebro Admin Capability Status Endpoint

The system MUST expose an admin-only Cerebro capability endpoint at `GET /web/admin/cerebro/status`.

The endpoint MUST:

- require the same bearer-token authentication, admin-role checks, and admin origin checks as the existing `/web/admin/memory*` endpoints,
- return a typed response instead of raw MCP JSON-RPC output,
- report a top-level `service_state` and per-tool readiness for the gateway-allowed Cerebro tools used by operator memory workflows,
- include the normalized states `available`, `unconfigured`, `unreachable`, `unsupported`, and `not_implemented`,
- treat `mem_save`, `mem_search`, `mem_delete`, `mem_get_observation`, `mem_update`, `mem_suggest_topic_key`, `mem_timeline`, and `mem_stats` as the currently available gateway-allowed tool inventory,
- treat `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and `mem_context` as deferred tools that MAY be reported only with unavailable states such as `not_implemented`, and
- sanitize internal MCP/auth details so that Cerebro endpoint URLs, bearer tokens, and raw JSON-RPC envelopes are never returned to clients.

The status contract MUST map states as follows:

- `unconfigured` — the runtime is missing the Cerebro endpoint and/or auth token configuration.
- `unreachable` — configuration exists but the gateway cannot reach or successfully authenticate to Cerebro within the bounded request window.
- `unsupported` — Cerebro is reachable but the queried tool is absent from the discovered callable inventory or rejected as unsupported by the backend.
- `not_implemented` — Cerebro recognizes the tool but returns its structured NotImplemented outcome.
- `available` — the tool is currently implemented and ready for operator use.

The endpoint MUST NOT report `mem_context` as `available` unless Cerebro actually implements it.

(Previously: The allowlisted inventory treated `mem_session_start`, `mem_session_end`, `mem_session_summary`, `mem_context`, and `mem_save_prompt` as part of the gateway-facing available inventory rather than as deferred tools.)

#### Scenario: Status reports implemented and deferred split accurately

- GIVEN Cerebro is configured and reachable
- AND `mem_save`, `mem_search`, `mem_delete`, `mem_get_observation`, `mem_update`, `mem_suggest_topic_key`, `mem_timeline`, and `mem_stats` succeed
- AND `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and `mem_context` return Cerebro's structured `NotImplemented` error
- WHEN an admin calls `GET /web/admin/cerebro/status`
- THEN `service_state` MUST be `available`
- AND the 8 implemented tools MUST report `available`
- AND each deferred tool MUST report `not_implemented`

#### Scenario: Status must not overstate mem_context availability

- GIVEN Cerebro is configured and reachable
- AND `mem_context` returns Cerebro's structured `NotImplemented` error
- WHEN an admin calls `GET /web/admin/cerebro/status`
- THEN the `mem_context` tool state MUST NOT be `available`
- AND the `mem_context` tool state MUST be `not_implemented` or another unavailable state consistent with the backend result

### Requirement: Allowlisted Typed Cerebro Proxy Endpoints

The system MUST expose admin-only, typed proxy endpoints under `/web/admin/cerebro/*` for approved operator workflows and MUST NOT expose arbitrary MCP passthrough.

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

If deferred workflow endpoints are exposed, they MUST return normalized unavailable behavior and MUST NOT be presented as successful operator workflows.

These endpoints MUST:

- reject request bodies that attempt to pass raw MCP envelopes or arbitrary tool names,
- validate only the typed fields documented for the corresponding operator workflow,
- return typed success/error bodies rather than raw `result` / `error` JSON-RPC objects, and
- preserve existing admin security boundaries and MUST NOT be available on non-admin routes.

(Previously: The proxy requirement listed deferred session, context, and prompt workflows alongside implemented workflows without distinguishing their unavailable status.)

#### Scenario: Implemented typed proxy succeeds

- GIVEN Cerebro is configured and `mem_search` is available
- AND an admin submits `POST /web/admin/cerebro/search` with a typed search payload
- WHEN the gateway proxies the request
- THEN the response status MUST be 200
- AND the response body MUST include a typed search result payload
- AND the response body MUST NOT expose JSON-RPC fields such as `jsonrpc`, `method`, or raw MCP transport metadata

#### Scenario: Deferred context workflow remains unavailable

- GIVEN Cerebro is configured and reachable
- AND `mem_context` returns Cerebro's structured `NotImplemented` error
- WHEN an admin calls `POST /web/admin/cerebro/context`
- THEN the response MUST use the normalized unavailable contract
- AND the workflow MUST NOT be presented as an available operator action
