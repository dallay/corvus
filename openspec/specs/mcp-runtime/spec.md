# MCP Runtime Tooling Specification

## Purpose

This delta specification defines secure v1 Model Context Protocol (MCP) integration in the Corvus
agent runtime. It covers startup-time MCP server discovery, tool registration into the existing
tool pipeline, policy and approval enforcement, bounded execution, and failure behavior.

This change is limited to config-defined MCP tools (stdio transport). It explicitly excludes MCP
resources/prompts and hot reload behavior.

## Requirements

### Requirement: MCP Server Configuration Validation

The runtime MUST validate `mcp.servers` configuration at load time using fail-safe defaults.
Malformed, ambiguous, or unsafe server definitions MUST be rejected before runtime initialization
completes.

#### Scenario: Reject malformed server definition

- GIVEN a runtime config containing an MCP server with missing required identity or command fields
- WHEN configuration loading and schema validation runs
- THEN the runtime MUST reject the configuration with a structured validation error
- AND the runtime MUST NOT register tools from that invalid server.

#### Scenario: Reject unsafe timeout and limit values

- GIVEN an MCP server definition with non-positive timeouts or non-positive output limits
- WHEN configuration validation runs
- THEN the runtime MUST reject the definition
- AND the error message MUST identify the invalid field without exposing secret values.

#### Scenario: Secret references are protected in diagnostics

- GIVEN an MCP server environment definition using secret references
- WHEN validation or startup emits diagnostics
- THEN the runtime MUST redact secret values in logs and surfaced errors
- AND the runtime MUST avoid printing raw environment values.

### Requirement: Startup Discovery and Registration

The runtime MUST discover and register MCP tools at startup, integrating them into the existing
tool registry path used by native tools.

#### Scenario: Register MCP tools during startup

- GIVEN one or more enabled MCP server definitions with valid stdio configuration
- WHEN runtime initialization executes tool discovery
- THEN the runtime MUST introspect each enabled server and build `Tool`-compatible registrations
- AND discovered MCP tools MUST be included in the unified dispatchable tool set.

#### Scenario: Bound startup discovery duration

- GIVEN an MCP server that does not respond during startup introspection
- WHEN the configured startup timeout elapses
- THEN the runtime MUST terminate discovery for that server within the timeout budget
- AND startup MUST continue without indefinite blocking.

#### Scenario: Disabled servers are not loaded

- GIVEN an MCP server definition marked disabled
- WHEN runtime startup discovery runs
- THEN the runtime MUST skip server startup and introspection
- AND the server MUST contribute no registered tools.

### Requirement: Namespaced Tool Identity and Collision Handling

The runtime MUST normalize MCP tools to canonical namespaced identifiers and enforce deterministic
collision handling to preserve dispatch correctness and prevent impersonation.

#### Scenario: Canonical MCP tool naming

- GIVEN a discovered tool named `search` from server `docs`
- WHEN the tool is normalized into runtime `ToolSpec`
- THEN the canonical tool identifier MUST be `mcp.docs.search`
- AND source metadata MUST retain server and provider origin for policy and audit decisions.

#### Scenario: Collision with existing tool identity

- GIVEN a discovered MCP tool whose canonical identifier matches another registered tool
- WHEN registry merge runs for native and MCP tools
- THEN the runtime MUST reject the ambiguous registration deterministically
- AND the runtime MUST return an actionable error describing the colliding identifier.

#### Scenario: Reserved namespace protection

- GIVEN a server or tool name that would produce a reserved or invalid identifier
- WHEN normalization runs
- THEN the runtime MUST reject that tool registration
- AND built-in tool identities MUST remain unshadowed.

### Requirement: MCP Policy and Approval Enforcement

MCP tool invocations MUST be treated as explicit risk-bearing operations and MUST pass through the
same policy and approval semantics across all runtime entry points.

#### Scenario: Deny-by-default policy for MCP tools

- GIVEN an MCP tool invocation without an explicit allow policy outcome
- WHEN the dispatcher evaluates security policy
- THEN the invocation MUST be denied or routed to approval rather than executed directly
- AND execution MUST only continue after policy/approval allows it.

#### Scenario: Unknown or high-risk MCP action requires approval

- GIVEN an MCP tool invocation classified as unknown or high-risk
- WHEN approval evaluation runs
- THEN the runtime MUST require explicit approval before execution
- AND if approval is not granted, the call MUST be blocked with a structured denial result.

#### Scenario: Entry-point parity for approval behavior

- GIVEN equivalent MCP tool calls arriving via CLI, gateway, and channel entry points
- WHEN policy and approval checks are applied
- THEN all entry points MUST enforce equivalent MCP risk and approval decisions
- AND no entry point MAY bypass MCP approval gates.

### Requirement: MCP Execution Limits and Timeouts

The runtime MUST enforce bounded execution for MCP startup and tool calls using configured
ceilings for time and output.

#### Scenario: Per-call timeout enforcement

- GIVEN an MCP tool call that exceeds its configured execution timeout
- WHEN the timeout budget is reached
- THEN the runtime MUST cancel or abort the in-flight tool call
- AND the runtime MUST return a timeout failure without hanging the agent loop.

#### Scenario: Output cap enforcement

- GIVEN an MCP tool call that produces output beyond the configured byte or token cap
- WHEN output processing reaches the limit
- THEN the runtime MUST truncate or fail the call per configured policy
- AND the returned result MUST indicate that output limits were enforced.

#### Scenario: Limit enforcement does not affect native tools

- GIVEN native tool execution in a runtime with MCP enabled
- WHEN MCP-specific limits and timeouts are enforced for MCP calls
- THEN existing native tool dispatch behavior MUST remain unchanged
- AND native tool execution MUST continue using its existing controls.

### Requirement: MCP Failure Handling and Safety

The runtime MUST handle MCP startup and invocation failures safely, preserving loop stability,
security posture, and operator diagnostics.

#### Scenario: Startup failure for one server does not crash runtime

- GIVEN multiple MCP server definitions where one server fails startup
- WHEN startup discovery completes
- THEN the runtime MUST isolate the failed server and continue with remaining valid servers
- AND diagnostics MUST report the failure with sensitive values redacted.

#### Scenario: Invocation failure returns structured error

- GIVEN a registered MCP tool that fails during invocation
- WHEN execution returns an error from transport or server
- THEN the runtime MUST return a structured failure result to the agent loop
- AND the runtime MUST NOT crash or deadlock the loop.

#### Scenario: Out-of-scope MCP capabilities are rejected

- GIVEN an MCP server advertising resources or prompts in addition to tools
- WHEN v1 registration runs
- THEN the runtime MUST ignore or reject non-tool capabilities
- AND only MCP tools MAY be registered in this change scope.
