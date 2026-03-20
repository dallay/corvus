# Proposal: Webhook Generated Session Isolation

## Intent

Close the remaining proof gap for dispatcher-backed gateway `/webhook` requests that omit
`X-Session-Id`. The current runtime already generates a `webhook-{uuid}` session and threads it
through canonical turn execution, but OpenSpec archive evidence still lacks one HTTP-boundary test
proving that generated session stays isolated end to end instead of reusing prior session state.

## Scope

### In Scope
- Add one dispatcher-backed `/webhook` end-to-end test that omits `X-Session-Id`.
- Assert the webhook response includes a generated `session_id` with the expected `webhook-`
  shape.
- Assert memory recall and auto-save both observe that same generated session and do not attach to
  any preexisting session.
- Make only the smallest supporting test-helper changes needed to capture session-scoping evidence.

### Out of Scope
- MCP `/webhook` success, timeout, or error response-mapping follow-up.
- Env-var flake stabilization around `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.
- `/whatsapp`, legacy fallback behavior, or rollout-observability follow-up.
- Broader session-model, session-lifecycle, or history-association changes.
- Production-path behavior changes unless the new proof exposes a real defect.

## Approach

Extend the existing dispatcher-backed gateway webhook test coverage in
`clients/agent-runtime/src/gateway/mod.rs` with a narrowly scoped missing-header scenario. The
test should run the real gateway handler with dispatcher mode enabled, omit `X-Session-Id`, and use
session-aware test doubles or tracking helpers to record the session ids seen by memory recall and
auto-save. The primary expected outcome is proof only. If the test fails because generated-session
scoping is broken, follow-up implementation work is allowed only to fix that concrete defect and no
further.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Add the dispatcher-backed `/webhook` missing-session isolation proof and any adjacent gateway test wiring. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Possible test-only adjacency | No semantic change expected; only relevant if a tiny test seam is needed to observe propagated session ids. |
| `openspec/changes/webhook-generated-session-isolation/proposal.md` | New | Record the narrow proposal for this follow-up. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Current gateway test helpers do not expose recall/store session ids cleanly | Medium | Keep helper changes minimal and local to gateway test code. |
| UUID-based generated ids make assertions brittle | Low | Assert prefix/shape and captured propagation equality, not exact values. |
| New proof exposes a real gateway-to-agent scoping defect | Low | Limit any production fix to the concrete failing path and keep the change narrowly targeted. |

## Rollback Plan

Revert the new test and any supporting test-helper changes if they prove unstable or incorrect. If
the change includes a defect fix, revert only the minimal production patch together with its test,
returning the codebase to the previously archived dispatcher behavior while preserving the follow-up
gap for later work.

## Dependencies

- Existing dispatcher-backed gateway webhook test harness in `clients/agent-runtime/src/gateway/mod.rs`.
- Existing `agent-loop` requirement for missing-session isolation in `openspec/specs/agent-loop/spec.md`.
- Archived warning context in `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md` and `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md`.

## Success Criteria

- [ ] One dispatcher-backed `/webhook` test omits `X-Session-Id` and passes with a generated
      `webhook-...` response session id.
- [ ] The test proves memory recall and auto-save both use the same generated session id and do not
      reuse an existing session.
- [ ] No production code changes are made unless the new proof reveals a real defect requiring a
      minimal fix.
- [ ] The change closes only the generated-session isolation proof gap and leaves the other archived
      follow-ups explicitly deferred.
