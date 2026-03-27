# Exploration: MCP Platform Capabilities Beyond Tools

**Change**: `mcp-platform-capabilities`
**Issue**: #258 — Expand MCP platform support beyond tools-only integration
**Date**: 2026-03-27
**Scope**: Planning only — define capability model, no code changes

---

## Current State

The Corvus agent runtime has a complete MCP v1 integration limited to **tools only**. The current implementation:

- **Config** (`McpConfig` / `McpServerConfig` in `config/schema.rs`): Defines MCP servers with `name`, `command`, `args`, `env`, timeouts, and output limits. No fields exist for resources or prompts.
- **Discovery** (`tools/mcp/mod.rs::discover_tools`): Iterates enabled servers, introspects via `McpClient::list_tools()`, and registers `McpToolAdapter` instances into the unified tool registry.
- **Client** (`tools/mcp/client.rs`): Parses server manifests. When `resources` or `prompts` keys appear in the payload, it logs a warning (`"MCP payload advertised unsupported non-tool capabilities; ignoring for v1"`) and skips them. If only resources/prompts exist with no tools, returns empty vec.
- **Adapter** (`tools/mcp/adapter.rs`): Wraps MCP tools as `impl Tool` with namespaced identity (`mcp.<server>.<tool>`), output limiting, and input validation.
- **Normalization** (`tools/mcp/normalize.rs`): Enforces `mcp.<server>.<tool>` canonical naming, reserved identifier protection, and character validation.
- **Policy** (`security/policy.rs`): All `mcp.*` tools require approval by default (`ToolPolicyDecision::ApprovalRequired`). The dispatcher (`agent/dispatcher.rs`) enforces this via `source_kind_for_tool()`.
- **Spec** (`openspec/specs/mcp-runtime/spec.md`): Line 9-11 explicitly states: *"This change is limited to config-defined MCP tools (stdio transport). It explicitly excludes MCP resources/prompts and hot reload behavior."* Scenario at line 224-227 mandates that non-tool capabilities are rejected in v1.

The agent loop (`agent/unified_loop.rs`) consumes tools via `ToolSpec` registrations. Context injection (system prompt, memory recall) flows through `agent/prompt.rs` sections. There is no concept of "resource" or "prompt template" in the current architecture.

---

## Investigation Areas

### 1. MCP Protocol Capabilities: Resources and Prompts

**Resources** in MCP are read-only data endpoints that servers expose. They follow a URI scheme (e.g., `docs://index`, `file:///path`) and return structured content. Key semantics:
- Stateless read-only access (no side effects)
- URI-based addressing with optional MIME types
- Subscription model for change notifications (optional)
- Used to inject context into LLM conversations (RAG-like)

**Prompts** in MCP are reusable prompt templates that servers expose. Key semantics:
- Named templates with typed arguments
- Return structured message arrays (system/user/assistant)
- Intended for workflow composition (e.g., "summarize", "code-review")
- Can reference resources as embedded context

**Key distinction from tools**: Resources and prompts are *context-providing* capabilities, not *action-executing* ones. Tools mutate state; resources/prompts supply information.

### 2. Architecture Fit Analysis

#### Resources → Corvus Mapping

Resources map most naturally to the **context injection** layer, not the tool layer:

| MCP Concept | Corvus Analog | Integration Point |
|---|---|---|
| Resource read | Memory recall / RAG retrieval | `agent/prompt.rs` context sections |
| Resource list | Tool registry discovery | Startup discovery (parallel to tools) |
| Resource subscribe | No current analog | Would require event loop extension |

**Approach options**:

- **A) Resource-as-Tool**: Wrap each resource as a read-only `Tool` (e.g., `mcp.<server>.resource.<name>`). LLM calls it like any tool. Simplest but adds noise to tool list and wastes a tool-call round-trip for static data.
- **B) Resource-as-Context**: New `ResourceProvider` trait parallel to `Tool`. Resources are fetched at prompt assembly time or on-demand and injected into conversation context. More architecturally clean but requires a new subsystem.
- **C) Hybrid**: Resources are discoverable as tools (for LLM-initiated reads) AND available as context sources (for prompt-time injection). Most flexible but highest complexity.

#### Prompts → Corvus Mapping

Prompts map to **prompt composition** and **workflow templating**:

| MCP Concept | Corvus Analog | Integration Point |
|---|---|---|
| Prompt template | System prompt sections | `agent/prompt.rs` `PromptSection` trait |
| Prompt arguments | Config/runtime parameters | Agent config or turn metadata |
| Prompt messages | Conversation history seeding | `ConversationMessage` history |

**Approach options**:

- **A) Prompt-as-Tool**: Expose a meta-tool that fetches and applies a prompt template. LLM calls `mcp.<server>.prompt.<name>` to get template content.
- **B) Prompt-as-Section**: New `PromptSection` implementation that pulls templates from MCP servers at prompt build time. Transparent to the LLM.
- **C) Prompt-as-Workflow**: Prompts become first-class workflow triggers, composable with the mission/scheduler system. Most powerful but highest scope.

### 3. Security Surface Analysis

| Capability | Risk Level | New Attack Surface | Mitigation Strategy |
|---|---|---|---|
| **Resources (read)** | Low-Medium | Data exfiltration if resources expose sensitive content; URI injection if user-controlled | URI allowlist per server; output size limits (reuse existing `output_limit_bytes`); redaction of sensitive content |
| **Resources (subscribe)** | Medium | Persistent connections; server-pushed content injection; resource exhaustion | Subscription limits; timeout budgets; content validation |
| **Prompts** | Medium-High | **Prompt injection** — MCP server can inject arbitrary system/user messages into the conversation; template argument injection | Prompt content sandboxing; operator-only prompt sources (no user-triggered); content scanning; clear provenance marking |

**Critical finding**: Prompts are the highest-risk capability because they directly inject content into the LLM conversation, potentially overriding safety instructions or manipulating agent behavior. This is a fundamentally different trust model than tools (which execute and return results).

### 4. Existing Patterns to Carry Forward

From the v1 MCP spec and implementation, these patterns MUST be preserved:

1. **Namespacing**: `mcp.<server>.<capability_type>.<name>` — extend the existing `mcp.<server>.<tool>` pattern with a capability type segment for disambiguation.
2. **Deny-by-default policy**: All MCP capabilities require explicit approval or policy allowance before use.
3. **Startup-time discovery**: Resources and prompts should be discovered alongside tools during server introspection.
4. **Bounded execution**: Timeouts and output limits apply to resource reads and prompt fetches.
5. **Failure isolation**: One server's resource/prompt failure must not crash runtime or affect other servers.
6. **Secret redaction**: All diagnostics must redact sensitive values.
7. **Collision handling**: Deterministic rejection of duplicate identifiers across all capability types.
8. **Config validation**: Fail-fast on malformed capability definitions.

### 5. Backward Compatibility

**Impact on existing MCP tool integrations: NONE if designed correctly.**

- The `McpConfig` / `McpServerConfig` schema can be extended additively (new optional fields).
- `discover_tools()` continues to work unchanged for tool-only servers.
- The `parse_tool_manifest_payload()` function already handles payloads with `resources`/`prompts` keys gracefully (warns and ignores).
- Policy enforcement for `mcp.*` tools is unaffected.
- The warning log at `client.rs:448-452` would be removed or downgraded once resources/prompts are supported.

**Migration path**: Servers that advertise resources/prompts today are silently ignored. Enabling support means those previously-ignored capabilities become active — operators need an opt-in mechanism (per-server capability flags or a global feature gate).

---

## Affected Areas

- `clients/agent-runtime/src/tools/mcp/client.rs` — Manifest parsing needs to extract resources/prompts
- `clients/agent-runtime/src/tools/mcp/mod.rs` — Discovery needs to register non-tool capabilities
- `clients/agent-runtime/src/tools/mcp/adapter.rs` — New adapter types for resources/prompts (or new modules)
- `clients/agent-runtime/src/tools/mcp/normalize.rs` — Extended naming scheme for new capability types
- `clients/agent-runtime/src/config/schema.rs` — `McpServerConfig` needs capability flags; possibly new config sections
- `clients/agent-runtime/src/agent/prompt.rs` — Context injection point for resources/prompts
- `clients/agent-runtime/src/agent/dispatcher.rs` — Risk classification for new capability types
- `clients/agent-runtime/src/security/policy.rs` — Policy decisions for resources vs prompts vs tools
- `openspec/specs/mcp-runtime/spec.md` — Spec needs to be extended or a new spec created

---

## Approaches

### 1. Incremental: Resources First, Prompts Later

Add MCP resource support as a read-only context provider. Defer prompts to a follow-up change.

- **Pros**: Smallest risk surface; resources are read-only; clear architectural fit with existing context injection; validates the extension pattern before tackling prompts
- **Cons**: Two changes instead of one; prompts remain undefined
- **Effort**: Medium (resources) + Medium (prompts later)

### 2. Full Platform: Resources + Prompts Together

Define and implement both capabilities in a single change.

- **Pros**: Complete MCP platform coverage; single design/spec cycle; unified naming and policy model
- **Cons**: Larger blast radius; prompt injection risk needs careful design; more complex review
- **Effort**: High

### 3. Capability Model Definition Only (Planning Scope)

Define the capability model, naming conventions, config schema, and security boundaries. No runtime implementation — output is a proposal + spec that downstream implementation changes can follow.

- **Pros**: Matches issue #258 scope ("desired MCP support level is clearly defined"); de-risks implementation; allows stakeholder review of security model before code
- **Cons**: No running code; requires follow-up implementation changes
- **Effort**: Low (this change) + Medium-High (implementation changes)

---

## Recommendation

**Approach 3 (Capability Model Definition Only)** for this change, with the model designed to support **Approach 1 (Resources First)** as the first implementation change.

Rationale:
1. Issue #258 acceptance criteria are about clarity of definition, not implementation.
2. The prompt injection risk from MCP prompts warrants careful security design review before any code.
3. Resources are lower-risk and more architecturally straightforward — they should ship first.
4. A well-defined capability model enables parallel work on spec, design, and implementation.

The proposal should define:
- Three-tier capability model: Tools (existing), Resources (read-only context), Prompts (template injection)
- Extended naming: `mcp.<server>.resource.<name>` and `mcp.<server>.prompt.<name>`
- Per-server capability flags: `capabilities = ["tools", "resources"]` (prompts opt-in later)
- Security tiers: Tools = approval-required (existing), Resources = allow-with-limits, Prompts = approval-required + content scanning
- Config schema extensions (additive, backward-compatible)

---

## Risks

- **Prompt injection via MCP prompts**: MCP servers can inject arbitrary LLM messages. This is the highest-risk capability and needs a dedicated security design before implementation.
- **Scope creep**: Resources and prompts could each become large implementation efforts. The planning-only scope of this change mitigates this.
- **Resource subscription complexity**: If MCP resource subscriptions are supported, the runtime needs persistent connection management and event handling — a significant architectural addition.
- **Naming collision across capability types**: The extended namespace (`mcp.<server>.resource.<name>`) could collide if a tool and resource share a name on the same server. The naming scheme must prevent this.
- **Backward compatibility of config**: New fields must be optional with sane defaults so existing `config.toml` files continue to work unchanged.

---

## Open Questions

1. **Resource subscription support**: Should Corvus support MCP resource subscriptions (server-push updates), or only on-demand reads? Subscriptions add significant complexity.
2. **Prompt trust model**: Should MCP prompts be treated as operator-trusted (like system prompts) or user-trusted (like conversation messages)? This fundamentally affects the security model.
3. **Resource caching**: Should resource reads be cached? If so, what invalidation strategy?
4. **Capability discovery granularity**: Should capability support be per-server (`server.capabilities`) or global (`mcp.enable_resources`)?
5. **Transport scope**: Should resources/prompts support the same transports as tools (stdio + HTTP), or start with stdio only?

---

## Ready for Proposal

**Yes.** The investigation has sufficient depth to proceed to proposal phase. The proposal should:
1. Define the three-tier capability model with clear boundaries
2. Specify the extended naming convention and config schema
3. Establish security tiers per capability type
4. Recommend Resources-first implementation ordering
5. Address the open questions above with explicit decisions or deferred-decision markers
