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

The system MUST route recognized slash commands through the existing pre-execution ingress seam.

`pre_execution::evaluate_ingress(...)` SHALL remain the canonical short-circuit seam for recognized
slash commands. CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths,
webhook dispatch, and channel-backed ingress MUST call that seam directly for slash interception
instead of using transport-local recognition or command-specific branching before shared ingress
evaluation.

Unknown commands or non-command input MUST preserve existing fallthrough behavior into normal prompt
handling or transport-specific non-command handling.

#### Scenario: CLI/runtime fast path uses the shared seam without a transport-local recognition gate

- GIVEN CLI/runtime message fast path receives a recognized slash command input
- WHEN it evaluates ingress
- THEN it MUST call `pre_execution::evaluate_ingress(...)` without requiring a separate
  transport-local registry recognition pre-check
- AND any handled slash result observed by the CLI/runtime fast path MUST come from the shared
  post-seam handled-result adaptation contract.

#### Scenario: Supported transports preserve one ingress dispatch path for recognized commands

- GIVEN CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming
  `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress each receive the same
  recognized slash command
- WHEN they classify that input
- THEN each transport MUST dispatch the command through `pre_execution::evaluate_ingress(...)`
- AND no transport MUST bypass that seam with a transport-specific command execution path for the
  handled slash case.

### Requirement: Shared Handled Slash Outcome Adaptation Contract

The system MUST adapt handled slash command outcomes through one shared internal contract
immediately after `pre_execution::evaluate_ingress(...)`.

For CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming
`/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress, that shared contract
MUST preserve whether ingress was:
- not handled and allowed to fall through;
- handled with a success outcome;
- handled with a blocking outcome; or
- handled with a failure outcome whose machine-readable failure kind remains observable.

Transport-specific code MUST be limited to constructing the transport-appropriate typed command
context before ingress evaluation and wrapping the adapted handled result into that transport's
existing external envelope after adaptation. The shared contract MUST NOT require those transports
to adopt one shared external payload, event, or text schema.

#### Scenario: Supported transports share one handled-success adaptation boundary

- GIVEN the same recognized slash command is submitted through CLI/runtime message fast path,
  gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and
  channel-backed ingress
- WHEN `pre_execution::evaluate_ingress(...)` handles that command successfully
- THEN each transport MUST consume the same shared handled-result adaptation contract after the
  pre-execution seam
- AND each transport MUST preserve its current outward envelope shape while wrapping that adapted
  success.

#### Scenario: Permission-denied failures stay machine-readable across all supported transports

- GIVEN a recognized slash command is denied for authorization reasons through CLI/runtime message
  fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher
  execution, and channel-backed ingress
- WHEN the handled result is adapted after `pre_execution::evaluate_ingress(...)`
- THEN the shared contract MUST preserve a machine-readable authorization-denied failure kind for
  every transport
- AND transport-specific code MUST derive its outward error wrapper from that shared classified
  failure instead of reclassifying the denial independently.

#### Scenario: Unknown slash-like input falls through consistently without transport-local recognition branches

- GIVEN slash-like input does not resolve to a registered command in CLI/runtime message fast path,
  gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and
  channel-backed ingress
- WHEN the transport evaluates ingress through `pre_execution::evaluate_ingress(...)`
- THEN the shared handled-result adaptation contract MUST report that the input was not handled
- AND each transport MUST preserve its existing non-command fallthrough behavior
- AND transports MUST NOT require a separate pre-dispatch recognition branch to determine
  fallthrough.

#### Scenario: Blocking outcomes remain shared internally while outward wrappers stay transport-specific

- GIVEN a recognized slash command produces a blocking outcome through gateway HTTP `/webhook`,
  gateway streaming `/web/chat/stream`, webhook dispatcher execution, or channel-backed ingress
- WHEN the handled result is adapted after `pre_execution::evaluate_ingress(...)`
- THEN the shared contract MUST preserve that blocking classification distinctly from success and
  failure
- AND each transport MAY continue rendering that blocking outcome through its current outward JSON,
  SSE, webhook, or channel text wrapper.

### Requirement: Existing Slash Session Behavior Preservation

The system MUST preserve the current user-visible and deterministic behavior of the existing slash
session commands while moving them onto the central registry core.

For this slice, `/resume`, `/suspend`, `/tldr`, and `/compact` MUST remain recognized slash session
commands. Their existing deterministic handling, error behavior, persistence expectations, and
session-state semantics defined by the session and agent-loop specifications MUST remain intact.

Registry adoption MUST reorganize lookup and dispatch only; it MUST NOT weaken or broaden current
session-command behavior.

#### Scenario: Existing slash session commands remain available after registry adoption

- GIVEN a user invokes `/tldr`, `/compact`, `/suspend`, or `/resume`
- WHEN the runtime uses the central slash command registry
- THEN the system MUST recognize those commands through the registry
- AND the user-visible command outcome MUST remain consistent with the existing session specifications.

#### Scenario: Registry migration does not change unsupported-backend behavior

- GIVEN the runtime is configured with a backend that does not satisfy the existing slash-session persistence requirements
- WHEN a user invokes `/tldr`, `/compact`, `/suspend`, or `/resume`
- THEN the system MUST preserve the existing explicit unsupported or authorization outcome for that command
- AND the registry core MUST NOT replace that behavior with fallback success, silent no-op handling, or generic conversational execution.

### Requirement: Transport Parity for Recognized Slash Commands

The system MUST preserve transport parity for slash command recognition, dispatch, and handled-result
adaptation across the canonical runtime entry points that rely on the shared ingress seam.

CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook
dispatch, and channel-backed ingress MUST classify recognized slash commands through the same
pre-execution seam and MUST adapt handled slash outcomes through the same shared internal
handled-result contract. Surface-specific caller identity semantics MAY differ, and those
differences MUST remain explicit in the typed execution context rather than being normalized away.
Transport-specific response-envelope formatting MUST remain outside this change.

#### Scenario: Supported transports share dispatch and handled adaptation while keeping caller semantics explicit

- GIVEN the same recognized slash command is submitted through CLI/runtime message fast path,
  gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and
  channel-backed ingress
- WHEN each transport reaches pre-execution ingress evaluation and post-seam handled-result
  adaptation
- THEN each transport MUST use the same shared dispatch path and the same shared handled-result
  adaptation contract
- AND each resulting internal execution context MUST preserve that transport's own caller-scope
  semantics
- AND the system MUST NOT collapse those semantics into a fake unified caller identity model.

#### Scenario: Transport parity remains internal and does not unify outward envelopes

- GIVEN supported transports already expose different external response-envelope shapes for handled
  slash commands
- WHEN slash transport parity is enforced for the shared seam and handled-result adaptation contract
- THEN those transports MAY continue using their existing JSON, SSE, webhook, CLI text, or channel
  text wrappers
- AND this change MUST NOT require envelope unification or the introduction of new slash commands.

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
