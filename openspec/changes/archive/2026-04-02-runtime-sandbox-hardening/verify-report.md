# Verification Report

**Change**: runtime-sandbox-hardening  
**Version**: N/A

---

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 8 |
| Tasks complete | 8 |
| Tasks incomplete | 0 |

All tasks T1-T8 are marked complete in `tasks.md`.

---

## Build & Tests Execution

**Build**: ✅ Passed

Commands run:
- `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`
- `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`

Results:
- `cargo fmt --check`: passed
- `cargo clippy --all-targets -- -D warnings`: passed

**Tests**: ✅ Passed

Commands run:
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --test mcp_native_regression`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --test mcp_runtime_e2e`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib tools::shell`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib security::audit`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib bootstrap`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib tools::browser`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib security::detect`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib config::schema::tests::sandbox_config`

Results:
- Full `cargo test`: ✅ `3107 passed; 0 failed; 0 ignored; 0 measured`
- `mcp_native_regression`: ✅ `2 passed; 0 failed`
- `mcp_runtime_e2e`: ✅ `3 passed; 0 failed`
- `tools::shell`: ✅ `23 passed; 0 failed`
- `security::audit`: ✅ `10 passed; 0 failed`
- `bootstrap`: ✅ `12 passed; 0 failed`
- `tools::browser`: ✅ `53 passed; 0 failed`
- `security::detect`: ✅ `17 passed; 0 failed`
- `config::schema::tests::sandbox_config`: ✅ `3 passed; 0 failed`

**Coverage**: ➖ Not configured

---

## Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| R1 | R1-S1 Sandbox wraps every shell command | `tools::shell::tests::shell_calls_wrap_command_on_injected_sandbox` | ✅ COMPLIANT |
| R1 | R1-S2 Sandbox error blocks execution | `tools::shell::tests::shell_returns_error_when_sandbox_wrap_fails` | ✅ COMPLIANT |
| R1 | R1-S3 NoopSandbox passes through unchanged | `tools::shell::tests::shell_executes_allowed_command` | ⚠️ PARTIAL |
| R1 | R1-S4 Wrapping happens after env sanitization | `tools::shell::tests::shell_wraps_after_env_sanitization` | ✅ COMPLIANT |
| R2 | R2-S1 Require mode with available backend succeeds | `security::detect::tests::docker_backend_falls_back_gracefully` | ⚠️ PARTIAL |
| R2 | R2-S2 Require mode with unavailable explicit backend fails | `security::detect::tests::require_landlock_on_non_linux_returns_error` | ✅ COMPLIANT |
| R2 | R2-S3 Require mode with auto finds nothing fails | `security::detect::tests::require_auto_no_backend_returns_error_or_ok` | ⚠️ PARTIAL |
| R2 | R2-S4 Non-require mode falls back to NoopSandbox | `security::detect::tests::explicit_none_returns_noop`, `security::detect::tests::default_security_config_produces_working_sandbox` | ⚠️ PARTIAL |
| R2 | R2-S5 Explicit none backend with require is an error | `security::detect::tests::require_none_backend_returns_error` | ✅ COMPLIANT |
| R2 | R2-S6 Disabled sandbox with require is an error | `security::detect::tests::require_disabled_returns_error` | ✅ COMPLIANT |
| R3 | R3-S1 Warning for mutating command with NoopSandbox | `tools::shell::tests::noop_sandbox_warning_helper_triggers_for_mutating_commands` | ⚠️ PARTIAL |
| R3 | R3-S2 No warning for read-only command with NoopSandbox | `tools::shell::tests::noop_sandbox_warning_helper_skips_read_only_commands` | ⚠️ PARTIAL |
| R3 | R3-S3 No warning when real sandbox is active | `tools::shell::tests::noop_sandbox_warning_helper_skips_real_sandbox` | ⚠️ PARTIAL |
| R4 | R4-S1 Audit event includes sandbox backend name | `security::audit::tests::audit_log_command_event_writes_structured_entry` | ✅ COMPLIANT |
| R4 | R4-S2 NoopSandbox is recorded as `none` | `security::audit::tests::audit_log_command_event_writes_structured_entry` | ⚠️ PARTIAL — test injects values into `CommandExecutionLog` proving serialization, but does not assert `NoopSandbox` name is emitted via the actual shell execution path; an end-to-end runtime propagation test is needed |
| R4 | R4-S3 Real backend name is recorded | `security::audit::tests::audit_log_command_event_records_real_sandbox_backend` | ⚠️ PARTIAL — test injects a backend name into `CommandExecutionLog` proving serialization, but does not assert the real sandbox backend name flows through the actual shell/browser execution path; an end-to-end runtime propagation test is needed |
| R5 | R5-S1 Healthy sidecar with isolation info | `tools::browser::tests::computer_use_sidecar_health_check_reports_isolation_info` | ✅ COMPLIANT |
| R5 | R5-S2 Sidecar health-check fails gracefully | `tools::browser::tests::computer_use_sidecar_health_check_fails_gracefully_when_optional` | ✅ COMPLIANT |
| R5 | R5-S3 Sidecar health-check with require mode | `tools::browser::tests::computer_use_sidecar_health_check_rejects_when_required` | ✅ COMPLIANT |
| R5 | R5-S4 Sidecar without health endpoint | `tools::browser::tests::computer_use_sidecar_missing_health_endpoint_is_treated_as_failure` | ✅ COMPLIANT |
| R6 | R6-S1 Default config produces identical behavior | `config::schema::tests::sandbox_config_default_require_is_false`, `bootstrap::tests::bootstrap_context_builds_core_components` | ⚠️ PARTIAL |
| R6 | R6-S2 Existing SecurityPolicy tests pass unchanged | full `cargo test` (`security::policy::tests::*`) | ✅ COMPLIANT |
| R6 | R6-S3 Config without require field deserializes correctly | `config::schema::tests::sandbox_config_missing_require_defaults_to_false` | ✅ COMPLIANT |
| R6 | R6-S4 CLI contract unchanged | full `cargo test`, `mcp_native_regression`, `mcp_runtime_e2e` | ⚠️ PARTIAL |

**Compliance summary**: 13/24 scenarios compliant

---

## Correctness (Static — Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| R1 Sandbox wiring into `ShellTool` | ✅ Implemented | `src/tools/shell.rs:177-200` calls `self.sandbox.wrap_command(&mut std_cmd)` after env sanitization and before process spawn. |
| R2 Fail-closed mode via `sandbox.require` | ✅ Implemented | `SandboxConfig.require` exists in `src/config/schema.rs`; `create_sandbox()` returns `Result` and fails closed in `src/security/detect.rs`; bootstrap startup propagates `?` in `src/bootstrap/mod.rs:227-233`. |
| R3 NoopSandbox warning for mutating commands | ✅ Implemented | Warning gate in `src/tools/shell.rs:18-24`; `tracing::warn!` emitted at `src/tools/shell.rs:191-195`. |
| R4 Audit propagation of `sandbox_backend` | ✅ Implemented | Backend is included in shell structured payload (`src/tools/shell.rs:26-35`), extracted by agent (`src/agent/agent.rs:596-616`), and persisted by audit logger (`src/security/audit.rs:242-252`). |
| R5 Computer-use sidecar health-check and security event logging | ✅ Implemented | Lazy verification in `src/tools/browser.rs:353-389`; first computer-use action consumes it at `src/tools/browser.rs:855`; audit `SecurityEvent` emitted in `src/agent/agent.rs:626-667`. |
| R6 Backward compatibility | ✅ Implemented | Additive config change, bootstrap default path succeeds, and previously failing integration tests now pass. |

---

## Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Inject `Arc<dyn Sandbox>` into `ShellTool` | ✅ Yes | Matches constructor and factory wiring in `src/tools/mod.rs`. |
| `create_sandbox()` returns `Result` | ✅ Yes | Implemented in `src/security/detect.rs` and propagated through bootstrap. |
| Wrap ALL shell executions | ✅ Yes | `wrap_command()` is unconditional after policy/env setup. |
| Detect NoopSandbox via `name() == "none"` | ✅ Yes | Used directly in `should_warn_for_noop_sandbox()`. |
| Lazy async sidecar health-check | ✅ Yes | Verification happens on first computer-use action, not construction. |
| File changes table | ⚠️ Deviated | Design expected docs in `docs/`; implementation wrote `clients/web/apps/docs/src/content/docs/guides/runtime-sandbox-isolation.md`. Intent is satisfied, path differs. |

---

## Issues Found

**CRITICAL** (must fix before archive):
None.

**WARNING** (should fix):
- R1-S3 is only indirectly covered; there is still no dedicated runtime test proving `NoopSandbox` wrapping plus identical pre-change behavior in one scenario.
- R2 success/fallback scenarios remain partially environment-dependent rather than fully deterministic (`docker_backend_falls_back_gracefully`, `require_auto_no_backend_returns_error_or_ok`).
- R3 tests validate decision logic, not captured `tracing::warn!` emission.
- R6 default-behavior and CLI-compatibility coverage is indirect rather than scenario-specific.
- Documentation path still deviates from the design file-change table.

**SUGGESTION** (nice to have):
- Add explicit log-capture tests for the NoopSandbox warning.
- Add deterministic sandbox-detection tests using controllable backend probes or abstraction seams.
- Add one end-to-end agent audit test that executes shell and browser actions and inspects serialized audit records.

---

## Verdict

PASS WITH WARNINGS

The change now passes all validation gates and has runtime proof for the previously missing critical scenarios, but a handful of spec scenarios are still only partially covered or indirectly proven.
