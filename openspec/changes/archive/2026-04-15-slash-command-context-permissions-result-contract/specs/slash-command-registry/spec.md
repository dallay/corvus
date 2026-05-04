# Delta for slash-command-registry

## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Transport Parity for Recognized Slash Commands

The system MUST preserve transport parity for slash command recognition and dispatch across the canonical runtime entry points that rely on the shared ingress seam.

CLI/direct runtime entry, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and channel-backed ingress MUST all classify recognized slash commands through the same pre-execution seam and central registry contract. Surface-specific caller identity semantics MAY differ, and those differences MUST remain explicit in the typed execution context rather than being normalized away. Transport-specific response-envelope formatting MUST remain outside this change.

#### Scenario: Multiple transports share dispatch while preserving distinct identity semantics

- GIVEN the same recognized slash command is submitted through two supported ingress transports with different caller identity semantics
- WHEN each transport reaches pre-execution ingress evaluation
- THEN each transport MUST resolve and dispatch the command through the same central registry contract
- AND each resulting internal execution context MUST preserve that transport's own caller-scope semantics
- AND the system MUST NOT collapse those semantics into a single fake identity model for parity.

#### Scenario: Transport parity does not force envelope parity in this slice

- GIVEN two supported transports already return different external response-envelope shapes for recognized slash commands
- WHEN the internal slash command contract is upgraded for typed context and non-lossy outcomes
- THEN the transports MAY continue using their existing external envelope shapes for this slice
- AND the internal contract MUST remain compatible with later transport-envelope work without requiring it now.
