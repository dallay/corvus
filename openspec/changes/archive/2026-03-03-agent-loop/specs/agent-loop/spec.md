# Agent Loop Specification

## Purpose

This specification defines the canonical Agent Loop behavior for the Corvus project, consolidating
the dual-loop paths (`loop_.rs` and `agent.rs` + `dispatcher.rs`) into a single explicit contract.
It covers the loop lifecycle, tool-dispatch semantics, session scoping, approval invariants, and
security requirements across all entry points (CLI, channels, and gateway).

## Requirements

### Requirement: Entry Points Alignment

The system MUST provide a unified loop contract across all entry points (CLI, channels, gateway
webhook). Any semantic differences MUST be explicitly justified and narrow in scope.

#### Scenario: Unified Loop Execution

- GIVEN a user prompt originating from any supported entry point (CLI, channel, or gateway)
- WHEN the request enters the agent loop
- THEN the system MUST initialize the loop with consistent session invariants, applying the same
  approval and security policies regardless of origin
- AND the system MUST route execution through the canonical dispatcher boundary.

### Requirement: Stream Events Lifecycle

The loop MUST emit predictable stream events during its lifecycle, ensuring callers can accurately
track prompt assembly, tool execution, and final response generation.

#### Scenario: Standard Iteration Events

- GIVEN an active agent loop
- WHEN a tool call is dispatched and completed
- THEN the system MUST emit start, progress, and completion events for the tool execution
- AND the system MUST append the results to the loop's context before the next iteration.

### Requirement: Context Compaction

The system MUST enforce context compaction to protect memory limits and runtime stability when the
loop iteration history grows beyond the configured threshold.

#### Scenario: Triggering Compaction

- GIVEN an agent loop iterating over multiple tool calls
- WHEN the cumulative context size exceeds the predefined safety threshold
- THEN the system MUST trigger a compaction routine to summarize or truncate older history
- AND the system MUST preserve the current `session_id` and essential context required for the
  ongoing task without interruption.

### Requirement: Timeout Aborts

The loop MUST respect per-turn latency and total iteration budgets to prevent runaway execution or
unresponsive loops.

#### Scenario: Runaway Loop Abortion

- GIVEN an active agent loop with a configured iteration budget or timeout limit
- WHEN the loop exceeds the maximum allowed iterations or processing time
- THEN the system MUST forcefully abort the loop
- AND the system MUST emit a timeout error event to the caller
- AND the system MUST safely release associated session resources.

### Requirement: Error Handling and Fallbacks

The system MUST gracefully handle tool execution failures, network timeouts, and model errors
without crashing the agent loop, utilizing retry and backoff discipline.

#### Scenario: Recoverable Tool Failure

- GIVEN a tool call dispatched during an active loop iteration
- WHEN the tool execution fails due to a transient error (e.g., network timeout)
- THEN the system SHOULD attempt to retry the tool call based on configured backoff policies
- AND if the failure persists, the system MUST return a structured error to the model to allow for
  an alternative strategy or graceful degradation.

#### Scenario: Unrecoverable Error

- GIVEN an active agent loop
- WHEN an unrecoverable error occurs (e.g., severe parsing failure or auth rejection)
- THEN the system MUST terminate the loop immediately
- AND the system MUST scrub sensitive values before logging or returning the error to the user.

### Requirement: Security Profiling and Invariants

The loop MUST enforce strict approval, risk classification, and authorization boundaries at every
iteration and tool dispatch phase.

#### Scenario: Tool Dispatch with High-Risk Classification

- GIVEN a tool dispatched by the model that requires elevated privileges
- WHEN the dispatcher intercepts the tool call request
- THEN the system MUST evaluate the action against the current session's risk classification and
  approval policy
- AND the system MUST block the execution and request explicit user approval if the action exceeds
  the permitted risk threshold
- AND the system MUST NOT proceed until explicit authorization is granted or the request is aborted.
