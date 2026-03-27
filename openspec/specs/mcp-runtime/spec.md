# MCP Runtime Tooling Specification

## Purpose

This delta specification defines secure v1 Model Context Protocol (MCP) integration in the Corvus
agent runtime. It covers startup-time MCP server discovery, tool registration into the existing
tool pipeline, policy and approval enforcement, bounded execution, and failure behavior.

The v1 scope covered config-defined MCP tools (stdio transport). Resources and prompts are defined
in the MCP Platform Capabilities delta spec
(`openspec/changes/2026-03-27-mcp-platform-capabilities/specs/mcp-platform-capabilities/spec.md`).
Hot reload behavior remains excluded.

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
same policy and approval semantics across canonical runtime entry points: CLI, channels, and
gateway `/webhook` when `/webhook` executes on the dispatcher-backed runtime path. No canonical
entry point MAY bypass MCP approval gates. If gateway `/webhook` is routed through the legacy
compatibility fallback instead of the canonical dispatcher path, MCP parity MUST be considered
inactive for that request rather than partially enforced.

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

- GIVEN equivalent MCP tool calls arrive via CLI, channels, and dispatcher-backed gateway
  `/webhook`
- WHEN policy and approval checks are applied
- THEN all three entry points MUST enforce equivalent MCP risk and approval decisions
- AND no entry point MAY bypass MCP approval gates.

#### Scenario: Fallback request does not claim MCP parity

- GIVEN gateway `/webhook` is processed through the legacy compatibility fallback path
- WHEN the request would otherwise rely on dispatcher-backed MCP parity
- THEN the system MUST treat MCP parity as inactive for that request
- AND the request MUST NOT be represented as canonical MCP-dispatch behavior.

### Requirement: Gateway Webhook MCP Capability Parity

When gateway `/webhook` executes through the dispatcher-backed runtime and MCP is enabled, the
system MUST expose the same registered MCP tool set, namespaced tool identities, and policy-visible
metadata that an equivalent canonical turn would receive through CLI or channels. Gateway response
shaping MAY change how results are delivered over HTTP, but it MUST NOT change which MCP tools are
available to the dispatcher for that turn.

For MCP `/webhook` response-mapping proof, the system MUST distinguish runtime-reachable outcomes
from outcomes that current dispatcher policy blocks before MCP execution begins. A runtime-reachable
MCP `/webhook` outcome MUST be proven end to end through the dispatcher-backed HTTP path. If
current dispatcher policy blocks live MCP execution for a non-denial outcome before tool execution,
the system MAY satisfy proof for that non-denial outcome at the gateway response-mapping seam, but
only if that seam proof preserves the same canonical completed, timeout, or failure result that the
runtime produces below the gateway boundary. Seam-only proof MUST NOT be represented as end-to-end
dispatcher-backed MCP execution proof.

#### Scenario: Runtime-reachable MCP denial is proven end to end

- GIVEN a dispatcher-backed gateway `/webhook` turn invokes an MCP tool
- AND current dispatcher policy denies that MCP invocation before execution
- WHEN the gateway returns the HTTP response
- THEN the denial outcome MUST be proven through the end-to-end dispatcher-backed `/webhook` path
- AND the HTTP response MUST preserve the canonical denied result without reinterpretation.

#### Scenario: Non-denial MCP outcome may be proven at the mapping seam when execution is blocked

- GIVEN a dispatcher-backed gateway `/webhook` turn references an MCP tool
- AND current dispatcher policy prevents live MCP execution before a completed, timeout, or failure
  outcome can be reached through `/webhook`
- WHEN the system proves how gateway HTTP mapping handles that non-denial canonical outcome
- THEN the proof MAY be satisfied at the gateway response-mapping seam instead of full dispatcher
  execution
- AND the proof MUST preserve the same canonical completed, timeout, or failure result already
  produced below the gateway boundary
- AND the proof MUST be identified as seam-level rather than end-to-end runtime evidence.

#### Scenario: Future reachable non-denial MCP outcome requires end-to-end proof

- GIVEN dispatcher-backed gateway `/webhook` later allows a completed, timeout, or failure MCP
  outcome to be reached through live execution
- WHEN that non-denial MCP outcome becomes runtime-reachable
- THEN the system MUST prove that outcome through the end-to-end dispatcher-backed `/webhook` path
- AND seam-only mapping evidence MUST no longer be treated as sufficient by itself for that
  reachable outcome.

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

#### Scenario: Capabilities not listed in server config are ignored

- GIVEN an MCP server whose `capabilities` config does not include a given capability type
- AND the server advertises that capability during introspection
- WHEN capability registration runs
- THEN the runtime MUST ignore the undeclared capability advertisements
- AND only capabilities listed in the server's `capabilities` config SHALL be registered.

> **Cross-reference**: Per-server capability gating, resource/prompt discovery, and the full
> three-tier capability model are defined in the MCP Platform Capabilities delta spec at
> `openspec/changes/2026-03-27-mcp-platform-capabilities/specs/mcp-platform-capabilities/spec.md`.
