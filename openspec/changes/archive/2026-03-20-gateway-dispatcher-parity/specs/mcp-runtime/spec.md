# Delta for MCP Runtime Tooling

## MODIFIED Requirements

### Requirement: MCP Policy and Approval Enforcement

MCP tool invocations MUST be treated as explicit risk-bearing operations and MUST pass through the
same policy and approval semantics across canonical runtime entry points: CLI, channels, and
gateway `/webhook` when `/webhook` executes on the dispatcher-backed runtime path. No canonical
entry point MAY bypass MCP approval gates. If gateway `/webhook` is routed through the legacy
compatibility fallback instead of the canonical dispatcher path, MCP parity MUST be considered
inactive for that request rather than partially enforced.

(Previously: MCP tool invocations MUST be treated as explicit risk-bearing operations and MUST pass
through the same policy and approval semantics across canonical runtime entry points (CLI and
channels). Gateway webhook is currently out of scope for MCP dispatch parity until it migrates from
`Provider::simple_chat()` to the canonical dispatcher path.)

#### Scenario: Entry-point parity now includes dispatcher-backed webhook

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

## ADDED Requirements

### Requirement: Gateway Webhook MCP Capability Parity

When gateway `/webhook` executes through the dispatcher-backed runtime and MCP is enabled, the
system MUST expose the same registered MCP tool set, namespaced tool identities, and policy-visible
metadata that an equivalent canonical turn would receive through CLI or channels. Gateway response
shaping MAY change how results are delivered over HTTP, but it MUST NOT change which MCP tools are
available to the dispatcher for that turn.

#### Scenario: Dispatcher-backed webhook receives canonical MCP tools

- GIVEN MCP servers are enabled and successfully registered at startup
- AND a gateway `/webhook` request is executed through the dispatcher-backed runtime path
- WHEN the dispatcher prepares the tool registry for that turn
- THEN the turn MUST receive the same MCP tool availability and canonical namespaced identifiers as
  an equivalent CLI or channel turn
- AND policy evaluation MUST see the same MCP source metadata.

#### Scenario: HTTP response mapping does not alter MCP execution semantics

- GIVEN a dispatcher-backed gateway `/webhook` turn invokes an MCP tool
- WHEN the gateway maps the completed turn into an HTTP response
- THEN the MCP invocation outcome MUST preserve the canonical success, denial, timeout, or failure
  result
- AND transport formatting MUST NOT change the underlying MCP execution semantics.
