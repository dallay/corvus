# Delta for MCP Runtime: Platform Capabilities Beyond Tools

## Purpose

This delta specification extends the MCP Runtime Tooling Specification to define a three-tier
capability model covering Tools (existing v1), Resources (Phase 1), and Prompts (Phase 2). It
establishes per-server capability declaration, namespaced identity for all capability types,
security tiers, bounded execution, failure isolation, and backward compatibility guarantees.

This is a planning-scope change. No runtime code is delivered — the output is a normative
capability model that downstream implementation changes MUST follow.

**Parent spec**: `openspec/specs/mcp-runtime/spec.md`
**Issue**: #258

---

## MODIFIED Requirements

### Requirement: Out-of-Scope MCP Capabilities Are Rejected

(Previously: The runtime MUST ignore or reject non-tool capabilities; only MCP tools MAY be
registered. See parent spec lines 222-227.)

The runtime MUST handle MCP resource and prompt capabilities according to each server's declared
`capabilities` list. Capabilities not included in a server's declaration MUST still be ignored.
The v1 blanket rejection of all non-tool capabilities is replaced by per-server, per-capability
gating.

#### Scenario: Server advertising resources without capability declaration

- GIVEN an MCP server whose config omits the `capabilities` field
- AND the server advertises resources during introspection
- WHEN capability registration runs
- THEN the runtime MUST ignore the advertised resources
- AND the runtime MUST register only tools (the default capability)
- AND the runtime SHOULD log a diagnostic noting ignored resource advertisements.

#### Scenario: Server advertising prompts without capability declaration

- GIVEN an MCP server whose config omits the `capabilities` field
- AND the server advertises prompts during introspection
- WHEN capability registration runs
- THEN the runtime MUST ignore the advertised prompts
- AND the runtime MUST register only tools.

### Requirement: Startup Discovery and Registration

(Previously: Discovery was limited to tools via `list_tools()`. See parent spec lines 42-65.)

The runtime MUST discover and register all declared capability types at startup. Discovery MUST
call `list_tools()`, `list_resources()`, and/or `list_prompts()` according to each server's
`capabilities` declaration. All discovered capabilities MUST be integrated into the unified
registry alongside native tools.

#### Scenario: Register resources alongside tools during startup

- GIVEN an MCP server with `capabilities = ["tools", "resources"]`
- AND valid stdio configuration
- WHEN runtime initialization executes capability discovery
- THEN the runtime MUST call both `list_tools()` and `list_resources()` on that server
- AND discovered tools and resources MUST both appear in the unified dispatchable set.

#### Scenario: Register prompts alongside tools and resources during startup

- GIVEN an MCP server with `capabilities = ["tools", "resources", "prompts"]`
- WHEN runtime initialization executes capability discovery
- THEN the runtime MUST call `list_tools()`, `list_resources()`, and `list_prompts()`
- AND all three capability types MUST appear in the unified registry.

#### Scenario: Discovery skips undeclared capability types

- GIVEN an MCP server with `capabilities = ["tools"]`
- WHEN runtime initialization executes capability discovery
- THEN the runtime MUST call only `list_tools()`
- AND the runtime MUST NOT call `list_resources()` or `list_prompts()`.

---

## ADDED Requirements

### Requirement: Per-Server Capability Declaration and Validation

The runtime MUST support a `capabilities` field on each MCP server configuration that declares
which MCP capability types that server is allowed to provide. The field MUST accept a list of
capability type identifiers. Valid values are `"tools"`, `"resources"`, and `"prompts"`.

When the `capabilities` field is absent, the runtime MUST default to `["tools"]` to preserve
backward compatibility with existing v1 configurations.

The runtime MUST validate capability declarations at config load time. Invalid capability type
identifiers MUST cause a config validation error.

#### Scenario: Explicit capability declaration is honored

- GIVEN an MCP server config with `capabilities = ["tools", "resources"]`
- WHEN configuration loading and validation runs
- THEN the runtime MUST mark that server as enabled for tools and resources
- AND the runtime MUST NOT enable prompts for that server.

#### Scenario: Missing capabilities field defaults to tools-only

- GIVEN an MCP server config that does not include a `capabilities` field
- WHEN configuration loading and validation runs
- THEN the runtime MUST treat the server as having `capabilities = ["tools"]`
- AND no resources or prompts SHALL be discovered for that server.

#### Scenario: Invalid capability type is rejected

- GIVEN an MCP server config with `capabilities = ["tools", "subscriptions"]`
- WHEN configuration validation runs
- THEN the runtime MUST reject the configuration with a structured validation error
- AND the error MUST identify `"subscriptions"` as an unrecognized capability type.

#### Scenario: Empty capabilities list is rejected

- GIVEN an MCP server config with `capabilities = []`
- WHEN configuration validation runs
- THEN the runtime MUST reject the configuration with a structured validation error
- AND the error MUST indicate that at least one capability type is required.

#### Scenario: Duplicate capability types are rejected

- GIVEN an MCP server config with `capabilities = ["tools", "tools"]`
- WHEN configuration validation runs
- THEN the runtime MUST reject the configuration with a structured validation error
- AND the error MUST identify the duplicate entry.

### Requirement: Capability Discovery During Server Introspection

The runtime MUST discover available capabilities from each server at startup by calling the
appropriate MCP introspection methods for each declared capability type. Discovery for each
capability type MUST be bounded by the server's configured timeout.

#### Scenario: Resource discovery timeout is bounded

- GIVEN an MCP server with `capabilities = ["tools", "resources"]`
- AND the server does not respond to `list_resources()` within the configured timeout
- WHEN the timeout budget elapses
- THEN the runtime MUST terminate resource discovery for that server
- AND tool discovery results (if already obtained) MUST still be registered
- AND startup MUST continue without indefinite blocking.

#### Scenario: Prompt discovery timeout is bounded

- GIVEN an MCP server with `capabilities = ["tools", "prompts"]`
- AND the server does not respond to `list_prompts()` within the configured timeout
- WHEN the timeout budget elapses
- THEN the runtime MUST terminate prompt discovery for that server
- AND tool discovery results (if already obtained) MUST still be registered.

#### Scenario: Partial discovery failure does not discard successful results

- GIVEN an MCP server with `capabilities = ["tools", "resources"]`
- AND `list_tools()` succeeds but `list_resources()` fails
- WHEN discovery completes
- THEN the runtime MUST register the successfully discovered tools
- AND the runtime MUST log a diagnostic about the failed resource discovery
- AND the runtime MUST NOT discard successfully discovered capabilities from that server.

---

### Requirement: Namespaced Resource Identity

The runtime MUST normalize MCP resources to canonical namespaced identifiers following the pattern
`mcp.<server>.resource.<name>`. The `resource` segment MUST be a literal string that
disambiguates resources from tools and prompts on the same server.

#### Scenario: Canonical MCP resource naming

- GIVEN a discovered resource named `docs-index` from server `knowledge`
- WHEN the resource is normalized into the runtime registry
- THEN the canonical resource identifier MUST be `mcp.knowledge.resource.docs-index`
- AND source metadata MUST retain server origin and capability type.

#### Scenario: Resource name with invalid characters is rejected

- GIVEN a discovered resource whose name contains characters not permitted by the naming schema
- WHEN normalization runs
- THEN the runtime MUST reject that resource registration
- AND the runtime MUST log a diagnostic identifying the invalid name.

#### Scenario: Resource name collides with existing tool identity

- GIVEN server `docs` exposes a tool named `search` (registered as `mcp.docs.search`)
- AND the same server exposes a resource named `search`
- WHEN registry merge runs
- THEN the resource MUST be registered as `mcp.docs.resource.search`
- AND the tool `mcp.docs.search` MUST remain unaffected
- AND both registrations MUST coexist without collision.

#### Scenario: Cross-server resource name collision

- GIVEN server `alpha` exposes a resource named `index`
- AND server `beta` also exposes a resource named `index`
- WHEN registry merge runs
- THEN `mcp.alpha.resource.index` and `mcp.beta.resource.index` MUST both be registered
- AND no collision error SHALL occur because the server segment disambiguates them.

#### Scenario: Duplicate resource identifier within a server

- GIVEN an MCP server that advertises two resources with identical names
- WHEN discovery normalization runs
- THEN the runtime MUST reject the duplicate registration deterministically
- AND the runtime MUST return an actionable error describing the collision.

### Requirement: Namespaced Prompt Identity

The runtime MUST normalize MCP prompts to canonical namespaced identifiers following the pattern
`mcp.<server>.prompt.<name>`. The `prompt` segment MUST be a literal string that disambiguates
prompts from tools and resources on the same server.

#### Scenario: Canonical MCP prompt naming

- GIVEN a discovered prompt named `code-review` from server `devtools`
- WHEN the prompt is normalized into the runtime registry
- THEN the canonical prompt identifier MUST be `mcp.devtools.prompt.code-review`
- AND source metadata MUST retain server origin and capability type.

#### Scenario: Prompt name with invalid characters is rejected

- GIVEN a discovered prompt whose name contains characters not permitted by the naming schema
- WHEN normalization runs
- THEN the runtime MUST reject that prompt registration
- AND the runtime MUST log a diagnostic identifying the invalid name.

#### Scenario: Prompt name does not collide with same-name tool or resource

- GIVEN server `devtools` exposes a tool `summarize`, a resource `summarize`, and a prompt
  `summarize`
- WHEN registry merge runs
- THEN `mcp.devtools.summarize` (tool), `mcp.devtools.resource.summarize` (resource), and
  `mcp.devtools.prompt.summarize` (prompt) MUST all coexist without collision.

#### Scenario: Duplicate prompt identifier within a server

- GIVEN an MCP server that advertises two prompts with identical names
- WHEN discovery normalization runs
- THEN the runtime MUST reject the duplicate registration deterministically
- AND the runtime MUST return an actionable error describing the collision.

### Requirement: Reserved Namespace Protection for All Capability Types

The existing reserved namespace protection for tools MUST extend to resources and prompts.
The `resource` and `prompt` literal segments MUST be reserved — a tool name that matches
these literals MUST be rejected to prevent ambiguity.

#### Scenario: Tool named "resource" is rejected

- GIVEN an MCP server that exposes a tool literally named `resource`
- WHEN normalization runs
- THEN the runtime MUST reject that tool registration
- AND the error MUST indicate that `resource` is a reserved capability-type segment.

#### Scenario: Tool named "prompt" is rejected

- GIVEN an MCP server that exposes a tool literally named `prompt`
- WHEN normalization runs
- THEN the runtime MUST reject that tool registration
- AND the error MUST indicate that `prompt` is a reserved capability-type segment.

---

### Requirement: MCP Resource Read Semantics

Resources MUST be read-only, on-demand, and stateless. A resource read MUST NOT produce side
effects on the MCP server. The runtime MUST treat resource reads as bounded data retrieval
operations, not action-executing operations.

#### Scenario: Resource read returns content

- GIVEN a registered MCP resource `mcp.knowledge.resource.docs-index`
- WHEN the runtime executes a resource read
- THEN the runtime MUST call the MCP `resource.read()` method on the originating server
- AND the result MUST contain the resource content and MIME type metadata
- AND the read MUST NOT produce server-side state changes.

#### Scenario: Resource read with URI parameter

- GIVEN a registered MCP resource that requires a URI parameter
- WHEN the runtime executes a resource read with the URI
- THEN the runtime MUST pass the URI to the server's `resource.read()` method
- AND the runtime MUST validate the URI against the server's declared resource scope.

#### Scenario: Resource read returns empty content

- GIVEN a registered MCP resource that returns empty or null content
- WHEN the resource read completes
- THEN the runtime MUST return a structured result indicating empty content
- AND the runtime MUST NOT treat empty content as an error.

### Requirement: Resource Timeout and Output Limit Enforcement

Resource reads MUST be bounded by the server's configured timeout and output limits. Resource-
specific limit overrides MAY be defined in config and MUST take precedence over server-level
defaults when present.

#### Scenario: Resource read exceeds timeout

- GIVEN a resource read that exceeds the configured timeout
- WHEN the timeout budget is reached
- THEN the runtime MUST cancel or abort the in-flight resource read
- AND the runtime MUST return a timeout failure to the caller
- AND the agent loop MUST NOT hang.

#### Scenario: Resource read exceeds output limit

- GIVEN a resource read whose response exceeds the configured `output_limit_bytes`
- WHEN output processing reaches the limit
- THEN the runtime MUST truncate or fail the read per configured policy
- AND the returned result MUST indicate that output limits were enforced.

#### Scenario: Resource-specific output limit overrides server default

- GIVEN an MCP server with `output_limit_bytes = 65536`
- AND resource-specific config with `resource_limits.output_limit_bytes = 131072`
- WHEN a resource read is executed
- THEN the resource-specific limit of 131072 bytes MUST apply
- AND the server-level default MUST NOT constrain the resource read.

#### Scenario: Resource limits do not affect tool execution

- GIVEN an MCP server with resource-specific limit overrides
- WHEN a tool invocation is executed on the same server
- THEN the tool MUST use the server-level limits, not the resource-specific overrides.

### Requirement: Resource Failure Isolation

A resource read failure on one MCP server MUST NOT affect resource or tool operations on other
servers. Resource failures MUST be isolated per-server and per-capability.

#### Scenario: Resource failure on one server does not crash others

- GIVEN multiple MCP servers with resources enabled
- AND one server's resource read fails with a transport error
- WHEN the failure is handled
- THEN the runtime MUST return a structured failure result for that resource
- AND resources and tools on other servers MUST remain operational
- AND the agent loop MUST NOT crash or deadlock.

#### Scenario: Resource failure diagnostics are redacted

- GIVEN a resource read that fails with an error containing sensitive values
- WHEN the failure is logged or surfaced to the operator
- THEN the runtime MUST redact secret values in the diagnostic output
- AND the diagnostic MUST include the resource identifier and server name.

### Requirement: Resource Policy Enforcement

MCP resource reads MUST pass through the runtime's policy enforcement layer. The default policy
for resources MUST be `AllowWithLimits` — resource reads are permitted without per-invocation
approval but are constrained by configured output limits and timeouts.

#### Scenario: Resource read is allowed by default policy

- GIVEN an MCP resource read invocation
- AND no explicit deny policy exists for that resource
- WHEN the dispatcher evaluates security policy
- THEN the invocation MUST be allowed without requiring approval
- AND output limits and timeouts MUST still be enforced.

#### Scenario: Operator deny policy blocks resource read

- GIVEN an MCP resource read invocation
- AND an explicit deny policy exists for that resource or its server
- WHEN the dispatcher evaluates security policy
- THEN the invocation MUST be denied with a structured denial result
- AND the denial MUST include the policy reason.

#### Scenario: Resource policy is evaluated consistently across entry points

- GIVEN equivalent MCP resource read requests arriving via CLI, channels, and dispatcher-backed
  gateway `/webhook`
- WHEN policy checks are applied
- THEN all three entry points MUST enforce equivalent resource policy decisions
- AND no entry point MAY bypass resource policy gates.

---

### Requirement: MCP Prompt Discovery and Registration

The runtime MUST discover and register MCP prompts at startup for servers that declare
`"prompts"` in their `capabilities` list. Prompts MUST be registered as template-retrieval
capabilities — they return structured message content, they do not execute actions.

#### Scenario: Register prompts during startup

- GIVEN an MCP server with `capabilities = ["tools", "prompts"]`
- AND the server advertises prompts during introspection
- WHEN runtime initialization executes prompt discovery
- THEN the runtime MUST call `list_prompts()` on that server
- AND discovered prompts MUST be registered with their parameter schemas
- AND prompts MUST be included in the unified capability registry.

#### Scenario: Prompt with no parameters is registered

- GIVEN an MCP server advertising a prompt with no required arguments
- WHEN prompt discovery runs
- THEN the runtime MUST register the prompt with an empty parameter schema
- AND the prompt MUST be invocable without arguments.

#### Scenario: Prompt with required parameters is registered with schema

- GIVEN an MCP server advertising a prompt with typed required arguments
- WHEN prompt discovery runs
- THEN the runtime MUST register the prompt with its full parameter schema
- AND the parameter names, types, and required flags MUST be preserved.

### Requirement: Prompt Parameter Validation

The runtime MUST validate prompt arguments against the prompt's declared parameter schema before
sending the expansion request to the MCP server. Missing required parameters MUST cause a
validation error. Unknown parameters SHOULD be rejected.

#### Scenario: Missing required parameter is rejected

- GIVEN a registered prompt with required parameter `language`
- WHEN the runtime receives a prompt expansion request without `language`
- THEN the runtime MUST return a structured validation error
- AND the error MUST identify the missing required parameter
- AND the request MUST NOT be sent to the MCP server.

#### Scenario: Valid parameters pass validation

- GIVEN a registered prompt with required parameter `language` and optional parameter `style`
- WHEN the runtime receives a prompt expansion request with `language = "rust"`
- THEN validation MUST pass
- AND the request MUST be forwarded to the MCP server.

#### Scenario: Unknown parameter is rejected

- GIVEN a registered prompt whose schema does not include parameter `foo`
- WHEN the runtime receives a prompt expansion request including `foo = "bar"`
- THEN the runtime SHOULD reject the request with a validation error
- AND the error MUST identify `foo` as an unrecognized parameter.

### Requirement: Prompt Expansion Semantics

Prompt expansion MUST return structured message content (an array of system/user/assistant
messages). Prompt expansion MUST NOT execute actions, mutate server state, or produce side
effects. The runtime MUST treat prompt expansion as a content-retrieval operation.

#### Scenario: Prompt expansion returns message array

- GIVEN a registered prompt `mcp.devtools.prompt.code-review`
- WHEN the runtime executes prompt expansion with valid arguments
- THEN the runtime MUST call the MCP `prompt.get()` method on the originating server
- AND the result MUST contain an array of structured messages
- AND each message MUST include a role (system, user, or assistant) and content.

#### Scenario: Prompt expansion returns empty messages

- GIVEN a registered prompt that returns an empty message array
- WHEN prompt expansion completes
- THEN the runtime MUST return a structured result with an empty message list
- AND the runtime MUST NOT treat empty expansion as an error.

### Requirement: Prompt Operator-Only Approval Model

MCP prompts MUST require explicit operator approval before they can be used. Prompts MUST NOT be
user-triggerable in the initial model. The `"prompts"` capability MUST be explicitly declared in
a server's `capabilities` list — it is never included by default.

#### Scenario: Prompts require explicit capability opt-in

- GIVEN an MCP server config with `capabilities = ["tools"]`
- AND the server advertises prompts during introspection
- WHEN capability registration runs
- THEN the runtime MUST NOT register the advertised prompts
- AND only tools SHALL be registered for that server.

#### Scenario: Prompt invocation requires approval

- GIVEN a registered MCP prompt
- AND no explicit allow policy exists for that prompt
- WHEN the dispatcher evaluates security policy
- THEN the invocation MUST be denied or routed to approval
- AND execution MUST only continue after policy/approval allows it.

#### Scenario: Prompt policy default is ApprovalRequired

- GIVEN a registered MCP prompt invocation
- WHEN the dispatcher classifies the risk level
- THEN the prompt MUST be classified as `ApprovalRequired`
- AND the prompt MUST NOT be auto-allowed even if resource reads on the same server are allowed.

### Requirement: Prompt Injection Mitigation

The runtime MUST apply safeguards against prompt injection from MCP prompt content. Prompt
content returned by MCP servers MUST be treated as untrusted input that could attempt to
override system-level safety instructions or manipulate agent behavior.

#### Scenario: Prompt content is tagged with provenance

- GIVEN a prompt expansion that returns message content from server `devtools`
- WHEN the content is prepared for injection into the LLM conversation
- THEN the runtime MUST tag the content with provenance metadata including source server name
  and fetch timestamp
- AND downstream consumers MUST be able to distinguish MCP prompt content from native system
  prompts.

#### Scenario: Prompt content does not override system safety instructions

- GIVEN prompt content that contains directives attempting to override system-level instructions
- WHEN the content is processed for conversation injection
- THEN the runtime MUST ensure system-level safety instructions take precedence
- AND MCP prompt content MUST NOT be placed in a position that overrides runtime safety
  boundaries.

#### Scenario: Content scanning hook is available

- GIVEN the runtime prompt processing pipeline
- WHEN MCP prompt content is about to be injected into a conversation
- THEN a content scanning hook SHOULD be available for operators to attach validation logic
- AND if the hook rejects the content, the runtime MUST block the prompt injection
- AND the runtime MUST return a structured rejection result.

### Requirement: Prompt Timeout and Output Limit Enforcement

Prompt expansion MUST be bounded by the server's configured timeout and output limits. The same
enforcement patterns used for tool calls and resource reads MUST apply to prompt expansion.

#### Scenario: Prompt expansion exceeds timeout

- GIVEN a prompt expansion request that exceeds the configured timeout
- WHEN the timeout budget is reached
- THEN the runtime MUST cancel or abort the in-flight prompt expansion
- AND the runtime MUST return a timeout failure to the caller
- AND the agent loop MUST NOT hang.

#### Scenario: Prompt expansion exceeds output limit

- GIVEN a prompt expansion whose response exceeds the configured `output_limit_bytes`
- WHEN output processing reaches the limit
- THEN the runtime MUST truncate or fail the expansion per configured policy
- AND the returned result MUST indicate that output limits were enforced.

### Requirement: Prompt Failure Isolation

A prompt expansion failure on one MCP server MUST NOT affect prompt, resource, or tool operations
on other servers. Prompt failures MUST be isolated per-server and per-capability.

#### Scenario: Prompt failure on one server does not crash others

- GIVEN multiple MCP servers with prompts enabled
- AND one server's prompt expansion fails with a transport error
- WHEN the failure is handled
- THEN the runtime MUST return a structured failure result for that prompt
- AND capabilities on other servers MUST remain operational
- AND the agent loop MUST NOT crash or deadlock.

#### Scenario: Prompt failure diagnostics are redacted

- GIVEN a prompt expansion that fails with an error containing sensitive values
- WHEN the failure is logged or surfaced to the operator
- THEN the runtime MUST redact secret values in the diagnostic output
- AND the diagnostic MUST include the prompt identifier and server name.

---

### Requirement: Entry-Point Parity for All Capability Types

All MCP capability types (tools, resources, prompts) MUST be available through the same set of
canonical runtime entry points: CLI, channels, and dispatcher-backed gateway `/webhook`. No
canonical entry point MAY offer a different subset of registered capabilities than another.

#### Scenario: Resources are available via CLI, channels, and gateway

- GIVEN registered MCP resources
- WHEN equivalent resource read requests arrive via CLI, channels, and dispatcher-backed gateway
- THEN all three entry points MUST have access to the same set of registered resources
- AND policy enforcement MUST be equivalent across all entry points.

#### Scenario: Prompts are available via CLI, channels, and gateway

- GIVEN registered MCP prompts
- WHEN equivalent prompt expansion requests arrive via CLI, channels, and dispatcher-backed
  gateway
- THEN all three entry points MUST have access to the same set of registered prompts
- AND approval enforcement MUST be equivalent across all entry points.

#### Scenario: Fallback gateway path does not claim resource or prompt parity

- GIVEN gateway `/webhook` processed through the legacy compatibility fallback path
- WHEN the request would otherwise rely on MCP resource or prompt capabilities
- THEN the system MUST treat resource and prompt parity as inactive for that request
- AND the request MUST NOT be represented as having access to MCP resources or prompts.

### Requirement: Backward Compatibility Guarantees

All changes introduced by this capability model MUST be additive and backward compatible.
Existing MCP tool-only configurations MUST continue to function identically without modification.

#### Scenario: Existing tool-only config works unchanged

- GIVEN an MCP server config that predates this capability model (no `capabilities` field)
- WHEN the runtime loads and processes the configuration
- THEN tool discovery and registration MUST behave identically to v1
- AND no resources or prompts SHALL be discovered or registered
- AND no configuration errors SHALL be raised.

#### Scenario: Tool discovery behavior is unaffected

- GIVEN an MCP server with `capabilities = ["tools"]` (explicit or defaulted)
- WHEN runtime initialization executes tool discovery
- THEN `list_tools()` behavior MUST be identical to v1
- AND tool naming, policy, timeouts, and output limits MUST be unchanged.

#### Scenario: Adding capabilities does not break tool registration

- GIVEN an MCP server previously configured for tools-only
- AND the operator adds `capabilities = ["tools", "resources"]`
- WHEN runtime initialization runs
- THEN all previously registered tools MUST still be registered with identical identifiers
- AND resources MUST be registered additionally without affecting tool registrations.

### Requirement: Diagnostic Redaction for All Capability Types

All diagnostics, logs, and error messages produced during resource and prompt discovery,
registration, execution, and failure handling MUST apply the same secret redaction rules
established for MCP tools in the v1 spec.

#### Scenario: Resource discovery diagnostic redacts secrets

- GIVEN an MCP server with environment secrets configured
- AND resource discovery fails with an error
- WHEN the diagnostic is emitted
- THEN secret values from the server's environment MUST be redacted
- AND the diagnostic MUST include the server name and capability type.

#### Scenario: Prompt expansion diagnostic redacts secrets

- GIVEN an MCP server with environment secrets configured
- AND prompt expansion fails with an error
- WHEN the diagnostic is emitted
- THEN secret values from the server's environment MUST be redacted
- AND the diagnostic MUST include the server name and prompt identifier.

#### Scenario: Prompt content in diagnostics is redacted when sensitive

- GIVEN a prompt expansion that returns content containing values matching known secret patterns
- WHEN the result is logged or surfaced in diagnostics
- THEN the runtime MUST redact values that match secret reference patterns
- AND raw prompt content MUST NOT appear in logs when it contains sensitive values.
