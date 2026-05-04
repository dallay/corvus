# Delta for Slash Command Registry

## MODIFIED Requirements

### Requirement: Centralized Dispatch Through the Pre-Execution Seam

The system MUST route `/resume`, `/suspend`, `/tldr`, and `/compact` through the existing registry-backed pre-execution ingress seam.

`pre_execution::evaluate_ingress(...)` SHALL remain the canonical production short-circuit seam for these four session commands. CLI/runtime message fast path, gateway HTTP request paths, gateway streaming paths, webhook dispatch, and channel-backed ingress MUST submit recognized in-scope session commands to that shared seam, and production routing MUST NOT depend on transport-local direct-handler branches for those commands.

(Previously: the spec required recognized slash commands in general to use the shared pre-execution seam, but it did not explicitly close issue #542 by naming the four migrated session commands or by prohibiting lingering production direct-handler routing for them.)

#### Scenario: In-scope session commands use the shared seam across supported ingress surfaces

- GIVEN CLI/runtime message fast path, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatcher execution, and channel-backed ingress each receive `/resume`, `/suspend`, `/tldr`, or `/compact`
- WHEN the runtime classifies that recognized command input
- THEN each supported ingress surface MUST route the command through `pre_execution::evaluate_ingress(...)`
- AND the handled command dispatch MUST come from the registry-backed ingress path rather than a transport-local command execution branch.

#### Scenario: Unknown or non-command input still falls through normally

- GIVEN any supported ingress surface receives non-command input or slash-like input that is not `/resume`, `/suspend`, `/tldr`, or `/compact`
- WHEN the runtime evaluates ingress
- THEN the system MUST preserve existing non-command or unknown-command fallthrough behavior
- AND this change MUST NOT introduce new command recognition behavior outside the four in-scope session commands.

### Requirement: Existing Slash Session Behavior Preservation

The system MUST preserve the current behavior of `/resume`, `/suspend`, `/tldr`, and `/compact` while finalizing their registry-backed routing.

For this change, registry-backed ingress routing MUST preserve the same command semantics already defined by the sessions specification. Authorization-sensitive `/resume` behavior, unsupported-backend outcomes for slash-session persistence commands, invalid-target handling, unknown-session handling, and success results MUST remain equivalent to the current session-command behavior. Routing cleanup for #542 MUST reorganize ingress and dispatch proof only; it MUST NOT broaden, weaken, or bypass service-layer authorization or backend checks.

(Previously: the spec required existing slash session behavior to remain intact during registry adoption, but it did not explicitly tie #542 completion to preservation of authorization and backend validation while removing lingering direct-routing assumptions.)

#### Scenario: Resume authorization rules remain intact after registry-backed dispatch

- GIVEN a recognized `/resume` command is routed through `pre_execution::evaluate_ingress(...)`
- AND the current caller scope is not authorized to view or resume the target session under the existing sessions rules
- WHEN the registry-backed handler evaluates the command
- THEN the system MUST return the same explicit authorization-denied outcome required by the sessions specification
- AND the runtime MUST NOT resume the target session
- AND registry-backed routing MUST NOT bypass or reinterpret that authorization decision.

#### Scenario: Slash-session backend checks remain intact after registry-backed dispatch

- GIVEN a recognized `/suspend`, `/tldr`, `/compact`, or `/resume` command is routed through `pre_execution::evaluate_ingress(...)`
- AND the configured backend does not satisfy the existing slash-session persistence requirements
- WHEN the registry-backed handler evaluates the command
- THEN the system MUST return the same explicit unsupported outcome required by the sessions specification
- AND the system MUST NOT replace that outcome with fallback success, silent no-op behavior, or generic conversational handling.

## ADDED Requirements

### Requirement: Registry Bindings Are the Sole Production Session-Command Dispatch Entry

The system MUST treat the registry's built-in bindings for `/resume`, `/suspend`, `/tldr`, and `/compact` as the only production command-name-to-handler dispatch entry for those commands.

Production runtime surfaces MAY keep transport-specific context construction and outward response-envelope adaptation, but they MUST NOT keep a separate production routing path that directly selects or invokes the four session-command handlers outside registry-backed dispatch. Any remaining compatibility or deprecation scaffolding MUST be isolated so it does not change production routing for the four in-scope commands.

#### Scenario: Registry binding remains the only production dispatch entry for in-scope session commands

- GIVEN `/resume`, `/suspend`, `/tldr`, and `/compact` are available as built-in session commands
- WHEN production runtime code resolves one of those command names for execution
- THEN the system MUST dispatch through the registry binding for that canonical command
- AND production routing MUST NOT require a separate direct-handler selection path for that command name.

#### Scenario: Transport-specific wrappers stay outside the production dispatch decision

- GIVEN a supported transport already has its own outward wrapper for handled slash-command results
- WHEN `/resume`, `/suspend`, `/tldr`, or `/compact` completes through registry-backed ingress dispatch
- THEN the transport MAY keep its existing outward response wrapper
- AND that wrapper MUST be applied after the shared handled-result classification
- AND the wrapper MUST NOT become a separate production routing path for selecting the command handler.
