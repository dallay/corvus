# Proposal: Slash Tools Listing

## Intent

Deliver the first safe expansion wave for the slash-command platform by adding a read-only `/tools`
command on top of the existing registry/context contract. This gives operators a transport-consistent
way to inspect the effective tool inventory available in the current runtime/profile without taking
on the semantic and persistence risks of settings writes, MCP mutation, or per-tool toggles.

## Scope

### In Scope
- Add a registry-backed `/tools` slash command as the first implementation slice of issue #544 under
  epic #527.
- Extend the shared slash execution inputs just enough to let the handler read the effective active
  runtime tool inventory.
- Return a read-only operator-facing listing of currently available tools, including profile- and
  MCP-derived effective availability where applicable.
- Preserve the existing transport-neutral registry dispatch and shared pre-execution handling model.
- Document the command sufficiently for operators and contributors using the established slash
  command surfaces.

### Out of Scope
- Any settings mutation commands, including `/model`, `/provider`, or `/temperature`.
- MCP mutation commands such as `/mcp add` and `/mcp remove`.
- Generic per-tool mutation commands such as `/tool enable` and `/tool disable`.
- Broader rollout of all command families from issue #544 in this change.
- Introducing a generic config-write service or new persistence model for tool toggles.

## Approach

Build `/tools` as a small, registry-backed read-only command that plugs into the existing
`session_commands` descriptor/handler model and executes through
`pre_execution::evaluate_ingress(...)` like the current built-in slash commands. The implementation
should expand the slash runtime context/service boundary narrowly to expose an effective tool
snapshot, rather than config mutation capabilities.

The listed inventory should reflect the effective runtime tool set, not merely configured tools, so
operators see what is actually available in the active profile and runtime composition. This makes
the change immediately useful and establishes a reusable metadata-access pattern for later read-only
surfaces such as `/mcp list` and richer slash help.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/slash-command-registry/spec.md` | Referenced | Existing registry/context contract and transport-parity rules this slice builds on. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Register `/tools` descriptor and handler wiring in the canonical slash registry. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modified | Expand the command service/context beyond memory-only access to expose read-only tool inventory data. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Preserve shared handled-ingress routing for the new read-only command across supported surfaces. |
| `clients/agent-runtime/src/tools/mod.rs` | Referenced/Modified | Source of effective runtime tool registry information for the `/tools` listing. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Referenced | Profile-based tool composition rules that determine what `/tools` should show. |
| `clients/agent-runtime/src/tools/mcp/mod.rs` | Referenced | MCP-derived tool discovery behavior that may contribute to the effective listing. |
| `tmp/claudio-issues/544-slash-initial-command-families.md` | Referenced | Parent implementation slice source, intentionally narrowed for this proposal. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `/tools` exposes configured tools instead of effective runtime tools | Medium | Define the output contract around active/effective availability and source data from the runtime-composed registry. |
| The current slash service boundary is too narrow for even read-only registry inspection | Medium | Limit the expansion to a read-only tool snapshot/context surface rather than broader admin or persistence capabilities. |
| Scope creep pulls in `/tool enable`, `/tool disable`, or MCP/settings writes | High | Keep proposal and follow-on specs explicit that this slice is read-only and excludes mutation semantics. |
| Operator output becomes transport-specific or inconsistent | Low | Reuse the shared registry/pre-execution seam and keep transport adaptation outside the core command outcome contract. |

## Rollback Plan

Revert the `/tools` registry binding and the narrow read-only service/context additions that support
it, restoring the slash platform to the current built-in command set only. Because this slice does
not introduce new persisted state or mutation flows, rollback should only require removing the
command registration, handler, and associated documentation/tests.

## Dependencies

- Existing slash-command registry/context contract in `openspec/specs/slash-command-registry/spec.md`
- Stable registry-backed ingress routing from the session-command migration work referenced by issue
  #544
- Effective runtime tool composition surfaces in `clients/agent-runtime/src/tools/mod.rs` and
  `clients/agent-runtime/src/bootstrap/mod.rs`

## Success Criteria

- [ ] A registry-backed read-only `/tools` command is defined as the only in-scope feature for this
      change.
- [ ] The slash execution boundary exposes enough read-only runtime metadata for `/tools` without
      introducing config-write or mutation semantics.
- [ ] `/tools` reports the effective currently available tools for the active runtime/profile,
      including MCP-derived tools when present.
- [ ] Supported ingress surfaces continue to evaluate the command through the shared pre-execution
      seam with transport-consistent handled outcomes.
- [ ] Proposal, follow-on specs, and implementation guidance explicitly defer settings writes,
      `/mcp add/remove`, `/tool enable/disable`, and the rest of the broader command-family rollout.
