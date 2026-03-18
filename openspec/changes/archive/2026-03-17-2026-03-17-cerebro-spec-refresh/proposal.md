# Proposal: Cerebro Spec Refresh (2026-03-17)

## Intent

Refresh the Cerebro specification to reflect the latest product philosophy, architecture, and tool
surface while reconciling conflicts between the current spec and the archived Cerebro change. The
goal is a single, coherent spec that matches the intended Rust-based MCP service, embedded
SurrealDB posture, and optional TUI without regressing the security-first and agent-runtime
separation guarantees.

## Scope

### In Scope
- Update `openspec/specs/cerebro/spec.md` to incorporate the user-provided philosophy, data model,
  MCP tools, memory hygiene, and TUI scope.
- Align the spec with the archived change at
  `openspec/changes/archive/2026-03-16-cerebro/` where it clarifies tool contracts, migration
  boundaries, and security defaults.
- Explicitly document deltas and conflicts (embedded SurrealDB and TUI scope vs current spec) and
  resolve them in the refreshed spec text.
- Ensure the spec continues to emphasize agent-runtime separation from storage, MCP auth, and
  secure defaults.

### Out of Scope
- Implementing code changes in `modules/cerebro` or `clients/agent-runtime`.
- Creating new spec/design/tasks artifacts beyond the refreshed spec.
- Shipping the TUI, SurrealDB embedding, or migration tooling; this change only updates spec text.

## Approach

Produce a spec refresh that consolidates the current spec and the archived change narrative into a
single source of truth, then apply the user-provided highlights as authoritative for philosophy,
tech, MCP tools, data model, memory hygiene, and TUI expectations. Where conflicts exist, the
refreshed spec will make an explicit decision and document rationale.

### Conflicts / Deltas to Resolve
- **Embedded SurrealDB**: Current spec removes SurrealDB from the runtime and declares embedded
  SurrealDB out of scope; the new proposal must clarify that embedded SurrealDB is a Cerebro
  service deployment mode (not agent-runtime) and update scope accordingly.
- **TUI scope**: Current spec marks TUI out of scope; the new proposal must decide whether TUI is
  part of the core spec or explicitly staged as optional. The user-provided spec expects a TUI
  dashboard, explorer, timeline, and live logs.
- **Prompt template**: Add the `prompt_template.md` integration guidance for drill-in and
  What/Why/Where/Learned format if absent or underspecified.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/cerebro/spec.md` | Modified | Refresh spec to match updated philosophy, tools, data model, and scope. |
| `openspec/changes/archive/2026-03-16-cerebro/` | Reference | Use archived change as authoritative background for tool and migration context. |
| `openspec/changes/cerebro/cerebro.md` | Reference | Confirm tool surface and security expectations remain aligned. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Spec ambiguity around embedded SurrealDB vs runtime removal | Medium | Explicitly scope embedded SurrealDB to the Cerebro service only; reaffirm runtime separation. |
| TUI scope inflation without implementation plan | Medium | Mark TUI as optional phase with clear boundaries and non-blocking status. |
| Tool contract drift from existing MCP expectations | Low | Cross-check `openspec/changes/cerebro/cerebro.md` and archived change tool list. |

## Rollback Plan

Revert `openspec/specs/cerebro/spec.md` to the previous version and reassert the current scope
statements (embedded SurrealDB and TUI out of scope) if the refreshed spec introduces conflicting
guidance or breaks alignment with existing implementations.

## Dependencies

- `openspec/specs/cerebro/spec.md` (current baseline)
- `openspec/changes/archive/2026-03-16-cerebro/` (prior change context)
- `openspec/changes/cerebro/cerebro.md` (tool contracts and security defaults)

## Success Criteria

- [ ] The refreshed spec explicitly reflects the user-provided philosophy, data model, MCP tools,
      memory hygiene, and agent prompt guidance.
- [ ] Conflicts (embedded SurrealDB and TUI scope) are resolved and clearly documented.
- [ ] The spec continues to enforce runtime separation, MCP auth, and secure defaults.
