# Proposal: Descriptive Capability Registry

## Intent

Implement GitHub issue #453 by introducing the M2 descriptive `CapabilityRegistry` seam for the safest initial family: tool-facing capabilities. The goal is to prove the capability architecture contract with real runtime-backed registration for native tools plus MCP-discovered tools/resources/prompts, while preserving the current Rust runtime behavior, approval semantics, and `Vec<Box<dyn Tool>>` execution flow as the compatibility baseline.

## Scope

### In Scope
- Introduce a runtime-facing `CapabilityDescriptor` and non-executing `CapabilityRegistry` for **tool-family capabilities only**.
- Register descriptors for active native tools and MCP-discovered tools/resources/prompts currently exposed through the tool layer.
- Finalize descriptor registration after final tool selection in bootstrap so the registry describes the exact runtime-visible tool set.
- Validate M2 descriptor shape/completeness, namespaced identity uniqueness, and collision handling deterministically.
- Preserve current security-relevant identity metadata needed by approval, policy, audit, and profile gating behavior.
- Add deterministic unit/bootstrap tests for descriptor building, uniqueness validation, and collision scenarios.

### Out of Scope
- Dependency resolution or compatibility solving.
- Registry-driven execution, dispatch, or provider/channel tool serialization changes.
- Execution-pipeline refactors or replacing `Vec<Box<dyn Tool>>` as runtime authority.
- Provider, channel, memory, observer, runtime, or security-policy family rollout.
- Renaming tool identifiers or changing MCP transport/runtime behavior.
- Broader adoption work planned for later M3/M4/M5 phases.

## Approach

Follow the exploration recommendation: introduce a new non-executing registry/descriptor module and keep execution ownership in the existing tool vector.

The implementation should:
- reuse current canonical tool names as descriptor ids so policy and dispatch semantics remain stable,
- derive descriptors from the final filtered tool set in bootstrap,
- reuse MCP canonical normalization rules already implemented under `clients/agent-runtime/src/tools/mcp/normalize.rs`,
- populate the shared descriptor minimum fields required by the canonical capability spec using deterministic M2 defaults,
- keep family-specific metadata separate from the shared contract,
- validate shape and uniqueness without attempting dependency resolution or execution binding.

This keeps M2 descriptive-only, matches the M1 contract, and preserves rollback clarity.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modified | Finalize registry creation after profile filtering so descriptors match active runtime-visible tools. |
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Preserve native tool assembly while exposing the minimum hooks needed for descriptive registration. |
| `clients/agent-runtime/src/tools/traits.rs` | Modified | Reuse or extend existing tool metadata seams without making the registry execution authority. |
| `clients/agent-runtime/src/tools/mcp/mod.rs` | Modified | Preserve current MCP discovery behavior while making normalized identities available to descriptor registration. |
| `clients/agent-runtime/src/tools/mcp/normalize.rs` | Modified | Reuse canonical namespacing rules for registry descriptor identity. |
| `clients/agent-runtime/src/tools/mcp/adapter.rs` | Modified | Surface stable MCP tool descriptor inputs without changing execution behavior. |
| `clients/agent-runtime/src/tools/mcp/resource_adapter.rs` | Modified | Provide cleaner descriptor metadata for MCP resources than current `ToolSpec.source` alone. |
| `clients/agent-runtime/src/tools/mcp/prompt_adapter.rs` | Modified | Provide cleaner descriptor metadata for MCP prompts than current `ToolSpec.source` alone. |
| `clients/agent-runtime/src/capabilities/` or similar new module | New | Hold `CapabilityDescriptor`, `CapabilityRegistry`, validation, and tool-family registration helpers. |
| `clients/agent-runtime/src/agent/agent.rs` | No behavioral change expected | Execution must remain bound to `Vec<Box<dyn Tool>>`. |
| `clients/agent-runtime/src/agent/dispatcher.rs` | No behavioral change expected | Approval logic must continue to rely on stable names and current policy semantics. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Descriptor ids drift from current tool names and weaken approval/profile behavior | High | Reuse `tool.name()` / current canonical MCP names as registry ids; treat name changes as out of scope. |
| Registry accidentally becomes execution authority | Medium | Keep dispatch, lookup, and provider/channel tool-spec generation on existing legacy paths in M2. |
| Collision handling becomes inconsistent or confusing | Medium | Define one deterministic M2 validation/reporting rule and test native/native, native/MCP, and MCP/MCP cases explicitly. |
| MCP resource/prompt metadata is incomplete or lossy | Medium | Build explicit descriptor mapping helpers instead of blindly reusing current `ToolSourceMetadata`. |
| Registry is created before profile filtering and describes inactive capabilities | Medium | Finalize registration only after bootstrap completes final tool selection. |
| M2 scope expands into M3/M4 work | Medium | Keep dependencies empty/defaulted in M2 and prohibit registry-driven execution changes in the proposal/specs/design. |

## Rollback Plan

If M2 introduces instability, remove the new registry/descriptor module and the bootstrap registration call sites, then fall back to the current tool assembly path only. Because execution, dispatch, approvals, and provider/channel serialization remain owned by the existing `Vec<Box<dyn Tool>>` path, rollback should be limited to deleting descriptive registration wiring and associated tests without changing runtime behavior.

## Dependencies

- Canonical architecture contract: `openspec/specs/capability-architecture/spec.md`
- Exploration findings: `openspec/changes/descriptive-capability-registry/exploration.md`
- Existing native/MCP tool seams in `clients/agent-runtime/src/tools/` and `clients/agent-runtime/src/bootstrap/`

## Success Criteria

- [ ] A non-executing `CapabilityDescriptor` and `CapabilityRegistry` exists for tool-family capabilities only.
- [ ] The registry contains descriptors for active native tools and active MCP-discovered tools/resources/prompts after final bootstrap tool selection.
- [ ] Descriptor validation rejects incomplete descriptors and duplicate namespaced identities deterministically.
- [ ] Existing runtime execution still resolves and executes tools from `Vec<Box<dyn Tool>>` with no registry-driven dispatch.
- [ ] Current approval/profile behavior remains keyed to the same tool names and MCP naming conventions.
- [ ] Deterministic tests cover descriptor building, uniqueness/collision handling, and bootstrap parity with the current tool set.
