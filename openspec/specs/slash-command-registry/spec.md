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
- argument-shape metadata suitable for deterministic parsing and help surfaces;
- command-level capability or permission metadata that describes requirements without enforcing
  backend policy inside the registry core.

The registry MUST expose descriptor metadata consistently for lookup and dispatch without requiring
transport-specific or backend-specific knowledge.

#### Scenario: Registered command exposes complete descriptor metadata

- GIVEN a slash command is registered with the runtime registry
- WHEN another runtime component requests that command's descriptor
- THEN the system MUST return the canonical name, aliases, description, and argument-shape metadata
- AND the system MUST expose any command-level capability or permission metadata attached to that descriptor.

#### Scenario: Registry metadata remains transport- and backend-neutral

- GIVEN a slash command requires backend-specific state or caller-specific authorization checks
- WHEN its descriptor is registered in the central registry
- THEN the registry MUST store only descriptive metadata about those requirements
- AND the registry MUST NOT require the descriptor to encode SQLite-only logic, gateway-only rules, or surface-specific caller identity semantics.

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
slash commands. Once ingress classification identifies a supported slash command, the system MUST
delegate lookup and dispatch to the central registry instead of using command-specific branching in
entry-point code.

Unknown commands or non-command input MUST preserve existing fallthrough behavior into normal prompt
handling.

#### Scenario: Recognized slash command dispatches through the shared seam

- GIVEN a canonical runtime entry point receives a recognized slash command input
- WHEN `pre_execution::evaluate_ingress(...)` evaluates ingress
- THEN the system MUST perform command lookup and dispatch through the central slash command registry
- AND the system MUST short-circuit normal prompt execution for that request.

#### Scenario: Unknown slash-like input preserves normal handling

- GIVEN a canonical runtime entry point receives an input that starts with `/` but does not resolve in the registry
- WHEN `pre_execution::evaluate_ingress(...)` evaluates ingress
- THEN the system MUST preserve existing non-command prompt handling semantics
- AND the registry MUST NOT synthesize or guess a closest command match.

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

The system MUST preserve transport parity for slash command recognition and dispatch across the
canonical runtime entry points that rely on the shared ingress seam.

CLI/direct runtime entry, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and
channel-backed ingress MUST all classify recognized slash commands through the same pre-execution
seam and central registry contract. Surface-specific caller identity semantics MAY differ, but the
registry dispatch path for recognized commands MUST remain equivalent.

#### Scenario: Multiple transports reach the same registry dispatch contract

- GIVEN the same recognized slash command is submitted through two supported ingress transports
- WHEN each transport reaches pre-execution ingress evaluation
- THEN each transport MUST resolve and dispatch the command through the same central registry contract
- AND each transport MUST short-circuit normal prompt execution before model handling.

#### Scenario: Surface-specific identity rules remain outside parity contract

- GIVEN two supported transports provide different caller identity or trust context for the same slash command
- WHEN the command reaches the registry and then its concrete handler
- THEN the system MUST preserve each surface's existing caller-scope and authorization semantics
- AND the registry MUST NOT normalize those differences by embedding transport-specific auth rules in registry-core behavior.

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
