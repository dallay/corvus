# Delta for Slash Command Registry

## ADDED Requirements

### Requirement: Effective Runtime Tool Inventory Listing

The system MUST provide a registry-backed read-only `/tools` slash command that reports the
effective runtime tool inventory available to the current runtime/profile.

The `/tools` command MUST describe the tools that are actually available for execution after
runtime composition, profile gating, and MCP-derived tool discovery are applied. The command MUST
NOT report merely configured-but-inactive tools as available. The shared slash execution boundary
MUST expose only the read-only runtime metadata needed to produce this inventory.

#### Scenario: `/tools` lists the effective active runtime tools

- GIVEN the runtime has a composed active tool inventory for the current profile
- WHEN a caller invokes `/tools`
- THEN the system MUST return a handled read-only slash-command result containing the effective
  active tools for that runtime
- AND the listing MUST reflect tools that are actually available for execution rather than raw
  configured entries.

#### Scenario: `/tools` includes MCP-derived tools only when they are effectively active

- GIVEN the runtime has MCP-derived tools that survive the active runtime composition rules
- WHEN a caller invokes `/tools`
- THEN the system MUST include those MCP-derived tools in the returned inventory
- AND the system MUST NOT include MCP tool entries that are configured but not effectively active
  in the current runtime/profile.

### Requirement: Transport Parity for `/tools` Through the Shared Slash Ingress Seam

The system MUST route recognized `/tools` commands through the same shared slash ingress seam used
by the registry-backed slash-command platform.

CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook
dispatch, and channel-backed ingress that already depend on the shared handled-command seam SHALL
submit recognized `/tools` input through `pre_execution::evaluate_ingress(...)`. Transport-specific
response shaping MAY differ externally, but the handled slash-command classification and underlying
`/tools` inventory semantics MUST remain shared and transport-neutral.

#### Scenario: Recognized `/tools` uses the shared ingress seam across supported transports

- GIVEN a supported ingress surface receives a recognized `/tools` command
- WHEN the runtime evaluates ingress for that request
- THEN the system MUST route `/tools` through `pre_execution::evaluate_ingress(...)`
- AND the handled command outcome MUST come from the shared registry-backed slash-command path
- AND the transport MUST NOT use a transport-local direct-handler branch for `/tools`.

#### Scenario: Transport wrappers do not change the `/tools` inventory meaning

- GIVEN two supported ingress surfaces both handle a recognized `/tools` command successfully
- WHEN each surface adapts the handled slash-command result to its outward response format
- THEN both surfaces MUST preserve the same underlying effective runtime tool inventory semantics
- AND any transport-specific envelope formatting MUST remain outside the core `/tools` command
  contract.

### Requirement: Read-Only Scope Boundary for Initial Tool Slash Commands

This change slice MUST remain limited to the read-only `/tools` command and MUST NOT introduce
mutation command semantics for tool state, MCP server management, or model/provider settings.

The system MUST treat `/tool enable`, `/tool disable`, `/mcp add`, `/mcp remove`, `/model`,
`/provider`, and `/temperature` as out of scope for this change. This slice MUST NOT require slash
handlers, persistence rules, config-mutation services, or new success/error contracts for those
commands.

#### Scenario: Mutation-oriented slash families remain out of scope in this slice

- GIVEN this change is evaluated for slash-command coverage
- WHEN the command set for the slice is enumerated
- THEN the system MUST include `/tools` as the only new in-scope command in this change
- AND the system MUST NOT require support for `/tool enable`, `/tool disable`, `/mcp add`,
  `/mcp remove`, `/model`, `/provider`, or `/temperature`.

#### Scenario: Out-of-scope mutation commands do not gain new handled semantics from this change

- GIVEN a caller submits `/tool enable`, `/tool disable`, `/mcp add`, `/mcp remove`, `/model`,
  `/provider`, or `/temperature`
- WHEN this change slice is implemented in isolation
- THEN this specification MUST NOT require those inputs to produce new mutation behavior
- AND this change MUST NOT introduce persistence or config-write expectations for those commands.
