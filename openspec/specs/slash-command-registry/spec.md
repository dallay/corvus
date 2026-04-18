# Slash Command Registry Specification

## Purpose

This specification defines the runtime contract for a central slash command registry in Corvus. It
establishes the user-visible and system-level requirements for slash command metadata, deterministic
lookup, centralized dispatch, transport parity, preservation of current slash-session behavior, and
strict separation between registry-core abstractions and backend-specific authorization or
persistence rules.

## Requirements

### Requirement: Command Descriptor Metadata Contract

The system MUST provide a central slash command descriptor model for every registered slash command.

Each descriptor MUST define, at minimum:
- one canonical command name;
- zero or more aliases;
- a user-visible description;
- argument-shape metadata suitable for deterministic parsing and help surfaces; and
- typed command-level capability, permission, and backend requirement metadata that describes requirements without enforcing backend policy inside the registry core.

The registry MUST expose descriptor metadata consistently for lookup and dispatch without requiring transport-specific or backend-specific knowledge.

(Previously: descriptors were required to expose command-level capability or permission metadata, but the spec did not require those requirements to be typed or to cover backend requirements explicitly.)

#### Scenario: Registered command exposes typed descriptor metadata

- GIVEN a slash command is registered with the runtime registry
- WHEN another runtime component requests that command's descriptor
- THEN the system MUST return the canonical name, aliases, description, and argument-shape metadata
- AND the system MUST expose typed capability, permission, and backend requirement metadata attached to that descriptor.

#### Scenario: Descriptor contract does not absorb transport identity rules

- GIVEN two ingress surfaces require different caller identity semantics for the same command
- WHEN their shared descriptor is registered in the central registry
- THEN the descriptor MUST remain transport-neutral descriptive metadata
- AND it MUST NOT encode gateway-only, channel-only, or CLI-only identity derivation rules.

### Requirement: Typed Slash Command Execution Context Contract

The system MUST execute each registered slash command with a typed internal execution context.

That context MUST, at minimum, preserve:
- the current session identity;
- the caller identity in a typed form that can represent authenticated, derived, or unavailable caller scope without collapsing those states into the same value;
- the originating surface or ingress kind;
- plan-mode or equivalent execution-mode state needed by handlers; and
- evaluated capability, permission, and backend facts required for deterministic handler decisions.

The execution context MUST be transport-neutral at the registry boundary while preserving each surface's existing identity semantics for downstream authorization decisions.

#### Scenario: Gateway request preserves authenticated caller facts in typed context

- GIVEN a recognized slash command arrives through a gateway ingress with a verifiable authenticated caller and a resolved session identity
- WHEN the runtime constructs the internal slash command execution context
- THEN the context MUST include the session identity, originating surface kind, and authenticated caller facts in typed fields
- AND the context MUST preserve that the caller was authenticated rather than representing it as an undifferentiated opaque string.

#### Scenario: CLI or channel request preserves non-gateway identity semantics without impersonating a bearer caller

- GIVEN a recognized slash command arrives through CLI or a channel-backed ingress without a gateway bearer token
- WHEN the runtime constructs the internal slash command execution context
- THEN the context MUST preserve the actual caller-scope semantics available for that surface
- AND the context MUST NOT relabel a derived channel scope or CLI scope as an authenticated gateway identity
- AND the context MUST keep the distinction between derived scope and unavailable scope observable to handlers.

### Requirement: Typed Command Requirement Metadata

The system MUST expose typed command requirement metadata for every registered slash command descriptor.

Descriptor metadata MUST represent capability requirements, permission requirements, and backend requirements as typed declarations rather than free-form descriptive tags. These typed declarations MUST remain descriptive at registry level and MUST NOT move backend policy, session ownership policy, or transport-specific authorization rules into registry-core.

#### Scenario: Descriptor exposes typed requirement declarations

- GIVEN a registered slash command that depends on resumable session state and caller authorization
- WHEN another runtime component retrieves that command's descriptor
- THEN the descriptor MUST expose typed requirement declarations for the relevant capability, permission, and backend needs
- AND the descriptor MUST NOT require downstream consumers to parse free-form strings to determine those requirements.

#### Scenario: Typed metadata remains descriptive rather than authoritative

- GIVEN a descriptor declares a backend or permission requirement
- WHEN the registry resolves and dispatches the command
- THEN the registry MUST expose that requirement metadata consistently
- AND the handler or service layer MUST remain responsible for evaluating whether the current context satisfies it
- AND the registry MUST NOT grant or deny execution solely because the metadata exists.

### Requirement: Non-Lossy Internal Slash Command Outcome Contract

The system MUST preserve a non-lossy internal outcome contract across the shared slash command dispatch seam.

The internal command result MUST preserve machine-readable success and error kinds, command-specific payload data, and sanitized user-facing messaging as separate concerns. Internal errors and denials MUST remain distinguishable from one another at the dispatch boundary, including at least authorization-sensitive failures, unsupported-backend failures, unknown-session failures, invalid-target failures, and internal execution failures.

#### Scenario: Authorization denial remains machine-readable at the internal seam

- GIVEN a slash command is denied because the current caller scope is not authorized for the target session
- WHEN the shared slash command dispatch seam produces the internal command outcome
- THEN the outcome MUST preserve a machine-readable authorization-denied kind
- AND it MUST keep any user-facing message separate from the internal denial classification
- AND it MUST NOT flatten that denial into a generic boolean success or failure field as the only observable result.

#### Scenario: Internal outcome richness is preserved while transport envelope shaping stays external

- GIVEN a slash command handler returns a command-specific success or error outcome
- WHEN the result leaves the internal slash command dispatch seam
- THEN the internal outcome MUST still preserve its machine-readable kind and associated data
- AND any HTTP, SSE, CLI, webhook, or channel response-envelope formatting MUST remain outside the scope of this internal contract
- AND this change MUST NOT require transport surfaces to adopt a single shared envelope format.

### Requirement: Deterministic Canonical Name and Alias Resolution

The system MUST resolve slash commands through one deterministic registry lookup path.

A parsed slash command identifier MUST resolve to exactly one canonical command or to no command.
The registry MUST reject ambiguous registrations, including duplicate canonical names, duplicate
aliases, and aliases that collide with another command's canonical name.

Successful alias lookup MUST resolve to the owning command's canonical identity before dispatch.
Unknown slash-like input MUST fall through without partial or best-effort matching.

#### Scenario: Alias resolves to canonical command deterministically

- GIVEN a command is registered with canonical name `/resume` and alias metadata that includes an alternate supported trigger
- WHEN a runtime ingress path submits that alias for lookup
- THEN the registry MUST resolve the request to the `/resume` canonical command identity
- AND the registry MUST dispatch the `/resume` handler rather than a separate alias-specific path.

#### Scenario: Duplicate registration is rejected before runtime dispatch

- GIVEN one registered command already owns a canonical name or alias
- WHEN another command attempts to register the same canonical name or any colliding alias
- THEN the registry MUST reject the new registration deterministically
- AND the system MUST NOT allow runtime dispatch behavior to depend on registration order or branch order.

### Requirement: Centralized Dispatch Through the Pre-Execution Seam

The system MUST route `/resume`, `/suspend`, `/tldr`, and `/compact` through the existing
registry-backed pre-execution ingress seam.

`pre_execution::evaluate_ingress(...)` SHALL remain the canonical production short-circuit seam for
these four session commands. CLI/runtime message fast path, gateway HTTP request paths, gateway
streaming paths, webhook dispatch, and channel-backed ingress MUST submit recognized in-scope
session commands to that shared seam, and production routing MUST NOT depend on transport-local
direct-handler branches for those commands.

#### Scenario: In-scope session commands use the shared seam across supported ingress surfaces

- GIVEN CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming
  `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress each receive
  `/resume`, `/suspend`, `/tldr`, or `/compact`
- WHEN the runtime classifies that recognized command input
- THEN each supported ingress surface MUST route the command through
  `pre_execution::evaluate_ingress(...)`
- AND the handled command dispatch MUST come from the registry-backed ingress path rather than a
  transport-local command execution branch.

#### Scenario: Unknown or non-command input still falls through normally

- GIVEN any supported ingress surface receives non-command input or slash-like input that is not
  `/resume`, `/suspend`, `/tldr`, or `/compact`
- WHEN the runtime evaluates ingress
- THEN the system MUST preserve existing non-command or unknown-command fallthrough behavior
- AND this change MUST NOT introduce new command recognition behavior outside the four in-scope
  session commands.

### Requirement: Shared Handled Slash Outcome Adaptation Contract

The system MUST adapt handled slash command outcomes through one shared internal contract
immediately after `pre_execution::evaluate_ingress(...)`.

For this change slice, CLI/runtime message fast path and gateway streaming `/web/chat/stream`
SHALL preserve the existing handled-command classifications needed by issue #543 without
broadening slash semantics. Recognized slash commands that fail because of authorization denial or
invalid arguments MUST remain handled failures derived from the shared slash-command seam, and
gateway-facing plan mode MUST continue to evaluate recognized slash commands through that seam
before any generic plan-mode turn blocking is applied.

This change MUST freeze existing behavior for current registered slash commands only. It MUST NOT
be interpreted as requiring new slash-command families or a full transport-by-command parity
matrix.

#### Scenario: CLI denied `/resume` stays on the handled-command failure path

- GIVEN CLI/runtime message fast path receives a recognized `/resume {session_id}` command
- AND the typed execution context does not provide caller scope authorized to view or resume that
  target session
- WHEN `pre_execution::evaluate_ingress(...)` handles the command
- THEN the system MUST preserve a handled failure outcome derived from the shared slash-command seam
- AND CLI adaptation MUST surface the existing denied-command error path
- AND the command MUST NOT fall through as normal user input
- AND the target session MUST NOT be resumed.

#### Scenario: Gateway SSE preserves machine-readable denial for recognized `/resume`

- GIVEN gateway streaming `/web/chat/stream` receives a recognized `/resume {session_id}` command
- AND the typed execution context represents a caller scope that is not authorized to view or
  resume that target session
- WHEN `pre_execution::evaluate_ingress(...)` handles the command
- THEN the gateway MUST emit its existing handled-command SSE error wrapper with a machine-readable
  authorization-denied classification
- AND the denial classification MUST come from the shared handled-result contract rather than
  transport-local reclassification
- AND downstream provider execution MUST NOT start.

#### Scenario: Gateway SSE preserves machine-readable invalid-argument failure for a recognized slash command

- GIVEN gateway streaming `/web/chat/stream` receives a recognized slash command whose arguments do
  not satisfy that command's declared argument shape
- WHEN `pre_execution::evaluate_ingress(...)` handles the command
- THEN the gateway MUST emit its existing handled-command SSE error wrapper with a machine-readable
  invalid-argument classification
- AND the failure MUST remain a handled slash-command result rather than unknown-command fallthrough
- AND downstream provider execution MUST NOT start.

#### Scenario: Recognized slash commands still short-circuit on a gateway-facing plan-mode path

- GIVEN a gateway-facing path is operating in `ExecutionMode::Plan`
- AND that path receives a recognized slash command in the current registry-backed session-command
  set
- WHEN ingress is evaluated
- THEN the recognized slash command MUST still be submitted to
  `pre_execution::evaluate_ingress(...)`
- AND any handled outcome MUST be derived from slash-command handling rather than generic plan-mode
  turn blocking
- AND the command MUST NOT be reclassified as ordinary plan-mode-blocked chat input.

### Requirement: Existing Slash Session Behavior Preservation

The system MUST preserve the current behavior of `/resume`, `/suspend`, `/tldr`, and `/compact`
while finalizing their registry-backed routing.

For this change, registry-backed ingress routing MUST preserve the same command semantics already
defined by the sessions specification. Authorization-sensitive `/resume` behavior,
unsupported-backend outcomes for slash-session persistence commands, invalid-target handling,
unknown-session handling, and success results MUST remain equivalent to the current
session-command behavior. Routing cleanup for #542 MUST reorganize ingress and dispatch proof only;
it MUST NOT broaden, weaken, or bypass service-layer authorization or backend checks.

#### Scenario: Resume authorization rules remain intact after registry-backed dispatch

- GIVEN a recognized `/resume` command is routed through `pre_execution::evaluate_ingress(...)`
- AND the current caller scope is not authorized to view or resume the target session under the
  existing sessions rules
- WHEN the registry-backed handler evaluates the command
- THEN the system MUST return the same explicit authorization-denied outcome required by the
  sessions specification
- AND the runtime MUST NOT resume the target session
- AND registry-backed routing MUST NOT bypass or reinterpret that authorization decision.

#### Scenario: Slash-session backend checks remain intact after registry-backed dispatch

- GIVEN a recognized `/suspend`, `/tldr`, `/compact`, or `/resume` command is routed through
  `pre_execution::evaluate_ingress(...)`
- AND the configured backend does not satisfy the existing slash-session persistence requirements
- WHEN the registry-backed handler evaluates the command
- THEN the system MUST return the same explicit unsupported outcome required by the sessions
  specification
- AND the system MUST NOT replace that outcome with fallback success, silent no-op behavior, or
  generic conversational handling.

### Requirement: Registry Bindings Are the Sole Production Session-Command Dispatch Entry

The system MUST treat the registry's built-in bindings for `/resume`, `/suspend`, `/tldr`, and
`/compact` as the only production command-name-to-handler dispatch entry for those commands.

Production runtime surfaces MAY keep transport-specific context construction and outward
response-envelope adaptation, but they MUST NOT keep a separate production routing path that
directly selects or invokes the four session-command handlers outside registry-backed dispatch. Any
remaining compatibility or deprecation scaffolding MUST be isolated so it does not change
production routing for the four in-scope commands.

#### Scenario: Registry binding remains the only production dispatch entry for in-scope session commands

- GIVEN `/resume`, `/suspend`, `/tldr`, and `/compact` are available as built-in session commands
- WHEN production runtime code resolves one of those command names for execution
- THEN the system MUST dispatch through the registry binding for that canonical command
- AND production routing MUST NOT require a separate direct-handler selection path for that command
  name.

#### Scenario: Transport-specific wrappers stay outside the production dispatch decision

- GIVEN a supported transport already has its own outward wrapper for handled slash-command results
- WHEN `/resume`, `/suspend`, `/tldr`, or `/compact` completes through registry-backed ingress
  dispatch
- THEN the transport MAY keep its existing outward response wrapper
- AND that wrapper MUST be applied after the shared handled-result classification
- AND the wrapper MUST NOT become a separate production routing path for selecting the command
  handler.

### Requirement: Transport Parity for Recognized Slash Commands

The system MUST preserve transport parity for slash command recognition, dispatch, and handled-result
adaptation across the canonical runtime entry points that rely on the shared ingress seam.

For issue #543, transport parity SHALL be hardened only at the specific regression edges that
remain exposed after existing service-layer and seam coverage: CLI denied `/resume`, gateway SSE
denied `/resume`, gateway SSE invalid-argument handling for a recognized slash command, and one
gateway-facing plan-mode proof for a recognized slash command. This slice MUST rely on the
existing sessions and pre-execution contracts as the behavioral baseline, and it MUST NOT expand
into exhaustive transport-by-command coverage.

#### Scenario: Focused transport-edge hardening relies on existing slash-command baseline

- GIVEN the runtime already has service-layer authorization coverage for `/resume` and seam-level
  coverage for recognized slash-command handling
- WHEN issue #543 transport parity is evaluated
- THEN the change MUST harden only the targeted CLI and gateway-facing regression edges still
  missing from transport coverage
- AND the system MUST NOT require duplicated acceptance scenarios for every registered command on
  every transport.

### Requirement: Registry-Core Separation from Backend and Authorization Policy

The slash command registry core MUST remain a transport-neutral lookup and dispatch abstraction.

The registry core MUST NOT own backend capability decisions, persistence policy, caller identity
resolution, or authorization outcomes. Those responsibilities MUST remain in command handlers and
service layers.

Descriptor metadata MAY declare that a command has capability, backend, or permission requirements,
but registry-core abstractions MUST NOT become the enforcement point for backend-specific storage
rules or surface-specific authorization policy.

#### Scenario: Handler enforces backend-specific requirements after registry dispatch

- GIVEN a registered slash command depends on SQLite-backed session state
- WHEN the registry dispatches that command to its handler
- THEN the handler or its service layer MUST determine whether the backend requirements are satisfied
- AND the registry core MUST remain unchanged regardless of which backend is configured.

#### Scenario: Handler enforces authorization after registry dispatch

- GIVEN a registered slash command has caller-scope restrictions that differ by transport or surface
- WHEN the registry dispatches the command
- THEN the handler or its service layer MUST evaluate the authorization outcome using existing semantics
- AND the registry core MUST NOT grant, deny, or reinterpret authorization based only on registry metadata.
