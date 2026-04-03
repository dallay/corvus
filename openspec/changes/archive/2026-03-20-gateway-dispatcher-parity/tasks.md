# Tasks: Gateway Dispatcher Parity

## Phase 1: Contract Harness and Session Plumbing

- [x] 1.1 Add RED parity tests in `clients/agent-runtime/tests/gateway_webhook_dispatcher_parity.rs`
  that prove `/webhook` uses the canonical dispatcher path, returns canonical terminal outcomes, and
  never claims parity when the legacy fallback path is selected.
- [x] 1.2 Add RED session-scoping tests in `clients/agent-runtime/src/agent/tests.rs` and/or
  `clients/agent-runtime/tests/gateway_webhook_dispatcher_parity.rs` covering explicit
  `X-Session-Id`, missing-session isolation, memory recall scoping, and auto-save session continuity
  for webhook turns.
- [x] 1.3 Extend the canonical turn API in `clients/agent-runtime/src/agent/agent.rs` with a small
  session-aware context/result surface (`TurnContext`, structured terminal outcome/event log fields)
  that can be consumed by gateway without changing CLI behavior.
- [x] 1.4 Update `clients/agent-runtime/src/agent/memory_loader.rs` and any session-aware memory
  write path used by `clients/agent-runtime/src/agent/agent.rs` so webhook turns load and persist
  memory with the resolved session id instead of an unscoped webhook key.
- [x] 1.5 Refactor `clients/agent-runtime/src/agent/agent.rs` tests after GREEN to keep the new turn
  context narrow, preserve existing CLI semantics, and document any new invariants in local test
  helpers only where needed.

## Phase 2: Webhook Dispatcher Adapter and Runtime Wiring

- [x] 2.1 Add RED unit tests in new `clients/agent-runtime/src/gateway/webhook_dispatch.rs` for
  mapping canonical agent results into webhook outcomes: `completed`, `approval_required`,
  `timeout`, `fallback`, and `error`.
- [x] 2.2 Create `clients/agent-runtime/src/gateway/webhook_dispatch.rs` with a gateway-only adapter
  that converts webhook requests into canonical agent turns, captures canonical event logs, and
  returns a structured `WebhookTurnResult` for HTTP mapping.
- [x] 2.3 Modify `clients/agent-runtime/src/bootstrap/mod.rs` so the gateway dispatcher path reuses
  the canonical provider, observer, dispatcher, memory, and tool-registry bootstrap without
  introducing a gateway-only MCP/tool divergence.
- [x] 2.4 Update `clients/agent-runtime/src/gateway/mod.rs` to route admitted `/webhook` requests
  through the adapter when the dispatcher flag is enabled, while preserving auth, pairing,
  webhook-secret validation, rate limiting, and idempotency checks at the HTTP boundary.
- [x] 2.5 Narrow `clients/agent-runtime/src/pre_execution/mod.rs` usage so legacy pre-check helpers
  remain only for the fallback path or compatibility cases, and dispatcher-backed webhook execution
  uses canonical agent approval/risk decisions as the source of truth.

## Phase 3: Response Mapping, Approval Semantics, and Rollout Controls

- [x] 3.1 Add RED HTTP/integration coverage in
  `clients/agent-runtime/tests/gateway_webhook_dispatcher_parity.rs` for webhook response mapping:
  `200` completed, `403` approval-required, `408` timeout-aborted, and `500` non-terminal runtime
  failure, always echoing `session_id`.
- [x] 3.2 Replace the synthetic preview path in `clients/agent-runtime/src/gateway/mod.rs` with
  dispatcher-derived `events_sse` mapping so compatibility frames come only from canonical event
  logs and never from `unified_loop` preview helpers.
- [x] 3.3 Add a dedicated gateway rollout switch in `clients/agent-runtime/src/gateway/mod.rs` and
  related config/env plumbing, default it to legacy-off for safety, and emit structured telemetry
  identifying `legacy_simple_chat` versus `dispatcher_agent` handling.
- [x] 3.4 Preserve retry-safe rollout behavior in `clients/agent-runtime/src/gateway/mod.rs` so
  approval-required and timeout-aborted dispatcher outcomes do not consume idempotency keys, while
  completed legacy and dispatcher turns still keep existing idempotency guarantees.
- [x] 3.5 Add explicit rollback/fallback logging in `clients/agent-runtime/src/gateway/mod.rs` and
  observer hooks so operators can tell why a request used the legacy path and can disable the
  dispatcher flag without changing the external webhook contract.

## Phase 4: MCP Parity and Regression Coverage

- [x] 4.1 Add RED MCP parity tests in `clients/agent-runtime/tests/mcp_policy_approval_parity.rs`
  and/or new `clients/agent-runtime/tests/gateway_webhook_mcp_parity.rs` covering dispatcher-backed
  `/webhook` access to canonical MCP tools, namespaced identities, and policy-visible metadata.
- [x] 4.2 Implement the gateway dispatcher wiring needed in
  `clients/agent-runtime/src/bootstrap/mod.rs`,
  `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and
  `clients/agent-runtime/src/gateway/mod.rs` so MCP-enabled webhook turns use `Provider::chat()`
  plus the canonical dispatcher registry instead of `simple_chat()`.
- [x] 4.3 Add integration coverage in
  `clients/agent-runtime/tests/gateway_webhook_dispatcher_parity.rs` proving approval-required
  native or MCP tool calls return structured blocked responses, do not execute the gated tool, and
  are marked as non-parity when the fallback path is active.
- [x] 4.4 Extend regression tests in `clients/agent-runtime/tests/whatsapp_webhook_security.rs` and
  related gateway coverage to prove `/webhook` transport invariants still hold across both runtime
  paths and `/whatsapp` remains unchanged in this change.

## Phase 5: Verification, Rollout Readiness, and Deferred Scope Guardrails

- [x] 5.1 Run targeted Rust test commands for the new/updated suites under
  `clients/agent-runtime/tests/` plus any affected unit tests in
  `clients/agent-runtime/src/agent/tests.rs`, then record any remaining gaps before broader
  verification.
- [x] 5.2 Run the standard project verification stack required for this repo (`make test`, and
  `make build` if runtime wiring changes require build validation) and confirm the implementation
  satisfies every scenario in `openspec/changes/gateway-dispatcher-parity/specs/agent-loop/spec.md`
  and `openspec/changes/gateway-dispatcher-parity/specs/mcp-runtime/spec.md`.
- [x] 5.3 Update operator-facing notes or inline config documentation near the gateway rollout flag
  definition to describe enablement, rollback, telemetry expectations, and the fact that `/whatsapp`
  remains explicitly deferred and unchanged.
