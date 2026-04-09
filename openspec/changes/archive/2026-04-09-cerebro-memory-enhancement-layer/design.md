# Design: Cerebro Memory Enhancement Layer

## Technical Approach

Implement an additive **local-first, remote-enhanced** operator layer on top of the Phase 1
SQLite-backed session/memory visibility baseline.

The gateway will add a dedicated admin-only Cerebro surface under `/web/admin/cerebro/*` with
typed REST contracts, explicit allowlisting, and centralized normalization of Cerebro outcomes. The
dashboard will extend the existing **Sessions** and **Memory** pages with Cerebro-aware panels
instead of replacing the current local SQLite views. Local data remains the operational source of
truth for session lifecycle and baseline memory visibility; Cerebro is treated as an optional,
remote enhancement for semantic recall, drill-in, timeline, stats, and session/context workflows.

Because current repository reality is partial, the design is intentionally capability-first:

- `mem_search`, `mem_get_observation`, `mem_timeline`, and `mem_stats` are treated as primary
  read workflows.
- existing remote write flows (`mem_save`, `mem_update`, `mem_delete`) are exposed through typed
  wrappers but are not required for the first dashboard slice.
- `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and
  `mem_context` are explicitly surfaced as **planned / partially implemented** capabilities with
  normalized `not_implemented` handling rather than hidden.

This design is grounded in:

- `openspec/changes/cerebro-memory-enhancement-layer/proposal.md`
- archived `session-memory-visibility` artifacts as the local baseline
- current main specs in `openspec/specs/cerebro/spec.md`, `openspec/specs/memory-visibility/spec.md`,
  and `openspec/specs/sessions/spec.md`
- current code realities in `clients/agent-runtime/src/gateway/*`,
  `clients/agent-runtime/src/tools/mcp/*`, `clients/agent-runtime/src/agent/memory_loader.rs`, and
  `clients/web/apps/dashboard/src/*`

## Architecture Decisions

### Decision: Add a dedicated gateway Cerebro module instead of expanding `gateway/admin.rs`

**Choice**: Create a new `clients/agent-runtime/src/gateway/cerebro.rs` module and register routes
from `gateway/mod.rs`, while reusing shared admin auth/origin guards from `gateway/utils.rs`.

**Alternatives considered**:
- Keep all handlers inside `gateway/admin.rs`
- Expose a generic `/web/admin/mcp/*` passthrough

**Rationale**:
- `gateway/admin.rs` already holds the Phase 1 session/memory handlers and is large; Cerebro adds a
  second operational surface with different normalization and contract needs.
- A dedicated module keeps the allowlisted proxy boundary explicit.
- A generic MCP passthrough would violate the proposal and broaden the remote execution surface.

### Decision: Use typed REST wrappers, not raw JSON-RPC passthrough

**Choice**: The gateway exposes task-oriented REST endpoints such as search, observation detail,
timeline, stats, capability status, and session/context actions. The gateway translates those
requests to the Cerebro MCP `tools/call` payloads internally.

**Alternatives considered**:
- Forward raw MCP `method/name/arguments`
- Expose one generic `POST /web/admin/cerebro/tool/:name`

**Rationale**:
- The dashboard already consumes typed REST responses from `/web/admin/sessions` and
  `/web/admin/memory*`.
- Typed wrappers let the gateway validate input, redact sensitive failures, normalize statuses, and
  evolve UI contracts without leaking MCP internals.
- A generic tool endpoint would still be too close to arbitrary remote execution.

### Decision: Capability detection is conservative and two-layered

**Choice**: Capability status is computed from:
1. local configuration + reachability checks, and
2. allowlisted tool inventory / safe probes,
with a conservative override for currently planned session/context tools.

**Alternatives considered**:
- Optimistically mark every documented tool as available when Cerebro is configured
- Probe every tool by executing it on startup/capability fetch
- Hardcode all tools as unavailable until upstream server metadata exists

**Rationale**:
- Optimistic status would over-promise features the docs currently mark `NotImplemented`.
- Executing mutating tools for discovery would be unsafe and can create unwanted remote state.
- A conservative matrix gives the dashboard dependable product states today while leaving a clean
  path to remove overrides once upstream implementation matures.

### Decision: Keep typed endpoint-specific payloads and use normalized non-2xx responses for degraded Cerebro states

**Choice**: Cerebro endpoints return typed, endpoint-specific payloads with a `state` field on
success, and use normalized non-2xx responses for degraded remote states such as `unconfigured`,
`unreachable`, `unsupported`, and `not_implemented`. Reserve `400`/`401`/`403` for input and auth
failures, and `502` for malformed upstream payloads.

**Alternatives considered**:
- Return `200 OK` for every Cerebro readiness outcome
- Return raw upstream HTTP/MCP error codes

**Rationale**:
- The dashboard already consumes typed endpoint-specific bodies and normalized degraded responses.
- Returning normalized `503` / `501` for remote availability problems keeps transport failures distinct
  from valid available payloads.
- Auth, input, and malformed-upstream failures remain true request failures and keep standard HTTP semantics.

### Decision: Keep Local Memory and Cerebro Memory as parallel views, not merged rows

**Choice**: Extend the existing **Sessions** and **Memory** dashboard pages with explicit Local vs
`Cerebro` segmentation, badges, and explanatory copy. Local pages remain the default visible state.

**Alternatives considered**:
- Mix local SQLite rows and Cerebro results into a single table
- Create an entirely separate top-level Cerebro page

**Rationale**:
- The proposal requires preserving the distinction between baseline local visibility and remote
  long-term memory.
- Mixed rows would blur authority and confuse operators about which backend owns which data.
- A totally separate page would duplicate navigation instead of enhancing the Phase 1 UX.

### Decision: Extend MCP Cerebro helpers to support live HTTP tool inventory

**Choice**: Expand `clients/agent-runtime/src/tools/mcp/cerebro.rs` (and, if needed,
`tools/mcp/client.rs`) with a small helper for live HTTP inventory/discovery so capability detection
does not depend on the command-based discovery path that current `McpClient::list_tools()` uses.

**Alternatives considered**:
- Keep inventory unavailable and hardcode the entire allowlist forever
- Add a generic new MCP abstraction before shipping the gateway feature

**Rationale**:
- Current live HTTP calls support `tools/call` but not `tools/list`; the design should close that
  gap because capability detection is first-class for this change.
- A narrow Cerebro-focused helper is enough for this phase and avoids a broad MCP refactor.

### Decision: Explicitly model partially implemented session/context tools as “planned” in capability responses and “not_implemented” in action responses

**Choice**: The capability endpoint advertises session/context tools as operator-visible but not yet
ready when current repository/docs reality indicates `NotImplemented`. Action endpoints normalize
upstream `NotImplemented` responses into `status: "not_implemented"` with stable UX copy.

**Alternatives considered**:
- Hide those tools until upstream implementation is complete
- Treat them as generic failures
- Pretend they are available because they exist in the 13-tool inventory

**Rationale**:
- Hiding them would violate the approved aggressive scope.
- Generic failures do not satisfy graceful degradation.
- Inventory presence alone is insufficient because the docs explicitly mark those tools as planned.

## Data Flow

### High-level operator flow

```text
Dashboard Memory/Sessions page
        │
        ├── Local SQLite views (existing Phase 1)
        │       └── /web/admin/memory* + /web/admin/sessions*
        │
        └── Cerebro enhancement panels
                └── /web/admin/cerebro/*
                            │
                            ├── admin auth + loopback origin guard
                            ├── typed validation + allowlist routing
                            ├── capability / error normalization
                            └── Cerebro MCP HTTP tools/call + tools/list
```

### Capability detection flow

```text
Dashboard ── GET /web/admin/cerebro/status ──→ Gateway
  │                                                   │
  │                                                   ├── validate memory.cerebro config
  │                                                   ├── enforce egress/read boundary
  │                                                   ├── fetch live tool inventory
  │                                                   ├── merge with allowlist + planned-tool overrides
  │                                                   └── emit normalized status map
  │
  └── enables / disables Cerebro panels without blocking Local Memory UI
```

### Read workflow sequence: search → detail → timeline

```text
Operator          Dashboard                Gateway Cerebro Layer         Cerebro MCP
   │                  │                             │                         │
   │ search query     │                             │                         │
   │─────────────────►│ POST /cerebro/search        │                         │
   │                  │────────────────────────────►│ validate + normalize    │
   │                  │                             │ tools/call(mem_search)  │
   │                  │                             │────────────────────────►│
   │                  │                             │ compact results         │
   │                  │◄────────────────────────────│                         │
   │ render summaries │                             │                         │
   │ click result     │                             │                         │
   │─────────────────►│ GET /cerebro/observations/:id                         │
   │                  │────────────────────────────►│ tools/call(get_obs)     │
   │                  │                             │────────────────────────►│
   │                  │◄────────────────────────────│ full observation        │
   │ click timeline   │                             │                         │
   │─────────────────►│ GET /cerebro/timeline/:id   │                         │
   │                  │────────────────────────────►│ tools/call(mem_timeline)│
   │                  │                             │────────────────────────►│
   │                  │◄────────────────────────────│ timeline entries        │
```

### Session/context action flow

```text
Session Detail Panel
      │
      ├── reads local session metadata from /web/admin/sessions/:id (authoritative)
      │
      └── Cerebro enhancement card
              ├── reads capability state for mem_context / mem_session_* / mem_save_prompt
              ├── invokes typed Cerebro session/context action only on explicit operator click
              └── renders normalized result:
                    - available
                    - not_implemented
                    - unreachable
                    - unsupported
                    - unconfigured
```

### Normalization path

```text
Gateway handler
   ├── request validation failure         -> HTTP 400
   ├── admin/origin failure               -> HTTP 401/403
   ├── missing Cerebro config             -> 200 { status: "unconfigured" }
   ├── egress/transport failure           -> 200 { status: "unreachable" }
   ├── tool not allowlisted / not in inv. -> 200 { status: "unsupported" }
   ├── upstream NotImplemented            -> 200 { status: "not_implemented" }
   ├── upstream success                   -> 200 { status: "available", data: ... }
   └── malformed upstream payload         -> HTTP 502/500 with redacted diagnostics
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Register `/web/admin/cerebro/*` routes alongside existing admin session/memory routes |
| `clients/agent-runtime/src/gateway/cerebro.rs` | Create | Implement typed admin handlers, request/response DTOs, allowlist mapping, and normalization helpers |
| `clients/agent-runtime/src/gateway/admin.rs` | Modify | Reuse/align shared response helpers where useful and extend Phase 1 stats linkage to capability-first dashboards |
| `clients/agent-runtime/src/gateway/utils.rs` | Reuse/Maybe Modify | Reuse admin auth/origin guards; only extend if a common helper is needed for typed domain responses |
| `clients/agent-runtime/src/tools/mcp/cerebro.rs` | Modify | Add reusable Cerebro client/inventory helpers for tool calls and live HTTP discovery |
| `clients/agent-runtime/src/tools/mcp/client.rs` | Modify | Add HTTP `tools/list` support or a helper used by Cerebro capability detection |
| `clients/agent-runtime/src/tools/mcp/normalize.rs` | Modify | Add canonical Cerebro tool constants, planned-tool set, normalized gateway states, and error classification helpers |
| `clients/agent-runtime/src/agent/memory_loader.rs` | Modify | Keep current session-filter limitation explicit; ensure no new gateway assumptions conflict with the existing remote recall skip for session-scoped turns |
| `clients/agent-runtime/src/gateway/cerebro.rs` tests or `clients/agent-runtime/tests/*` | Create/Modify | Add capability/auth/normalization/inventory/proxy tests |
| `clients/web/apps/dashboard/src/composables/useAdmin.ts` | Modify | Add Cerebro capability/search/detail/timeline/stats/session-context API methods and per-request loading/error state |
| `clients/web/apps/dashboard/src/types/admin-sessions.ts` | Modify | Add typed Cerebro status, tool capability, search result, observation, timeline, stats, and session/context contracts |
| `clients/web/apps/dashboard/src/App.vue` | Modify | Add page-level state for Local vs Cerebro panels and selected observation/session enhancement state |
| `clients/web/apps/dashboard/src/components/memory/MemoryStats.vue` | Modify | Evolve from simple `cerebro_configured` badge to richer Cerebro capability summary |
| `clients/web/apps/dashboard/src/components/memory/CerebroStatusCard.vue` | Create | Render overall Cerebro readiness + per-tool status overview |
| `clients/web/apps/dashboard/src/components/memory/CerebroSearchPanel.vue` | Create | Semantic search UI for `mem_search` |
| `clients/web/apps/dashboard/src/components/memory/CerebroObservationDetail.vue` | Create | Drill-in panel for `mem_get_observation` payloads and optional relationship/ontology insights |
| `clients/web/apps/dashboard/src/components/memory/CerebroTimelinePanel.vue` | Create | Timeline/history visualization for `mem_timeline` |
| `clients/web/apps/dashboard/src/components/memory/MemoryList.vue` | Modify | Keep Local Memory list unchanged as default/fallback while linking to Cerebro exploration mode |
| `clients/web/apps/dashboard/src/components/sessions/SessionDetail.vue` | Modify | Add Cerebro enhancement card for context/session actions while preserving local session metadata as authoritative |
| `clients/web/apps/dashboard/src/components/sessions/CerebroSessionActions.vue` | Create | Encapsulate session/context/prompt actions and normalized planned/unavailable states |
| `clients/web/apps/dashboard/src/composables/useAdmin.spec.ts` | Modify | Cover new typed API methods and domain-state parsing |
| `clients/web/apps/dashboard/src/components/memory/*.spec.ts` | Create/Modify | Add UI state coverage for available/unreachable/not_implemented flows |
| `clients/web/apps/dashboard/src/components/sessions/*.spec.ts` | Create/Modify | Add session enhancement card state coverage |
| `clients/web/apps/docs/src/content/docs/cerebro/mcp-tools.md` | Modify | Clarify dashboard/gateway capability semantics and operator-visible planned-tool states |

## Interfaces / Contracts

### Gateway status model

```ts
export type CerebroGatewayState =
  | "available"
  | "unconfigured"
  | "unreachable"
  | "unsupported"
  | "not_implemented";

export type CerebroToolName =
  | "mem_search"
  | "mem_get_observation"
  | "mem_timeline"
  | "mem_stats"
  | "mem_save"
  | "mem_update"
  | "mem_delete"
  | "mem_save_prompt"
  | "mem_session_start"
  | "mem_session_end"
  | "mem_session_summary"
  | "mem_context";

export interface CerebroToolCapability {
  tool: CerebroToolName;
  state: CerebroGatewayState;
  category: "read" | "write" | "session" | "context" | "prompt";
  detection: "inventory" | "probe" | "planned_override" | "config";
  message?: string;
}

export interface AdminCerebroStatusResponse {
  service_state: CerebroGatewayState;
  tools: Record<CerebroToolName, CerebroToolCapability>;
}
```

### Endpoint strategy

```text
GET  /web/admin/cerebro/status
GET  /web/admin/cerebro/stats
POST /web/admin/cerebro/search
GET  /web/admin/cerebro/observations/:memoryId
POST /web/admin/cerebro/timeline

POST /web/admin/cerebro/memories          -> mem_save
PATCH /web/admin/cerebro/memories/:id     -> mem_update
DELETE /web/admin/cerebro/memories/:id    -> mem_delete

POST /web/admin/cerebro/prompts           -> mem_save_prompt
POST /web/admin/cerebro/sessions/start    -> mem_session_start
POST /web/admin/cerebro/sessions/:session_id/end      -> mem_session_end
POST /web/admin/cerebro/sessions/:session_id/summary  -> mem_session_summary
POST /web/admin/cerebro/context           -> mem_context
```

Notes:

- All routes remain admin-only and reuse existing bearer-token + loopback-origin requirements.
- Requests use typed JSON bodies or query params shaped for dashboard/operator workflows, not raw MCP
  `input` envelopes.
- The gateway internally maps those requests to the Cerebro MCP schema payloads documented under
  `clients/web/apps/docs/src/content/docs/guides/cerebro/mcp-schema/`.

### Typed response contracts

```ts
export interface AdminCerebroActionSuccess {
  state: "available";
  tool: CerebroToolName;
  data: unknown;
}

export interface AdminCerebroActionError {
  state: Exclude<CerebroGatewayState, "available">;
  tool: CerebroToolName;
  message: string;
}
```

Examples:

```json
{
  "state": "available",
  "results": [
    {
      "memory_id": "mem_123",
      "summary": "User prefers dark mode",
      "score": 0.92,
      "topic_key": "preferences",
      "scope": "shared",
      "timestamp": "2026-04-01T12:00:00Z"
    }
  ],
  "truncated": false,
  "results_count": 1
}
```

```json
{
  "state": "not_implemented",
  "tool": "mem_session_summary",
  "message": "Cerebro defines this tool but the current server returns NotImplemented. Local session data remains available."
}
```

### Normalization rules

```text
Input/config problem at gateway
  -> HTTP 400 / 401 / 403

No endpoint or auth token configured
  -> status=unconfigured

Egress denial, timeout, DNS/connectivity failure, HTTP transport failure
  -> status=unreachable

Allowlisted tool absent from live inventory
  -> status=unsupported

Upstream JSON-RPC error contains NotImplemented / known planned-tool marker
  -> status=not_implemented

Valid upstream response
  -> status=available
```

### Handling partially implemented session/context tools

The following tools receive explicit conservative treatment in this phase:

- `mem_save_prompt`
- `mem_session_start`
- `mem_session_end`
- `mem_session_summary`
- `mem_context`

Design rules:

1. They appear in `/web/admin/cerebro/status` even when not ready.
2. Capability responses default them to `planned_override` + `not_implemented` unless the gateway
   has positive evidence of support beyond current repository reality.
3. Session Detail UI renders them as visible actions/cards with explanatory status copy.
4. Operator invocation is explicit and opt-in only; there is no automatic agent-loop dependency.
5. Failures from these tools never blank or degrade the existing local session/memory views.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Gateway normalization helpers map config/transport/inventory/NotImplemented outcomes to stable states | Rust unit tests around new normalization/classification functions in `tools/mcp/normalize.rs` and `gateway/cerebro.rs` |
| Unit | HTTP Cerebro inventory helper discovers tool manifests without leaking auth tokens | Rust tests for new `tools/list` HTTP helper with mock payloads / error payloads |
| Unit | Dashboard composable parses typed capability/action envelopes and preserves separate loading/error buckets | Vitest tests for `useAdmin.ts` |
| Unit | Session detail and memory panels render `available`, `unconfigured`, `unreachable`, `unsupported`, and `not_implemented` states distinctly | Vitest component tests for new Cerebro cards/panels |
| Integration | Admin auth + loopback origin guard still apply to all `/web/admin/cerebro/*` routes | Axum handler tests reusing existing gateway test patterns in `gateway/admin.rs` / new `gateway/cerebro.rs` |
| Integration | Search/detail/timeline/stats wrappers translate typed requests into MCP payloads and normalize upstream failures | Runtime integration tests with mock MCP/Cerebro responses |
| Integration | Session/context routes preserve local session authority while exposing normalized planned-tool states | Runtime handler tests plus dashboard component integration tests |
| E2E | Operator can open dashboard, see Local Memory by default, inspect Cerebro capability status, and safely hit a planned tool without crashing the UI | Dashboard Playwright smoke if harness is available; otherwise defer to targeted Vitest + gateway integration coverage |

## Migration / Rollout

No migration required.

### Rollout

1. Ship gateway capability endpoint and dashboard Cerebro status card first.
2. Ship read-only implemented flows next: `mem_search`, `mem_get_observation`, `mem_timeline`,
   `mem_stats`.
3. Ship write/session/context routes after typed normalization is in place so partially implemented
   flows degrade predictably.
4. Keep Local Memory and Local Session views as the default operator path throughout rollout.

### Rollback

1. Remove `/web/admin/cerebro/*` route registration from `gateway/mod.rs`.
2. Revert new dashboard Cerebro panels/components and restore the current Phase 1-only navigation.
3. Leave existing `memory.cerebro` runtime configuration untouched.
4. No database rollback or data cleanup is required because the enhancement layer is additive and
   remote-state tolerant.

## Open Questions

- [ ] Relationship / ontology payload shape is not yet formalized in the current JSON schemas. The
      design assumes the dashboard treats extra observation metadata as read-only optional insights
      until upstream contracts are stabilized.
- [ ] If upstream Cerebro begins implementing session/context tools without a new explicit metadata
      signal, the conservative `planned_override` list will need a follow-up relaxation strategy.
