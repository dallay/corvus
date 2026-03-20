# Design: Gateway Dispatcher Parity

## Technical Approach

Move gateway `/webhook` execution from the current transport-guarded `Provider::simple_chat()` path
to the same dispatcher-backed turn execution used by the canonical runtime, while keeping gateway-
specific HTTP, auth, rate-limit, idempotency, and response-shaping responsibilities inside the
gateway module.

The implementation should reuse the existing bootstrap path for provider, memory, observer,
security, and tool registry construction (`clients/agent-runtime/src/bootstrap/mod.rs`) and the
existing canonical `Agent` turn loop (`clients/agent-runtime/src/agent/agent.rs`). Gateway should
not create a second runtime contract. Instead, it should adapt inbound webhook requests into a
canonical turn request, then map the canonical result back into gateway JSON and optional SSE-
compatible event output.

This design intentionally preserves a narrow transport shim:
- Gateway owns request authentication, pairing, webhook secret validation, rate limiting,
  idempotency, and HTTP status/JSON mapping.
- Canonical runtime owns prompt preparation, memory loading, dispatcher selection, tool registry,
  MCP availability, tool execution, approval/risk gates, compaction, retries/fallbacks, and final
  response semantics.
- Any remaining response-format differences are transport compatibility behavior, not alternate
  runtime behavior.

## Architecture Decisions

### Decision: Reuse the canonical `Agent` turn loop instead of expanding the preview loop helpers

**Choice**: Execute `/webhook` through a gateway adapter around `Agent::turn()` and related turn
primitives, not through `agent::unified_loop` / `unified_entrypoint` preview helpers.

**Alternatives considered**:
- Expand `agent::unified_loop` into the production dispatcher runtime
- Keep `simple_chat()` as the main path and only improve preview/guard behavior
- Extract a larger shared runtime abstraction for channels and gateway first

**Rationale**: `unified_loop` and `unified_entrypoint` are test/preview scaffolding today, while the
real canonical behavior lives in `Agent` (`prepare_turn`, `step`, gated tool execution, MCP-aware
registry, strict validation). Reusing `Agent` gives true parity with the least semantic drift and
avoids creating a third execution model.

### Decision: Keep gateway transport/security concerns outside the dispatcher boundary

**Choice**: Preserve `handle_webhook()` as the HTTP boundary for pairing, secret validation, rate
limit checks, idempotency checks, session-id normalization, and response shaping, then call a
gateway-specific dispatcher adapter after those checks pass.

**Alternatives considered**:
- Push transport concerns into `Agent`
- Introduce a global entrypoint abstraction that owns both HTTP and runtime semantics

**Rationale**: Current gateway code already centralizes security-sensitive HTTP concerns in
`clients/agent-runtime/src/gateway/mod.rs`. Keeping those concerns at the edge preserves secure-by-
default behavior, limits blast radius, and cleanly separates transport exceptions from canonical
runtime guarantees.

### Decision: Add explicit session-aware turn context to `Agent`

**Choice**: Extend the canonical agent path with a small turn/session context object so callers can
provide a concrete `session_id`, external conversation history scope, and memory session scope.

**Alternatives considered**:
- Keep webhook stateless and only echo `X-Session-Id`
- Store gateway session state separately from the canonical runtime
- Encode the session id only in memory keys without changing agent interfaces

**Rationale**: Current `Agent` auto-save and memory loading are mostly unscoped (`None` session for
memory loader and stores). Webhook parity requires `X-Session-Id` to affect memory association,
conversation continuity, and audit/observability correlation. The narrowest durable fix is to let
the canonical turn receive an explicit session context rather than bolting session semantics onto
gateway-only code.

### Decision: Approval parity is synchronous deny-or-complete for webhook

**Choice**: For `/webhook`, approval-required dispatcher outcomes terminate the request immediately
with a structured denial payload and no resumable approval protocol in this change.

**Alternatives considered**:
- Block the HTTP request waiting for interactive approval
- Add a resumable approval token/protocol as part of this change
- Auto-approve gateway requests to preserve previous simplicity

**Rationale**: The current canonical pre-check path already models synchronous approval gating for
gateway via structured denial. A resumable approval workflow is a larger product/protocol change and
is explicitly out of scope. Synchronous denial preserves security parity at the dispatcher boundary
without inventing new transport semantics.

### Decision: Preserve synchronous JSON response as the default webhook contract

**Choice**: Keep `/webhook` returning a synchronous JSON body with `response`, `model`, and
`session_id` by default. When preview/stream-compatible output is enabled, attach dispatcher-derived
event frames in `events_sse`; do not make long-lived SSE transport mandatory in this change.

**Alternatives considered**:
- Replace `/webhook` with real-time SSE-only responses
- Emit raw internal dispatcher events directly in JSON
- Keep synthetic preview frames generated from `unified_loop`

**Rationale**: Existing webhook consumers already expect a JSON response. Replacing that contract
would create unnecessary compatibility risk. Using real dispatcher-derived events instead of the
synthetic preview path closes the semantic gap while keeping future real SSE support possible.

### Decision: Roll out behind a gateway-scoped dispatcher flag with legacy fallback

**Choice**: Add an explicit gateway runtime flag that selects between legacy `simple_chat()` and the
new dispatcher-backed webhook path, plus comparative observability that records which path handled a
request and why a fallback occurred.

**Alternatives considered**:
- Hard-cut all webhook traffic to the new path
- Shadow-execute both paths for every request
- Reuse the preview flag as the rollout control

**Rationale**: The existing preview flag controls synthetic event output, not runtime selection. A
dedicated rollout switch makes rollback safe, keeps semantics obvious, and avoids binding migration
control to a debugging feature.

## Data Flow

### End-to-End `/webhook` Flow After Change

1. `handle_webhook()` validates transport/auth concerns:
   - pairing / bearer token
   - `X-Webhook-Secret`
   - client rate limit
   - JSON body parsing
   - `X-Session-Id` normalization or generated fallback
2. Gateway checks idempotency before runtime invocation for normal execution; blocking outcomes that
   intentionally do not complete a turn (approval required, timeout abort) must not consume the key.
3. Gateway builds a `WebhookTurnRequest` and calls the dispatcher adapter.
4. The adapter constructs or reuses a canonical `Agent` with the bootstrapped provider, observer,
   memory, tool registry, and dispatcher mode selection.
5. The canonical turn executes:
   - session-aware memory recall/context assembly
   - model request via `Provider::chat(...)`
   - dispatcher parsing and tool execution
   - risk/approval gates using the standard dispatcher policy
   - MCP tool availability identical to other dispatcher-backed entry points
   - strict validation and final assistant message construction
6. The adapter returns a `WebhookTurnResult` with:
   - terminal outcome (`completed`, `approval_required`, `timeout`, `fallback`, `error`)
   - final text when available
   - structured denial metadata when blocked
   - dispatcher event transcript for optional SSE-compatible mapping
   - session/model metadata for response shaping and observability
7. Gateway maps that result to HTTP JSON (and optional `events_sse`) and records path/outcome
   telemetry.

Sequence diagram:

```text
Client
  |
  | POST /webhook
  v
Gateway handle_webhook
  |-- auth/pairing/rate-limit/idempotency/session-id -->
  |-- WebhookDispatcherAdapter::execute(request) ------>
  |                                                    Canonical Agent turn
  |                                                    |-- memory recall/load --> Memory
  |                                                    |-- tool registry ------> Bootstrap tools
  |                                                    |-- model call ---------> Provider::chat
  |                                                    |-- tool gating --------> Dispatcher/Security
  |                                                    |-- tool exec ----------> Tools/MCP
  |<-------------------- WebhookTurnResult ------------|
  |-- JSON / events_sse mapping -->
  v
HTTP response
```

### Dispatcher Boundary vs Gateway Boundary

Gateway boundary responsibilities:
- request auth and secret validation
- pairing enforcement
- IP/header-aware rate limiting
- idempotency policy
- request parsing and session-id normalization
- response status/body mapping
- feature-flag routing and rollback selection

Dispatcher boundary responsibilities:
- prompt construction
- memory/session context propagation
- provider `chat` request formation
- dispatcher parsing and tool registry use
- MCP tool parity
- approval/risk decisions
- tool execution, retries, fallback semantics, strict validation

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Replace direct `simple_chat()` webhook execution with dispatcher adapter invocation; keep auth, rate-limit, idempotency, session normalization, and HTTP mapping here. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Create | Gateway-only adapter that translates webhook requests/responses to and from the canonical `Agent` turn contract, including event capture for SSE-compatible output. |
| `clients/agent-runtime/src/agent/agent.rs` | Modify | Add explicit session-aware turn execution surfaces and expose structured turn outcomes needed by gateway without changing CLI semantics. |
| `clients/agent-runtime/src/agent/memory_loader.rs` | Modify | Allow memory recall to receive an optional `session_id` so webhook turns can scope memory loading to the supplied/generated session. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Reuse canonical bootstrap context for gateway dispatcher execution and avoid any gateway-only tool registry divergence. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Narrow / Modify | Reduce webhook use of synthetic canonical pre-checks once dispatcher-backed execution is primary; retain only compatibility helpers still needed for non-dispatcher fallback. |
| `openspec/specs/agent-loop/spec.md` | Modify | Remove the webhook exception and define parity at the dispatcher boundary while preserving transport-specific response shims. |
| `openspec/specs/mcp-runtime/spec.md` | Modify | Extend MCP parity expectations to gateway webhook once dispatcher-backed execution is enabled. |

## Interfaces / Contracts

Proposed gateway adapter contract:

```rust
pub struct WebhookTurnRequest {
    pub session_id: String,
    pub message: String,
    pub include_sse_frames: bool,
}

pub enum WebhookTerminalOutcome {
    Completed,
    ApprovalRequired { tool: String, reason: String },
    Timeout,
    Fallback,
    Error,
}

pub struct WebhookTurnResult {
    pub session_id: String,
    pub model: String,
    pub outcome: WebhookTerminalOutcome,
    pub response_text: Option<String>,
    pub event_frames: Vec<String>,
}
```

Proposed canonical turn context extension:

```rust
pub struct TurnContext {
    pub session_id: Option<String>,
    pub origin: ExecutionOrigin,
}

pub struct AgentTurnResult {
    pub session_id: Option<String>,
    pub final_text: Option<String>,
    pub approval_required: Option<serde_json::Value>,
    pub timeout_aborted: bool,
    pub used_fallback: bool,
    pub event_log: Vec<CanonicalTurnEvent>,
}
```

Response mapping contract for `/webhook`:
- `200 OK` + `response` for completed turns
- `403 Forbidden` + structured `error` payload for approval-required tool actions
- `408 Request Timeout` + `aborted: true` for canonical timeout aborts
- `500 Internal Server Error` only for transport/runtime failures that do not produce a canonical
  terminal outcome
- `session_id` MUST always be echoed in terminal responses
- `events_sse` MAY be included when preview/stream-compatible mode is enabled, but the frames MUST
  be derived from the real canonical execution path, not synthetic preview helpers

## Session Model and Memory Propagation

- `X-Session-Id` remains the caller-provided session handle when valid.
- Missing or invalid headers continue to generate `webhook-{uuid}` so every dispatcher-backed turn
  has a concrete session identifier.
- The resolved `session_id` MUST flow into:
  - memory recall for context loading
  - memory auto-save writes for conversation/audit continuity
  - any gateway-side conversation history cache if one is introduced for multi-turn continuity
  - observer/audit correlation fields where available
- Auto-save keys should stop using a single global webhook memory key for dispatcher-backed turns.
  Use session-scoped turn keys so one webhook conversation cannot overwrite another.
- The design does not require persistent long-lived gateway conversation storage outside existing
  memory backends; session continuity may be achieved by session-scoped memory recall plus canonical
  history assembly.

## Approval Handling

- Dispatcher risk evaluation becomes the source of truth for `/webhook`.
- Gateway MUST NOT bypass approval checks for native or MCP tools.
- If the canonical turn produces an approval-required result, the request terminates synchronously
  with a structured denial payload and no tool execution.
- Gateway MUST NOT hold the HTTP request open waiting for human approval in this change.
- No resumable token, callback URL, or separate approval endpoint is introduced here.
- Existing env-driven approval override behavior may remain as an operator/testing compatibility
  control, but it must affect the dispatcher path rather than only the synthetic pre-check path.

## Response Mapping and Streaming Strategy

- Default behavior stays synchronous JSON to protect existing webhook clients.
- The old `CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW` synthetic preview path should be replaced by a mode
  that captures actual dispatcher events from the webhook turn and serializes them through the
  existing `map_loop_event_to_sse_frame(...)`-style mapper.
- The event mapper should be transport-neutral: the same canonical event transcript can be emitted
  either as:
  - `events_sse: ["event: ..."]` in JSON for compatibility, or
  - a future `text/event-stream` response without changing dispatcher semantics.
- This change does not require fully streamed live SSE delivery. It only requires that any SSE-
  compatible output come from the canonical execution path.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Webhook adapter outcome mapping | Cover completed, approval-required, timeout, fallback, and internal error result mapping to HTTP responses. |
| Unit | Session propagation | Verify session-aware memory recall/store calls receive the normalized/generated `session_id`. |
| Unit | Event mapping | Verify `events_sse` frames are generated from canonical dispatcher events, not synthetic preview output. |
| Integration | Dispatcher parity for `/webhook` | Exercise a webhook request through the canonical tool loop with a mock provider and mock tools; assert `Provider::chat()` is used, not `simple_chat()`. |
| Integration | MCP parity | With MCP enabled, verify webhook can surface or block `mcp.*` tool calls under the same dispatcher policy as CLI/channels. |
| Integration | Approval constraints | Verify approval-required tool calls return `403`, preserve `session_id`, and do not consume idempotency keys. |
| Integration | Rollout/fallback | Verify the feature flag cleanly selects legacy vs dispatcher-backed runtime and records the chosen path in telemetry. |
| Regression | Gateway transport invariants | Preserve pairing, webhook-secret auth, rate limits, and idempotency behavior regardless of runtime selection. |

## Migration / Rollout

- Add a dedicated gateway-scoped dispatcher flag, for example `gateway.webhook_dispatcher_enabled`
  plus an env override for fast rollback in operations.
- Default the flag to `false` initially.
- When disabled:
  - keep the current `simple_chat()` path
  - retain current auth/rate-limit/idempotency behavior
- When enabled:
  - route `/webhook` through the dispatcher adapter
  - emit runtime-path telemetry (`legacy_simple_chat` vs `dispatcher_agent`)
  - emit structured outcome counters (`completed`, `approval_required`, `timeout`, `fallback`,
    `error`)
- Preserve a one-step rollback: disable the flag and return to legacy execution without changing the
  external webhook contract.

## Observability and Risk Mitigation

- Add per-request logs/metrics identifying runtime path, terminal outcome, and whether fallback was
  used; never log raw prompt secrets.
- Continue using existing observer events, but tag or extend them so gateway dispatcher requests can
  be separated from legacy webhook traffic during rollout.
- Add explicit logging when the system falls back to the legacy webhook path because the dispatcher
  flag is off or a guarded rollback condition is active.
- Preserve idempotency semantics for non-terminal dispatcher-blocked requests so approval-required or
  timeout-aborted turns do not poison caller retries.
- Prefer fail-closed behavior on approval/policy ambiguity and fail-open only for the existing
  feature-flag rollback path controlled by operators.

## Out of Scope

- `WhatsApp` parity is explicitly out of scope for implementation in this change.
- `WhatsApp` may be referenced only as a deferred follow-up surface in parity documentation and
  rollout planning.
- No redesign of admin, pairing, tunnel, or unrelated gateway endpoints.
- No resumable approval protocol.
- No broad channels/gateway shared-runtime refactor beyond the minimal adapter needed for webhook
  parity.

## Open Questions

- [ ] Should session-scoped webhook conversation continuity rely only on memory recall, or should a
      lightweight persistent conversation history store also be introduced for higher fidelity with
      CLI/channel multi-turn history?
- [ ] Which exact config/env names should be standardized for the dispatcher rollout flag and the
      compatibility event-output mode?
- [ ] Does the observer model need a first-class `session_id` or `entry_point` field to make
      rollout comparison reliable without parsing logs?
