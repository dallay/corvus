# Tasks: MCP Platform Capabilities Beyond Tools

**Change**: `mcp-platform-capabilities`
**Issue**: #258
**Scope**: Planning-only — these tasks describe follow-up implementation work, not work to do now.

Each task references the spec requirements it addresses and the design decisions it follows.
Complexity: XS (<30 min), S (30-60 min), M (1-2 hrs), L (2-4 hrs).

---

## Phase 0: Infrastructure (shared foundations for Resources and Prompts)

- [x] **0.1** Add `capabilities` field to `McpServerConfig` in `src/config/schema.rs`
    - Add `capabilities: Vec<String>` with `#[serde(default = "default_mcp_capabilities")]`
      defaulting to `["tools"]`
    - Add `resource_output_limit_bytes: Option<usize>` and
      `prompt_output_limit_bytes: Option<usize>` optional override fields
    - Add `default_mcp_capabilities()` helper returning `vec!["tools".to_string()]`
    - **Files**: `src/config/schema.rs`
    - **Spec**: Per-Server Capability Declaration and Validation (all 5 scenarios); Backward
      Compatibility Guarantees (scenario: existing tool-only config works unchanged)
    - **Design**: Decision 4
    - **Complexity**: S

- [x] **0.2** Add config validation for `capabilities` field
    - Reject unknown capability types (only `"tools"`, `"resources"`, `"prompts"` are valid)
    - Reject empty capabilities list
    - Reject duplicate capability entries
    - Validation runs at config load time, fail-fast with structured errors
    - **Files**: `src/config/schema.rs` (or `src/config/validate.rs` if validation is separate)
    - **Spec**: Per-Server Capability Declaration and Validation (scenarios: invalid type rejected,
      empty list rejected, duplicates rejected)
    - **Design**: Decision 4
    - **Complexity**: S

- [x] **0.3** Extend `ToolSourceKind` enum and `source_kind_for_tool()` in policy module
    - Add `McpResource` and `McpPrompt` variants to `ToolSourceKind`
    - Update `source_kind_for_tool()` to detect `.resource.` and `.prompt.` segments in tool names
    - Both new variants map to `ApprovalRequired` policy default
    - **Files**: `src/security/policy.rs`
    - **Spec**: Resource Policy Enforcement (scenario: policy default); Prompt Operator-Only
      Approval Model (scenario: policy default is ApprovalRequired)
    - **Design**: Decision 7
    - **Complexity**: S

- [x] **0.4** Extend normalization with `normalize_resource_name()` and `normalize_prompt_name()`
    - Add `normalize_resource_name(server, resource_name) -> Result<String>` producing
      `mcp.<server>.resource.<name>`
    - Add `normalize_prompt_name(server, prompt_name) -> Result<String>` producing
      `mcp.<server>.prompt.<name>`
    - Add `"resource"` and `"prompt"` to reserved word list (as both tool names and server names)
    - Reuse existing `validate_identifier()` for character validation
    - **Files**: `src/tools/mcp/normalize.rs`
    - **Spec**: Namespaced Resource Identity (all 5 scenarios); Namespaced Prompt Identity (all 4
      scenarios); Reserved Namespace Protection (scenarios: tool named "resource"/"prompt" rejected)
    - **Design**: Decision 8
    - **Complexity**: S

- [x] **0.5** Add JSON-RPC methods to `McpClient` for resources and prompts
    - Add `McpResourceManifest`, `McpPromptManifest`, `PromptArgument`, `PromptMessage` structs
    - Add `list_resources() -> Vec<McpResourceManifest>` method
    - Add `read_resource(uri: &str) -> String` method
    - Add `list_prompts() -> Vec<McpPromptManifest>` method
    - Add `get_prompt(name: &str, arguments: Value) -> Vec<PromptMessage>` method
    - Replace warn-and-ignore behavior at `client.rs:447-452` with actual manifest parsing
    - All methods follow existing timeout and error handling patterns
    - **Files**: `src/tools/mcp/client.rs`
    - **Spec**: Capability Discovery During Server Introspection; MCP Resource Read Semantics;
      Prompt Expansion Semantics
    - **Design**: Decision 3
    - **Complexity**: M

- [x] **0.6** Refactor `discover_tools()` → `discover_capabilities()` with capability gating
    - Rename `discover_tools` to `discover_capabilities` in `src/tools/mcp/mod.rs`
    - Gate `list_tools()`, `list_resources()`, `list_prompts()` calls on each server's
      `capabilities` config
    - Extend `seen_names: HashSet<String>` collision detection across all capability types
    - Partial discovery failure must not discard successful results from same server
    - Log diagnostics for ignored advertisements (server advertises capability not in config)
    - **Files**: `src/tools/mcp/mod.rs`
    - **Spec**: Startup Discovery and Registration (all 4 scenarios); Capability Discovery During
      Server Introspection (scenarios: timeout bounded, partial failure); Out-of-Scope MCP
      Capabilities Are Rejected (both scenarios)
    - **Design**: Decision 5
    - **Complexity**: M

- [x] **0.7** Unit tests for Phase 0 infrastructure
    - Test `default_mcp_capabilities()` returns `["tools"]`
    - Test config deserialization with missing `capabilities` field defaults correctly
    - Test config validation rejects `"subscriptions"`, empty list, duplicates
    - Test `source_kind_for_tool()` detects `McpResource` and `McpPrompt` from name patterns
    - Test `normalize_resource_name()` and `normalize_prompt_name()` produce correct canonical names
    - Test reserved words `"resource"` and `"prompt"` are rejected as tool names and server names
    - Test collision detection across tools + resources + prompts in `discover_capabilities()`
    - **Files**: `src/config/schema.rs` (tests mod), `src/security/policy.rs` (tests mod),
      `src/tools/mcp/normalize.rs` (tests mod), `src/tools/mcp/mod.rs` (tests mod)
    - **Spec**: All config validation scenarios; all naming scenarios; all collision scenarios
    - **Complexity**: M

---

## Phase 1: Resources (lower risk, higher value)

- [x] **1.1** Create `McpResourceAdapter` implementing `Tool` trait
    - Create `src/tools/mcp/resource_adapter.rs`
    - Implement `Tool` for `McpResourceAdapter` with: `name()`, `description()`,
      `parameters_schema()` (empty object — URI is fixed at discovery), `execute()` (calls
      `McpClient::read_resource(uri)`), `spec()` (with `kind: "mcp_resource"`)
    - `execute()` enforces `output_limit_bytes` (resource-specific override or server default)
    - `execute()` returns content with MIME type metadata
    - Handle empty/null content as valid result, not error
    - **Files**: `src/tools/mcp/resource_adapter.rs` (new), `src/tools/mcp/mod.rs` (module
      declaration)
    - **Spec**: MCP Resource Read Semantics (all 3 scenarios); Resource Timeout and Output Limit
      Enforcement (all 4 scenarios)
    - **Design**: Decision 2, Decision 6
    - **Complexity**: M

- [x] **1.2** Wire resource registration into `discover_capabilities()`
    - When `"resources"` is in server's capabilities, call `list_resources()` and create
      `McpResourceAdapter` instances
    - Register each as `Box<dyn Tool>` in unified registry with `mcp.<server>.resource.<name>`
      identity
    - Apply collision detection against tools and other resources
    - Log diagnostics for skipped resources (invalid names, duplicates)
    - **Files**: `src/tools/mcp/mod.rs`
    - **Spec**: Startup Discovery and Registration (scenario: register resources alongside tools);
      Namespaced Resource Identity (scenarios: canonical naming, cross-server no collision)
    - **Design**: Decision 5, Decision 8
    - **Complexity**: S

- [x] **1.3** Add resource failure isolation
    - Resource read failure returns structured `ToolResult` with error info
    - Failure on one server does not affect other servers' resources or tools
    - Agent loop must not crash or deadlock on resource failure
    - Diagnostic output redacts secret values from server environment
    - **Files**: `src/tools/mcp/resource_adapter.rs`
    - **Spec**: Resource Failure Isolation (both scenarios); Diagnostic Redaction for All Capability
      Types (scenario: resource discovery diagnostic)
    - **Complexity**: S

- [x] **1.4** Update dispatcher for `McpResource` risk classification
    - Update `evaluate_tool_risk_for_origin()` (or equivalent) to handle
      `ToolSourceKind::McpResource`
    - Map to `ApprovalRequired` policy
    - **Files**: `src/agent/dispatcher.rs`
    - **Spec**: Resource Policy Enforcement (all 3 scenarios)
    - **Design**: Decision 7
    - **Complexity**: XS

- [x] **1.5** Unit and integration tests for resource support
    - Unit: `McpResourceAdapter::execute()` returns resource content with output limiting
    - Unit: `McpResourceAdapter::execute()` handles empty content without error
    - Unit: `McpResourceAdapter::execute()` enforces timeout
    - Unit: resource-specific `output_limit_bytes` overrides server default
    - Unit: resource limits do not affect tool execution limits
    - Integration: `discover_capabilities()` with mock server advertising tools + resources
    - Integration: policy evaluation for `mcp.*.resource.*` names returns `ApprovalRequired`
    - Integration: end-to-end resource read through adapter → client → mock server
    - Integration: resource failure isolation (one server fails, others unaffected)
    - Integration: diagnostic redaction on resource failure
    - **Files**: `src/tools/mcp/resource_adapter.rs` (tests mod), `src/tools/mcp/mod.rs` (tests
      mod), `src/agent/dispatcher.rs` (tests mod)
    - **Spec**: All Resource Read, Timeout, Output Limit, Failure Isolation, and Policy scenarios
    - **Complexity**: L

---

## Phase 2: Prompts (higher risk, requires security review)

- [x] **2.1** Create `McpPromptAdapter` implementing `Tool` trait
    - Create `src/tools/mcp/prompt_adapter.rs`
    - Implement `Tool` for `McpPromptAdapter` with: `name()`, `description()`,
      `parameters_schema()` (generated from `PromptArgument` list), `execute()` (calls
      `McpClient::get_prompt()`), `spec()` (with `kind: "mcp_prompt"`)
    - `execute()` validates arguments against schema before calling server (reject missing required,
      reject unknown)
    - `execute()` adds provenance header: `[mcp_prompt source=<server> fetched=<timestamp>]`
    - `execute()` serializes `Vec<PromptMessage>` into formatted output with structured metadata
    - `execute()` enforces `output_limit_bytes`
    - Handle empty message array as valid result, not error
    - **Files**: `src/tools/mcp/prompt_adapter.rs` (new), `src/tools/mcp/mod.rs` (module
      declaration)
    - **Spec**: Prompt Expansion Semantics (both scenarios); Prompt Parameter Validation (all 3
      scenarios); Prompt Timeout and Output Limit Enforcement (both scenarios)
    - **Design**: Decision 2, Decision 6
    - **Complexity**: M

- [x] **2.2** Add prompt injection mitigation to `McpPromptAdapter`
    - Tag all prompt content with provenance metadata (source server, fetch timestamp)
    - Ensure prompt content is returned as tool result, NOT placed in system-level instruction
      position
    - Implement content scanning hook point (trait or closure) for operator-attached validation
    - If scanning hook rejects content, return structured rejection result
    - **Files**: `src/tools/mcp/prompt_adapter.rs`
    - **Spec**: Prompt Injection Mitigation (all 3 scenarios)
    - **Design**: Decision 6 (rationale point 5: tool results distinguished from system
      instructions)
    - **Complexity**: M

- [x] **2.3** Wire prompt registration into `discover_capabilities()`
    - When `"prompts"` is in server's capabilities, call `list_prompts()` and create
      `McpPromptAdapter` instances
    - Register each as `Box<dyn Tool>` in unified registry with `mcp.<server>.prompt.<name>`
      identity
    - Apply collision detection against tools, resources, and other prompts
    - Prompts MUST NOT be registered if `"prompts"` is not explicitly in capabilities (never
      defaulted)
    - **Files**: `src/tools/mcp/mod.rs`
    - **Spec**: Prompt Discovery and Registration (all 3 scenarios); Prompt Operator-Only Approval
      Model (scenario: explicit capability opt-in)
    - **Design**: Decision 5
    - **Complexity**: S

- [x] **2.4** Add prompt failure isolation
    - Prompt expansion failure returns structured `ToolResult` with error info
    - Failure on one server does not affect other servers' capabilities
    - Agent loop must not crash or deadlock on prompt failure
    - Diagnostic output redacts secret values
    - **Files**: `src/tools/mcp/prompt_adapter.rs`
    - **Spec**: Prompt Failure Isolation (both scenarios); Diagnostic Redaction (scenario: prompt
      expansion diagnostic, prompt content redaction)
    - **Complexity**: S

- [x] **2.5** Update dispatcher for `McpPrompt` risk classification
    - Update `evaluate_tool_risk_for_origin()` to handle `ToolSourceKind::McpPrompt`
    - Map to `ApprovalRequired` — prompt must NOT be auto-allowed even if resources on same server
      are allowed
    - **Files**: `src/agent/dispatcher.rs`
    - **Spec**: Prompt Operator-Only Approval Model (scenarios: invocation requires approval, policy
      default is ApprovalRequired)
    - **Design**: Decision 7
    - **Complexity**: XS

- [x] **2.6** Unit and integration tests for prompt support
    - Unit: `McpPromptAdapter::execute()` returns formatted prompt with provenance header
    - Unit: `McpPromptAdapter::execute()` rejects missing required argument
    - Unit: `McpPromptAdapter::execute()` rejects unknown argument
    - Unit: `McpPromptAdapter::execute()` handles empty message array
    - Unit: `McpPromptAdapter::execute()` enforces timeout and output limits
    - Unit: provenance metadata includes source server and timestamp
    - Unit: content scanning hook can reject prompt content
    - Integration: `discover_capabilities()` with mock server advertising tools + prompts
    - Integration: prompts NOT registered when `"prompts"` absent from capabilities
    - Integration: policy evaluation for `mcp.*.prompt.*` returns `ApprovalRequired`
    - Integration: end-to-end prompt get through adapter → client → mock server
    - Integration: prompt failure isolation
    - Integration: diagnostic redaction on prompt failure (including content with secret patterns)
    - **Files**: `src/tools/mcp/prompt_adapter.rs` (tests mod), `src/tools/mcp/mod.rs` (tests mod),
      `src/agent/dispatcher.rs` (tests mod)
    - **Spec**: All Prompt Discovery, Parameter Validation, Expansion, Approval, Injection
      Mitigation, Failure Isolation, and Diagnostic scenarios
    - **Complexity**: L

---

## Phase 3: Integration (cross-cutting concerns and parity)

- [x] **3.1** Verify entry-point parity for resources and prompts
    - Confirm registered resources and prompts are accessible via CLI, channels, and
      dispatcher-backed gateway `/webhook`
    - Confirm policy enforcement is equivalent across all entry points
    - Confirm fallback gateway path does NOT claim resource/prompt parity
    - **Files**: `src/agent/dispatcher.rs`, gateway integration points
    - **Spec**: Entry-Point Parity for All Capability Types (all 3 scenarios)
    - **Complexity**: M

- [x] **3.2** Cross-capability collision detection end-to-end test
    - Test: tool `mcp.docs.search` and resource `mcp.docs.resource.search` coexist without collision
    - Test: tool, resource, and prompt all named `summarize` on same server resolve to distinct
      identifiers
    - Test: duplicate resource/prompt within a server is rejected deterministically
    - Test: cross-server same-name resources do not collide
    - **Files**: `src/tools/mcp/mod.rs` (integration tests)
    - **Spec**: Namespaced Resource Identity (scenarios: collision with tool, cross-server,
      duplicate); Namespaced Prompt Identity (scenarios: no collision with tool/resource, duplicate)
    - **Complexity**: M

- [x] **3.3** Backward compatibility regression tests
    - Test: config without `capabilities` field works identically to v1 (tools-only discovery)
    - Test: adding `capabilities = ["tools", "resources"]` does not break existing tool
      registrations
    - Test: tool discovery behavior (naming, policy, timeouts, output limits) is unchanged
    - **Files**: `src/tools/mcp/mod.rs` (integration tests), `src/config/schema.rs` (tests)
    - **Spec**: Backward Compatibility Guarantees (all 3 scenarios)
    - **Complexity**: S

- [x] **3.4** Update `openspec/specs/mcp-runtime/spec.md` parent spec
    - Remove v1 exclusion clause (lines 9-11 stating resources/prompts are excluded)
    - Remove or update scenario at lines 224-227 mandating non-tool capability rejection
    - Add cross-reference to this delta spec for resource and prompt requirements
    - **Files**: `openspec/specs/mcp-runtime/spec.md`
    - **Spec**: N/A (spec maintenance)
    - **Complexity**: XS
