# Proposal: Tooling parity for search, fetch, and task tools #536

## Intent

Close the highest-value Claude-style tooling parity gaps in Corvus by formalizing the already implemented search, fetch, and persistent task tools as a stable parity family, then adding compatibility aliases and published mapping metadata without disrupting existing PascalCase-first runtime surfaces.

The repository already exposes the relevant tool backends and a main `tooling-parity` specification that treats `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` as canonical parity names. What remains for this change is to make runtime-facing parity more practical for Claude-style and script-oriented consumers by publishing and resolving snake_case compatibility aliases consistently while preserving current security and backend behavior.

## Why

Corvus already has most of the functional capability needed for search, fetch, and task parity, but operator and agent-facing surfaces still have a naming and discoverability gap:
- existing PascalCase names are stable for current Corvus surfaces;
- Claude-style and script-oriented workflows commonly expect snake_case names such as `glob`, `grep`, `web_fetch`, and `task_*`;
- without published mapping and alias-aware resolution, parity remains partially implicit and easier to misinterpret.

This change should make those relationships explicit and durable without forcing disruptive renames.

## Scope

### In Scope
- Define the change as an additive parity slice in the existing `tooling-parity` domain.
- Add stable compatibility aliases for the search, fetch, and persistent task parity family.
- Define published mapping requirements so inventory surfaces and agent-facing documentation present canonical names and aliases consistently.
- Preserve existing tool implementations, security boundaries, backend availability behavior, and canonical PascalCase names.
- Define bounded regression expectations for alias invocation parity and deterministic inventory publication.

### Out of Scope
- Renaming or removing the existing PascalCase canonical tool names.
- Replacing current tool backends or changing their security model.
- Introducing new search, fetch, or task tool capabilities beyond naming, publication, and resolution parity.
- Broad tool inventory redesign unrelated to parity mapping.
- Changing non-parity tool families that are not part of this slice.

## Affected Areas

### Affected modules/packages
- Runtime tool registration and inventory publication surfaces in `clients/agent-runtime`
- Existing tool implementations for `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop`
- Agent-facing or operator-facing documentation surfaces that publish tool inventory or mapping

### Affected spec domains
- `tooling-parity`

Rationale:
- The main `tooling-parity` spec already owns the canonical parity contract for these tools.
- This slice is additive and should extend that existing domain rather than creating a new one.

## Success Criteria

- Canonical PascalCase names remain stable and supported.
- Snake_case compatibility aliases resolve to the same implementations and behavior as their canonical counterparts.
- Published tool inventory and docs clearly distinguish canonical names from aliases in a deterministic format.
- Alias invocation parity is regression-tested so behavior, permissions, and backend support do not diverge.

## Risks

- Alias publication can create confusion if canonical versus compatibility status is not clearly labeled.
- Tool registration may drift if aliases are implemented ad hoc instead of through shared metadata.
- Changing visible inventory output could surprise existing consumers if order or naming presentation is unstable.

## Rollback Plan

If alias-aware parity introduces confusion or runtime regression:
- remove alias resolution while keeping existing canonical PascalCase tools intact;
- revert inventory/documentation mapping changes;
- preserve the underlying tool implementations and current security boundaries.

This rollback is low risk because the change is additive to naming and publication behavior rather than a replacement of core tool functionality.
