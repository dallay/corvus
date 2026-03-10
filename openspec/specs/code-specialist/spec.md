# Code Specialist Specification

## Purpose

This specification defines the MVP behavior for Corvus code-specialist sessions. It covers
explicit code-mode entry, the structured code-session output contract, delegated code-session
execution, security and approval parity, and observability and validation expectations.

This specification extends the existing canonical agent loop rather than introducing a separate
runtime. Code-specialist sessions MUST continue to respect the loop and MCP invariants defined by
`openspec/specs/agent-loop/spec.md` and `openspec/specs/mcp-runtime/spec.md`.

## Requirements

### Requirement: Explicit Code-Mode Entry

The system MUST provide an explicit runtime entry for code-specialist sessions. Entering code mode
MUST use the existing canonical runtime stack with the `code` capability profile or an equivalent
configuration outcome, and the runtime MUST make that specialized mode visible to the caller.

#### Scenario: User starts an explicit code session

- GIVEN a user invokes Corvus through an entry point that supports the canonical dispatcher
- WHEN the user selects or invokes explicit code mode
- THEN the system MUST start a code-specialist session using the canonical bootstrap, prompt,
  approval, and dispatcher flow
- AND the session MUST use coding-scoped tools and limits consistent with the `code` profile
- AND the runtime MUST indicate that the session is running in code mode.

#### Scenario: Existing generic entry remains outside code mode

- GIVEN a user invokes a generic agent entry without selecting explicit code mode
- WHEN the session starts
- THEN the system MUST NOT silently upgrade that session into code-specialist mode
- AND any code-specialist behavior MUST only apply when explicitly selected or configured.

### Requirement: Structured Code-Session Output Contract

Every completed code-specialist session MUST return a structured final result that is consumable by
both humans and machines. The result MUST include final status, task summary, changed-file summary,
commands executed, validations attempted, and any blockers or follow-up work.

#### Scenario: Successful code session returns structured result

- GIVEN a code-specialist session that reaches a normal completion state
- WHEN the runtime emits the final result
- THEN the result MUST include a machine-readable final status
- AND the result MUST include a human-readable summary of the work performed
- AND the result MUST report files changed or explicitly report that no files were changed
- AND the result MUST report commands executed and validations attempted during the session.

#### Scenario: Blocked or partial session returns structured gaps

- GIVEN a code-specialist session that cannot fully complete because of an error, denial, or unmet
  prerequisite
- WHEN the runtime emits the final result
- THEN the result MUST include a non-success status
- AND the result MUST describe the blocker or incomplete work
- AND the result MUST preserve any completed changes, commands, or validations already performed.

### Requirement: Delegated Code-Session Execution

The system MUST support delegated code-specialist work as a bounded session executed through the
canonical agent loop rather than as a single provider response. Delegated code sessions MUST accept
explicit scope controls, including iteration and time bounds, and MUST return the same structured
code-session result contract as direct code mode.

#### Scenario: Parent agent delegates bounded code work

- GIVEN a parent agent operating on a canonical runtime entry point
- WHEN it delegates a task to a code specialist with an allowed delegated code-session policy
- THEN the system MUST create a bounded delegated code session using the canonical loop
- AND the delegated session MUST respect its configured iteration and timeout limits
- AND the delegated session MUST return a structured code-session result to the parent agent.

#### Scenario: Delegated session exceeds its budget

- GIVEN a delegated code-specialist session with explicit iteration or time limits
- WHEN the delegated session exceeds one of those limits before completing
- THEN the system MUST terminate the delegated session safely
- AND the final result MUST indicate that execution stopped because a configured budget was reached
- AND the parent agent MUST receive a structured non-success result rather than an unbounded retry.

### Requirement: Security and Approval Parity

Code-specialist sessions, including delegated code sessions, MUST preserve the same workspace,
policy, and approval protections as the canonical agent loop. Code mode MUST NOT bypass existing
path restrictions, command controls, secret redaction, MCP approval requirements, or explicit
approval gates for risky actions.

#### Scenario: Delegated code session requests a high-risk action

- GIVEN a delegated code-specialist session attempts an action classified as high-risk or outside
  the session's allowed policy
- WHEN policy and approval evaluation runs
- THEN the system MUST apply the same risk classification and approval rules used for direct
  canonical sessions
- AND the action MUST be blocked or routed for explicit approval before execution
- AND the delegated path MUST NOT bypass workspace-only or MCP fail-closed protections.

#### Scenario: Session attempts access outside allowed workspace

- GIVEN a code-specialist session attempts to read, write, or execute against a location outside
  the allowed workspace scope
- WHEN the request is evaluated
- THEN the system MUST deny the action unless an existing approved policy explicitly allows it
- AND the denial MUST be represented in session output or audit data without exposing secrets.

### Requirement: Observability and Validation Reporting

The system MUST record code-session observability and validation outcomes as structured runtime
artifacts or events. For MVP, the runtime MUST capture enough data to audit files changed,
commands executed, validations attempted, final status, and notable failures for both direct and
delegated code-specialist sessions.

#### Scenario: Successful session emits audit-ready telemetry

- GIVEN a code-specialist session completes after performing code changes and verification work
- WHEN observability data is recorded
- THEN the system MUST persist or emit structured records for the final status, files changed,
  commands executed, and validations attempted
- AND those records MUST be attributable to the specific direct or delegated session.

#### Scenario: Validation cannot run or fails

- GIVEN a code-specialist session is configured to attempt validation that fails or cannot be run
- WHEN the session completes
- THEN the final structured result MUST record the attempted or skipped validation outcome
- AND observability data MUST reflect that failure or omission
- AND the session MAY still complete with a non-success or partial status rather than pretending
  validation succeeded.
