## Exploration: capability-based-architecture-v3

### Current State

The Rust runtime is already trait-driven at the edges, but composition is still runtime-owned rather
than capability-owned.

- **Trait boundaries exist**:
    - `clients/agent-runtime/src/providers/traits.rs` — `Provider` plus `ProviderCapabilities` for
      native tools and image input.
    - `clients/agent-runtime/src/tools/traits.rs` — `Tool` plus self-describing `ToolSpec`.
    - `clients/agent-runtime/src/channels/traits.rs` — `Channel`.
    - `clients/agent-runtime/src/memory/traits.rs` — `Memory`.
    - `clients/agent-runtime/src/observability/traits.rs` — `Observer`.
    - `clients/agent-runtime/src/runtime/traits.rs` — `RuntimeAdapter`.
- **Wiring is centralized and concrete**:
    - `clients/agent-runtime/src/bootstrap/mod.rs` constructs runtime, security, memory, observer,
      and tools, then filters tools by profile.
    - `clients/agent-runtime/src/providers/mod.rs`, `memory/mod.rs`, `observability/mod.rs`, and
      `runtime/mod.rs` use factory `match` statements over config strings.
    - `clients/agent-runtime/src/tools/mod.rs` builds one hard-coded `Vec<Box<dyn Tool>>`,
      optionally extending with MCP-discovered capabilities.
    - `clients/agent-runtime/src/channels/mod.rs` uses a static `CHANNEL_REGISTRY` and a large
      runtime context struct.
- **Execution flow today**:
    - `Agent::from_bootstrap_with_provider()` in `clients/agent-runtime/src/agent/agent.rs`
      assembles one provider, one memory backend, one observer, one tool list, and one dispatcher.
    - `Agent::step_with_context()` sends `history + tool_specs` to the provider, parses tool calls,
      risk-gates them, executes tools from the in-memory list, then feeds formatted results back
      into the same loop.
    - Channels run a parallel but separate loop in `clients/agent-runtime/src/channels/mod.rs` (
      `run_unified_channel_tool_loop()`), with its own history handling and tool execution path.
    - Gateway canonical webhook flow in `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
      reuses bootstrap + agent execution, while fallback paths in `gateway/mod.rs` still call
      `provider.simple_chat()` directly.
- **What already looks capability-like**:
    - `ToolSpec` is self-describing.
    - `ProviderCapabilities` is an explicit feature declaration.
    - MCP discovery in `clients/agent-runtime/src/tools/mcp/mod.rs` already performs capability
      discovery, normalization, namespacing, collision handling, and partial-failure isolation.
    - `CHANNEL_REGISTRY` in `channels/mod.rs` is a small registry pattern already in use.
- **What is still tightly coupled**:
    - The runtime assumes most extensibility collapses back into a single tool list.
    - Agent logic knows too much about tool dispatch, provider messaging, approvals, memory loading,
      budgets, and mission behavior in one place.
    - Non-tool subsystems are not described through a shared contract model, so there is no unified
      capability graph or dependency model.

### Affected Areas

- `clients/agent-runtime/src/bootstrap/mod.rs` — central composition root; any capability model will
  pass through here first.
- `clients/agent-runtime/src/agent/agent.rs` — current orchestration loop couples provider calls,
  tool execution, approvals, cost, memory, and mission logic.
- `clients/agent-runtime/src/agent/dispatcher.rs` — current dispatch contract is tool-centric and
  approval logic is keyed off tool identity strings.
- `clients/agent-runtime/src/tools/mod.rs` — hard-coded tool assembly; closest place where a
  registry abstraction could emerge.
- `clients/agent-runtime/src/tools/mcp/mod.rs` — strongest existing example of capability
  discovery/normalization/registration.
- `clients/agent-runtime/src/providers/traits.rs` — contains the current feature declaration pattern
  via `ProviderCapabilities`.
- `clients/agent-runtime/src/providers/mod.rs` — large string-matched provider factory; strong
  evidence of runtime-owned registration.
- `clients/agent-runtime/src/providers/router.rs` — shows incremental composition already exists for
  routing, capability merging, and fail-closed image gating.
- `clients/agent-runtime/src/channels/mod.rs` — separate runtime loop with its own registry and
  orchestration path; major migration hotspot.
- `clients/agent-runtime/src/memory/mod.rs` — factory-driven backend selection, not declarative
  capability registration.
- `clients/agent-runtime/src/observability/mod.rs` and `src/runtime/mod.rs` — additional
  factory-owned extension points.
- `clients/agent-runtime/src/security/policy.rs` — policy decisions depend on tool name namespaces
  today, so capability contracts must preserve security classification.
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs` and `src/gateway/mod.rs` — must preserve
  canonical path vs legacy fallback behavior during migration.

### Approaches

1. **Design-first capability contract change** — define a shared capability model, descriptors,
   lifecycle, dependency semantics, and migration rules in spec/design artifacts only.
    - Pros: Lowest risk; aligns with M1; lets security/testing/migration rules be agreed before
      code; avoids fake plugin architecture.
    - Cons: No immediate runtime prototype; some stakeholders may feel progress is slower.
    - Effort: Low

2. **Internal registry prototype for discovery only** — add an internal `CapabilityDescriptor`/
   `CapabilityRegistry` seam behind bootstrap, initially wrapping existing native tools + MCP
   capabilities without changing the agent loop contract.
    - Pros: Incremental; leverages existing MCP discovery patterns; gives real code feedback before
      broad refactor.
    - Cons: Still risky if it touches agent execution too early; can become a second registry
      layered on top of old factories if not tightly scoped.
    - Effort: Medium

3. **Full capability-platform refactor** — replace factories, loop orchestration, and
   extension-point wiring with a unified capability graph across providers, channels, tools, memory,
   observability, and security.
    - Pros: Closer to the long-term v3 vision.
    - Cons: Too disruptive for first pass; high regression risk across gateway, channels, missions,
      approvals, and tests; likely creates migration freeze.
    - Effort: High

### Recommendation

Start with **Approach 1**.

The smallest safe first change is **M1 as a pure design/spec change**. The codebase is not missing
abstractions in the abstract; it is missing a **shared contract model** that can unify existing
trait seams without breaking current behavior. The current runtime already has useful seeds (
`ToolSpec`, `ProviderCapabilities`, MCP discovery, channel registry), but execution remains deeply
tool-centric and bootstrap-owned. Jumping straight to registry or pipeline implementation would
force architecture decisions before the team has settled:

- what a capability is vs a provider/channel/tool implementation,
- which capabilities are executable vs descriptive,
- how dependencies are declared and resolved,
- how policy/approval attaches to capabilities,
- and how legacy factories coexist during migration.

**Recommended first implementation scope after design**: a later, narrowly-scoped change that
introduces a registry only for **describing and registering capabilities**, not yet for replacing
the agent loop. In other words: registry before resolver, resolver before pipeline, pipeline before
full runtime inversion.

Concrete guidance for first-pass scope:

- **Do now**: capability taxonomy, descriptor schema, lifecycle, dependency rules, namespace rules,
  security model, migration invariants, and proof points.
- **Do not do yet**: dynamic plugin loading, generalized hot-pluggable runtime modules, replacing
  all factories at once, or forcing channels/memory/security into a new runtime graph in one shot.
- **Do next**: prototype registry around the already capability-like surfaces (native tools + MCP
  tools/resources/prompts), because that area already has namespacing and discovery semantics.

### Risks

- **Security regression risk** — policy is currently keyed off tool identity and MCP namespaces in
  `security/policy.rs` and `agent/dispatcher.rs`; a new capability model must not dilute
  deny/approval defaults.
- **Parallel-runtime risk** — channels and gateway have partially separate orchestration paths; a
  rushed refactor could create inconsistent capability behavior across CLI, channels, and webhook.
- **Fake plugin architecture risk** — introducing a registry that is only another hard-coded list in
  front of existing hard-coded factories adds indirection without real decoupling.
- **Migration blast-radius risk** — `Agent` currently owns many concerns in one loop; touching
  execution, approval, missions, budget, and memory in the same first change is too disruptive.
- **Testing risk** — there is good local test coverage around factories and adapters, but no
  existing unified dependency-graph test harness; dependency resolution should not ship without
  deterministic test fixtures.
- **Performance/startup risk** — capability discovery can increase startup work; MCP already shows
  the need for bounded discovery, collision handling, and partial-failure isolation.

### Ready for Proposal

**Yes** — Recommend a proposal for a **design-first M1 change** that produces the v3 capability
contract and migration plan, followed by separate implementation changes.

Suggested roadmap mapping for DALLAY-250:

- **Change 1 (M1)**: design/spec only — capability taxonomy, descriptor schema, dependency
  semantics, security attachment points, migration boundaries.
- **Change 2 (M2)**: internal `CapabilityRegistry` for descriptive registration of native tools +
  MCP capabilities, preserving current factories and execution.
- **Change 3 (M3)**: dependency resolution for registry entries, with deterministic validation and
  test fixtures; no broad execution refactor yet.
- **Change 4 (M4)**: execution pipeline seam for capability invocation, starting with tool-like
  capabilities only; keep channels/memory/security adapters behind compatibility boundaries.
- **Change 5 (M5)**: integration tests, docs, migration guides, and selective adoption by additional
  capability families once parity is proven.

Anti-patterns to avoid in the proposal:

- "Everything is a capability" without distinct lifecycle and security semantics.
- Dynamic plugin loading before contract stability.
- Replacing trait boundaries that already work instead of adapting them.
- Coupling dependency resolution to runtime side effects.
- Claiming composability while still requiring central hard-coded registration everywhere.
