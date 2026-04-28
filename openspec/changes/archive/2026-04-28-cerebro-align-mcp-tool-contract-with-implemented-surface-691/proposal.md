# Proposal: Align Cerebro MCP Tool Contract With Implemented Surface

## Intent

Align the published Cerebro MCP contract with the tools the service actually implements today so downstream runtime, gateway, dashboard, and docs behavior stop advertising unavailable capabilities as callable. This change narrows the declared supported surface to current reality while preserving explicit `NotImplemented` responses for known future tools where that behavior is still intentional.

## Scope

### In Scope
- Update the OpenSpec contract to distinguish currently implemented Cerebro MCP tools from planned-but-unimplemented tools that return `NotImplemented`.
- Align inventory, availability, and introspection expectations with the server's currently supported behavior, including the fact that current server handling appears centered on `tools/call`.
- Align downstream runtime/dashboard/docs expectations so `mem_context` and the other unimplemented tools are not treated as available for normal use.
- Add or tighten tests/spec expectations around tool inventory and unavailable-tool behavior to prevent future contract drift.

### Out of Scope
- Implementing `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, or `mem_context`.
- Expanding Cerebro to a larger MCP surface than is already implemented.
- Reworking Cerebro transport architecture beyond the minimum wording needed to reflect current behavior.
- Designing the eventual product contract for session-oriented or prompt-memory workflows beyond preserving their deferred status.

## Approach

Adopt the smallest safe reviewable change: update the source-of-truth specs and adjacent product wording to publish an 8-tool supported surface for normal operation, explicitly document the 5 deferred tools as intentionally unavailable or reserved for future implementation, and remove wording that implies full introspection parity when the server does not currently support it. Then align gateway/admin/dashboard-facing expectations so UI and runtime logic do not present `mem_context` as an available callable capability, and add verification that supported tools are listed consistently while deferred tools fail with structured `NotImplemented` behavior where applicable.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/cerebro/spec.md` | Modified | Narrow canonical Cerebro MCP tool contract to the implemented surface and document deferred tools explicitly. |
| `openspec/specs/gateway/spec.md` | Modified | Align gateway-facing expectations and any HTTP/admin wording that currently implies unavailable Cerebro tools are callable or visible as active. |
| `openspec/specs/memory-visibility/spec.md` | Modified | Clarify dashboard/admin memory visibility behavior so `mem_context` is not treated as an available Cerebro capability. |
| `clients/cerebro/src/tools.rs` | Modified | Preserve or clarify structured `NotImplemented` semantics for deferred tools and ensure supported inventory is authoritative. |
| `clients/cerebro/src/server.rs` | Modified | Align introspection and tool-surface behavior with current supported endpoints and documented expectations. |
| `clients/cerebro` contract tests | Modified | Assert supported inventory, excluded/deferred tools, and unavailable-tool behavior. |
| `clients/web/apps/docs` and related docs | Modified | Remove wording that implies the full 13-tool surface is implemented today. |
| downstream runtime/dashboard logic for `mem_context` availability | Modified | Stop advertising or assuming `mem_context` is callable when Cerebro returns `NotImplemented`. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Downstream consumers may rely on the old 13-tool declaration | Medium | Preserve explicit deferred-tool semantics in code/docs and call out the contract correction clearly in specs and release notes/docs. |
| Narrowing the published surface may conflict with future roadmap expectations | Low | Document deferred tools as reserved/planned rather than silently removing them from history. |
| Introspection wording may still be ambiguous if server behavior differs by endpoint | Medium | Specify exactly what is guaranteed now and add tests around current supported discovery behavior. |

## Rollback Plan

Revert the spec and documentation deltas, restore the previous published inventory wording, and back out any downstream gating/UI changes that hide deferred tools. If rollback is required after runtime/dashboard changes ship, keep the code returning structured `NotImplemented` for deferred tools so behavior remains safe while the contract is restored.

## Dependencies

- Existing Cerebro implementation in `clients/cerebro/src/tools.rs` and `clients/cerebro/src/server.rs` as the current behavioral source for this correction.
- Downstream runtime/dashboard surfaces that currently consume or present Cerebro tool availability.

## Success Criteria

- [ ] OpenSpec deltas clearly separate supported Cerebro MCP tools from deferred `NotImplemented` tools.
- [ ] No spec, doc, runtime, or dashboard surface claims `mem_context` is currently available for normal use.
- [ ] Contract tests verify the supported inventory and structured unavailable behavior for deferred tools.
- [ ] Published wording no longer depends on unsupported or stale introspection assumptions.
