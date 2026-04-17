# Proposal: Slash Command Transport Parity

## Intent

Close the remaining transport-parity gap for slash commands by removing duplicated post-dispatch branching around the shared pre-execution seam while preserving each transport's existing external response envelope.

The registry, typed command context, and shared `pre_execution::evaluate_ingress(...)` seam are already in place from #539 and #540. Issue #541 is now a focused integration cleanup: centralize handled slash outcome adaptation and failure classification so CLI/runtime, gateway HTTP, gateway streaming, webhook dispatch, and channel ingress all consume the same internal handled-result contract instead of maintaining separate transport-side match trees.

## Scope

### In Scope

- Introduce one shared handled-slash adaptation layer after `pre_execution::evaluate_ingress(...)` that preserves machine-readable outcome and failure-kind data while exposing a transport-neutral handled result.
- Replace duplicated transport-side branching in CLI/runtime message mode, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher, and channel ingress with that shared adapter.
- Remove the extra CLI `recognizes(...)` pre-check so CLI/runtime message entry uses the same shared seam and handled-result contract as the other transports.
- Preserve existing external transport envelopes while making permission-denied, command-not-found fallthrough, and generic command failure classification consistent at the internal adaptation boundary.
- Add focused regression coverage proving the supported transports share dispatch and handled-result adaptation while keeping their current outward payload/text shapes.

### Out of Scope

- Broad envelope unification across HTTP JSON, SSE, webhook results, CLI text, and channel message bodies.
- New slash command families beyond the existing integrated command set needed to validate transport parity.
- Reworking command handler authorization policy, backend persistence policy, or registry-core responsibilities beyond the minimal adapter integration required for #541.
- Expanding scope into interactive CLI UX beyond the current message-based runtime entrypoints unless already covered by the existing fast path.

## Approach

Keep `pre_execution::evaluate_ingress(...)` as the canonical interception seam and add a small transport-neutral adapter near `pre_execution`/`session_commands` that converts handled slash outcomes into a shared internal shape such as `NotHandled | Handled(Success | Failure | Blocking)`. That adapter will preserve the existing non-lossy command outcome and failure kinds from `session_commands/types.rs`, especially authorization-sensitive denials, while leaving final envelope shaping outside the shared contract.

Each transport surface will then do only the last mile:

1. Build the transport-appropriate `CommandContext` using its current caller/session semantics.
2. Call the shared ingress seam once.
3. Consume the shared handled-result adapter.
4. Wrap the adapted result into its existing external envelope (CLI text, HTTP JSON, SSE events, webhook result, or channel send operations).

This keeps #541 small: parity is achieved by sharing the handled slash adaptation path, not by forcing transport output schemas to match.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Keep the canonical ingress seam and expose the shared handled-slash adaptation boundary. |
| `clients/agent-runtime/src/session_commands/types.rs` | Modified | Reuse or lightly extend machine-readable slash outcome/failure kinds consumed by the adapter. |
| `clients/agent-runtime/src/main.rs` | Modified | Remove CLI/runtime duplicate recognition branching and consume the shared handled-result adapter for message-mode entry. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Replace duplicated `/webhook` and `/web/chat/stream` slash outcome mapping with the shared adapter while preserving JSON and SSE envelopes. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modified | Reuse the shared handled-result adapter before provider execution and preserve webhook-specific result wrapping. |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Replace channel-specific handled slash branching with shared adaptation while preserving channel send text behavior and derived caller scope semantics. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Small internal failure-kind changes alter existing HTTP/SSE/channel test expectations | Medium | Preserve external envelopes exactly and limit changes to internal classification/adaptation; add regression tests per transport. |
| CLI message-mode parity work accidentally expands into broader interactive-loop behavior | Medium | Keep proposal/spec scope explicitly tied to current message-based runtime fast paths unless exploration proves the same seam already governs interactive entry. |
| Shared adapter grows into transport-envelope unification | Low | Keep the adapter transport-neutral and internal; require each transport to continue owning its final response wrapper. |

## Rollback Plan

Revert the shared handled-result adapter and restore the current transport-local outcome mapping in `main.rs`, `gateway/mod.rs`, `gateway/webhook_dispatch.rs`, and `channels/mod.rs`. Because this slice is a routing/integration cleanup with no persistence or schema changes, rollback is limited to restoring the prior branching paths and internal mapping helpers.

## Dependencies

- Slash command registry baseline from #539.
- Typed command context and non-lossy outcome contract from #540.
- Existing runtime surfaces already wired to `pre_execution::evaluate_ingress(...)`, especially CLI/runtime, gateway HTTP, gateway streaming, webhook dispatch, and channel ingress.
- Parent epic #527 Slash Commands Platform and issue #541 as the scoped transport-parity integration slice.

## Success Criteria

- [ ] CLI/runtime message mode, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher, and channel ingress all adapt handled slash outcomes through one shared internal adapter after `pre_execution::evaluate_ingress(...)`.
- [ ] Transport-specific divergence for recognized slash commands is reduced to caller-context construction and final envelope wrapping only.
- [ ] Permission-denied and generic command failure classification remain machine-readable and consistent across transports without forcing a shared external envelope.
- [ ] Unknown slash-like input continues to fall through consistently in all supported transports.
- [ ] Focused regression tests cover the supported transport paths and confirm current outward envelope/text shapes are preserved.
