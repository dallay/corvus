# Design: MCP Platform Capabilities Beyond Tools

## Technical Approach

Extend the existing MCP subsystem (`src/tools/mcp/`) to support Resources (Phase 1) and Prompts
(Phase 2) as first-class capabilities alongside Tools. The design reuses the existing `McpClient`
transport, `Tool` trait adapter pattern, and `mcp.<server>.*` naming convention. Resources and
Prompts are exposed to the agent loop as tool-like callables — the LLM invokes them via the
standard tool-call mechanism, keeping the agent loop unchanged.

This maps directly to the proposal's three-tier capability model and phased rollout strategy.

## Architecture Decisions

### Decision 1: Module Structure — Extend `src/tools/mcp/`, Don't Create Top-Level `src/mcp/`

**Choice**: Add `resource_adapter.rs` and `prompt_adapter.rs` as siblings to the existing
`adapter.rs` inside `src/tools/mcp/`. Shared MCP infrastructure (client, transport, config
validation, normalization) stays in place.

**Alternatives considered**:
- **New top-level `src/mcp/` module**: Would separate MCP from the tool subsystem, requiring new
  registry infrastructure, new dispatch paths, and duplicated wiring. High cost, low benefit for
  what are effectively "tool-shaped" capabilities.
- **Everything in `adapter.rs`**: Would bloat a single file. Separate adapter files per capability
  type keeps each focused.

**Rationale**: Resources and prompts are invoked through the same agent loop tool-call mechanism
as MCP tools. They share the same transport (`McpClient`), naming scheme (`normalize.rs`), config
(`McpServerConfig`), and policy evaluation (`source_kind_for_tool`). Keeping them in `src/tools/mcp/`
avoids duplicating infrastructure and means the dispatcher, approval flow, and output limiting all
work without changes to the agent loop.

```
src/tools/mcp/
├── mod.rs               # Extended: discover_tools → discover_capabilities
├── client.rs            # Extended: list_resources(), read_resource(), list_prompts(), get_prompt()
├── adapter.rs           # Unchanged: McpToolAdapter
├── resource_adapter.rs  # New: McpResourceAdapter (impl Tool)
├── prompt_adapter.rs    # New: McpPromptAdapter (impl Tool)
├── normalize.rs         # Extended: normalize_resource_name(), normalize_prompt_name()
└── cerebro.rs           # Unchanged
```

### Decision 2: Trait Design — Reuse `Tool` Trait, Not Dedicated Traits

**Choice**: `McpResourceAdapter` and `McpPromptAdapter` both implement the existing `Tool` trait
from `src/tools/traits.rs`. They register as `Box<dyn Tool>` in the unified tool set.

**Alternatives considered**:
- **Dedicated `Resource` and `Prompt` traits**: Would require a parallel registry, new dispatch
  paths in the agent loop (`unified_loop.rs`), new policy evaluation paths, and new output
  formatting. Architecturally cleaner in theory but enormous implementation cost for the initial
  model.
- **Generic `McpCapability` trait**: Over-abstraction for two concrete types. Resources and prompts
  have different execution semantics (read-only vs template expansion) that would fight a single
  generic trait.

**Rationale**: The `Tool` trait is the universal dispatch surface in Corvus. The agent loop already
knows how to discover, approve, execute, and format `Tool` results. By implementing `Tool` for
resources and prompts:
- The LLM sees them in the tool list and can call them naturally.
- The dispatcher's `check_tool_risk()` applies automatically via `source_kind_for_tool()`.
- Output limiting via `enforce_output_limit()` works out of the box.
- No changes to `unified_loop.rs`, `ToolSpec`, or provider message formatting.

The `ToolSourceMetadata` struct already has `kind` and `provider` fields that can distinguish
capability types (e.g., `kind: "mcp_resource"` vs `kind: "mcp"`) for policy differentiation.

### Decision 3: Client Extension — Add JSON-RPC Methods to Existing `McpClient`

**Choice**: Extend `McpClient` with four new methods:
- `list_resources() -> Vec<McpResourceManifest>`
- `read_resource(uri: &str) -> String`
- `list_prompts() -> Vec<McpPromptManifest>`
- `get_prompt(name: &str, arguments: Value) -> Vec<PromptMessage>`

These follow the same transport pattern as `list_tools()` and `call_tool()`.

**Alternatives considered**:
- **Separate client per capability type**: Would duplicate transport, timeout, and error handling
  logic. The MCP protocol uses a single connection per server for all capability types.
- **Generic `call_method(method: &str, params: Value)` approach**: Too loose — loses type safety
  on return values and makes mock testing harder.

**Rationale**: MCP is a single JSON-RPC protocol per server. The transport layer (`McpClient`)
already handles process lifecycle, timeout enforcement, and output limiting. Adding methods for
new JSON-RPC endpoints is the natural extension. The existing `parse_tool_manifest_payload()`
function already detects `resources` and `prompts` keys in the payload (client.rs:445-452) — this
becomes the entry point for parsing them instead of warning and ignoring.

### Decision 4: Config Schema — Add `capabilities` Field to `McpServerConfig`

**Choice**: Add an optional `capabilities` field to `McpServerConfig`:

```rust
// In config/schema.rs, extend McpServerConfig:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    // ... existing fields unchanged ...

    /// Which MCP capability types to discover and register.
    /// Default: ["tools"] for backward compatibility.
    /// Valid values: "tools", "resources", "prompts"
    #[serde(default = "default_mcp_capabilities")]
    pub capabilities: Vec<String>,

    /// Optional per-capability output limit overrides.
    #[serde(default)]
    pub resource_output_limit_bytes: Option<usize>,

    #[serde(default)]
    pub prompt_output_limit_bytes: Option<usize>,
}

fn default_mcp_capabilities() -> Vec<String> {
    vec!["tools".to_string()]
}
```

**Alternatives considered**:
- **Boolean flags (`enable_resources: bool`)**: Less extensible if new capability types are added.
  A list is more composable.
- **Nested config objects per capability**: Over-engineered for the initial model. Per-capability
  output limit overrides as optional flat fields are sufficient.
- **Global `mcp.enable_resources` flag**: Lacks per-server granularity. The proposal explicitly
  chose per-server flags as authoritative.

**Rationale**: The `capabilities` list defaults to `["tools"]`, ensuring zero breaking changes for
existing configs. Operators opt in to resources or prompts per server. Config validation rejects
unknown capability values at load time (fail-fast). The flat optional override fields
(`resource_output_limit_bytes`, `prompt_output_limit_bytes`) inherit from `output_limit_bytes`
when absent, following the existing pattern.

### Decision 5: Discovery Flow — Single Introspection, Capability-Gated Registration

**Choice**: The discovery function queries all capability types in a single server introspection
call, but only registers capabilities that appear in the server's `capabilities` config list.

```
discover_capabilities(config) {
    for server in config.servers {
        client = McpClient::new(server)
        manifest = client.introspect()   // Returns tools + resources + prompts

        if "tools" in server.capabilities {
            register_tools(manifest.tools)
        }
        if "resources" in server.capabilities {
            register_resources(manifest.resources)
        }
        if "prompts" in server.capabilities {
            register_prompts(manifest.prompts)
        }
    }
}
```

**Alternatives considered**:
- **Separate introspection calls per capability type** (`list_tools()`, then `list_resources()`,
  then `list_prompts()`): Three process spawns per server for stdio transport. Wasteful when the
  server's manifest payload already contains all capability types.
- **Auto-discover all advertised capabilities**: Ignores operator intent. A server may advertise
  prompts, but the operator may not trust that server for prompt injection.

**Rationale**: MCP servers typically return a single manifest payload with all capability types
(the current `parse_tool_manifest_payload` already sees `resources` and `prompts` keys). Parsing
all types from one payload is efficient. The `capabilities` config list acts as an allowlist gate —
only explicitly enabled capability types proceed to registration. This is secure-by-default: a
server advertising prompts gets those prompts ignored unless the operator explicitly opts in.

**Note on transport**: For stdio servers that use separate JSON-RPC method calls rather than a
single manifest, the client falls back to individual `list_tools()`, `list_resources()`,
`list_prompts()` calls. The discovery function handles both patterns.

### Decision 6: Agent Loop Integration — Tool-Shaped Callables, No Loop Changes

**Choice**: Resources and prompts appear as tools in the agent's tool list. The LLM calls them
like any other tool.

- **Resources**: LLM calls `mcp.<server>.resource.<name>` with optional arguments. The adapter
  calls `read_resource(uri)` and returns the content as a `ToolResult`.
- **Prompts**: LLM calls `mcp.<server>.prompt.<name>` with template arguments. The adapter calls
  `get_prompt(name, arguments)` and returns the expanded prompt messages as a formatted
  `ToolResult`.

**Alternatives considered**:
- **Resources as context injection (prompt-time fetch)**: Resources would be fetched during
  `build_system_prompt()` and injected as `PromptSection` content. Transparent to LLM but requires
  changes to `agent/prompt.rs`, adds startup latency, and forces all resources to load even if
  unused. Also loses LLM agency — the model can't decide _which_ resources to read.
- **Prompts as `PromptSection` implementations**: Templates would inject directly into the system
  prompt. Removes LLM control over when to use a prompt template and creates a prompt injection
  surface that's harder to audit (content appears as system instructions rather than tool results).

**Rationale**: Tool-shaped callables are the lowest-risk integration path:
1. The agent loop (`unified_loop.rs`) needs zero changes.
2. The LLM retains agency over when to invoke resources/prompts.
3. All invocations pass through the existing approval and policy flow.
4. Output is clearly bounded by `output_limit_bytes`.
5. Prompt content returned as tool results is naturally distinguished from system instructions —
   it appears in the conversation as an assistant-requested tool response, not as system-level
   directives. This mitigates the prompt injection risk identified in the exploration.

### Decision 7: Policy Model — Differentiated Policy via `ToolSourceMetadata.kind`

**Choice**: Extend `ToolSourceMetadata.kind` to distinguish capability types. Policy evaluation
uses this for differentiated defaults:

| `kind` value | Policy Default | Rationale |
|---|---|---|
| `"mcp"` (existing tools) | `ApprovalRequired` | Unchanged from v1 |
| `"mcp_resource"` | `ApprovalRequired` | Read-only but may expose sensitive data; start restrictive, relax later |
| `"mcp_prompt"` | `ApprovalRequired` | Highest risk — prompt injection surface |

**Alternatives considered**:
- **`AllowWithLimits` for resources by default**: The proposal suggested this, but after reviewing
  the actual code, the `ToolPolicyDecision` enum has three values: `Allow`, `ApprovalRequired`,
  `Deny`. There is no `AllowWithLimits` variant. Adding one would require changes to the policy
  enum, the dispatcher, the approval flow, and the agent loop. For the initial implementation,
  using `ApprovalRequired` for all MCP capability types is the safest choice. Operators can
  configure blanket-allow for specific resource tools once trust is established.
- **New policy enum variants per capability type**: Over-engineering. The existing `mcp.` prefix
  detection in `source_kind_for_tool()` handles all MCP capabilities. Differentiating resource
  vs prompt policy can be done by extending `ToolSourceKind` to `McpResource` / `McpPrompt`
  variants, which is a small, contained change.

**Rationale**: Fail closed. All MCP capabilities require approval by default. This matches the
project's security-first posture and the AGENTS.md non-negotiable: "Do not weaken sandboxing,
auth, secrets handling, or policy boundaries." Operators who trust a resource server can configure
policy exceptions. The `ToolSourceMetadata.kind` field provides the data needed for future
fine-grained policy without changing the dispatch architecture.

**Implementation path for `source_kind_for_tool()`**:

```rust
// security/policy.rs — extend ToolSourceKind
pub enum ToolSourceKind {
    Native,
    Mcp,           // existing: mcp.<server>.<tool>
    McpResource,   // new: mcp.<server>.resource.<name>
    McpPrompt,     // new: mcp.<server>.prompt.<name>
    Unknown,
}

pub fn source_kind_for_tool(tool_name: &str) -> ToolSourceKind {
    if let Some(rest) = tool_name.strip_prefix("mcp.") {
        if rest.contains(".resource.") {
            ToolSourceKind::McpResource
        } else if rest.contains(".prompt.") {
            ToolSourceKind::McpPrompt
        } else {
            ToolSourceKind::Mcp
        }
    } else if tool_name.is_empty() {
        ToolSourceKind::Unknown
    } else {
        ToolSourceKind::Native
    }
}
```

### Decision 8: Naming and Registry — Unified Registry, Extended Normalization

**Choice**: Resources and prompts register in the same `Vec<Box<dyn Tool>>` as MCP tools. The
naming convention adds a capability-type segment:

```
mcp.<server>.<tool_name>              # Existing tools (unchanged)
mcp.<server>.resource.<resource_name> # Resources
mcp.<server>.prompt.<prompt_name>     # Prompts
```

Collision detection spans all capability types in a single `HashSet<String>` (the existing
`seen_names` set in `discover_tools()`).

**Alternatives considered**:
- **Separate registries per capability type**: Would require the dispatcher to check multiple
  registries and the agent loop to merge tool lists from multiple sources. The existing unified
  registry handles this naturally.
- **No capability-type segment** (e.g., `mcp.<server>.<resource_name>`): Collides if a server
  has a tool and resource with the same name. The segment prevents this.

**Rationale**: The unified registry is the simplest approach. Collision detection already exists
in `discover_tools()` — the extended `discover_capabilities()` reuses the same `seen_names` set
across all capability types. The `resource.` and `prompt.` segments in the name serve double duty:
disambiguation AND policy routing (used by `source_kind_for_tool()`).

**Normalization extension**:

```rust
// normalize.rs — new functions alongside existing normalize_tool_name()
pub fn normalize_resource_name(server: &str, resource_name: &str) -> anyhow::Result<String> {
    validate_identifier("server", server)?;
    validate_identifier("resource", resource_name)?;
    Ok(format!("mcp.{server}.resource.{resource_name}"))
}

pub fn normalize_prompt_name(server: &str, prompt_name: &str) -> anyhow::Result<String> {
    validate_identifier("server", server)?;
    validate_identifier("prompt", prompt_name)?;
    Ok(format!("mcp.{server}.prompt.{prompt_name}"))
}
```

Reserved words extended: `"resource"` and `"prompt"` are reserved as server names to prevent
ambiguity (e.g., `mcp.resource.resource.foo` would be confusing).

## Data Flow

### Resource Discovery and Read Flow

```
                        STARTUP
                        ───────
Config (TOML)
    │
    ▼
McpServerConfig { capabilities: ["tools", "resources"] }
    │
    ▼
discover_capabilities()
    │
    ├──▶ McpClient::list_tools()     ──▶ Vec<McpToolManifest>
    │                                        │
    │                                        ▼
    │                                   McpToolAdapter::from_manifest()
    │                                        │
    │                                        ▼
    │                                   Box<dyn Tool> ──▶ unified registry
    │
    └──▶ McpClient::list_resources() ──▶ Vec<McpResourceManifest>
                                             │
                                             ▼
                                        McpResourceAdapter::from_manifest()
                                             │
                                             ▼
                                        Box<dyn Tool> ──▶ unified registry

                        RUNTIME
                        ───────
LLM ──tool_call──▶ "mcp.docs.resource.api-spec"
    │
    ▼
Dispatcher::check_tool_risk()
    │
    ├── source_kind_for_tool() → McpResource
    ├── policy: ApprovalRequired
    └── (approval granted)
            │
            ▼
      McpResourceAdapter::execute(args)
            │
            ▼
      McpClient::read_resource(uri)
            │
            ▼
      ToolResult { success: true, output: "<resource content>" }
            │
            ▼
      enforce_output_limit()
            │
            ▼
      Agent loop receives tool result
```

### Prompt Discovery and Expansion Flow

```
                        STARTUP
                        ───────
McpServerConfig { capabilities: ["tools", "prompts"] }
    │
    ▼
discover_capabilities()
    │
    └──▶ McpClient::list_prompts()  ──▶ Vec<McpPromptManifest>
                                            │
                                            ▼
                                       McpPromptAdapter::from_manifest()
                                            │
                                            ▼
                                       Box<dyn Tool> ──▶ unified registry

                        RUNTIME
                        ───────
LLM ──tool_call──▶ "mcp.workflows.prompt.code-review"
                   { "arguments": { "language": "rust", "focus": "security" } }
    │
    ▼
Dispatcher::check_tool_risk()
    │
    ├── source_kind_for_tool() → McpPrompt
    ├── policy: ApprovalRequired
    └── (approval granted)
            │
            ▼
      McpPromptAdapter::execute(args)
            │
            ▼
      McpClient::get_prompt("code-review", { "language": "rust", "focus": "security" })
            │
            ▼
      Vec<PromptMessage> ──serialize──▶ formatted output with provenance
            │
            ▼
      ToolResult {
          success: true,
          output: "[mcp_prompt source=workflows fetched=2026-03-27T...]\n<template content>",
          structured: Some({ "messages": [...], "provenance": {...} })
      }
            │
            ▼
      enforce_output_limit()
            │
            ▼
      Agent loop receives tool result (as tool response, NOT system instructions)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src/config/schema.rs` | Modify | Add `capabilities`, `resource_output_limit_bytes`, `prompt_output_limit_bytes` to `McpServerConfig`; add validation for capability values |
| `src/tools/mcp/mod.rs` | Modify | Rename `discover_tools` → `discover_capabilities`; add resource/prompt registration branches gated by `capabilities` config; extend `seen_names` collision detection |
| `src/tools/mcp/client.rs` | Modify | Add `McpResourceManifest`, `McpPromptManifest` structs; add `list_resources()`, `read_resource()`, `list_prompts()`, `get_prompt()` methods; extend manifest parsing to extract resources/prompts (replace warn-and-ignore at line 447-452) |
| `src/tools/mcp/resource_adapter.rs` | Create | `McpResourceAdapter` implementing `Tool` trait; wraps `read_resource()` with output limiting, URI validation, and source metadata |
| `src/tools/mcp/prompt_adapter.rs` | Create | `McpPromptAdapter` implementing `Tool` trait; wraps `get_prompt()` with output limiting, provenance tagging, and structured output |
| `src/tools/mcp/normalize.rs` | Modify | Add `normalize_resource_name()`, `normalize_prompt_name()`; extend reserved word list to include `"resource"` and `"prompt"` as server names |
| `src/security/policy.rs` | Modify | Extend `ToolSourceKind` with `McpResource`, `McpPrompt` variants; update `source_kind_for_tool()` to detect `.resource.` and `.prompt.` segments; map both to `ApprovalRequired` |
| `src/agent/dispatcher.rs` | Modify | Update `evaluate_tool_risk_for_origin()` to handle `McpResource` and `McpPrompt` source kinds (both → `ApprovalRequired`) |
| `openspec/specs/mcp-runtime/spec.md` | Modify | Remove v1 exclusion clause (line 9-10); add resource and prompt requirements and scenarios |

## Interfaces / Contracts

### New Manifest Structs (client.rs)

```rust
#[derive(Debug, Clone)]
pub struct McpResourceManifest {
    pub name: String,
    pub uri: String,
    pub description: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpPromptManifest {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: String,       // "system", "user", "assistant"
    pub content: String,
}
```

### McpResourceAdapter (resource_adapter.rs)

```rust
pub struct McpResourceAdapter {
    name: String,           // "mcp.<server>.resource.<resource_name>"
    description: String,
    uri: String,
    mime_type: Option<String>,
    server_name: String,
    output_limit_bytes: usize,
    client: McpClient,
}

#[async_trait]
impl Tool for McpResourceAdapter {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }

    fn parameters_schema(&self) -> serde_json::Value {
        // Resources take no user arguments — the URI is fixed at discovery time
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // Calls McpClient::read_resource(self.uri)
        // Enforces output_limit_bytes
        // Returns content as ToolResult
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters_schema(),
            source: Some(ToolSourceMetadata {
                kind: "mcp_resource".to_string(),
                provider: Some("mcp".to_string()),
                server: Some(self.server_name.clone()),
                original_name: Some(self.uri.clone()),
            }),
        }
    }
}
```

### McpPromptAdapter (prompt_adapter.rs)

```rust
pub struct McpPromptAdapter {
    name: String,           // "mcp.<server>.prompt.<prompt_name>"
    description: String,
    original_name: String,
    arguments: Vec<PromptArgument>,
    server_name: String,
    output_limit_bytes: usize,
    client: McpClient,
}

#[async_trait]
impl Tool for McpPromptAdapter {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }

    fn parameters_schema(&self) -> serde_json::Value {
        // Generate JSON schema from self.arguments
        // Required arguments → "required" array
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // Calls McpClient::get_prompt(self.original_name, args)
        // Adds provenance header: [mcp_prompt source=<server> fetched=<timestamp>]
        // Serializes Vec<PromptMessage> into formatted output
        // Enforces output_limit_bytes
        // Returns structured field with messages + provenance metadata
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters_schema(),
            source: Some(ToolSourceMetadata {
                kind: "mcp_prompt".to_string(),
                provider: Some("mcp".to_string()),
                server: Some(self.server_name.clone()),
                original_name: Some(self.original_name.clone()),
            }),
        }
    }
}
```

### Extended Config (schema.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_mcp_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
    #[serde(default = "default_mcp_call_timeout_ms")]
    pub call_timeout_ms: u64,
    #[serde(default = "default_mcp_output_limit_bytes")]
    pub output_limit_bytes: usize,

    // --- New fields ---
    #[serde(default = "default_mcp_capabilities")]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub resource_output_limit_bytes: Option<usize>,
    #[serde(default)]
    pub prompt_output_limit_bytes: Option<usize>,
}
```

### Extended Policy (security/policy.rs)

```rust
pub enum ToolSourceKind {
    Native,
    Mcp,
    McpResource,
    McpPrompt,
    Unknown,
}

pub fn source_kind_for_tool(tool_name: &str) -> ToolSourceKind {
    if let Some(rest) = tool_name.strip_prefix("mcp.") {
        if rest.contains(".resource.") {
            ToolSourceKind::McpResource
        } else if rest.contains(".prompt.") {
            ToolSourceKind::McpPrompt
        } else {
            ToolSourceKind::Mcp
        }
    } else if tool_name.is_empty() {
        ToolSourceKind::Unknown
    } else {
        ToolSourceKind::Native
    }
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `normalize_resource_name()`, `normalize_prompt_name()` produce correct canonical names | Mirror existing `normalize_tool_name` tests in `normalize.rs` |
| Unit | `source_kind_for_tool()` detects `McpResource` and `McpPrompt` variants | Extend existing `source_kind_distinguishes_mcp_from_native` test |
| Unit | `McpResourceAdapter::execute()` returns resource content with output limiting | Mock client pattern from existing `adapter.rs` tests |
| Unit | `McpPromptAdapter::execute()` returns formatted prompt with provenance | Mock client, verify provenance header and structured output |
| Unit | `McpPromptAdapter::execute()` rejects missing required arguments | Validation test |
| Unit | Config validation rejects unknown capability values | Extend existing config validation tests |
| Unit | Config `capabilities` defaults to `["tools"]` when absent | Deserialization test |
| Unit | Collision detection spans tools + resources + prompts | Extend `collision_error_message` test |
| Integration | `discover_capabilities()` with mock servers advertising all three types | Extend existing mock discovery tests; use `__mcp_mock__` with resources/prompts in payload |
| Integration | Policy evaluation for `mcp.*.resource.*` and `mcp.*.prompt.*` names | Extend existing policy tests |
| Integration | End-to-end resource read through adapter → client → mock server | Async test with mock transport |
| Integration | End-to-end prompt get through adapter → client → mock server | Async test with mock transport |

## Migration / Rollout

No data migration required. This is a planning-scope change — no runtime code ships with this
change.

For the implementation phases:

**Phase 1 (Resources)**:
- Add `capabilities` field with `["tools"]` default — zero config migration needed.
- Existing `__mcp_mock__` test infrastructure extends naturally (mock payload already contains
  `resources` key).
- Feature is invisible until operator adds `"resources"` to a server's capabilities list.

**Phase 2 (Prompts)**:
- Same pattern — invisible until operator adds `"prompts"` to capabilities.
- Provenance tagging ensures prompt-sourced content is always attributable.
- Prompts require the most thorough security review before implementation ships.

**Rollback**: Remove capability-gated code paths; restore warn-and-ignore behavior in
`parse_tool_manifest_payload()`. Config files with `capabilities` field would need the field
to be ignored (serde `#[serde(default)]` handles this — unknown values use defaults).

## Open Questions

- [x] Should resources use `AllowWithLimits` or `ApprovalRequired`? → **Resolved**: `ApprovalRequired`.
  The `ToolPolicyDecision` enum lacks an `AllowWithLimits` variant. Start restrictive; operators
  can configure exceptions. Adding a new policy variant is a future enhancement.
- [x] Single or separate introspection calls? → **Resolved**: Single manifest parse preferred,
  with fallback to separate calls for servers that use individual JSON-RPC methods.
- [x] Should `discover_tools` be renamed? → **Resolved**: Yes, to `discover_capabilities()`.
  The old name is misleading once resources/prompts are registered. Internal refactor, no public
  API impact.
- [ ] Resource URI validation strictness: Should the adapter validate resource URIs against a
  per-server allowlist, or trust the server's advertised URIs? Initial implementation trusts
  advertised URIs (they come from the server's own manifest), but a future `allowed_uri_patterns`
  config field could add operator control.
- [ ] Prompt argument sanitization: Should prompt template arguments be sanitized (e.g., strip
  control characters, limit length) before passing to the MCP server? Likely yes, but the exact
  sanitization rules need definition during Phase 2 implementation.
