# Delta for slash-command-registry

## ADDED Requirements

### Requirement: Shared Handled Slash Outcome Adaptation Contract

The system MUST adapt handled slash command outcomes through one shared internal contract immediately after `pre_execution::evaluate_ingress(...)`.

For CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress, that shared contract MUST preserve whether ingress was:
- not handled and allowed to fall through;
- handled with a success outcome;
- handled with a blocking outcome; or
- handled with a failure outcome whose machine-readable failure kind remains observable.

Transport-specific code MUST be limited to constructing the transport-appropriate typed command context before ingress evaluation and wrapping the adapted handled result into that transport's existing external envelope after adaptation. The shared contract MUST NOT require those transports to adopt one shared external payload, event, or text schema.

#### Scenario: Supported transports share one handled-success adaptation boundary

- GIVEN the same recognized slash command is submitted through CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress
- WHEN `pre_execution::evaluate_ingress(...)` handles that command successfully
- THEN each transport MUST consume the same shared handled-result adaptation contract after the pre-execution seam
- AND each transport MUST preserve its current outward envelope shape while wrapping that adapted success.

#### Scenario: Permission-denied failures stay machine-readable across all supported transports

- GIVEN a recognized slash command is denied for authorization reasons through CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress
- WHEN the handled result is adapted after `pre_execution::evaluate_ingress(...)`
- THEN the shared contract MUST preserve a machine-readable authorization-denied failure kind for every transport
- AND transport-specific code MUST derive its outward error wrapper from that shared classified failure instead of reclassifying the denial independently.

#### Scenario: Unknown slash-like input falls through consistently without transport-local recognition branches

- GIVEN slash-like input does not resolve to a registered command in CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress
- WHEN the transport evaluates ingress through `pre_execution::evaluate_ingress(...)`
- THEN the shared handled-result adaptation contract MUST report that the input was not handled
- AND each transport MUST preserve its existing non-command fallthrough behavior
- AND transports MUST NOT require a separate pre-dispatch recognition branch to determine fallthrough.

#### Scenario: Blocking outcomes remain shared internally while outward wrappers stay transport-specific

- GIVEN a recognized slash command produces a blocking outcome through gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, or channel-backed ingress
- WHEN the handled result is adapted after `pre_execution::evaluate_ingress(...)`
- THEN the shared contract MUST preserve that blocking classification distinctly from success and failure
- AND each transport MAY continue rendering that blocking outcome through its current outward JSON, SSE, webhook, or channel text wrapper.

## MODIFIED Requirements

### Requirement: Centralized Dispatch Through the Pre-Execution Seam

The system MUST route recognized slash commands through the existing pre-execution ingress seam.

`pre_execution::evaluate_ingress(...)` SHALL remain the canonical short-circuit seam for recognized slash commands. CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and channel-backed ingress MUST call that seam directly for slash interception instead of using transport-local recognition or command-specific branching before shared ingress evaluation.

Unknown commands or non-command input MUST preserve existing fallthrough behavior into normal prompt handling or transport-specific non-command handling.

(Previously: `pre_execution::evaluate_ingress(...)` was required to be the canonical short-circuit seam for recognized slash commands, but the requirement did not forbid transport-local recognition branches before the shared seam.)

#### Scenario: CLI/runtime fast path uses the shared seam without a transport-local recognition gate

- GIVEN CLI/runtime message fast path receives a recognized slash command input
- WHEN it evaluates ingress
- THEN it MUST call `pre_execution::evaluate_ingress(...)` without requiring a separate transport-local registry recognition pre-check
- AND any handled slash result observed by the CLI/runtime fast path MUST come from the shared post-seam handled-result adaptation contract.

#### Scenario: Supported transports preserve one ingress dispatch path for recognized commands

- GIVEN CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress each receive the same recognized slash command
- WHEN they classify that input
- THEN each transport MUST dispatch the command through `pre_execution::evaluate_ingress(...)`
- AND transports MUST NOT bypass that seam with a transport-specific command execution path for the handled slash case.

### Requirement: Transport Parity for Recognized Slash Commands

The system MUST preserve transport parity for slash command recognition, dispatch, and handled-result adaptation across the canonical runtime entry points that rely on the shared ingress seam.

CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and channel-backed ingress MUST classify recognized slash commands through the same pre-execution seam and MUST adapt handled slash outcomes through the same shared internal handled-result contract. Surface-specific caller identity semantics MAY differ, and those differences MUST remain explicit in the typed execution context rather than being normalized away. Transport-specific response-envelope formatting MUST remain outside this change.

(Previously: transport parity required the supported entry points to share slash recognition and dispatch through the central registry contract, but it did not explicitly require a shared handled-result adaptation path after the seam.)

#### Scenario: Supported transports share dispatch and handled adaptation while keeping caller semantics explicit

- GIVEN the same recognized slash command is submitted through CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress
- WHEN each transport reaches pre-execution ingress evaluation and post-seam handled-result adaptation
- THEN each transport MUST use the same shared dispatch path and the same shared handled-result adaptation contract
- AND each resulting internal execution context MUST preserve that transport's own caller-scope semantics
- AND the system MUST NOT collapse those semantics into a fake unified caller identity model.

#### Scenario: Transport parity remains internal and does not unify outward envelopes

- GIVEN supported transports already expose different external response-envelope shapes for handled slash commands
- WHEN slash transport parity is enforced for the shared seam and handled-result adaptation contract
- THEN those transports MAY continue using their existing JSON, SSE, webhook, CLI text, or channel text wrappers
- AND this change MUST NOT require envelope unification or the introduction of new slash commands.
