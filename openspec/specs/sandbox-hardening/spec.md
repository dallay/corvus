# Spec: Runtime Sandbox Hardening

**Change:** runtime-sandbox-hardening
**Issue:** #37
**Date:** 2026-04-02
**Type:** Delta spec (security hardening)

## Overview

Harden the Corvus agent-runtime so that OS-level sandbox backends are actually invoked during
user-triggered tool execution, with fail-closed capability, audit visibility, and a documented
isolation contract for sidecar-assisted actions.

## Requirements

### R1: Sandbox Wiring into Shell Execution

`ShellTool` must invoke `wrap_command()` on the injected `Arc<dyn Sandbox>` for every shell command
execution, after application-layer policy validation passes and before the process is spawned.

**Rationale:** The Sandbox trait and four backends exist but are never called. Defense in depth
requires OS-level isolation to complement application-layer policy.

#### Scenarios

**R1-S1: Sandbox wraps every shell command**

- Given a `ShellTool` with an injected sandbox backend
- When a shell command passes policy validation
- Then `wrap_command()` is called on the sandbox before `cmd.output()`
- And the command executes with the sandbox-modified `Command`

**R1-S2: Sandbox error blocks execution**

- Given a `ShellTool` with a sandbox whose `wrap_command()` returns `Err`
- When a shell command is executed
- Then the command is NOT spawned
- And `ToolResult.success` is `false`
- And `ToolResult.error` contains the sandbox error message

**R1-S3: NoopSandbox passes through unchanged**

- Given a `ShellTool` with `NoopSandbox`
- When a shell command is executed
- Then `wrap_command()` is called (returns `Ok(())`)
- And the command executes identically to pre-change behavior

**R1-S4: Sandbox wrapping happens after env sanitization**

- Given a `ShellTool` with any sandbox backend
- When a shell command is executed
- Then the execution order is: policy validation → env_clear + safe vars → `wrap_command()` → output

### R2: Fail-Closed Mode

New `sandbox.require` config option (bool, default `false`). When `true`, `create_sandbox()` must
return an error if no OS-level backend is available. The runtime must refuse to proceed with tool
registration if the sandbox requirement cannot be met.

**Rationale:** Operators must be able to mandate OS-level isolation and get a clear failure rather
than silent degradation to NoopSandbox.

#### Scenarios

**R2-S1: Require mode with available backend succeeds**

- Given `sandbox.require = true` and `sandbox.backend = docker`
- When Docker is available on the system
- Then `create_sandbox()` returns `Ok(DockerSandbox)`

**R2-S2: Require mode with unavailable explicit backend fails**

- Given `sandbox.require = true` and `sandbox.backend = landlock`
- When running on macOS (Landlock unavailable)
- Then `create_sandbox()` returns `Err`
- And the error message identifies the unavailable backend

**R2-S3: Require mode with auto finds nothing fails**

- Given `sandbox.require = true` and `sandbox.backend = auto`
- When no OS-level backend is available
- Then `create_sandbox()` returns `Err`
- And the error message indicates no sandbox backend was found

**R2-S4: Non-require mode falls back to NoopSandbox**

- Given `sandbox.require = false` (default)
- When no OS-level backend is available
- Then `create_sandbox()` returns `Ok(NoopSandbox)`
- And existing behavior is preserved

**R2-S5: Explicit none backend with require is an error**

- Given `sandbox.require = true` and `sandbox.backend = none`
- Then `create_sandbox()` returns `Err`
- And the error indicates contradiction (require + none)

**R2-S6: Disabled sandbox with require is an error**

- Given `sandbox.require = true` and `sandbox.enabled = false`
- Then `create_sandbox()` returns `Err`

### R3: NoopSandbox Warning

When `NoopSandbox` is the active backend and a non-read-only command executes, emit a structured
warning log. Read-only commands do not trigger the warning.

**Rationale:** Operators running without OS-level isolation should have clear visibility that their
commands are not sandboxed, without flooding logs for harmless read operations.

#### Scenarios

**R3-S1: Warning for mutating command with NoopSandbox**

- Given `NoopSandbox` is active (name = "none")
- When a command with risk level Medium or High is executed
- Then a `tracing::warn!` is emitted containing "OS-level sandbox not active"

**R3-S2: No warning for read-only command with NoopSandbox**

- Given `NoopSandbox` is active
- When a command with risk level Low is executed (e.g., `ls`, `git status`)
- Then no sandbox warning is emitted

**R3-S3: No warning when real sandbox is active**

- Given a real sandbox backend is active (e.g., Firejail)
- When any command is executed
- Then no NoopSandbox warning is emitted

### R4: Audit Propagation

Every `CommandExecution` audit event must include the actual sandbox backend name in
`security.sandbox_backend`. This field must never be `None` when a command was executed with audit
logging enabled.

**Rationale:** Security auditing requires knowing which isolation was in effect for each action.

#### Scenarios

**R4-S1: Audit event includes sandbox backend name**

- Given audit logging is enabled and a sandbox backend is active
- When a shell command is executed
- Then the `AuditEvent.security.sandbox_backend` equals the sandbox's `name()` value
- And the field is `Some(...)`, never `None`

**R4-S2: NoopSandbox is recorded as "none"**

- Given `NoopSandbox` is active
- When a shell command is executed
- Then `sandbox_backend` is `Some("none")`

**R4-S3: Real backend name is recorded**

- Given `FirejailSandbox` is active
- When a shell command is executed
- Then `sandbox_backend` is `Some("firejail")`

### R5: Computer-Use Sidecar Isolation Contract

At `BrowserTool` initialization (lazy, on first use), perform an optional async health-check to the
sidecar endpoint. Log the sidecar's reported isolation level as a `SecurityEvent` audit entry.
Document the expected isolation contract for operators.

**Rationale:** The sidecar performs OS-level actions (mouse, keyboard, screenshots) and operators
need visibility into its isolation posture.

#### Scenarios

**R5-S1: Healthy sidecar with isolation info**

- Given the sidecar is running and responds to `GET /v1/health`
- When `BrowserTool` performs its health-check
- Then the response `isolation.type` and `isolation.runtime` are logged as a `SecurityEvent` audit
  entry

**R5-S2: Sidecar health-check fails gracefully**

- Given the sidecar is not running or returns an error
- When `BrowserTool` performs its health-check
- Then a warning is logged
- And `BrowserTool` continues to function (when `sandbox.require = false`)
- And no audit event with isolation info is emitted

**R5-S3: Sidecar health-check with require mode**

- Given `sandbox.require = true` and the sidecar health-check fails
- When `BrowserTool` attempts its first computer-use action
- Then the action is rejected with an error explaining sidecar isolation could not be verified

**R5-S4: Sidecar without health endpoint**

- Given the sidecar does not implement `/v1/health`
- When `BrowserTool` performs its health-check
- Then this is treated the same as R5-S2 (warning, continue)

### R6: Backward Compatibility

All existing behavior must be preserved when `sandbox.require = false` and
`sandbox.backend = auto|none`. No breaking changes to CLI contract, config schema (additive only),
or existing `SecurityPolicy` tests.

#### Scenarios

**R6-S1: Default config produces identical behavior**

- Given a config file with no `sandbox.require` field
- When the runtime starts
- Then `require` defaults to `false`
- And behavior is identical to pre-change (NoopSandbox fallback)

**R6-S2: Existing SecurityPolicy tests pass unchanged**

- Given the existing test suite for `SecurityPolicy` in `policy.rs`
- When tests are run after this change
- Then all existing tests pass without modification

**R6-S3: Config without require field deserializes correctly**

- Given a TOML/YAML config with `[security.sandbox]` section but no `require` key
- When the config is deserialized
- Then `SandboxConfig.require` is `false`
- And all other fields have their existing default values

**R6-S4: CLI contract unchanged**

- Given the existing CLI commands and flags
- When the runtime is invoked with pre-change arguments
- Then behavior is unchanged

## Acceptance Criteria

- [ ] `ShellTool::execute()` calls `wrap_command()` on every shell execution
- [ ] `create_sandbox()` returns `Result` and fails when `require = true` + no backend
- [ ] NoopSandbox warning emitted for non-read-only commands only
- [ ] `AuditEvent.security.sandbox_backend` is always populated for command executions
- [ ] Computer-use sidecar health-check logs isolation level in audit
- [ ] All existing tests pass without modification
- [ ] New tests cover all scenarios above
- [ ] `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass
