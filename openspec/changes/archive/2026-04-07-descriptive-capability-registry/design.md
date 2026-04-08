# Design: Descriptive Capability Registry

## Technical Approach

Implement M2 as a **descriptive-only tool-family registry** that lives beside the existing runtime tool assembly path. The new registry will hold `CapabilityDescriptor` values for native tools and MCP-derived tool-layer surfaces, but the authoritative execution path will remain `Vec<Box<dyn Tool>>` from bootstrap through agent, channel, and gateway flows.

This design follows the canonical capability architecture contract in `openspec/specs/capability-architecture/spec.md` and the M2 delta requirements in `openspec/changes/descriptive-capability-registry/specs/capability-architecture/spec.md`. The key move is to finalize descriptor registration **after final bootstrap tool selection**, so the registry describes the exact active runtime-visible tool set instead of the broader construction candidate set.

## Architecture Decisions

### Decision: Keep the registry non-executing and bootstrap-owned

**Choice**: Introduce a new capability module under `clients/agent-runtime/src/` and attach a `CapabilityRegistry` to `BootstrapContext`, with registry finalization occurring immediately after `profile.allows_tool(tool.name())` filtering.

**Alternatives considered**:
- Build and return the registry directly from `tools::all_tools_with_runtime()`.
- Let agent/channel/provider layers build the registry lazily from `ToolSpec`.

**Rationale**: Bootstrap already defines the final runtime-visible tool set. Registering earlier would describe inactive tools that are later filtered out. Registering later in agent/channels would duplicate work and risk divergence. Keeping the registry in bootstrap preserves a single source of descriptive truth while leaving execution authority untouched.

### Decision: Reuse current tool ids as descriptor ids

**Choice**: Use the exact current runtime-visible tool identifier as `CapabilityDescriptor.id` in M2.

**Alternatives considered**:
- Introduce a new capability-specific id unrelated to `tool.name()`.
- Split ids into internal registry ids and external runtime ids.

**Rationale**: Current approval, audit, and profile behavior depends on stable tool names and the `mcp.` prefix. Reusing `tool.name()` avoids behavior drift and keeps M2 strictly descriptive.

### Decision: Treat all M2 registrations as tool-family descriptors

**Choice**: Model native tools, MCP tools, MCP resources, and MCP prompts as `family = Tool` descriptors in M2, even when their internal origins differ.

**Alternatives considered**:
- Introduce separate registry families for MCP resources and prompts in M2.
- Delay MCP resources/prompts until a later phase.

**Rationale**: All three MCP surfaces are currently exposed through the `Tool` trait and participate in the same runtime tool vector. Keeping them in the tool family preserves current runtime shape and avoids premature family expansion.

### Decision: Use explicit descriptor builders instead of deriving from `ToolSpec` alone

**Choice**: Add dedicated descriptor builder functions for native tools and MCP adapters, using `ToolSpec` as an input but not as the sole descriptor source.

**Alternatives considered**:
- Convert `ToolSpec` directly into `CapabilityDescriptor`.
- Build descriptors only from trait methods on `Tool`.

**Rationale**: `ToolSpec` is too small for the M2 contract and some MCP metadata is lossy today, especially for resources where `ToolSpec.source.original_name` currently carries the URI. Explicit builders preserve deterministic mapping and allow M2-specific defaults without mutating execution semantics.

### Decision: Fail validation deterministically, preserve runtime merge behavior separately

**Choice**: The registry API will validate descriptor completeness and uniqueness deterministically and return structured collision/validation errors. Runtime tool assembly behavior for MCP discovery/merge remains unchanged unless a minimal compatibility hook is needed to surface the same current outcome descriptively.

**Alternatives considered**:
- Make the registry silently deduplicate.
- Redesign MCP merge behavior around the registry in M2.

**Rationale**: Silent dedupe hides operator-relevant state. Rewiring MCP merge around the registry would drift into execution ownership. M2 needs explicit validation without absorbing M3/M4 concerns.

## Data Flow

### High-level runtime flow in M2

```text
Config
  │
  ▼
BootstrapContext::from_config / for_gateway
  │
  ├── tools::all_tools_with_runtime(...)
  │      │
  │      ├── native tool construction
  │      └── MCP discovery + adapter construction
  │
  ├── profile filter on Vec<Box<dyn Tool>>
  │
  ├── CapabilityRegistry::from_tools(&tools)
  │      │
  │      ├── native descriptor builders
  │      ├── MCP descriptor builders
  │      └── deterministic validation / collision checks
  │
  ▼
BootstrapContext {
  tools,                // execution authority
  capability_registry,  // descriptive metadata only
  ...
}
```

### Execution path remains unchanged

```text
BootstrapContext.tools
  │
  ├── AgentBuilder -> Agent.tools / Agent.tool_specs
  ├── channels::run_unified_channel_tool_loop(...tools_registry...)
  └── gateway/bootstrap reuse of same tool vector

Tool dispatch lookup:
  self.tools.iter().find(|t| t.name() == call.name)
```

The new registry is intentionally absent from execution lookup, dispatcher risk evaluation, and provider/channel tool payload conversion.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/capabilities/mod.rs` | Create | Module exports for capability descriptor, registry, validation, and tool-family registration helpers. |
| `clients/agent-runtime/src/capabilities/descriptor.rs` | Create | Defines `CapabilityDescriptor` and supporting enums/structs for M2. |
| `clients/agent-runtime/src/capabilities/registry.rs` | Create | Defines `CapabilityRegistry`, registration, lookup, iteration, and validation APIs. |
| `clients/agent-runtime/src/capabilities/tool_registration.rs` | Create | Maps native and MCP tool-layer runtime surfaces into M2 descriptors. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Adds registry finalization after final tool filtering and stores the registry on `BootstrapContext`. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Preserve tool construction while exposing any minimal metadata helpers needed for descriptor building. |
| `clients/agent-runtime/src/tools/traits.rs` | Modify | Reuse existing metadata types and possibly add a descriptor-support helper without changing execution semantics. |
| `clients/agent-runtime/src/tools/mcp/mod.rs` | Modify | Preserve discovery flow while exposing explicit descriptor-relevant MCP metadata if needed. |
| `clients/agent-runtime/src/tools/mcp/adapter.rs` | Modify | Expose explicit descriptor input fields for MCP tool mapping. |
| `clients/agent-runtime/src/tools/mcp/resource_adapter.rs` | Modify | Expose explicit descriptor input fields for resource mapping beyond current lossy `ToolSpec.source`. |
| `clients/agent-runtime/src/tools/mcp/prompt_adapter.rs` | Modify | Expose explicit descriptor input fields for prompt mapping beyond current `ToolSpec.source`. |
| `clients/agent-runtime/src/agent/agent.rs` | No change expected | Execution lookup must remain unchanged. |
| `clients/agent-runtime/src/channels/mod.rs` | No change expected | Channel loop must continue using runtime tools/tool specs, not the registry. |
| `clients/agent-runtime/src/providers/*` | No change expected | Provider tool payload conversion remains based on existing `ToolSpec` behavior. |

## Interfaces / Contracts

### M2 runtime descriptor schema

```rust
pub struct CapabilityDescriptor {
    pub id: String,
    pub namespace: String,
    pub version: String,
    pub family: CapabilityFamily,
    pub kind: CapabilityKind,
    pub dependencies: CapabilityDependencies,
    pub lifecycle: CapabilityLifecycle,
    pub security: CapabilitySecurity,
    pub compatibility: CapabilityCompatibility,
    pub metadata: CapabilityMetadata,
}

pub enum CapabilityFamily {
    Tool,
}

pub enum CapabilityKind {
    Executable,
}

pub struct CapabilityDependencies {
    pub required: Vec<CapabilityDependency>,
    pub optional: Vec<CapabilityDependency>,
}

pub struct CapabilityDependency {
    pub target: String,
    pub family: Option<String>,
    pub version_constraint: Option<String>,
    pub reason: Option<String>,
}

pub struct CapabilityLifecycle {
    pub discovery_mode: DiscoveryMode,
    pub activation_mode: ActivationMode,
    pub teardown_mode: Option<TeardownMode>,
}

pub enum DiscoveryMode {
    Static,
    Discovered,
}

pub enum ActivationMode {
    RuntimeWired,
}

pub enum TeardownMode {
    None,
}

pub struct CapabilitySecurity {
    pub policy_scope: String,
    pub audit_namespace: String,
    pub source_classification: SourceClassification,
    pub risk_tags: Vec<String>,
}

pub enum SourceClassification {
    Native,
    Mcp,
    McpResource,
    McpPrompt,
}

pub struct CapabilityCompatibility {
    pub runtime_contracts: Vec<String>,
    pub entrypoint_parity_scope: Vec<String>,
}

pub struct CapabilityMetadata {
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub source: Option<crate::tools::traits::ToolSourceMetadata>,
    pub mcp: Option<McpCapabilityMetadata>,
}

pub struct McpCapabilityMetadata {
    pub server: String,
    pub upstream_name: Option<String>,
    pub resource_uri: Option<String>,
    pub mime_type: Option<String>,
    pub prompt_arguments: Vec<PromptArgumentDescriptor>,
}

pub struct PromptArgumentDescriptor {
    pub name: String,
    pub description: String,
    pub required: bool,
}
```

### Required now vs deferred

#### Required in M2
- shared required fields from the spec,
- exact runtime-visible id,
- deterministic namespace,
- `family = Tool`,
- `kind = Executable`,
- structurally present dependency metadata,
- deterministic lifecycle/security/compatibility defaults,
- description + parameter schema,
- origin-sensitive MCP metadata needed to distinguish tools/resources/prompts safely.

#### Deferred beyond M2
- dependency graph semantics beyond empty/default declarations,
- compatibility constraint solving,
- execution binding or dispatch ownership,
- non-tool families,
- typed `implementation_ref` or resolver artifacts,
- broader policy-family modeling.

### Descriptor builder contracts

```rust
pub fn build_native_tool_descriptor(tool: &dyn Tool) -> Result<CapabilityDescriptor, CapabilityError>;

pub fn build_mcp_tool_descriptor(adapter: &McpToolAdapter) -> Result<CapabilityDescriptor, CapabilityError>;

pub fn build_mcp_resource_descriptor(
    adapter: &McpResourceAdapter,
) -> Result<CapabilityDescriptor, CapabilityError>;

pub fn build_mcp_prompt_descriptor(
    adapter: &McpPromptAdapter,
) -> Result<CapabilityDescriptor, CapabilityError>;
```

These builders should produce stable M2 defaults:
- `version = "1.0.0"`
- `dependencies.required = []`
- `dependencies.optional = []`
- `compatibility.runtime_contracts = ["tool-trait-v1"]`
- `compatibility.entrypoint_parity_scope = ["agent", "channels", "gateway"]`
- `lifecycle.activation_mode = RuntimeWired`
- `lifecycle.discovery_mode = Static` for native, `Discovered` for MCP-derived registrations
- `security.policy_scope = "tool"`
- `security.audit_namespace = id`

### Registry API

```rust
pub struct CapabilityRegistry {
    descriptors: Vec<CapabilityDescriptor>,
    by_id: std::collections::BTreeMap<String, usize>,
}

impl CapabilityRegistry {
    pub fn empty() -> Self;

    pub fn from_descriptors(
        descriptors: Vec<CapabilityDescriptor>,
    ) -> Result<Self, CapabilityError>;

    pub fn register(
        &mut self,
        descriptor: CapabilityDescriptor,
    ) -> Result<(), CapabilityError>;

    pub fn get(&self, id: &str) -> Option<&CapabilityDescriptor>;

    pub fn iter(&self) -> impl Iterator<Item = &CapabilityDescriptor>;

    pub fn len(&self) -> usize;

    pub fn validate_descriptor(
        descriptor: &CapabilityDescriptor,
    ) -> Result<(), CapabilityError>;
}
```

### Error model

```rust
pub enum CapabilityError {
    MissingField { field: &'static str, id: Option<String> },
    InvalidNamespace { id: String, namespace: String },
    DuplicateId { id: String },
    InvalidKindForM2 { id: String, kind: String },
    InvalidFamilyForM2 { id: String, family: String },
    InvalidMetadata { id: String, reason: String },
}
```

Design expectations:
- registration order is deterministic,
- iteration order is deterministic and matches successful registration order,
- duplicate ids fail deterministically,
- error messages are explicit enough for tests and operator diagnostics,
- registry never mutates ids or silently rewrites collisions.

## Integration Design

### `tools/mod.rs`
- Remains responsible for constructing the candidate `Vec<Box<dyn Tool>>`.
- MUST NOT return the registry or assume descriptor ownership.
- MAY expose helper functions for native-tool descriptor classification if needed, but tool construction stays imperative.

### `tools/mcp/*`
- MCP discovery remains responsible for canonical normalization and adapter construction.
- Adapters should expose explicit descriptor inputs through lightweight getters or metadata methods, not through behavior-changing refactors.
- `normalize.rs` remains the source of canonical MCP identity formats.

### `bootstrap/mod.rs`
- `BootstrapContext` gains `capability_registry: CapabilityRegistry`.
- After `tools` are filtered by `profile.allows_tool(tool.name())`, bootstrap constructs descriptors from the final active set and builds the registry.
- Both `from_config` and `for_gateway` must use the same finalization path so descriptive state stays parity-safe across entry points.

### What must remain unchanged
- `AgentBuilder` still consumes `Vec<Box<dyn Tool>>`.
- `Agent` still computes `tool_specs` from `tools.iter().map(|tool| tool.spec())`.
- `Agent::execute_tool_call()` still resolves execution with `self.tools.iter().find(|t| t.name() == call.name)`.
- `channels::run_unified_channel_tool_loop()` still builds `tool_specs` from the runtime tool vector.
- Provider conversion paths continue consuming `ToolSpec`, not descriptors.
- Dispatcher risk checks continue keying off current names and `mcp.` prefixes.

## Security and Parity Preservation

### Preserve current ids and namespace behavior
- Native descriptor ids are exactly the current tool names, like `shell` or `file_read`.
- MCP descriptor ids are exactly the normalized runtime-visible names already emitted today:
  - `mcp.<server>.<tool>`
  - `mcp.<server>.resource.<resource>`
  - `mcp.<server>.prompt.<prompt>`
- Namespace parsing in the registry must never rewrite these ids.

### Avoid accidental approval/profile drift
- Do not replace `source_kind_for_tool()` or `classify_tool_capability()` with registry lookups in M2.
- Do not infer safer behavior from descriptor metadata.
- Treat descriptor security metadata as descriptive parity data only in M2.
- Add parity tests that assert names seen by approval/profile logic are unchanged after registry introduction.

### Entry-point parity
- Build the registry in bootstrap for both agent and gateway bootstrap paths.
- Keep channel/provider behavior dependent on the same runtime tool vector.
- Ensure the registry is observational metadata shared across entry points, not a new control path.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Native descriptor builder completeness/defaults | Construct representative native tools and assert required fields, ids, lifecycle/security/compatibility defaults. |
| Unit | MCP tool/resource/prompt descriptor mapping | Build adapters from manifests and assert canonical ids plus MCP-specific metadata mapping. |
| Unit | Registry validation and deterministic iteration | Register descriptors in a known order and assert deterministic order, lookup behavior, and explicit validation failures. |
| Unit | Collision handling | Cover duplicate id rejection, native/MCP collision behavior, and MCP cross-kind coexistence when ids differ. |
| Integration | Bootstrap registry finalization | Build bootstrap contexts with profile variations and assert registry contents match final active tool names exactly. |
| Integration | Gateway bootstrap parity | Reuse gateway bootstrap path and assert the registry reflects the same active MCP/native tool set semantics. |
| Non-goal | Execution refactor coverage | Do not add registry-driven dispatch tests; only add parity assertions proving execution paths still depend on the tool vector. |

### Specific fixture ideas
- Full profile with MCP enabled: registry includes native tools + MCP tool/resource/prompt descriptors.
- Lite/code profile: registry excludes filtered-out tools exactly as runtime visibility does.
- Duplicate canonical MCP ids in one discovery set: registry construction fails deterministically.
- Same local name across MCP tool and MCP prompt/resource: both register when canonical ids differ.
- Native/MCP id conflict: explicit collision error or preserved current compatibility outcome, but always deterministic and testable.

## Migration / Rollout

No user-visible rollout or migration is required. This is an internal runtime change with strict behavioral parity expectations.

Rollout philosophy:
- keep the registry additive,
- keep execution on the current paths,
- make removal easy if the new abstraction causes confusion.

Rollback plan:
- remove `clients/agent-runtime/src/capabilities/`,
- remove `BootstrapContext.capability_registry`,
- remove bootstrap registration/finalization,
- keep `tools::all_tools_with_runtime()` and all execution paths untouched.

Because M2 does not change dispatch ownership, rollback remains local and low-risk.

## Open Questions

- [ ] Should `CapabilityRegistry` preserve successful registration order as canonical iteration order, or should it expose sorted iteration by id while keeping insertion order internal? Determinism is required either way, but tests should lock the choice down.
- [ ] Should native/MCP collision handling in the registry mirror current runtime behavior exactly when MCP extension is skipped, or should registry construction fail while runtime tool assembly still preserves the legacy path? This needs one explicit implementation rule so descriptive behavior and runtime behavior do not look contradictory.
- [ ] Should M2 descriptor builders be implemented via explicit adapter/native metadata traits, or through narrow typed helper functions local to the capability module to avoid widening the public `Tool` contract too early?
