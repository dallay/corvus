# Tasks: MCP Webhook Response Mapping

## Phase 1: Proof-First Tests

- [x] 1.1 In `clients/agent-runtime/src/gateway/mod.rs`, add a RED seam test for
  `webhook_response_from_dispatch_result(...)` that proves MCP-labeled
  `WebhookTerminalOutcome::Completed` maps to the expected `200` webhook JSON without claiming
  end-to-end MCP execution.
- [x] 1.2 In `clients/agent-runtime/src/gateway/mod.rs`, add one RED seam test for exactly one
  MCP-labeled non-success outcome, preferring `WebhookTerminalOutcome::Error` and using `Timeout`
  only if that is the smaller truthful uncovered branch.
- [x] 1.3 In `clients/agent-runtime/src/gateway/mod.rs`, keep the existing dispatcher-backed MCP
  denial integration proof as the runtime-reachable `/webhook` evidence and make the new test
  names/comments explicitly distinguish seam-level proof from end-to-end proof.

## Phase 2: Contingent Fix Only If RED Reveals A Defect

- [x] 2.1 If either new seam test fails for real behavior, apply the smallest production fix in
  `clients/agent-runtime/src/gateway/mod.rs` or
  `clients/agent-runtime/src/gateway/webhook_dispatch.rs` needed to preserve the canonical HTTP
  mapping; otherwise leave production code unchanged.

## Phase 3: Focused Validation

- [x] 3.1 Run targeted Rust tests for the new proof in `clients/agent-runtime/src/gateway/mod.rs`
  and confirm the existing MCP denial test still passes.
- [x] 3.2 Verify the completed plus chosen non-success assertions match
  `openspec/changes/mcp-webhook-response-mapping/specs/mcp-runtime/spec.md` and do not expand scope
  into session isolation, env-var flake work, `/whatsapp`, policy relaxations, or broader dispatcher
  refactors.
