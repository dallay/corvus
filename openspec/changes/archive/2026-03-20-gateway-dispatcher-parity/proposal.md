# Proposal: Gateway Dispatcher Parity

## Intent

Define the explicit parity contract and remaining implementation scope required to move
gateway-driven execution onto the same canonical dispatcher-backed runtime used by the main
runtime flow. The goal is to remove ambiguity around what "gateway parity" means, identify the
behavioral gaps that still exist, and bound the work so follow-up spec and design artifacts can
drive implementation without reopening runtime semantics.

## Scope

### In Scope
- Define the canonical parity contract across CLI, channels, and gateway webhook execution.
- Define the `/webhook` target state: dispatcher-backed execution, canonical tool availability,
  equivalent approval/risk gating, explicit session semantics, and a defined response/streaming
  contract.
- Produce a parity matrix that distinguishes current behavior, required parity, accepted
  compatibility constraints, and known follow-up slices.
- Identify gateway-specific gaps that remain after webhook parity is defined, including approval
  interactivity, streaming fidelity, and session persistence behavior.
- Define rollout, compatibility, fallback, and observability expectations for a safe migration off
  `Provider::simple_chat()`.

### Out of Scope
- Implementing the dispatcher migration itself.
- Refactoring CLI and channels beyond documenting them as the canonical baseline.
- Admin, pairing, tunnel, or other unrelated gateway endpoints.
- A broad shared-runtime refactor across channels and gateway.
- WhatsApp execution parity in this change.

## Approach

Use the existing dispatcher-backed runtime as the source of truth and scope this change around
planning parity for `/webhook`, not around building a new shared abstraction first. The proposal
defines parity as equivalence at the dispatcher boundary: the gateway webhook MUST execute through
the canonical tool loop with the same tool registry, policy evaluation, approval gates, context
assembly, and result semantics as CLI/channels, except where gateway transport constraints require
an explicitly documented compatibility shim.

The proposal intentionally keeps transport concerns separate from execution concerns:
- Gateway-specific auth, rate limiting, idempotency, webhook validation, and HTTP response shaping
  remain owned by gateway handlers.
- Canonical loop semantics remain owned by the dispatcher-backed runtime.
- Any allowed deviation must be narrow, explicit, and justified as transport compatibility rather
  than runtime behavior.

### Parity Decision: WhatsApp

WhatsApp parity is deferred.

Rationale:
- `/webhook` is the spec-defined interim exception and the smallest path to eliminate the current
  canonical-runtime gap.
- `/whatsapp` diverges more deeply today: it bypasses canonical pre-checks, has no dispatcher
  loop/session contract, and likely needs a product-specific interaction contract rather than a
  direct copy of webhook behavior.
- Including WhatsApp now would expand scope from "define webhook parity and gateway contract" into
  a broader gateway product redesign, which increases ambiguity and delays the more urgent parity
  correction.

This proposal still requires WhatsApp to appear in the parity matrix as an explicitly deferred
surface with identified gaps and a recommended follow-up change.

### Parity Matrix

| Surface | Current Execution Model | Target Contract After Follow-up Work | Decision in This Change |
|------|--------|-------------|
| CLI | Canonical dispatcher-backed loop | Baseline source of truth | Baseline only |
| Channels | Dispatcher-backed loop with channel-native streaming | Match CLI runtime semantics, preserve channel transport behavior | Baseline only |
| Gateway `/webhook` | Canonical pre-checks + `Provider::simple_chat()` | Migrate to canonical dispatcher-backed execution with explicit gateway response contract | In scope |
| Gateway `/whatsapp` | Direct `Provider::simple_chat()` with deeper semantic gaps | Requires separate parity follow-up after webhook contract is defined | Deferred |

### Behavioral Gaps To Resolve In Follow-up Spec/Design

1. Dispatcher parity for `/webhook`
   - Replace direct `simple_chat()` execution with canonical dispatcher-backed turn execution.
   - Reuse the canonical tool registry, including MCP-enabled tools when configured.
   - Preserve gateway transport/security guards around the runtime invocation.

2. Approval and risk semantics
   - Define webhook parity as equivalent policy/risk decisions at the dispatcher boundary.
   - Make explicit whether gateway approval parity is synchronous-only in the first implementation
     slice, with blocked actions returned as structured denial/needs-approval results instead of a
     resumable approval protocol.

3. Session semantics
   - Define how `X-Session-Id` scopes conversation history, memory association, and event/audit
     continuity for webhook-driven turns.
   - Define the fallback behavior when no session id is provided.

4. Response and streaming contract
   - Define whether webhook returns a synchronous final response, a preview event list, or a
     transport-specific mapping of canonical loop events.
   - Require that any non-identical streaming behavior be documented as a transport shim rather
     than a runtime semantic difference.

5. Compatibility and fallback
   - Define a rollout guard that allows fallback to the current `simple_chat()` webhook path while
     parity implementation is validated.
   - Define the required telemetry/logging to compare legacy and dispatcher-backed behavior during
     rollout.

### Recommended Follow-up Split

If subsequent artifacts need smaller implementation slices, split the work into:
- Slice A: `/webhook` dispatcher execution parity and session contract.
- Slice B: `/webhook` response/streaming and approval compatibility contract.
- Slice C: `/whatsapp` parity proposal/spec once webhook behavior is canonicalized.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/agent-loop/spec.md` | Modified | Remove or narrow the gateway exception and define the explicit webhook parity contract |
| `openspec/specs/mcp-runtime/spec.md` | Modified | Extend MCP parity expectations to gateway webhook once it is dispatcher-backed |
| `openspec/changes/gateway-dispatcher-parity/proposal.md` | New | Proposal artifact for parity scope, decisions, and rollout expectations |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Main implementation surface for webhook runtime invocation, response shaping, and fallback hooks |
| `clients/agent-runtime/src/agent/agent.rs` | Modified | Possible reuse or adaptation point for canonical turn execution from gateway |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Reference surface for shared semantics and possible reuse patterns, if needed |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modified | Ensure gateway path can bootstrap the same dispatcher/tool registry capabilities |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Clarify how gateway pre-checks map into canonical dispatcher approval semantics |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Scope expands into a full multi-surface runtime refactor | High | Keep proposal centered on `/webhook` parity and record WhatsApp as explicit follow-up work |
| Gateway transport behavior is conflated with runtime semantics | Medium | Separate transport compatibility shims from canonical dispatcher guarantees in spec/design |
| Approval parity is overspecified before a resumable protocol exists | Medium | Define first-step parity as equivalent decision outcomes with synchronous gateway behavior |
| Rollout causes regressions in existing webhook consumers | Medium | Require feature-flagged fallback, compatibility notes, and comparative telemetry |

## Rollback Plan

Ship webhook dispatcher parity behind a gateway-scoped runtime flag. If regressions appear,
disable the flag and return `/webhook` to the current `Provider::simple_chat()` path while keeping
gateway auth, rate-limit, and idempotency behavior unchanged. Preserve comparative logging during
rollback so mismatches can be diagnosed before re-enabling the canonical path.

## Dependencies

- Completed exploration artifact at `openspec/changes/gateway-dispatcher-parity/exploration.md`
- Existing canonical loop contract in `openspec/specs/agent-loop/spec.md`
- Existing MCP entry-point parity contract in `openspec/specs/mcp-runtime/spec.md`
- Agreement in follow-up spec/design on synchronous-vs-resumable gateway approval handling

## Success Criteria

- [ ] The parity contract clearly states what gateway webhook must match versus where transport
      compatibility shims are allowed.
- [ ] The current and target behavior for CLI, channels, `/webhook`, and `/whatsapp` is captured in
      a parity matrix.
- [ ] Remaining gateway gaps are explicitly identified: dispatcher execution, approval semantics,
      session semantics, streaming/response behavior, rollout, and fallback.
- [ ] WhatsApp is explicitly marked as deferred with rationale and a follow-up recommendation.
- [ ] Follow-up spec and design work can proceed without reopening scope on canonical parity goals.
