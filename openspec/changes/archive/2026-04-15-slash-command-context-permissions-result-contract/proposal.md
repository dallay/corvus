# Proposal: Slash Command Context, Permissions, and Result Contract

## Intent

Standardize the runtime contract for slash command execution so every registered slash command runs with the same core context, declares typed permission/capability requirements, and returns a non-lossy success/error outcome across the shared ingress seam.

This change addresses the gaps identified in #540 and the exploration artifact: the current `CommandContext` is too narrow, descriptor requirement metadata is string-tag based, and `SessionCommandError` is flattened into transport-friendly but lossy results. The goal is to tighten the contract for existing slash session commands without pulling transport-integration work from #541 or introducing new command families.

## Scope

### In Scope
- Define a shared slash-command execution context contract that includes session identity, caller identity shape, transport/source identity, plan-mode state, and evaluated capability/permission facts needed by handlers.
- Replace descriptive string-tag requirement metadata with typed command-level requirement declarations that remain registry-visible but do not move backend/auth policy into registry-core.
- Define a non-lossy result/error contract for slash command dispatch, including normalized machine-readable outcome kinds and documented sanitization expectations for user-facing errors.
- Cover the contract with focused runtime tests for context construction, permission/capability denial, `/resume` ownership-sensitive behavior, and error normalization at the ingress seam.

### Out of Scope
- Transport-specific response envelope rewrites or end-to-end transport integration work tracked by #541.
- New slash command families beyond the existing slash session command set.
- Broad registry-core authorization ownership, backend policy inversion, or unrelated capability-architecture migration.

## Approach

Introduce a shared command contract at the `clients/agent-runtime/src/session_commands` seam and thread it through `pre_execution::evaluate_ingress(...)` as the canonical dispatch boundary. The runtime will:

1. Expand command context from `{ session_id, caller_token_hash }` into a typed execution context that preserves transport-specific identity semantics without hiding them behind stringly fields.
2. Upgrade descriptor requirement metadata from free-form tags to typed requirement declarations that handlers and services can evaluate deterministically while registry-core remains descriptive, not authoritative.
3. Replace lossy flattening with a normalized command outcome model that keeps machine-readable success/error variants intact and separately exposes sanitized user-facing messaging.
4. Preserve current command behavior for `/resume`, `/suspend`, `/tldr`, and `/compact`, while tightening the contract so authorization and unsupported-backend failures remain explicit and testable.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/session_commands/types.rs` | Modified | Define typed execution context, requirement metadata, and non-lossy command outcome/error types. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Expose typed requirement metadata from descriptors without moving policy enforcement into registry-core. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modified | Evaluate typed requirements and preserve explicit command outcome/error variants, including `/resume` ownership-sensitive checks. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Standardize ingress-to-command context construction and error normalization at the shared seam. |
| `clients/agent-runtime/src/{main.rs,gateway/mod.rs,gateway/webhook_dispatch.rs,channels/mod.rs}` | Modified | Pass the richer context inputs into the shared slash-command contract without expanding transport scope. |
| `openspec/changes/slash-command-context-permissions-result-contract/specs/` | New | Follow-on delta specs for the shared contract, permission model, and result/error semantics. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Tightening the contract exposes an existing `/resume` ownership enforcement gap | High | Make ownership-sensitive behavior explicit in the proposal/specs and require regression coverage before refactoring. |
| Contract work drifts into transport envelope redesign from #541 | Medium | Keep this slice focused on shared runtime types and ingress normalization only; defer transport-specific shaping to the follow-up change. |
| Non-lossy outcomes create compatibility friction for current callers expecting `success: bool`-style flattening | Medium | Preserve an adapter layer at the ingress boundary so internal outcome richness can coexist with current transport envelopes until #541 lands. |

## Rollback Plan

Revert the shared contract changes in `session_commands` and `pre_execution` to the current minimal context and flattened result path, retaining existing descriptor tags and ingress behavior. Because this proposal keeps scope inside the shared runtime seam and avoids transport rewrites, rollback is limited to restoring current Rust types, handler signatures, and normalization logic.

## Dependencies

- Issue #539 as the prerequisite registry baseline for command registration and shared dispatch.
- Existing slash command registry and session lifecycle specifications as the current behavioral baseline.

## Success Criteria

- [ ] Slash command handlers receive a stable typed execution context that covers session identity, caller identity/source, plan mode, and evaluated requirement facts.
- [ ] Command descriptors declare typed permission/capability requirements instead of free-form string tags.
- [ ] Slash command dispatch preserves machine-readable success/error outcomes without lossy flattening at the internal contract boundary.
- [ ] Sanitized user-facing error expectations are documented separately from internal error detail.
- [ ] Focused tests cover context construction, permission denial, `/resume` ownership-sensitive behavior, and ingress error normalization.
