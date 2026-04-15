# Proposal: Slash Command Registry Core

## Intent

Corvus currently handles slash-session commands through a narrow, hard-coded path. Parsing exists, but
lookup and dispatch are still embedded in `session_commands/registry.rs` as a fixed match over four
commands, and ingress wiring depends on callers reaching the shared pre-execution seam correctly.

This change introduces a central slash command registry core in `clients/agent-runtime` so command
metadata, lookup, alias resolution, and dispatch become a stable runtime capability instead of an ad
hoc session-command implementation detail.

## Problem

- The current "registry" is not a true registry; it is a hard-coded list plus match-based dispatch.
- There is no shared command descriptor model for metadata, aliases, argument hints, or capability/
  permission rules.
- Follow-up command families cannot extend the system safely without copying patterns or adding more
  branching.
- Transport parity is fragile because the command platform is weaker than the centralized ingress seam
  it depends on.

## Why Now

- GitHub #539 is the foundation issue for the Slash Commands Platform epic (#527).
- The current slash-session slice already proved the ingress seam works, so this is the right moment
  to replace hard-coded dispatch before more commands pile onto the old shape.
- This issue blocks follow-up work (#540, #541, #542, #543, #544), so delaying it increases rework and
  architectural drift.

## Scope

### In Scope

- Define a runtime slash-command interface/trait and descriptor model for command registration.
- Define shared metadata for commands, including canonical name, aliases, description, argument shape,
  and command-level capability/permission metadata.
- Implement central registry storage, duplicate validation, name/alias lookup, and deterministic
  dispatch.
- Preserve the existing ingress short-circuit at `pre_execution::evaluate_ingress(...)` while routing
  recognized commands through the new registry core.
- Document the layering and extension model clearly enough for follow-up command-family issues.
- Add focused tests for registration, duplicate handling, lookup, alias resolution, and centralized
  dispatch behavior.

### Out of Scope

- Migrating every future slash command family in this issue.
- Shipping new command families such as `/mcp`, `/tools`, `/model`, or `/provider` in this slice.
- Reworking backend-specific session persistence semantics beyond the minimum integration needed for
  current session commands.
- Changing surface-specific caller identity rules or weakening existing authorization semantics.

## Non-Goals

- A full product-level slash command UX across web, mobile, and dashboard surfaces.
- Replacing the existing early-ingress interception contract with a late command-processing model.
- Mixing registry-core responsibilities with session snapshot/state persistence policy.

## Intended Outcome

Corvus ends this slice with one runtime slash command registry abstraction that:

- accepts command registration through a stable descriptor/handler contract;
- resolves commands by canonical name or alias;
- centralizes dispatch instead of hard-coded per-command branching; and
- gives follow-up issues a documented extension point without changing ingress semantics.

## Approach

Introduce a real registry core patterned after stronger existing runtime registry practices
(`capabilities/registry.rs`), but scoped to slash commands. The registry should own descriptor
validation, duplicate detection, alias resolution, and dispatch contracts, while concrete handlers
remain responsible for command-specific behavior.

Keep `pre_execution::evaluate_ingress(...)` as the canonical short-circuit seam so CLI, gateway,
webhook, and channel-backed flows continue to intercept slash commands before normal prompt side
effects. Existing session commands should be adapted to register into the new core instead of being
dispatched through hard-coded branching.

The registry core must stay separate from backend capability decisions: session command handlers may
still depend on SQLite-backed state and caller-scope rules, but those concerns belong in handler logic
and service layers, not in the registry abstraction itself.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Replace the hard-coded list/match path with a real registry abstraction or adapt it into the new core. |
| `clients/agent-runtime/src/session_commands/types.rs` | Modified | Add shared command descriptor/metadata and dispatch contract types. |
| `clients/agent-runtime/src/session_commands/parser.rs` | Modified | Align parser output with canonical command names/alias resolution expectations. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modified | Keep session-specific behavior behind registered handlers instead of registry branching. |
| `clients/agent-runtime/src/session_commands/mod.rs` | Modified | Expose the central registry API and registration surface. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Preserve ingress interception while delegating recognized commands to the central registry core. |
| `clients/agent-runtime/src/gateway/` | Modified | Verify existing gateway/webhook paths continue to rely on the shared ingress seam. |
| `clients/agent-runtime/src/channels/` | Modified | Verify channel ingress preserves parity with the central registry path. |
| `clients/agent-runtime/src/main.rs` | Modified | Keep CLI/direct runtime entry aligned with centralized slash-command dispatch. |
| `openspec/changes/slash-command-registry-core/` | Modified | Follow-up spec/design/tasks artifacts will define exact contracts and rollout boundaries. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| One ingress path bypasses the shared seam and misses registry dispatch | Medium | Preserve `pre_execution::evaluate_ingress(...)` as the single short-circuit contract and add cross-entry regression tests. |
| Alias support introduces ambiguous or duplicate command registration | Medium | Validate canonical names and aliases at registration time and reject duplicates deterministically. |
| Registry core grows into a broad platform rewrite | Medium | Keep this slice limited to registration, metadata, lookup, dispatch, and documentation. |
| Session-command backend/auth semantics leak into registry-core abstractions | Medium | Keep persistence and caller-scope rules inside handlers/services, not registry types. |
| Follow-up issues depend on undocumented extension behavior | Low | Document registration and layering expectations in the proposal/spec/design chain before implementation. |

## Rollback Plan

This change should be rolled back by reverting the registry-core wiring and restoring the current
hard-coded dispatch path, while leaving any additive documentation or non-destructive helper types in
place if needed. Because this slice is intended to reorganize command routing rather than mutate stored
session data, rollback should not require data migration or cleanup of persisted session state.

## Dependencies

- Existing slash-session command slice under `clients/agent-runtime/src/session_commands/`.
- Existing ingress seam in `clients/agent-runtime/src/pre_execution/mod.rs`.
- GitHub #539 under epic #527.
- Follow-up issues #540, #541, #542, #543, and #544, which depend on this foundation.

## Success Criteria

- [ ] There is a single slash command registry abstraction in the runtime.
- [ ] Commands can be registered with metadata and discovered by canonical name or alias.
- [ ] Dispatch is centralized instead of hard-coded per-command path.
- [ ] Extension points and layering are documented clearly enough for follow-up issues.
- [ ] Targeted tests cover registration, lookup, alias resolution, duplicate handling, and dispatch.
