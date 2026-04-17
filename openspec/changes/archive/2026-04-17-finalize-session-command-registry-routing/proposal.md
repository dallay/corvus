# Proposal: Finalize Session Command Registry Routing

## Intent

Issue #542 is no longer a core migration of `/resume`, `/suspend`, `/tldr`, and `/compact` into the slash command registry; exploration shows those commands already execute through the registry in production. This change finalizes that migration by removing leftover migration shims and naming noise, and by adding focused proof that the canonical ingress seam still preserves current behavior, authz checks, and transport parity.

## Scope

### In Scope
- Remove or isolate leftover migration-only routing helpers and comments that imply a separate session-command execution path when recognized commands already short-circuit through `pre_execution::evaluate_ingress(...)`.
- Simplify registry-adjacent compatibility noise in the runtime surfaces involved in slash command ingress, while preserving existing outward transport envelopes.
- Add or refresh targeted proof tests for `/resume`, `/suspend`, `/tldr`, and `/compact` across the existing ingress surfaces so #542 can be closed with evidence.

### Out of Scope
- Adding new slash command families beyond `/resume`, `/suspend`, `/tldr`, and `/compact`.
- Broad refactors of transport envelope shaping, gateway response models, or cross-surface payload unification.
- Reworking session-command service behavior, ownership/authz policy, or persistence semantics defined by the sessions spec.
- Folding in adjacent slash-command platform work from #543 or other #527 follow-ups.

## Approach

Treat this slice as closure cleanup, not a feature migration. Keep `pre_execution::evaluate_ingress(...)` as the canonical interception seam, keep `session_commands/registry.rs` as the only production binding from command names to handlers, and prune leftover compatibility artifacts that obscure that reality. Then add narrow tests proving that CLI/runtime, gateway, webhook, and channel-backed ingress continue to route the four in-scope commands through the shared registry-backed path without changing command semantics or transport-specific outward wrappers.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Preserve and clarify the canonical shared ingress seam for recognized slash commands. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Remove leftover migration noise and keep built-in command registration as the sole production dispatch binding. |
| `clients/agent-runtime/src/session_commands/service.rs` | Verified | Preserve existing command behavior, ownership checks, and backend validation behind registry handlers without broad logic changes. |
| `clients/agent-runtime/src/main.rs` | Modified | Clean up CLI compatibility naming/shims that still suggest a separate session-command path. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Isolate or rename early-response helpers/comments so they clearly reflect shared ingress dispatch rather than legacy special-case routing. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modified | Keep webhook dispatch aligned to the shared ingress seam and add proof coverage where needed. |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Keep channel-backed ingress aligned to the shared ingress seam and add proof coverage where needed. |
| `clients/agent-runtime/tests/` or relevant runtime test modules | Modified | Add targeted regression coverage proving registry-backed routing for the four in-scope commands. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Cleanup-only scope may look smaller than reviewers expect for a “migration” issue | Medium | State clearly that functional migration is already complete and this slice closes the remaining parity and deprecation evidence. |
| Renames around shared slash routing may conflict with nearby #543 work | Medium | Keep patch small, prefer localized helper/comment cleanup, and avoid unrelated structural refactors. |
| Tests may accidentally assert transport envelope details instead of shared dispatch behavior | Low | Focus assertions on ingress routing, handled-result classification, and preserved command semantics rather than outward wrapper formatting. |

## Rollback Plan

Revert the cleanup patch and targeted tests in a single changeset. Because the proposal preserves current behavior and does not introduce new command families or transport contracts, rollback returns the runtime to its prior compatibility naming and proof state without data migration or persistent state changes.

## Dependencies

- Exploration artifact: `openspec/changes/finalize-session-command-registry-routing/exploration.md`
- Issue definition: `tmp/claudio-issues/542-migrate-session-commands.md`
- Parent epic: `#527 Slash Commands Platform`
- Runtime contract references: `openspec/specs/slash-command-registry/spec.md`, `openspec/specs/sessions/spec.md`

## Success Criteria

- [ ] The proposal remains narrowly scoped to finalizing #542 for `/resume`, `/suspend`, `/tldr`, and `/compact` only.
- [ ] Leftover migration helpers, naming, or comments that imply duplicate session-command routing are removed or clearly isolated for deprecation.
- [ ] Targeted tests prove recognized in-scope commands short-circuit through `pre_execution::evaluate_ingress(...)` across the supported runtime ingress surfaces.
- [ ] Existing behavior remains intact, including session ownership/authz checks, unsupported-backend handling, and current transport-specific outward envelopes.
- [ ] The resulting runtime surfaces are easier to review and maintain because registry-backed routing is explicit and legacy migration noise is reduced.
