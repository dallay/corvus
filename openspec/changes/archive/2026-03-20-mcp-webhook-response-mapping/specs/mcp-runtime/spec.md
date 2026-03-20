# Delta for mcp-runtime

## MODIFIED Requirements

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

(Previously: When gateway `/webhook` executes through the dispatcher-backed runtime and MCP is
enabled, the system MUST expose the same registered MCP tool set, namespaced tool identities, and
policy-visible metadata that an equivalent canonical turn would receive through CLI or channels.
Gateway response shaping MAY change how results are delivered over HTTP, but it MUST NOT change
which MCP tools are available to the dispatcher for that turn.)

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
