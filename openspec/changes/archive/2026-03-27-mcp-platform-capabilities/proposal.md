# Proposal: MCP Platform Capabilities Beyond Tools

## Intent

Corvus's MCP integration is limited to tools only. The runtime actively ignores resources and
prompts advertised by MCP servers (v1 spec, line 222-227). Issue #258 asks us to define the
desired support level for the full MCP capability surface — resources and prompts — so that
follow-up implementation work can proceed without ambiguity.

This is a **planning-scope change**. The deliverable is a capability model definition, config
schema extension design, and security tier classification — not runtime code.

## Scope

### In Scope

- **Three-tier capability model** defining Tools (existing), Resources (read-only context), and
  Prompts (template injection) with explicit support levels and semantics.
- **Extended naming convention**: `mcp.<server>.resource.<name>` and `mcp.<server>.prompt.<name>`
  alongside existing `mcp.<server>.<tool>`.
- **Per-server capability flags** in `McpServerConfig` — additive config schema extensions that
  preserve backward compatibility.
- **Security tier classification** per capability type with policy and approval expectations.
- **Phased rollout strategy** — resources first (lower risk), then prompts (higher risk).
- **Backward compatibility guarantee** — existing tool-only configs and discovery are unaffected.
- **Rollback plan** for each phase.

### Out of Scope

- **Runtime implementation** — no Rust code changes in this change.
- **MCP resource subscriptions** (server-push change notifications) — deferred. Subscriptions
  require persistent connection management and an event loop extension that is architecturally
  significant. On-demand reads are sufficient for the initial resource model.
- **Hot-reload of MCP capabilities** — deferred per v1 spec exclusion.
- **SSE/HTTP transport for resources/prompts** — stdio only for initial implementation. Transport
  expansion is orthogonal and can be added later without model changes.
- **Prompt-as-Workflow composition** — prompts as first-class workflow triggers (composable with
  mission/scheduler) is deferred. Initial prompt support is limited to template retrieval.

## Open Question Resolutions

### Q1: Resource subscriptions → Deferred

Subscriptions add persistent connection management, event loop extension, and resource exhaustion
risk. On-demand `resource.read()` covers the primary use case (context injection) with no new
infrastructure. Subscriptions can be added as a follow-up once the base resource model is proven.

### Q2: Prompt trust model → Operator-approved-only (initially)

MCP prompts inject arbitrary content into LLM conversations — a direct prompt injection vector.
The initial trust model treats all MCP prompts as **operator-scoped**: they require explicit
operator approval in config and are never user-triggerable. This matches the most restrictive
tier and can be relaxed later with a content-scanning layer.

### Q3: Resource caching → Application-layer, opt-in

Resource caching is the responsibility of the calling layer (agent loop or prompt builder), not
the MCP client. The capability model defines resources as stateless reads. A future `cache_ttl`
config field per resource can be added without model changes.

### Q4: Capability discovery granularity → Per-server flags

Per-server `capabilities` list (e.g., `["tools", "resources"]`) gives operators fine-grained
control. A global `mcp.enable_resources = true` shorthand MAY be added as sugar but the
per-server flag is authoritative. This allows mixed fleets where some servers expose resources
and others don't.

### Q5: Transport scope → Stdio-only initially

Resources and prompts follow the same transport as tools for a given server. Since v1 is
stdio-only, resources/prompts start stdio-only. When HTTP/SSE transport is added for tools, it
automatically extends to resources/prompts — no model change needed.

## Approach

### Three-Tier Capability Model

| Tier | Capability           | Semantics                                | Risk   | Default Policy                         |
|------|----------------------|------------------------------------------|--------|----------------------------------------|
| 1    | **Tools** (existing) | Action execution, may mutate state       | Medium | Approval required                      |
| 2    | **Resources**        | Read-only context provision, stateless   | Low    | Allow with limits                      |
| 3    | **Prompts**          | Template injection into LLM conversation | High   | Approval required + content provenance |

### Extended Naming Convention

```
mcp.<server>.<tool_name>              # Existing (unchanged)
mcp.<server>.resource.<resource_name> # New — resources
mcp.<server>.prompt.<prompt_name>     # New — prompts
```

The capability-type segment (`resource.`, `prompt.`) disambiguates across types. A tool named
`foo` and a resource named `foo` on the same server resolve to distinct identifiers. Collision
detection spans all capability types within a server — no two capabilities may share a fully
qualified name.

### Config Schema Extension (Additive)

```toml
# Existing config — unchanged, continues to work
[mcp.servers.my-server]
name = "my-server"
command = "my-server-bin"
args = ["--stdio"]
timeout_seconds = 30
output_limit_bytes = 65536

# New optional fields
capabilities = ["tools", "resources"]  # Default: ["tools"] for backward compat

# Resource-specific limits (optional, inherit server defaults)
[mcp.servers.my-server.resource_limits]
output_limit_bytes = 131072  # Override for resources if needed
```

Key design decisions:

- `capabilities` defaults to `["tools"]` when absent — **zero breaking changes**.
- `"prompts"` is a valid capability value but requires explicit opt-in.
- Resource and prompt limits inherit from server-level defaults but can be overridden.

### Security Tiers

| Capability | Policy Default     | Approval Flow                                                              | Content Handling                                                     |
|------------|--------------------|----------------------------------------------------------------------------|----------------------------------------------------------------------|
| Tools      | `ApprovalRequired` | Per-invocation or blanket allow                                            | Output bounded by `output_limit_bytes`                               |
| Resources  | `AllowWithLimits`  | No approval needed; bounded by output limits and timeouts                  | Read-only; URI validated against server scope; output bounded        |
| Prompts    | `ApprovalRequired` | Operator must explicitly enable per-server; content marked with provenance | Injected content tagged with source; content scanning hook available |

Prompt-specific security constraints:

- Prompt content MUST be marked with provenance metadata (source server, fetch timestamp).
- Prompt content MUST NOT override system-level safety instructions.
- A content scanning hook SHOULD be available for operators to attach validation logic.
- Prompts are never user-triggerable in the initial model — operator config only.

### Phased Rollout

**Phase 1: Resources** (lower risk, higher value)

- Extend discovery to call `list_resources()` alongside `list_tools()`.
- Register resources as read-only capabilities with `AllowWithLimits` policy.
- Expose via `mcp.<server>.resource.<name>` naming.
- Resources available as tool-like callables (LLM can request a resource read).

**Phase 2: Prompts** (higher risk, requires security review)

- Extend discovery to call `list_prompts()`.
- Register prompts with `ApprovalRequired` + provenance tagging.
- Expose via `mcp.<server>.prompt.<name>` naming.
- Prompts available as template retrieval (returns structured message content).

Phases are independently deployable. Phase 2 does not require Phase 1 but benefits from the
infrastructure patterns established in Phase 1.

## Affected Areas

| Area                                               | Impact             | Description                                                                             |
|----------------------------------------------------|--------------------|-----------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/config/schema.rs`       | Modified           | Add `capabilities` field and resource/prompt limit overrides to `McpServerConfig`       |
| `clients/agent-runtime/src/tools/mcp/client.rs`    | Modified           | Parse resource/prompt manifests from server introspection (currently warns and ignores) |
| `clients/agent-runtime/src/tools/mcp/mod.rs`       | Modified           | Extend discovery to register resources and prompts alongside tools                      |
| `clients/agent-runtime/src/tools/mcp/adapter.rs`   | New modules        | `ResourceAdapter` and `PromptAdapter` (or new sibling files)                            |
| `clients/agent-runtime/src/tools/mcp/normalize.rs` | Modified           | Extended naming validation for `resource.` and `prompt.` segments                       |
| `clients/agent-runtime/src/security/policy.rs`     | Modified           | Per-capability-type policy defaults (AllowWithLimits for resources)                     |
| `clients/agent-runtime/src/agent/dispatcher.rs`    | Modified           | Risk classification for new capability types                                            |
| `clients/agent-runtime/src/agent/prompt.rs`        | Modified (Phase 2) | Context injection for prompt template content                                           |
| `openspec/specs/mcp-runtime/spec.md`               | Modified           | Remove v1 exclusion clause; add resource/prompt requirements and scenarios              |

## Risks

| Risk                                                          | Likelihood                | Mitigation                                                                                                                           |
|---------------------------------------------------------------|---------------------------|--------------------------------------------------------------------------------------------------------------------------------------|
| Prompt injection via MCP prompt templates                     | High (if prompts enabled) | Operator-only approval; provenance tagging; content scanning hook; Phase 2 deferred until security review complete                   |
| Resource data exfiltration (sensitive content exposed to LLM) | Low                       | Resources bounded by `output_limit_bytes`; URI scope limited to server declaration; operator controls which servers expose resources |
| Naming collision across capability types                      | Low                       | Fully qualified names include capability-type segment; collision detection spans all types per server                                |
| Config migration confusion                                    | Low                       | `capabilities` defaults to `["tools"]`; existing configs unchanged; migration guide in release notes                                 |
| Scope creep into subscriptions or hot-reload                  | Medium                    | Explicitly out-of-scope in this proposal; implementation PRs gated by spec scenarios                                                 |

## Rollback Plan

**This change is planning-only** — rollback means discarding the proposal/spec/design artifacts.
No runtime behavior changes.

For future implementation phases:

- **Phase 1 (Resources) rollback**: Remove `capabilities` field handling; restore `client.rs`
  warn-and-ignore behavior for resources; discovery reverts to tools-only. Config files with
  `capabilities = ["tools", "resources"]` would emit a deprecation warning but not fail (unknown
  fields are ignored by the existing parser).

- **Phase 2 (Prompts) rollback**: Remove prompt discovery and registration; `capabilities`
  containing `"prompts"` emits a warning. Phase 1 (resources) remains unaffected.

Both rollbacks are independent and do not affect existing tool functionality.

## Dependencies

- No external dependencies for this planning change.
- Implementation Phase 1 depends on this proposal being accepted and spec/design being written.
- Implementation Phase 2 depends on Phase 1 patterns being validated and a dedicated security
  review of the prompt injection surface.

## Success Criteria

- [ ] Three-tier capability model (Tools/Resources/Prompts) is clearly defined with explicit
  semantics, risk levels, and policy defaults.
- [ ] Extended naming convention is specified and collision handling is defined.
- [ ] Per-server `capabilities` config schema extension is designed with backward compatibility
  guaranteed.
- [ ] Security tier per capability type is documented with policy expectations.
- [ ] Phased rollout (Resources → Prompts) is justified and scoped.
- [ ] All five open questions from exploration are resolved with explicit decisions.
- [ ] Follow-up spec and design work can proceed without major ambiguity (issue #258 acceptance
  criteria).
