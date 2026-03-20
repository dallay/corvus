## Exploration: webhook-generated-session-isolation

### Current State
The archived `gateway-dispatcher-parity` change already moved `/webhook` onto the dispatcher-backed runtime behind `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`, and the gateway now resolves `X-Session-Id` at the HTTP edge before building `WebhookTurnRequest` (`clients/agent-runtime/src/gateway/mod.rs:711`, `clients/agent-runtime/src/gateway/mod.rs:1598`, `clients/agent-runtime/src/gateway/mod.rs:1610`).

When the header is missing, `resolve_session_id(...)` generates a fresh `webhook-{uuid}` value and marks the request as `WebhookSessionSource::Generated` (`clients/agent-runtime/src/gateway/mod.rs:728`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs:15`). That generated id is propagated into `TurnContext`, so canonical auto-save and memory recall run with a scoped session instead of `None` (`clients/agent-runtime/src/gateway/webhook_dispatch.rs:145`, `clients/agent-runtime/src/agent/agent.rs:613`, `clients/agent-runtime/src/agent/agent.rs:625`, `clients/agent-runtime/src/agent/agent.rs:681`).

The remaining gap is not implementation but runtime proof at the HTTP boundary. Current dispatcher-backed webhook tests cover preview frames, allowed-tool completion, native-tool denial, MCP denial, runtime error, rollout fallback, and transport checks, but they do not include a dispatcher-backed `/webhook` request that omits `X-Session-Id` and proves the generated session stays isolated end to end (`clients/agent-runtime/src/gateway/mod.rs:3718`, `clients/agent-runtime/src/gateway/mod.rs:3776`, `clients/agent-runtime/src/gateway/mod.rs:3838`, `clients/agent-runtime/src/gateway/mod.rs:3929`, `clients/agent-runtime/src/gateway/mod.rs:3992`).

Isolation is currently proven only one layer lower in canonical agent tests, where `TurnContext::default()` keeps recall/store session scope as `None` (`clients/agent-runtime/src/agent/tests.rs:842`). That proves the agent contract, but not the gateway-specific generated-session behavior required by `agent-loop`'s `/webhook` session-scoping scenario (`openspec/specs/agent-loop/spec.md:77`).

### Affected Areas
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md` — records the exact warning this follow-up should close.
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md` — marks missing-session isolation as only partially proven at gateway level.
- `openspec/specs/agent-loop/spec.md` — already contains the normative requirement and scenario for missing `X-Session-Id` isolation.
- `clients/agent-runtime/src/gateway/mod.rs` — owns session-id resolution, dispatcher webhook tests, and is the most direct place for the missing end-to-end proof.
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs` — carries generated-session ids into canonical `TurnContext`; unit tests already prove that handoff in isolation.
- `clients/agent-runtime/src/agent/tests.rs` — current lower-layer isolation proof that this change should complement, not replace.

### Approaches
1. **Add one gateway integration-style test with session-tracking memory** — extend the dispatcher-backed `/webhook` test suite to omit `X-Session-Id`, assert the response returns a generated `webhook-` session id, and verify memory recall/store saw only that same generated session.
   - Pros: Directly closes the archived warning, exercises the real gateway adapter, keeps scope minimal, no production behavior changes.
   - Cons: Likely needs a richer test double than the current gateway `TrackingMemory`, because that helper records keys but not session ids.
   - Effort: Low.

2. **Add broader webhook parity coverage bundle** — combine missing-session proof with MCP success/timeout/error mapping and other archived warnings in one follow-up.
   - Pros: Reduces the number of change artifacts.
   - Cons: Mixes unrelated proof gaps, increases review and regression surface, and weakens the focus of this follow-up.
   - Effort: Medium.

### Recommendation
Use approach 1. The narrowest useful scope is a proof-only follow-up that adds exactly one dispatcher-backed `/webhook` runtime test for the missing-header path and, if necessary, the smallest supporting test helper changes to observe session-scoped memory usage.

What this change should include:
- One end-to-end gateway test with dispatcher enabled and no `X-Session-Id` header.
- Assertions that the JSON response includes a generated `session_id` matching `^webhook-`.
- Assertions that memory recall and auto-save both use that same generated session id and do not reuse any prior session.
- No production-path semantic changes unless the test exposes a real bug.

What stays out of scope:
- MCP success/timeout/error HTTP mapping proof.
- Env-var flake cleanup for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.
- Verification command wiring (`make test` vs direct Rust suites).
- `/whatsapp`, legacy fallback behavior, or broader session-model refactors.

Minimum spec delta likely needed: none to the main spec semantics. `openspec/specs/agent-loop/spec.md` already says missing `X-Session-Id` requests MUST be standalone and MUST NOT attach to an existing session. If OpenSpec process still requires a delta spec for the follow-up, it should be a very small `agent-loop` delta that restates this as a proof/evidence closure, not a new behavioral requirement.

### Risks
- The current gateway test helper may need to start recording session ids for `store(...)` and `recall(...)`; keeping that helper minimal avoids scope creep.
- Generated session ids are UUID-based, so the test should assert stable shape/prefix and captured propagation, not exact values.
- Auto-save must stay enabled in the test to prove both recall and persistence scoping; that introduces slightly more setup than the existing dispatcher webhook tests.
- If the new test fails, the change may uncover a real adapter mismatch between generated gateway sessions and canonical agent isolation semantics.

### Ready for Proposal
Yes — the proposal can stay tightly scoped to closing the archived runtime-evidence gap for dispatcher-backed `/webhook` requests that omit `X-Session-Id`.
