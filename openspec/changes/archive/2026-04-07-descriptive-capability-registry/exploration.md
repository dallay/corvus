## Exploration: descriptive-capability-registry

### Current State

Native tool registration is still fully imperative. `clients/agent-runtime/src/tools/mod.rs` builds
a `Vec<Box<dyn Tool>>` in `default_tools_with_runtime()` and `all_tools_with_runtime()`, adding
native tools in fixed order and extending the vector with optional
browser/http/web/delegate/composio tools before appending MCP tools via `extend_with_mcp_tools()`.

MCP discovery is already descriptor-adjacent but still runtime-oriented.
`clients/agent-runtime/src/tools/mcp/mod.rs` discovers tools/resources/prompts per server,
normalizes their canonical names through
`normalize::{normalize_tool_name, normalize_resource_name, normalize_prompt_name}`, and creates
adapters that implement `Tool`. Within MCP discovery, duplicate canonical names fail registration;
at the native/MCP boundary, `extend_with_mcp_tools()` detects a collision with any existing native
name and skips the entire MCP extension with a warning.

There is no shared capability registry today. The closest reusable seam is `Tool::spec()` /
`ToolSpec` in `clients/agent-runtime/src/tools/traits.rs`: agent and channel paths independently
derive `tool_specs` from the tool vector, providers serialize those specs, and execution still
resolves tools by `tool.name()` against the original `Vec<Box<dyn Tool>>`. Security and profile
behavior also depend on current string names: MCP approval is inferred from the `mcp.` prefix in
`security/policy.rs`, and profile gating classifies MCP tools the same way in `bootstrap/mod.rs`.

### Affected Areas

- `clients/agent-runtime/src/tools/mod.rs` — current native tool construction seam; MCP/native
  collision handling happens here.
- `clients/agent-runtime/src/tools/traits.rs` — `ToolSpec`/`ToolSourceMetadata` is the nearest
  existing descriptor shape, but it is far smaller than the capability contract.
- `clients/agent-runtime/src/tools/mcp/mod.rs` — current MCP discovery/normalization/merge path and
  duplicate detection.
- `clients/agent-runtime/src/tools/mcp/normalize.rs` — canonical namespaced identity rules already
  exist here and should be reused, not reinvented.
- `clients/agent-runtime/src/tools/mcp/adapter.rs` — MCP tool adapter exposes canonical name plus
  source metadata.
- `clients/agent-runtime/src/tools/mcp/resource_adapter.rs` — MCP resource adapter exposes canonical
  name, but current `spec()` stores the URI in `source.original_name`, so it is not a trustworthy
  descriptor source-of-truth by itself.
- `clients/agent-runtime/src/tools/mcp/prompt_adapter.rs` — MCP prompt adapter exposes prompt
  arguments and source metadata.
- `clients/agent-runtime/src/bootstrap/mod.rs` — final runtime-visible tool set is produced here
  after `profile.allows_tool(tool.name())` filtering; safest final insertion point if the registry
  should describe active capabilities only.
- `clients/agent-runtime/src/agent/agent.rs` — execution still finds tools directly from
  `self.tools`; this must remain unchanged in M2.
- `clients/agent-runtime/src/agent/dispatcher.rs` and
  `clients/agent-runtime/src/security/policy.rs` — approval behavior depends on existing tool
  names/prefixes, so descriptor work must not change names or routing semantics.
- `clients/agent-runtime/src/channels/mod.rs` — channel loop independently regenerates `ToolSpec`
  from the tool vector; registry work must not break parity.

### Approaches

1. **Registry finalized in bootstrap, descriptor builders near tools** — keep native/MCP
   construction unchanged, derive descriptors from the finished tool vector after profile filtering,
   and store a non-executing `CapabilityRegistry` alongside legacy wiring.
    - Pros: Safest behavior-wise; registry describes the actual runtime-visible surfaces; avoids
      changing agent dispatch, provider conversion, or MCP merge behavior; easy rollback.
    - Cons: Descriptor construction logic is split between bootstrap orchestration and tool/MCP
      helpers; native duplicate validation happens after tool construction, not during it.
    - Effort: Medium.

2. **Registry assembled inside `tools/mod.rs` during tool construction** — build descriptors while
   native tools and MCP adapters are added, then return both tools and registry from the tool
   assembly path.
    - Pros: Keeps registration logic close to the construction seam identified in M1; single place
      for native+MCP descriptive registration.
    - Cons: Signature changes ripple into bootstrap/tests; registry may include tools later filtered
      out by profile rules unless bootstrap does another pass; higher risk of accidental behavior
      drift.
    - Effort: Medium/High.

### Recommendation

Use **Approach 1**.

Safest M2 insertion point: introduce a new non-executing `CapabilityRegistry` module under
`clients/agent-runtime/src/`, but finalize registration in `BootstrapContext::from_config` /
`for_gateway` **after** `all_tools_with_runtime()` and after profile filtering. That keeps the
existing `Vec<Box<dyn Tool>>` as the execution source of truth while allowing descriptor
registration for the exact active native and MCP surfaces.

Minimal M2 descriptor/registry responsibilities:

- Reuse current canonical tool name as descriptor `id` to preserve approval, audit, and dispatch
  continuity.
- Store the shared minimum contract fields required by the spec: `id`, `namespace`, `version`,
  `family`, `kind`, `dependencies`, `lifecycle`, `security`, and `compatibility`.
- For M2, keep `family=tool` and `kind=executable` for native tools plus MCP
  tools/resources/prompts, because they are all exposed through the `Tool` trait today.
- Use deterministic defaults rather than speculative inference:
    - `dependencies`: empty required/optional lists in M2.
    - `version`: fixed descriptor contract version (for example `1.0.0`) until real versioning
      exists.
    - `lifecycle`: derived from current construction (`static` for native, `discovered` for MCP;
      activation remains legacy runtime-wired).
    - `security`: preserve current naming/policy continuity (`policy_scope=tool`, audit namespace =
      id, source classification from native/mcp/mcp_resource/mcp_prompt).
    - `compatibility`: encode current runtime assumptions (`tool-trait-v1`, parity scope
      `agent/channels/gateway`) without adding resolver logic.
- Carry family-specific metadata separately, not in the shared contract. For M2 that likely means
  `ToolSpec`-adjacent data: description, parameters schema, source metadata, and MCP-only fields
  such as server name, upstream name, resource URI, mime type, or prompt arguments.
- Validate descriptor shape and namespaced uniqueness deterministically; do not perform dependency
  resolution or execution binding.

What stays legacy for now:

- `Vec<Box<dyn Tool>>` creation, ordering, and runtime execution lookup.
- `Tool::spec()` generation and provider/channel consumption paths.
- `extend_with_mcp_tools()` runtime extension behavior and MCP failure-isolation semantics.
- `source_kind_for_tool()` / dispatcher approval behavior based on current tool names.
- Profile allowlists and MCP classification via `mcp.` prefix.
- MCP client execution behavior, including the fact that live resource/prompt read/get flows are
  still unimplemented and currently return empty/error paths for non-mock servers.

Testing approach for M2:

- Add pure unit tests for descriptor builders: native tool descriptor shape, MCP
  tool/resource/prompt descriptor shape, and required-field validation.
- Add deterministic registry tests: same input order => same registry order/content; duplicate ids
  are rejected consistently; cross-namespace same local names are allowed.
- Add collision tests that model current baselines explicitly:
    - native vs MCP collision
    - MCP tool vs MCP resource same local name (must coexist because canonical ids differ)
    - duplicate within one namespace/server (must fail)
- Add bootstrap-level tests proving registry creation does not change the final tool names or
  profile-filtered tool set.
- Keep execution tests separate from M2; only parity/smoke assertions should confirm behavior did
  not change.

### Risks

- **Name drift breaks security semantics**: approval and profile gating depend on existing string
  names/prefixes, so descriptor ids MUST stay aligned with `tool.name()` in M2.
- **Registry becomes accidental execution authority**: if code starts reading the registry instead
  of the tool vector for dispatch, M2 scope is violated.
- **Blind reuse of `ToolSourceMetadata` is insufficient**: native tools usually have no `source`,
  and MCP resources currently put URI into `original_name`, so descriptor constructors need explicit
  mapping logic.
- **Collision policy inconsistency**: current native/MCP boundary skips all MCP tools on one
  collision, while in-MCP duplicates fail discovery; proposal should choose a clear M2
  validation/reporting rule without silently changing runtime execution.
- **Profile mismatch**: registering before profile filtering would describe capabilities the runtime
  does not actually expose.
- **Overreach into M3/M4**: dependency inference, compatibility solving, registry-based dispatch, or
  approval rewiring would exceed M2.

### Ready for Proposal

Yes — with a narrow proposal.

Recommended proposal scope:

- add a new non-executing `CapabilityDescriptor` + `CapabilityRegistry` for tool-family capabilities
  only,
- cover native tools and MCP-discovered tools/resources/prompts,
- register after final tool selection in bootstrap,
- validate shared descriptor shape and namespaced uniqueness,
- preserve all existing execution, dispatcher, profile, and provider/channel behavior unchanged.

Explicit non-goals for the proposal:

- dependency resolution,
- registry-driven dispatch/execution,
- provider/channel/memory/observer family rollout,
- renaming tool ids,
- changing MCP transport/runtime behavior.