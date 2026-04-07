# Design: Capability-Based Architecture v3 (M1 Design Contract)

## Technical Approach

This M1 change defines a capability-descriptor architecture layer **without** changing the current Rust runtime behavior. The design introduces a contract model that sits **alongside** the existing trait-based runtime seams (`Provider`, `Tool`, `Channel`, `Memory`, `Observer`, `RuntimeAdapter`) rather than replacing them.

The immediate goal is to separate:
- **descriptor concerns**: identity, family, dependency intent, lifecycle intent, security intent, compatibility intent
- **implementation concerns**: actual Rust traits, factories, bootstrap wiring, dispatcher behavior, channel loops, and gateway execution

This maps directly to the proposal and spec:
- the runtime remains bootstrap/factory/dispatcher-driven in M1,
- capability descriptors become the future contract source of truth,
- and later phases adopt the contract in controlled steps: registration first, resolution second, execution third.

## Architecture Decisions

### Decision: Descriptors are contracts, not runtime implementations

**Choice**: Model capabilities as metadata-bearing contract descriptors that reference or describe existing runtime implementations, but do not replace those implementations in M1.

**Alternatives considered**:
- Treat existing trait objects as the capability model directly.
- Introduce a new runtime abstraction layer and immediately route execution through it.

**Rationale**: The current runtime is already trait-driven, but the coupling problem is not missing indirection; it is missing a shared contract model. Making descriptors the contract layer avoids fake progress while preserving existing runtime stability.

### Decision: Preserve family boundaries explicitly

**Choice**: Keep provider, tool, channel, memory, observer, runtime, and security-policy-related concerns as separate capability families under one shared descriptor schema.

**Alternatives considered**:
- Define one generic `Capability` type with minimal specialization.
- Restrict v3 to tools only.

**Rationale**: One generic type would erase meaningful differences in lifecycle, security, and execution semantics. Tools-only would be too narrow for the stated v3 architecture goal. Family boundaries preserve clarity while still allowing a shared contract.

### Decision: Keep M1 behaviorally inert

**Choice**: M1 produces only artifacts (`proposal.md`, spec, `design.md`) and does not require registry behavior, dependency resolution, or execution changes.

**Alternatives considered**:
- Add a prototype registry during M1.
- Start migrating bootstrap/factories immediately.

**Rationale**: The exploration showed strong coupling in `bootstrap/mod.rs`, `agent/agent.rs`, `channels/mod.rs`, and `gateway/mod.rs`. Changing execution before settling the contract would increase migration and security risk.

### Decision: Use descriptive registration as the first implementation seam

**Choice**: M2 should wrap capability-like surfaces first, especially native tools plus MCP tools/resources/prompts, before any broader runtime inversion.

**Alternatives considered**:
- Start with providers first.
- Start with channels first.
- Start with all families in one registry pass.

**Rationale**: `ToolSpec`, MCP namespacing, and existing discovery/merge patterns are already close to a descriptor model. That makes tools the lowest-risk proving ground for the registry.

### Decision: Security semantics remain attached to capability identity

**Choice**: Descriptor design carries explicit security metadata so current approval and policy behavior remains keyed to stable, namespaced identity.

**Alternatives considered**:
- Infer security policy from family only.
- Push approval semantics entirely into later runtime code.

**Rationale**: Current enforcement is identity-sensitive (`agent/dispatcher.rs`, `security/policy.rs`) and already uses namespaces such as `mcp.<server>...`. Losing stable identity would weaken policy and auditability.

## Data Flow

### Current runtime composition baseline

```text
Config
  │
  ▼
bootstrap::BootstrapContext::from_config()
  ├── runtime::create_runtime()
  ├── memory::create_memory()
  ├── observability::create_observer()
  ├── SecurityPolicy::from_config()
  └── tools::all_tools_with_runtime()
         └── optional MCP discovery/extension
                │
                ▼
        Vec<Box<dyn Tool>>
                │
                ▼
       Agent / Channels / Gateway paths
```

### Proposed M1 contract relationship

```text
Existing Rust implementations
  ├── Provider trait impls
  ├── Tool trait impls
  ├── Channel trait impls
  ├── Memory backends
  ├── Observer backends
  └── Runtime adapters
            │
            │ described by
            ▼
Capability Descriptors (contract layer only in M1)
  ├── identity + namespace
  ├── family + kind
  ├── dependencies
  ├── lifecycle intent
  ├── security metadata
  └── compatibility metadata
            │
            │ later consumed by
            ▼
M2 registry → M3 resolver → M4 execution adoption
```

### Future adoption sequence diagram

```text
Author/Runtime Maintainer -> Descriptor Contract: define capability metadata
Descriptor Contract -> M2 Registry: register descriptors only
M2 Registry -> M3 Validation/Resolution: validate dependencies deterministically
M3 Validation/Resolution -> M4 Execution Adoption: bind approved resolved descriptors to runtime seams
M4 Execution Adoption -> Agent/Channels/Gateway: preserve canonical behavior and parity
```

### Compatibility-first coexistence diagram

```text
                    +------------------------------+
                    | Existing runtime behavior    |
                    | (source of truth in M1)      |
                    +--------------+---------------+
                                   |
                                   | unchanged execution
                                   v
+------------------+       +-------+--------+       +----------------------+
| Bootstrap/factory| ----> | Agent / Channel| <---- | Gateway canonical    |
| composition      |       | loops          |       | dispatcher path      |
+------------------+       +----------------+       +----------------------+
         ^
         |
         | future descriptive mapping only
         |
+--------+------------------------------+
| Capability descriptors (M1 contract)  |
+---------------------------------------+
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/capability-based-architecture-v3/design.md` | Create | Technical design for the M1 capability architecture contract. |
| `openspec/changes/capability-based-architecture-v3/specs/capability-architecture/spec.md` | Referenced | Normative spec defining the capability contract boundaries. |
| `openspec/changes/capability-based-architecture-v3/proposal.md` | Referenced | Scope, intent, rollout boundaries, and risk framing for M1. |
| `openspec/changes/capability-based-architecture-v3/exploration.md` | Referenced | Evidence for current coupling and migration hotspots. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | No change in M1 / Planned future touchpoint | Current composition root that later phases will adapt carefully. |
| `clients/agent-runtime/src/tools/mod.rs` | No change in M1 / Planned future touchpoint | Strongest initial seam for descriptive registration in M2. |
| `clients/agent-runtime/src/agent/dispatcher.rs` | No change in M1 / Planned future touchpoint | Current approval boundary that descriptors must preserve. |
| `clients/agent-runtime/src/security/policy.rs` | No change in M1 / Planned future touchpoint | Namespace- and source-kind-based policy model to preserve. |
| `clients/agent-runtime/src/channels/mod.rs` | No change in M1 / Planned future touchpoint | Static channel registry and separate loop that must remain parity-safe. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | No change in M1 / Planned future touchpoint | Canonical gateway path whose semantics must remain aligned with agent/channels. |

## Interfaces / Contracts

### Shared descriptor schema

This is a recommended contract shape for later implementation. Fields marked **M1-required** are required by the design/spec contract; fields marked **future-facing** are allowed now so later phases do not need to redesign the schema.

```rust
struct CapabilityDescriptor {
    // M1-required
    id: CapabilityId,
    namespace: CapabilityNamespace,
    version: CapabilityVersion,
    family: CapabilityFamily,
    kind: CapabilityKind,
    dependencies: CapabilityDependencies,
    lifecycle: CapabilityLifecycle,
    security: CapabilitySecurity,
    compatibility: CapabilityCompatibility,

    // future-facing
    implementation_ref: Option<ImplementationRef>,
    family_metadata: serde_json::Value,
    registration_metadata: serde_json::Value,
}
```

### Supporting types

```rust
enum CapabilityFamily {
    Provider,
    Channel,
    Tool,
    Memory,
    Observer,
    Runtime,
    SecurityPolicy,
}

enum CapabilityKind {
    Executable,
    Descriptive,
}

struct CapabilityDependencies {
    required: Vec<CapabilityDependency>,
    optional: Vec<CapabilityDependency>,
}

struct CapabilityDependency {
    target: String,
    family: Option<CapabilityFamily>,
    version_constraint: Option<String>,
    compatibility_tags: Vec<String>,
    reason: Option<String>,
}

struct CapabilityLifecycle {
    discovery_mode: String,
    activation_mode: String,
    teardown_mode: Option<String>,
}

struct CapabilitySecurity {
    policy_scope: String,
    approval_behavior: String,
    audit_namespace: String,
    source_classification: String,
    risk_tags: Vec<String>,
}

struct CapabilityCompatibility {
    runtime_contracts: Vec<String>,
    counterpart_constraints: Vec<String>,
    entrypoint_parity_scope: Vec<String>,
}
```

### Descriptor examples

#### Tool-like capability descriptor

```yaml
id: native.tool.file_read
namespace: native.tool
version: 1.0.0
family: tool
kind: executable
dependencies:
  required:
    - target: runtime.security-policy.default
      family: security-policy
      reason: read access must be policy-evaluable
  optional: []
lifecycle:
  discovery_mode: static
  activation_mode: runtime-wired
  teardown_mode: none
security:
  policy_scope: tool
  approval_behavior: policy-defined
  audit_namespace: native.tool.file_read
  source_classification: native
  risk_tags: [read]
compatibility:
  runtime_contracts: [tool-trait-v1]
  counterpart_constraints: []
  entrypoint_parity_scope: [agent, channels, gateway]
implementation_ref: clients/agent-runtime/src/tools/file_read.rs
family_metadata:
  parameters_schema_source: Tool::parameters_schema
registration_metadata: {}
```

#### Provider-like capability descriptor

```yaml
id: native.provider.openrouter
namespace: native.provider
version: 1.0.0
family: provider
kind: executable
dependencies:
  required:
    - target: runtime.adapter.native
      family: runtime
      reason: provider requests execute through runtime adapter context
  optional:
    - target: runtime.security-policy.default
      family: security-policy
      reason: provider capability declarations may inform safety routing
lifecycle:
  discovery_mode: static
  activation_mode: bootstrap-created
  teardown_mode: runtime-owned
security:
  policy_scope: provider-metadata
  approval_behavior: delegated-to-dispatch-boundary
  audit_namespace: native.provider.openrouter
  source_classification: native
  risk_tags: [model-execution]
compatibility:
  runtime_contracts: [provider-trait-v1]
  counterpart_constraints: [supports-chat]
  entrypoint_parity_scope: [agent, channels, gateway]
implementation_ref: clients/agent-runtime/src/providers/mod.rs
family_metadata:
  capability_signals:
    - native_tool_calling
    - image_input
registration_metadata: {}
```

#### Channel-like capability descriptor

```yaml
id: native.channel.telegram
namespace: native.channel
version: 1.0.0
family: channel
kind: executable
dependencies:
  required:
    - target: native.provider.default-route
      family: provider
      reason: admitted channel turns require provider execution path
    - target: runtime.security-policy.default
      family: security-policy
      reason: canonical turns must preserve policy semantics
  optional:
    - target: native.observer.default
      family: observer
      reason: telemetry should be emitted when available
lifecycle:
  discovery_mode: static
  activation_mode: channel-runtime-startup
  teardown_mode: listener-stop
security:
  policy_scope: transport-plus-canonical-runtime
  approval_behavior: dispatcher-defined
  audit_namespace: native.channel.telegram
  source_classification: native
  risk_tags: [transport-ingress]
compatibility:
  runtime_contracts: [channel-trait-v1]
  counterpart_constraints: [canonical-dispatcher-parity]
  entrypoint_parity_scope: [channels, gateway]
implementation_ref: clients/agent-runtime/src/channels/mod.rs
family_metadata:
  transport_type: realtime
registration_metadata: {}
```

### Required vs future-facing fields

**Required in M1 contract**
- `id`
- `namespace`
- `version`
- `family`
- `kind`
- `dependencies.required`
- `dependencies.optional`
- `lifecycle`
- `security`
- `compatibility`

**Future-facing in M1**
- `implementation_ref`
- `family_metadata`
- `registration_metadata`
- normalized counterpart capability tags
- richer version-constraint grammar
- activation hints for later runtime orchestration

The rule is simple: M1 defines the shape and meaning, not the runtime mechanics.

## Dependency Model Design

### Representation

Dependencies are represented as two explicit lists:
- `required[]`
- `optional[]`

This keeps the model deterministic and easy to validate. A single list with flags was considered, but separate lists are less error-prone in both human review and later machine validation.

### Compatibility handling

Compatibility is split across two layers:
1. **Dependency-local constraints** — what a dependency expects from the target descriptor.
2. **Descriptor-wide compatibility metadata** — broader assumptions such as supported runtime contracts or parity scope.

This separation prevents dependency edges from becoming overloaded with unrelated runtime metadata.

### Deterministic validation model for M3

M3 should validate in this order:
1. **Schema completeness** — all required fields present.
2. **Identity validity** — namespace/id/version shape valid.
3. **Family/kind validity** — valid combinations only.
4. **Dependency shape validity** — every dependency is structurally valid.
5. **Constraint validity** — version/compatibility constraints parse successfully.
6. **Reference validity** — dependency targets resolve uniquely in the candidate descriptor set.
7. **Compatibility evaluation** — constraints are evaluated deterministically.
8. **Cycle and ambiguity checks** — only where applicable to the family model.

Determinism rules:
- identical descriptor sets MUST yield identical validation results,
- results MUST NOT depend on registration order,
- ambiguous references MUST be rejected, not guessed,
- optional dependencies MAY be absent, but if present they MUST still validate cleanly.

## Migration / Rollout

### Coexistence model

Current architecture remains the runtime truth:
- `bootstrap::BootstrapContext` still creates runtime, memory, observer, security, and tool lists.
- `tools::all_tools_with_runtime()` still produces `Vec<Box<dyn Tool>>`.
- `agent/dispatcher.rs` still evaluates risk from stable tool identities.
- `channels/mod.rs` still uses `CHANNEL_REGISTRY` and the current channel loop.

Capability descriptors coexist as **non-executing metadata artifacts** in M1.

### What should be adapted first later

1. **Tool-like capabilities**
   - Best first fit because `ToolSpec` already exists.
   - MCP already proves namespacing, discovery, collision handling, and partial-failure isolation.
2. **Provider-like descriptors**
   - Good second step because `ProviderCapabilities` already expresses explicit feature declarations.
3. **Memory/observer/runtime descriptors**
   - Good later for completeness, but not first because they do not yet resemble a shared registry shape as directly as tools do.
4. **Channel descriptors**
   - Later than tools/providers because channels include transport/runtime parity risk.

### What should explicitly NOT be adapted first

- Full bootstrap replacement.
- Channel listener/runtime restructuring.
- Gateway fallback removal or broad webhook behavior rewiring.
- Security policy inversion away from stable namespaced identity.
- Dynamic plugin loading or external module packaging.
- Cross-family dependency resolution mixed with execution changes in the same phase.

### Rollback philosophy

Every later phase must be reversible to the previous compatibility baseline:
- **M2 rollback**: remove descriptor registration while keeping factories/tool assembly intact.
- **M3 rollback**: disable dependency validation/resolution without removing descriptors.
- **M4 rollback**: return execution selection to legacy bootstrap/factory/dispatcher wiring.
- **M5 rollback**: revert expanded adoption while preserving validated contract artifacts.

For M1 specifically: no migration required, because runtime behavior does not change.

Canonical promotion decision for M1: no `openspec/specs/capability-architecture/spec.md` file is required before verify. The approved change-scoped spec under `openspec/changes/capability-based-architecture-v3/specs/` remains authoritative for M1, and any main-spec promotion is deferred to archive after verification.

## Security Model Mapping

### Current security anchors in the runtime

The current runtime ties approval and policy to stable identity:
- `agent/dispatcher.rs` classifies tool risk using tool names and `source_kind_for_tool()`.
- `security/policy.rs` distinguishes `Native`, `Mcp`, `McpResource`, and `McpPrompt` using namespaced identity.
- MCP specs already require canonical namespacing and deny/approval semantics.

### Descriptor mapping

Descriptor security metadata should map current behavior like this:

| Current concept | Descriptor field | Why |
|---|---|---|
| tool/provider/channel identity | `id`, `namespace`, `security.audit_namespace` | preserves stable policy and audit reference |
| source kind (`Native`, `Mcp`, etc.) | `security.source_classification` | preserves current classification intent |
| approval requirement semantics | `security.approval_behavior` | makes approval intent explicit without weakening enforcement |
| policy boundary ownership | `security.policy_scope` | prevents family confusion |
| risk surface | `security.risk_tags` | future-friendly without replacing real policy |

### Guardrails to avoid weakening security

- Never infer approval only from family; keep identity- and source-aware mapping.
- Never replace namespaced identity with display names or human-friendly labels.
- Never let descriptor presence imply allow-by-default behavior.
- Never allow cross-entry-point drift in approval semantics.
- Treat descriptors as policy inputs, not policy bypasses.

### Security-specific design principle

**Descriptors may explain why a capability is risky; they do not get to decide that risk alone.**
The dispatcher/security boundary remains authoritative until a later phase explicitly and safely rebinds that authority.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Spec contract | Descriptor taxonomy, required fields, migration constraints, anti-pattern guardrails | Review against scenarios in `specs/capability-architecture/spec.md` |
| Design consistency | Alignment between proposal, spec, and runtime hotspots | Cross-check design decisions against `exploration.md`, `proposal.md`, and referenced runtime files |
| Future M2 unit tests | Descriptor normalization/registration for native tools and MCP capabilities | Add pure data-model tests and deterministic merge tests |
| Future M3 integration tests | Dependency validation, uniqueness, compatibility, deterministic outcomes | Fixture-based descriptor-set validation tests |
| Future M4 integration tests | Parity across agent/channels/gateway after execution adoption | Canonical behavior parity tests referencing existing agent-loop requirements |
| Future M5 rollout tests | Docs, migration guides, expanded family adoption | End-to-end verification plus documentation checks |

## Implementation Roadmap Design

### M2 — Descriptive registration first

**Goal**: Introduce a non-invasive registry of descriptors.

**Scope**:
- register native tools and MCP-discovered capabilities descriptively,
- preserve existing `Vec<Box<dyn Tool>>` execution path,
- do not add dependency resolution,
- do not change dispatcher execution.

**Primary touchpoints**:
- `clients/agent-runtime/src/tools/mod.rs`
- `clients/agent-runtime/src/tools/mcp/mod.rs`
- likely new descriptor/registry module under `clients/agent-runtime/src/`

### M3 — Dependency resolution and validation

**Goal**: Validate descriptor sets deterministically.

**Scope**:
- parse and validate dependency edges,
- enforce identity uniqueness and compatibility constraints,
- produce deterministic validation results,
- still avoid major execution-path rewiring.

**Primary touchpoints**:
- new validation/resolution modules,
- bootstrap preflight validation seam,
- test fixtures for descriptor graphs.

### M4 — Execution pipeline adoption

**Goal**: Introduce controlled execution binding through descriptor-backed composition.

**Scope**:
- start with tool-like execution selection only,
- preserve current approval semantics,
- preserve agent/channel/gateway parity,
- avoid broad channel/bootstrap inversion in one pass.

**Primary touchpoints**:
- `clients/agent-runtime/src/agent/agent.rs`
- `clients/agent-runtime/src/agent/dispatcher.rs`
- selective bootstrap composition seams

### M5 — Tests, docs, and broader adoption

**Goal**: Prove and document the architecture under real usage.

**Scope**:
- integration tests for parity and safety,
- operator/developer docs,
- selective expansion to additional capability families,
- migration guides and rollback instructions.

## Open Questions

- [ ] Should security-policy-related capabilities be modeled as a first-class family descriptor or as mandatory metadata attached to executable families first, with a separate family added later only if needed?
- [ ] Should `observer` and `memory` remain purely descriptive in early adoption, even if they eventually participate in composition-time validation?
- [ ] What is the minimum version-constraint grammar needed in M3 before it becomes over-engineered for the actual adoption plan?
- [ ] Should `implementation_ref` remain an opaque string/path in early phases or become a typed internal reference once registry code exists?
