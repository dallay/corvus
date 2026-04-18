# Delta for Slash Command Registry

## MODIFIED Requirements

### Requirement: Shared Handled Slash Outcome Adaptation Contract

The system MUST adapt handled slash command outcomes through one shared internal contract immediately after `pre_execution::evaluate_ingress(...)`.

For this change slice, CLI/runtime message fast path and gateway streaming `/web/chat/stream` SHALL preserve the existing handled-command classifications needed by issue #543 without broadening slash semantics. Recognized slash commands that fail because of authorization denial or invalid arguments MUST remain handled failures derived from the shared slash-command seam, and gateway-facing plan mode MUST continue to evaluate recognized slash commands through that seam before any generic plan-mode turn blocking is applied.

This change MUST freeze existing behavior for current registered slash commands only. It MUST NOT be interpreted as requiring new slash-command families or a full transport-by-command parity matrix.

(Previously: the requirement preserved shared handled-result adaptation across transports, but it did not explicitly freeze CLI denied `/resume`, gateway SSE invalid-argument handling, or recognized slash-command behavior on a gateway-facing plan-mode path.)

#### Scenario: CLI denied `/resume` stays on the handled-command failure path

- GIVEN CLI/runtime message fast path receives a recognized `/resume {session_id}` command
- AND the typed execution context does not provide caller scope authorized to view or resume that target session
- WHEN `pre_execution::evaluate_ingress(...)` handles the command
- THEN the system MUST preserve a handled failure outcome derived from the shared slash-command seam
- AND CLI adaptation MUST surface the existing denied-command error path
- AND the command MUST NOT fall through as normal user input
- AND the target session MUST NOT be resumed.

#### Scenario: Gateway SSE preserves machine-readable denial for recognized `/resume`

- GIVEN gateway streaming `/web/chat/stream` receives a recognized `/resume {session_id}` command
- AND the typed execution context represents a caller scope that is not authorized to view or resume that target session
- WHEN `pre_execution::evaluate_ingress(...)` handles the command
- THEN the gateway MUST emit its existing handled-command SSE error wrapper with a machine-readable authorization-denied classification
- AND the denial classification MUST come from the shared handled-result contract rather than transport-local reclassification
- AND downstream provider execution MUST NOT start.

#### Scenario: Gateway SSE preserves machine-readable invalid-argument failure for a recognized slash command

- GIVEN gateway streaming `/web/chat/stream` receives a recognized slash command whose arguments do not satisfy that command's declared argument shape
- WHEN `pre_execution::evaluate_ingress(...)` handles the command
- THEN the gateway MUST emit its existing handled-command SSE error wrapper with a machine-readable invalid-argument classification
- AND the failure MUST remain a handled slash-command result rather than unknown-command fallthrough
- AND downstream provider execution MUST NOT start.

#### Scenario: Recognized slash commands still short-circuit on a gateway-facing plan-mode path

- GIVEN a gateway-facing path is operating in `ExecutionMode::Plan`
- AND that path receives a recognized slash command in the current registry-backed session-command set
- WHEN ingress is evaluated
- THEN the recognized slash command MUST still be submitted to `pre_execution::evaluate_ingress(...)`
- AND any handled outcome MUST be derived from slash-command handling rather than generic plan-mode turn blocking
- AND the command MUST NOT be reclassified as ordinary plan-mode-blocked chat input.

### Requirement: Transport Parity for Recognized Slash Commands

The system MUST preserve transport parity for slash command recognition, dispatch, and handled-result adaptation across the canonical runtime entry points that rely on the shared ingress seam.

For issue #543, transport parity SHALL be hardened only at the specific regression edges that remain exposed after existing service-layer and seam coverage: CLI denied `/resume`, gateway SSE denied `/resume`, gateway SSE invalid-argument handling for a recognized slash command, and one gateway-facing plan-mode proof for a recognized slash command. This slice MUST rely on the existing sessions and pre-execution contracts as the behavioral baseline, and it MUST NOT expand into exhaustive transport-by-command coverage.

(Previously: the requirement established parity expectations across canonical transports, but it did not bound this regression-hardening slice away from an exhaustive command-by-transport matrix.)

#### Scenario: Focused transport-edge hardening relies on existing slash-command baseline

- GIVEN the runtime already has service-layer authorization coverage for `/resume` and seam-level coverage for recognized slash-command handling
- WHEN issue #543 transport parity is evaluated
- THEN the change MUST harden only the targeted CLI and gateway-facing regression edges still missing from transport coverage
- AND the system MUST NOT require duplicated acceptance scenarios for every registered command on every transport.
