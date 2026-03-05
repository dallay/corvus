# Update System Specification

## Purpose

Define a secure, observable, and deterministic update experience across CLI, conversation channels,
and client-facing admin surfaces.

## Requirements

### Requirement: Multi-Surface Update Visibility

The system MUST expose consistent update availability and update policy state at CLI startup, during
eligible in-conversation interactions, and through client-facing admin/status surfaces.

#### Scenario: CLI startup shows update availability

- GIVEN update checks are enabled and a newer version is available
- WHEN a supported CLI entrypoint starts
- THEN the user is shown an update notice during startup
- AND the notice includes the current version, available version, and actionable next step

#### Scenario: In-conversation mention is policy-gated

- GIVEN update checks are enabled and a conversation channel is active
- WHEN update availability is evaluated during conversation flow
- THEN the system surfaces an update mention only when channel visibility is enabled by policy
- AND the mention uses the same version/status facts as CLI and admin surfaces

#### Scenario: Client/admin surface reflects same status model

- GIVEN the runtime has a computed update status and policy state
- WHEN a client/admin status endpoint is queried
- THEN the response includes update availability, current version, available version, last check
  result, and policy flags
- AND the values are consistent with the latest CLI-visible status

### Requirement: Update Configuration Model and Safe Defaults

The system MUST provide a structured update configuration model with safe defaults, where automatic
checks and notifications are enabled by default and automatic installation is disabled by default.

#### Scenario: Default policy is safe-by-default

- GIVEN no user-specific update configuration is set
- WHEN the runtime resolves effective update policy
- THEN update checks and visibility notifications are enabled
- AND automatic installation is disabled

#### Scenario: Environment override precedence is deterministic

- GIVEN configuration file values and one or more update-related environment overrides are present
- WHEN effective update policy is resolved
- THEN environment overrides take precedence over persisted configuration
- AND only explicitly provided environment keys alter effective values

#### Scenario: Invalid environment override fails safely

- GIVEN an invalid value for an update-related environment override
- WHEN effective policy resolution is attempted
- THEN the system does not apply the invalid override
- AND the system records a validation warning without enabling less-safe behavior

### Requirement: Installation Method Detection and Execution Routing

The system MUST determine an effective installation method (detected or user-overridden), route
update execution through the method-specific strategy, and provide deterministic fallback
instructions when unsupported or unavailable.

#### Scenario: Supported method is detected and used

- GIVEN the runtime can infer a supported installation method for the current installation
- WHEN `update install` is requested
- THEN the system selects that method as the execution strategy
- AND reports the selected method in status/output

#### Scenario: User override takes priority over detection

- GIVEN a valid user-configured installation method override exists
- WHEN install planning is performed
- THEN the system uses the override as the effective method
- AND the chosen method is explicitly surfaced in status/output

#### Scenario: Unsupported method falls back safely

- GIVEN no supported execution strategy is available for the effective installation method
- WHEN installation is requested
- THEN the system does not attempt an unsafe or unknown install path
- AND returns deterministic manual update instructions with non-success status

### Requirement: Process Safety and Atomic Update State

The system MUST prevent concurrent install transactions across processes and MUST persist update
state atomically such that interrupted writes do not produce corrupt state.

#### Scenario: Concurrent install attempts are serialized

- GIVEN two update install requests arrive from different runtime processes
- WHEN both attempt to start an install transaction
- THEN at most one install transaction becomes active
- AND the other request receives a deterministic busy or deferred outcome without corrupting state

#### Scenario: Update state write is interruption-safe

- GIVEN an update state persistence operation is interrupted before completion
- WHEN the system next loads update state
- THEN it reads a valid previous or completed state snapshot
- AND it does not read a partially written state artifact

### Requirement: CLI Update Command Contract

The CLI MUST provide `update status`, `update check`, `update install`, `update auto-enable`,
`update auto-disable`, and `update history` commands with deterministic outputs and exit semantics
suitable for interactive and scripted use.

#### Scenario: `update status` reports effective state

- GIVEN update metadata and effective policy are available
- WHEN the user runs `update status`
- THEN output includes current version, latest known version status, installation method, and
  auto-update policy state
- AND the command returns success when status can be resolved

#### Scenario: `update check` performs explicit refresh

- GIVEN network access to update source is available
- WHEN the user runs `update check`
- THEN the command performs an explicit availability check
- AND output reflects whether an update is available with deterministic success/failure signaling

#### Scenario: `update install` enforces policy and method routing

- GIVEN an update is available
- WHEN the user runs `update install`
- THEN the command evaluates policy and effective installation method before execution
- AND returns a non-success result when prerequisites fail or install cannot proceed safely

#### Scenario: `update auto-enable` and `update auto-disable` toggle policy

- GIVEN a user with permission to modify local runtime configuration
- WHEN the user runs `update auto-enable` or `update auto-disable`
- THEN the effective auto-install policy is updated accordingly
- AND `update status` reflects the new policy state in the same session

#### Scenario: `update history` returns auditable events

- GIVEN one or more prior update checks or install attempts exist
- WHEN the user runs `update history`
- THEN the command returns chronologically ordered update events
- AND each entry includes enough metadata to identify what occurred and outcome class

#### Scenario: `update confirm <nonce>` compatibility

- GIVEN a channel-initiated install provides a nonce via in-conversation flow
- WHEN the user or automated agent runs `update confirm <nonce>`
- THEN the CLI accepts the nonce, completes the install handshake
- AND returns deterministic success/failure semantics
- AND appends an audit entry to history reflecting the nonce-confirmed install
- NOTE: This is an advanced/internal flow; `update status` and `update history` reflect actions from
  nonce-confirmed installs alongside other update events

### Requirement: Integrity Verification and Audit Logging

The system MUST verify artifact integrity before activation for update paths that consume
downloadable artifacts, MUST fail closed on verification failure, and MUST append structured audit
events for update checks and install attempts.

#### Scenario: Successful verification permits activation

- GIVEN an install path that downloads or stages an artifact
- WHEN integrity verification succeeds against trusted release metadata
- THEN the update may proceed to activation
- AND an audit event records verification success and install outcome

#### Scenario: Verification failure blocks activation

- GIVEN an install path that downloads or stages an artifact
- WHEN integrity verification fails or required verification metadata is unavailable
- THEN the update MUST NOT activate the new artifact
- AND the system returns a non-success install result with a recorded audit failure event

#### Scenario: Audit history includes both checks and installs

- GIVEN periodic checks and user-initiated installs occur over time
- WHEN update audit history is queried
- THEN the history contains both check events and install events
- AND each event includes timestamp, action type, effective method, and outcome classification
