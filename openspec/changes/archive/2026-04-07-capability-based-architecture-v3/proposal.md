# Proposal: Capability-Based Architecture v3 (M1 Design Contract)

## Intent

Corvus needs a source-of-truth architecture contract for a composable capability platform, but the current runtime is still organized around centralized bootstrap, factories, and tool-centric execution. This change defines the v3 capability model as a design/spec-only milestone so later implementation work can be incremental, secure, and testable instead of forcing a risky runtime inversion up front.

## Scope

### In Scope
- Define the Corvus v3 capability taxonomy, including executable vs descriptive capabilities and the initial capability families for provider, channel, tool, memory, observer, runtime, and security-policy-related concerns.
- Define a shared capability descriptor contract covering identity, namespace, versioning, capability kind, declared dependencies, lifecycle metadata, and security metadata.
- Define dependency semantics for later implementation, including required vs optional dependencies, compatibility/version rules, and validation expectations.
- Define migration boundaries from existing trait-based components into capability descriptors, including what remains on legacy bootstrap/factory wiring during the first implementation pass.
- Define security and approval attachment points so capability descriptors preserve or strengthen current policy semantics across agent, channels, and gateway.
- Produce formal spec and technical design artifacts only for M1.

### Out of Scope
- Implementing `CapabilityRegistry` or any runtime registry behavior.
- Implementing dependency resolution or capability graph execution.
- Changing agent, channel, gateway, provider, or memory runtime behavior.
- Dynamic plugin loading, hot-loading, or external module packaging.
- Replacing existing factories, bootstrap flow, or orchestration loops.
- Promising a generalized plugin platform before contract stability exists.

## Approach

Use a design-first change that formalizes the capability model without changing runtime behavior.

The proposal will anchor v3 around a shared descriptor contract rather than a new implementation framework. It will explicitly map existing extension seams (`Provider`, `Tool`, `Channel`, `Memory`, `Observer`, `RuntimeAdapter`) into a future capability model, while preserving current factories and dispatcher-backed execution as the compatibility baseline. For M1, the authoritative spec remains change-scoped under `openspec/changes/capability-based-architecture-v3/specs/`; canonical promotion into `openspec/specs/` is intentionally deferred until archive so verify evaluates the approved change artifact set without implying runtime adoption.

This M1 change will also define strict non-goals for the first implementation pass:
- registry before resolver,
- resolver before execution-pipeline changes,
- and no full runtime inversion until security, migration, and parity rules are specified.

The design should reuse proven patterns already present in the codebase, especially:
- self-description via `ToolSpec`,
- explicit feature declaration via `ProviderCapabilities`,
- namespaced discovery and collision handling from MCP capability discovery,
- and canonical entry-point parity requirements from the agent-loop and MCP specs.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/capability-based-architecture-v3/proposal.md` | New | Defines intent, scope, risk, and phased delivery for M1. |
| `openspec/changes/capability-based-architecture-v3/exploration.md` | Referenced | Exploration evidence for current coupling, risks, and recommended sequencing. |
| `openspec/specs/agent-loop/spec.md` | Referenced | Preserves canonical parity expectations across CLI, channels, and gateway. |
| `openspec/specs/mcp-runtime/spec.md` | Referenced | Reuses current approval, namespacing, and bounded discovery expectations. |
| `openspec/specs/mcp-platform-capabilities/spec.md` | Referenced | Reuses existing capability-oriented naming, gating, and discovery patterns. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Planned Future Impact | Primary composition root that later changes must adapt incrementally, not replace in M1. |
| `clients/agent-runtime/src/agent/agent.rs` | Planned Future Impact | Current tool-centric orchestration that later phases must decouple carefully. |
| `clients/agent-runtime/src/agent/dispatcher.rs` | Planned Future Impact | Current approval/risk attachment point that v3 descriptors must preserve. |
| `clients/agent-runtime/src/tools/mod.rs` | Planned Future Impact | Likely first implementation seam for registry-backed descriptive registration. |
| `clients/agent-runtime/src/channels/mod.rs` | Planned Future Impact | Separate runtime path that must stay behaviorally aligned during migration. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Planned Future Impact | Canonical webhook path whose security and parity semantics must remain unchanged. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Capability contracts weaken or bypass current approval semantics | Medium | Specify that descriptors must preserve deny/approval attachment points already enforced in dispatcher/security policy. |
| Agent, channels, and gateway drift into inconsistent capability behavior later | Medium | Make canonical parity and migration invariants explicit in the spec/design before any runtime work begins. |
| A future registry becomes indirection without real decoupling | High | Define registry scope narrowly as descriptive registration first; forbid broad factory replacement in the first implementation pass. |
| Later discovery/resolution work adds startup cost or blocking behavior | Medium | Carry forward bounded discovery, collision handling, and partial-failure isolation expectations from MCP specs. |
| The proposal is treated as permission for immediate runtime refactor | Medium | State explicitly that M1 is design/spec only and that M2-M5 must be split into separate changes. |

## Rollback Plan

This is a design/spec-only change, so rollback is straightforward: revert the proposal/spec/design artifacts for `capability-based-architecture-v3` and keep the runtime on the current trait/factory architecture with no code-path impact. Because M1 introduces no runtime behavior changes, rollback does not require data migration, config rollback, or operational mitigation.

## Dependencies

- `openspec/changes/capability-based-architecture-v3/exploration.md`
- `openspec/specs/agent-loop/spec.md`
- `openspec/specs/mcp-runtime/spec.md`
- `openspec/specs/mcp-platform-capabilities/spec.md`
- Follow-up changes for M2-M5 after M1 is approved:
  - M2: descriptive `CapabilityRegistry`
  - M3: dependency resolution
  - M4: execution pipeline seam
  - M5: integration tests and documentation

## Success Criteria

- [ ] A formal M1 spec/design package defines the v3 capability taxonomy, descriptor contract, dependency semantics, migration boundaries, and security attachment points.
- [ ] The proposal explicitly states that M1 is design/spec only and defers all runtime implementation to later changes.
- [ ] The proposal identifies affected modules/packages and preserves current canonical parity/security expectations as non-negotiable migration constraints.
- [ ] Later implementation work can be split into separate M2-M5 changes without re-deciding the core capability contract.
- [ ] The proposal clearly prevents fake plugin architecture by forbidding broad registry/pipeline/runtime replacement in the first pass.
