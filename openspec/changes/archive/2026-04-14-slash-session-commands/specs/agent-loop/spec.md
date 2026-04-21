# Delta for Agent Loop

## ADDED Requirements

### Requirement: Slash Session Command Ingress Classification

The system MUST classify `/resume`, `/suspend`, `/tldr`, and `/compact` as slash session commands at
runtime ingress before autosave, memory enrichment, normal pre-execution evaluation, tool planning,
and model/provider execution.

Recognized slash session commands MUST take precedence over normal prompt handling across the
canonical agent-runtime entry points covered by this change.

#### Scenario: Recognized slash command bypasses normal prompt side effects

- GIVEN a canonical runtime entry point receives the exact user input `/tldr`
- WHEN ingress classification runs
- THEN the system MUST classify the input as a slash session command before autosave, memory enrichment, and normal pre-execution handling
- AND the system MUST route the request to the dedicated slash command handler instead of the normal agent loop.

#### Scenario: Unknown slash-like input falls through to normal prompt handling

- GIVEN a canonical runtime entry point receives the user input `/resume-later`
- WHEN ingress classification runs
- THEN the system MUST NOT classify the input as one of the supported slash session commands
- AND the system MUST preserve existing prompt handling semantics for the request.

#### Scenario: Leading supported slash command wins over conversational interpretation

- GIVEN a canonical runtime entry point receives the user input `/compact please help me later`
- WHEN ingress classification runs
- THEN the system MUST classify the request as `/compact`
- AND any remaining command text MUST be interpreted by the slash command handler rather than by the model.

### Requirement: Deterministic Slash Session Command Handling Path

The system MUST handle the supported slash session commands through a deterministic non-LLM path.

For this slice, classification, validation, persistence, session-state mutation, snapshot lookup,
and user-visible result generation for `/resume`, `/suspend`, `/tldr`, and `/compact` MUST complete
without invoking model inference, tool execution, or generic conversational memory as the source of
truth.

#### Scenario: Supported slash command does not invoke model execution

- GIVEN the runtime receives `/suspend` for a valid active session
- WHEN the command handler processes the request
- THEN the system MUST complete the command without invoking model/provider inference or tool dispatch
- AND the returned result MUST come from deterministic command logic backed by persisted session state.

#### Scenario: Slash command failure remains deterministic

- GIVEN the runtime receives `/resume missing-session`
- WHEN the target session cannot be resolved as a resumable suspended session
- THEN the system MUST return a deterministic command error result
- AND the system MUST NOT fall back to model execution to interpret or repair the request.
