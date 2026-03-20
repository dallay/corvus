## Exploration: mcp-webhook-response-mapping

### Current State
Gateway `/webhook` already routes dispatcher-enabled requests through the canonical `Agent::turn_with_context(...)` path, then projects `WebhookTurnResult` into HTTP status/body via `webhook_response_from_dispatch_result(...)` (`clients/agent-runtime/src/gateway/mod.rs:1518`, `clients/agent-runtime/src/gateway/mod.rs:1610`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs:310`).

The transport mapping itself is MCP-agnostic. It only looks at `WebhookTerminalOutcome` (`Completed`, `ApprovalRequired`, `Timeout`, `Fallback`, `Error`) and does not branch on whether the originating tool was native or MCP (`clients/agent-runtime/src/gateway/mod.rs:1521`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs:30`).

Current direct proof is uneven:
- MCP denial is covered end to end at the gateway HTTP layer by `webhook_dispatcher_blocks_mcp_tool_with_structured_denial` (`clients/agent-runtime/src/gateway/mod.rs:3941`).
- Non-MCP success is covered end to end by `webhook_dispatcher_executes_allowed_tool_and_returns_completed_response` (`clients/agent-runtime/src/gateway/mod.rs:3785`).
- Non-MCP runtime error is covered end to end by `webhook_dispatcher_returns_500_with_session_id_on_runtime_error` (`clients/agent-runtime/src/gateway/mod.rs:4004`).
- Canonical adapter mapping is covered below HTTP by `maps_completed_agent_turn_into_completed_webhook_result`, `maps_timeout_block_into_timeout_outcome`, and `maps_runtime_error_into_error_outcome` (`clients/agent-runtime/src/gateway/webhook_dispatch.rs:418`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs:458`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs:484`).

What is missing is MCP-labeled proof at the final HTTP mapping boundary for outcomes other than denial. There are currently no direct tests for `webhook_response_from_dispatch_result(...)` itself (`clients/agent-runtime/src/gateway/mod.rs:1518`).

There is also an important reachability constraint: dispatcher risk policy hard-denies all `mcp.*` tools before execution (`clients/agent-runtime/src/agent/dispatcher.rs:66`). Under current behavior, a real dispatcher-backed gateway request cannot naturally reach MCP success, MCP transport timeout, or MCP transport error at runtime, because the MCP call is blocked before tool execution. Existing MCP timeout/error tests live at the tool adapter layer, not the gateway runtime layer (`clients/agent-runtime/tests/mcp_execution_limits.rs:29`, `clients/agent-runtime/tests/mcp_execution_limits.rs:63`).

### Affected Areas
- `openspec/specs/mcp-runtime/spec.md` — current scenario language names success, denial, timeout, and failure preservation.
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md` — explicit archived warning that MCP HTTP mapping lacks direct proof beyond denial.
- `clients/agent-runtime/src/gateway/mod.rs` — final HTTP mapping function and gateway integration tests.
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs` — canonical-to-webhook outcome adapter tests already proving non-HTTP mapping behavior.
- `clients/agent-runtime/src/agent/dispatcher.rs` — deny-by-default MCP policy that makes MCP success/timeout/error unreachable in real gateway runtime today.
- `clients/agent-runtime/tests/mcp_execution_limits.rs` — existing MCP timeout/transport-error proof below the gateway boundary.

### Approaches
1. **Proof-only at the HTTP mapping seam** — add focused tests around `webhook_response_from_dispatch_result(...)`, using dispatcher-shaped `WebhookTurnResult` fixtures tagged as the MCP follow-up evidence.
   - Pros: smallest scope; closes the exact evidence gap at the HTTP boundary; no behavior change.
   - Cons: MCP success/timeout/error are still not runtime-reachable through `/webhook`; proof is seam-level, not full end-to-end runtime execution.
   - Effort: Low.

2. **Introduce a test-only/runtime bypass so MCP can execute through `/webhook`** — allow approved MCP calls in tests, then add end-to-end success/timeout/error gateway cases.
   - Pros: gives literal runtime proof for all named variants.
   - Cons: larger change; touches security-sensitive approval behavior; risks turning a proof follow-up into behavior work.
   - Effort: Medium.

3. **Tighten the requirement to match reachable behavior, then add focused proof** — clarify that denial is the only currently runtime-reachable MCP webhook outcome until explicit MCP approval/execution exists, while still proving generic HTTP mapping for completed/timeout/error outcomes at the gateway seam.
   - Pros: most honest to current architecture; avoids inventing a test-only policy loophole.
   - Cons: needs small spec wording correction in addition to tests.
   - Effort: Low/Medium.

### Recommendation
Treat this follow-up as primarily a proof-closure change, but verify first whether the spec intends literal end-to-end MCP success coverage or only preservation at the HTTP mapping boundary.

Narrowest useful scope:
- Add direct tests for `webhook_response_from_dispatch_result(...)` covering `Completed`, `Timeout`, and `Error` HTTP payload/status behavior.
- Make at least one of those tests explicitly part of the MCP follow-up narrative by documenting why MCP denial is runtime-reachable today while MCP success/timeout/error are not.
- Reuse existing evidence from `webhook_dispatch.rs` and `mcp_execution_limits.rs` rather than changing gateway runtime behavior.

What should stay out of scope:
- Any new approval/resume protocol for webhook.
- Any production change that allows MCP tools to execute without the current deny-by-default gate.
- WhatsApp parity, session-isolation follow-up, or broader gateway refactors.

If proposal authors decide the current `mcp-runtime` wording requires literal MCP success over `/webhook`, then this is not proof-only anymore; it becomes a small behavior/spec-correction decision first.

### Risks
- The biggest risk is false confidence from tests that are labeled "MCP" but only exercise generic outcome mapping without a real MCP execution path.
- Adding a test-only approval bypass would touch the security boundary in `clients/agent-runtime/src/agent/dispatcher.rs`, which is high-risk for a narrow follow-up.
- Leaving spec wording unchanged may keep the archive warning alive if reviewers interpret "preserve canonical success, denial, timeout, or failure result" as demanding runtime-reachable MCP success proof.

### Ready for Proposal
Yes — with one explicit framing choice up front: either (a) this change closes an HTTP-mapping evidence gap only, or (b) it first corrects/specifies that MCP success/timeout/error are unreachable on `/webhook` until MCP approval/execution is introduced.
