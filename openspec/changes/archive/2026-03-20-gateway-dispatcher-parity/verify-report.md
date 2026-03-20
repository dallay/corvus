# Verification Report

**Change**: `gateway-dispatcher-parity`
**Mode**: `openspec`
**Date**: 2026-03-20

---

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 18 |
| Tasks complete | 18 |
| Tasks incomplete | 0 |

All checklist items in `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/tasks.md` are marked complete.

---

## Build, Test, and Coverage Execution

**Configured verify commands** from `openspec/config.yaml`:

- Test: `make test`
- Build: `make build`
- Coverage threshold: `60%`

### Executed

1. `make test` -> exit `0`
2. `make build` -> exit `0`
3. `make test-coverage` -> exit `0`
4. Targeted Rust verification:
    - `cargo test webhook_dispatcher && cargo test turn_with_context_ && cargo test --test mcp_policy_approval_parity && cargo test --test whatsapp_webhook_security` -> exit `101` once due an intermittent failure in `config::schema::tests::env_override_gateway_webhook_dispatcher` under the `src/main.rs` test binary
    - Follow-up reruns all passed:
      - `cargo test turn_with_context_` -> exit `0`
      - `cargo test --test mcp_policy_approval_parity` -> exit `0`
      - `cargo test --test whatsapp_webhook_security` -> exit `0`
      - `cargo test config::schema::tests::env_override_gateway_webhook_dispatcher -- --exact` -> exit `0`
5. Explicit Rust hygiene checks for `clients/agent-runtime/**/*.rs`:
   - `cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check` -> exit `1` during later review because the workspace references a pre-existing missing file (`modules/cerebro/src/bin/cerebro.rs`); this was not caused by the `clients/agent-runtime/**/*.rs` edits.
   - `cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings` -> exit `0`
   - `cargo test --manifest-path "clients/agent-runtime/Cargo.toml"` -> exit `0`

### Build / Test Evidence

- `make test`: passed (`BUILD SUCCESSFUL`), but this repo command only exercised Gradle/JVM test tasks and did not run the Rust webhook parity suites directly.
- `make build`: passed (`BUILD SUCCESSFUL`). Gradle reported `:agent-runtime:cargoTest SKIPPED`, so Rust runtime proof still depended on direct `cargo test` execution.
- `cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check`: failed with exit `1` because of a workspace-level missing file reference outside `clients/agent-runtime/**/*.rs`; formatting of the changed Rust files was still verified separately before merge.
- `cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings`: passed with exit `0`.
- `cargo test --manifest-path "clients/agent-runtime/Cargo.toml"`: passed with exit `0`.
- `make test-coverage`: passed and produced `coverage/agent-runtime-coverage.lcov`.
- Coverage run reported zero Rust test failures, including:
  - `src/lib.rs`: `2415 passed, 0 failed`
  - `src/main.rs`: `2427 passed, 0 failed`
- Targeted behavior proof passed for the implemented parity path, including:
  - `gateway::tests::webhook_dispatcher_flag_routes_through_canonical_chat_path`
  - `gateway::tests::webhook_dispatcher_config_flag_routes_through_canonical_chat_path`
  - `gateway::tests::webhook_dispatcher_preview_returns_canonical_event_frames`
  - `gateway::tests::webhook_dispatcher_executes_allowed_tool_and_returns_completed_response`
  - `gateway::tests::webhook_dispatcher_blocks_native_tool_and_keeps_idempotency_retryable`
  - `gateway::tests::webhook_dispatcher_blocks_mcp_tool_with_structured_denial`
  - `gateway::tests::webhook_dispatcher_returns_500_with_session_id_on_runtime_error`
  - `gateway::tests::webhook_without_dispatcher_flag_stays_on_legacy_simple_chat_path`
  - `gateway::tests::webhook_rollout_observability_distinguishes_dispatcher_and_legacy_requests`
  - `gateway::tests::legacy_webhook_with_mcp_enabled_marks_parity_inactive`
  - `gateway::tests::webhook_dispatcher_rollout_flag_does_not_change_whatsapp_behavior`
  - `gateway::tests::webhook_dispatcher_keeps_secret_auth_before_runtime_execution`
  - `agent::tests::turn_with_context_scopes_memory_recall_and_auto_save_to_session`
  - `agent::tests::turn_with_context_keeps_missing_session_isolated`
  - `agent::tests::turn_with_context_reports_approval_required_payload_for_blocked_tool`
  - `bootstrap::tests::gateway_bootstrap_reuses_canonical_mcp_tool_registry`

### Coverage

- Rust LCOV report: `coverage/agent-runtime-coverage.lcov`
- Total Rust coverage: `75.70%` (`51066 / 67459` lines) -> above configured `60%`
- Changed-file coverage:
  - `clients/agent-runtime/src/agent/agent.rs`: `86.78%`
  - `clients/agent-runtime/src/agent/memory_loader.rs`: `74.81%`
  - `clients/agent-runtime/src/bootstrap/mod.rs`: `90.98%`
  - `clients/agent-runtime/src/gateway/mod.rs`: `84.50%`
  - `clients/agent-runtime/src/gateway/webhook_dispatch.rs`: `84.29%`
  - `clients/agent-runtime/src/config/schema.rs`: `94.05%`

---

## Correctness (Static Structural Evidence)

| Requirement | Status | Evidence |
|------------|--------|----------|
| Entry Points Alignment | ✅ Implemented | `/webhook` selects dispatcher routing in `clients/agent-runtime/src/gateway/mod.rs:1600` and executes canonical turns via `Agent::turn_with_context(...)` in `clients/agent-runtime/src/gateway/webhook_dispatch.rs:344`. |
| Gateway Transport Boundary Preservation | ✅ Implemented | Auth, request parsing, idempotency, and gateway rejection paths stay at the HTTP edge before runtime dispatch in `clients/agent-runtime/src/gateway/mod.rs:1587`, `clients/agent-runtime/src/gateway/mod.rs:1604`, and `clients/agent-runtime/src/gateway/mod.rs:1654`. |
| Gateway Webhook Approval Outcome Parity | ✅ Implemented | Dispatcher-backed turns surface structured approval denials and keep blocked outcomes non-success in `clients/agent-runtime/src/gateway/webhook_dispatch.rs:349` and `clients/agent-runtime/src/gateway/mod.rs:1537`. |
| Gateway Webhook Session Scoping | ✅ Implemented | Session ids are resolved at the gateway edge, passed into `WebhookTurnRequest`, then into `TurnContext`, memory recall, and auto-save in `clients/agent-runtime/src/gateway/mod.rs:1598`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs:145`, and `clients/agent-runtime/src/agent/agent.rs:613`. |
| Gateway Webhook Response and Streaming Contract | ✅ Implemented | Completed, approval-required, timeout, fallback, and error HTTP mappings live in `clients/agent-runtime/src/gateway/mod.rs:1518`, while `events_sse` frames are derived from canonical turn events in `clients/agent-runtime/src/gateway/webhook_dispatch.rs:235`. |
| Gateway Compatibility Fallback and Rollout Safety | ✅ Implemented | Dedicated rollout controls exist in config/env override plumbing and runtime-path logging in `clients/agent-runtime/src/config/schema.rs:666`, `clients/agent-runtime/src/config/schema.rs:2799`, and `clients/agent-runtime/src/gateway/mod.rs:738`. |
| MCP Policy and Approval Enforcement | ✅ Implemented | Dispatcher-backed webhook uses canonical approval denial extraction and fallback requests remain explicitly legacy-path only in `clients/agent-runtime/src/gateway/webhook_dispatch.rs:349` and `clients/agent-runtime/src/gateway/mod.rs:1639`. |
| Gateway Webhook MCP Capability Parity | ✅ Implemented | Gateway bootstrap reuses the canonical tool assembly, including MCP registration when enabled, in `clients/agent-runtime/src/bootstrap/mod.rs:188` and `clients/agent-runtime/src/bootstrap/mod.rs:206`. |

---

## Coherence (Design Match)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Reuse canonical `Agent` turn loop | ✅ Yes | Gateway adapter calls `Agent::turn_with_context(...)` instead of extending preview helpers. |
| Keep transport/security outside dispatcher boundary | ✅ Yes | `handle_webhook()` still owns auth, webhook secret validation, rate limits, idempotency, and response shaping. |
| Add explicit session-aware turn context | ✅ Yes | `TurnContext` and `AgentTurnResult` carry session-scoped turn metadata used by gateway. |
| Synchronous approval denial for webhook | ✅ Yes | Approval-required outcomes map to immediate `403` responses with no resumable approval protocol. |
| Preserve synchronous JSON webhook contract | ✅ Yes | `/webhook` still returns JSON and may attach canonical `events_sse` compatibility frames. |
| Roll out behind dedicated gateway flag | ✅ Yes | `gateway.webhook_dispatcher_enabled` and `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` exist and default off. |
| File changes table adherence | ⚠️ Partial | `clients/agent-runtime/src/pre_execution/mod.rs` was not modified even though the design's file-changes table expected a narrowing change there; the narrowing was effectively implemented from the new gateway adapter/call site instead. |

---

## Spec Compliance Matrix

| Requirement | Scenario | Test Evidence | Result |
|-------------|----------|---------------|--------|
| Entry Points Alignment | Gateway webhook uses canonical dispatcher semantics | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_flag_routes_through_canonical_chat_path`, `webhook_dispatcher_config_flag_routes_through_canonical_chat_path`, `webhook_dispatcher_executes_allowed_tool_and_returns_completed_response` | ✅ COMPLIANT |
| Entry Points Alignment | Transport shim does not change runtime semantics | `clients/agent-runtime/src/gateway/webhook_dispatch.rs` -> `maps_completed_agent_turn_into_completed_webhook_result`, `maps_approval_required_block_into_webhook_denial`, `maps_timeout_block_into_timeout_outcome`, `maps_runtime_error_into_error_outcome`; `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_preview_returns_canonical_event_frames` | ✅ COMPLIANT |
| Entry Points Alignment | WhatsApp remains outside this parity contract | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_rollout_flag_does_not_change_whatsapp_behavior` | ✅ COMPLIANT |
| Gateway Transport Boundary Preservation | Transport checks gate runtime entry | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_keeps_secret_auth_before_runtime_execution`, `webhook_idempotency_skips_duplicate_provider_calls` | ✅ COMPLIANT |
| Gateway Webhook Approval Outcome Parity | Approved action proceeds normally | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_executes_allowed_tool_and_returns_completed_response` | ✅ COMPLIANT |
| Gateway Webhook Approval Outcome Parity | Approval-required action is returned as blocked | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_blocks_native_tool_and_keeps_idempotency_retryable` | ✅ COMPLIANT |
| Gateway Webhook Session Scoping | Explicit session id reuses canonical state | `clients/agent-runtime/src/agent/tests.rs` -> `turn_with_context_scopes_memory_recall_and_auto_save_to_session`; `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_executes_allowed_tool_and_returns_completed_response` | ✅ COMPLIANT |
| Gateway Webhook Session Scoping | Missing session id is isolated | `clients/agent-runtime/src/agent/tests.rs` -> `turn_with_context_keeps_missing_session_isolated`; gateway edge still lacks an end-to-end dispatcher webhook test that omits `X-Session-Id` and proves the generated `webhook-{uuid}` session stays isolated through the adapter | ⚠️ PARTIAL |
| Gateway Webhook Response and Streaming Contract | Synchronous final result mirrors canonical outcome | `clients/agent-runtime/src/gateway/webhook_dispatch.rs` mapping tests plus `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_executes_allowed_tool_and_returns_completed_response`, `webhook_dispatcher_blocks_native_tool_and_keeps_idempotency_retryable`, `webhook_dispatcher_returns_500_with_session_id_on_runtime_error` | ✅ COMPLIANT |
| Gateway Webhook Response and Streaming Contract | Event projection remains informational | `clients/agent-runtime/src/gateway/webhook_dispatch.rs` -> `canonical_event_frames_are_emitted_when_requested`; `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_preview_returns_canonical_event_frames`, `legacy_webhook_preview_does_not_emit_synthetic_events_sse` | ✅ COMPLIANT |
| Gateway Compatibility Fallback and Rollout Safety | Fallback disables parity claims for a request | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_without_dispatcher_flag_stays_on_legacy_simple_chat_path`, `legacy_webhook_with_mcp_enabled_marks_parity_inactive` | ✅ COMPLIANT |
| Gateway Compatibility Fallback and Rollout Safety | Comparative observability supports rollout | `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_rollout_observability_distinguishes_dispatcher_and_legacy_requests` | ✅ COMPLIANT |
| MCP Policy and Approval Enforcement | Entry-point parity now includes dispatcher-backed webhook | `clients/agent-runtime/tests/mcp_policy_approval_parity.rs` plus `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_blocks_mcp_tool_with_structured_denial` | ✅ COMPLIANT |
| MCP Policy and Approval Enforcement | Fallback request does not claim MCP parity | `clients/agent-runtime/src/gateway/mod.rs` -> `legacy_webhook_with_mcp_enabled_marks_parity_inactive` | ✅ COMPLIANT |
| Gateway Webhook MCP Capability Parity | Dispatcher-backed webhook receives canonical MCP tools | `clients/agent-runtime/src/bootstrap/mod.rs` -> `gateway_bootstrap_reuses_canonical_mcp_tool_registry` | ✅ COMPLIANT |
| Gateway Webhook MCP Capability Parity | HTTP response mapping does not alter MCP execution semantics | Denial semantics are covered by `clients/agent-runtime/src/gateway/mod.rs` -> `webhook_dispatcher_blocks_mcp_tool_with_structured_denial`, but MCP-specific success, timeout, and error HTTP mappings are not directly exercised | ⚠️ PARTIAL |

**Compliance summary**: `14 / 16` scenarios compliant, `2 / 16` partial, `0 / 16` untested.

---

## Issues Found

### CRITICAL

None.

### WARNING

1. Missing-session isolation is still proven only at the canonical agent layer. There is no dispatcher-backed `/webhook` test that omits `X-Session-Id`, lets gateway generate `webhook-{uuid}`, and proves the resulting turn stays isolated end to end.
2. MCP HTTP response mapping is behaviorally proven for denial outcomes, but the spec language also names success, timeout, and failure variants, and those MCP-specific response variants are not directly covered.
3. One focused Rust verification command failed once with an intermittent `config::schema::tests::env_override_gateway_webhook_dispatcher` failure in the `src/main.rs` test binary before passing on exact rerun, which pointed to env-var test interference in the changed area. This was later stabilized by the archived `gateway-webhook-dispatcher-env-flake` follow-up, which added a shared guard and recorded 60/60 repeated targeted passes.
4. The configured repo verify command `make test` does not execute the Rust agent-runtime test suite directly, so spec verification for this change depends on additional `cargo test` execution outside the configured `openspec` verify command.

### SUGGESTION

1. Add one dispatcher-backed webhook integration test that omits `X-Session-Id` and asserts generated-session isolation through gateway, adapter, and memory propagation.
2. Add one focused MCP webhook test for a successful MCP turn or a synthetic timeout/error mapping path to close the remaining response-contract gap.
3. Stabilize or serialize the env-override test group around `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` if the intermittent failure reproduces.
4. Consider updating `openspec/config.yaml` or `make test` so the standard verification command includes Rust runtime suites for `clients/agent-runtime`.

---

## Verdict

**PASS WITH WARNINGS**

The change satisfies the implemented gateway dispatcher parity contract with passing build/test/coverage evidence and strong structural alignment to the specs and design. Remaining risk is narrow: two spec scenarios are only partially proven behaviorally, and one focused Rust command showed intermittent test flakiness that should be cleaned up before archive.
