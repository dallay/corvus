# Proposal: Capability-Based Agent Composition

## Intent

Implement the first production delivery of the capability-based architecture described in `tmp/PRD-Capability-Based-Architecture-for-Composable-AI-Agents.md` for `clients/agent-runtime`.

This change turns the existing architectural direction into a usable runtime path by completing capability crate extraction, introducing real registry/factory APIs, aligning the agent manifest with the PRD v1 contract, and delivering a boot-time composition MVP that resolves a manifest into the existing `AgentBuilder`.

This proposal extends `openspec/specs/capability-architecture/spec.md`; it does not create a competing capability model.

## Scope

### In Scope

- Complete the extraction path for reusable capability families used by composed agents.
- Add the missing `corvus-observability` crate.
- Introduce real registry/factory APIs in extracted capability crates instead of placeholder re-exports.
- Replace stale hardcoded capability tables in `corvus-composer` with registry-backed validation.
- Align the composer manifest schema with the PRD v1 TOML contract.
- Implement boot-time composition MVP that resolves manifest-selected capabilities into the existing runtime `AgentBuilder`.
- Preserve the main runtime binary as the full-capability compatibility path.
- Add targeted regression, parity, and validation tests for manifest resolution and capability availability.

### Out of Scope

- Dynamic plugin loading or hot-swapping.
- Cross-language capability authoring.
- Full runtime inversion or replacement of the existing bootstrap baseline.
- Broad dependency-resolution orchestration beyond deterministic v1 manifest validation and composition.
- GUI-based agent builders.
- Rewriting trait contracts already extracted into `corvus-traits` unless required for compatibility fixes.

## Approach

Implement the change in thin, reversible phases:

1. Complete crate extraction so each capability family has a real crate-level registry/factory surface.
2. Keep root runtime modules as compatibility shims while the extracted crates become the reusable source of truth.
3. Upgrade `corvus-composer` to the PRD manifest shape and validate requested capabilities against live registries.
4. Add a boot-time composition path that maps manifest configuration into runtime components and feeds the existing `AgentBuilder`.
5. Verify behavioral parity between composed agents and the monolithic runtime for canonical execution paths.

This follows the migration boundaries in `openspec/specs/capability-architecture/spec.md`: preserve current runtime behavior as the compatibility baseline, defer full inversion, and keep deterministic validation semantics.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `clients/agent-runtime/Cargo.toml` | Update | Add/adjust workspace crates and feature strategy for capability composition. |
| `clients/agent-runtime/crates/corvus-*` | Major | Move from placeholder extraction to reusable registries/factories and PRD-aligned composition support. |
| `clients/agent-runtime/src/agent/agent.rs` | Integrate | Reuse the existing `AgentBuilder` as the composition target. |
| `clients/agent-runtime/src/bootstrap/` | Preserve + adapt | Keep the current bootstrap baseline while introducing composed boot paths. |
| `clients/agent-runtime/src/composer.rs` | Major | Replace placeholder CLI behavior with real manifest-driven composition flows. |
| `openspec/specs/capability-architecture/spec.md` | Delta reference | Existing architecture contract extended by this implementation change. |
| `openspec/changes/capability-based-agent-composition/specs/agent-composer/spec.md` | New | Define the manifest and composition behavior requirements. |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Extraction refactor introduces behavioral drift from the monolithic runtime | Medium | Keep root shims, reuse existing builder/runtime seams, add parity tests before removing any compatibility logic. |
| Registry-backed validation rejects valid runtime capabilities because registration is incomplete | Medium | Make registries the single source of truth and add tests that compare registry availability to known runtime paths. |
| Composer manifest migration breaks existing templates or placeholder flows | Medium | Add migration-focused tests and update templates in the same change. |
| Observer extraction adds coupling late in the implementation | Medium | Add `corvus-observability` early and integrate it before final composer wiring. |
| Scope expands into full dependency resolution or runtime inversion | High | Keep MVP boundary explicit in spec, design, and tasks; defer broader dependency orchestration to follow-up changes. |

## Rollback Plan

If the change introduces regressions, roll back by disabling the new compose-from-manifest runtime path and keeping the existing monolithic bootstrap path as the only active composition route. Because the runtime baseline remains preserved during this change, rollback is primarily code-path selection rather than architectural recovery.

## Dependencies

- `tmp/PRD-Capability-Based-Architecture-for-Composable-AI-Agents.md`
- `openspec/specs/capability-architecture/spec.md`
- Existing `AgentBuilder` and root runtime bootstrap flow in `clients/agent-runtime`

## Success Criteria

- [ ] Extracted capability crates expose real registry/factory APIs needed for composition.
- [ ] `corvus-observability` exists and participates in composition.
- [ ] `corvus-composer` supports a PRD-aligned manifest v1 schema.
- [ ] Manifest validation uses live registries rather than stale hardcoded lists.
- [ ] A manifest can be resolved into the existing `AgentBuilder` at boot time for the MVP path.
- [ ] The main runtime remains a valid full-capability agent path.
- [ ] Targeted tests cover manifest validation, unavailable capabilities, and behavioral parity on canonical paths.
