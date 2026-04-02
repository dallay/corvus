# Exploration: Runtime Sandbox Hardening

**Change:** runtime-sandbox-hardening
**Issue:** #37 — Harden runtime sandboxing for user-triggered execution and sidecar-assisted actions
**Date:** 2026-04-02

## Executive Summary

The Corvus runtime has a well-designed **application-layer security model** (command allowlists, path validation, rate limiting, risk classification) that is actively enforced. It also has a **Sandbox trait with four OS-level backends** (Landlock, Firejail, Bubblewrap, Docker) and auto-detection logic. However, **the OS-level sandbox is never actually invoked during tool execution**. The `wrap_command()` method from the Sandbox trait is not called anywhere in the execution path. This is the primary gap.

## Current Architecture

### Security Module Structure (`src/security/`)

| File | Purpose | Status |
|------|---------|--------|
| `traits.rs` | `Sandbox` trait + `NoopSandbox` | Defined, not wired |
| `detect.rs` | Backend auto-detection, `create_sandbox()` | Works but result unused |
| `policy.rs` | `SecurityPolicy` — command allowlist, path validation, rate limits, risk levels | **Actively enforced** |
| `audit.rs` | `AuditLogger` — structured event logging | Active, has `sandbox_backend` field (always None) |
| `egress.rs` | Cerebro endpoint validation (loopback-only for insecure) | Active |
| `secrets.rs` | Encrypted secret store | Active |
| `pairing.rs` | Pairing guard for client auth | Active |
| `landlock.rs` | Landlock LSM backend (Linux kernel 5.13+) | Implemented, unused |
| `firejail.rs` | Firejail user-space sandbox (Linux) | Implemented, unused |
| `bubblewrap.rs` | Bubblewrap user-namespace sandbox | Implemented, unused |
| `docker.rs` | Docker container isolation | Implemented, unused |

### Sandbox Trait Contract

```rust
pub trait Sandbox: Send + Sync {
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
```

Simple, synchronous command wrapping. Each backend modifies a `std::process::Command` to add isolation (e.g., prefixing with `firejail`, adding Landlock rules, wrapping in `docker run`).

### Backend Detection Flow (`detect.rs`)

`create_sandbox(config)` works as follows:

1. If `backend = None` or `enabled = false` → `NoopSandbox`
2. If explicit backend requested → try that backend, **fall back to NoopSandbox on failure** (with warning log)
3. If `backend = Auto` → try in order: Landlock → Firejail → Bubblewrap → Docker → NoopSandbox

### Application-Layer Policy (`policy.rs`)

Actively enforced in `ShellTool::execute()`:

- **AutonomyLevel**: ReadOnly (blocks all), Supervised (default, approval gates), Full
- **Command allowlist**: only listed commands can execute, all segments validated
- **Risk classification**: Low/Medium/High with approval gates
- **Path validation**: workspace-only by default, forbidden paths, traversal blocking
- **Rate limiting**: sliding window per hour
- **Command injection protection**: blocks backticks, `$()`, `${}`, redirects, `tee`, background `&`, `-exec`

### Computer-Use Sidecar Controls (`browser.rs`)

- Endpoint defaults to `http://127.0.0.1:8787/v1/actions` (loopback only)
- `allow_remote_endpoint` defaults to `false`
- Non-global IP validation blocks SSRF (private, link-local, reserved ranges)
- Coordinate bounds validation (`max_coordinate_x`, `max_coordinate_y`)
- Window allowlist forwarded as policy to sidecar
- Domain allowlist for URL actions
- Bearer token auth optional
- Per-action timeout (15s default)

### Config Schema (`config/schema.rs`)

```rust
pub struct SandboxConfig {
    pub enabled: Option<bool>,       // None = auto-detect
    pub backend: SandboxBackend,     // Auto | Landlock | Firejail | Bubblewrap | Docker | None
    pub firejail_args: Vec<String>,
}
```

No `require_sandbox` or `fail_closed` option exists.

## Critical Findings

### Finding 1: OS-Level Sandbox Is Never Invoked (CRITICAL)

**The `ShellTool` does NOT call `wrap_command()`.** The execution path is:

```
ShellTool::execute()
  → security.validate_command_execution()  // app-layer only
  → runtime.build_shell_command()          // no sandbox wrapping
  → cmd.output()                           // direct execution
```

The `Sandbox` trait and all four backends are dead code from an execution perspective. `create_sandbox()` is exported but never called in any tool or agent module.

### Finding 2: Silent Fallback to NoopSandbox (HIGH)

When an operator explicitly requests `backend = Landlock` but runs on macOS, `detect.rs` logs a warning and returns `NoopSandbox`. There is no option to fail closed. An operator who believes they have OS-level sandboxing may actually have none.

### Finding 3: No Sandbox Requirement for Tool Categories (HIGH)

There is no mechanism to require OS-level sandboxing for specific tool categories. Shell execution, file operations, and computer-use actions all use the same application-layer policy regardless of risk level.

### Finding 4: Audit Trail Missing Sandbox Context (MEDIUM)

`AuditEvent.security.sandbox_backend` is always `None` because no sandbox is ever selected during execution. Even when `create_sandbox()` is called, the result is not propagated to the audit logger.

### Finding 5: Computer-Use Sidecar Not Sandbox-Wrapped (MEDIUM)

Computer-use actions go through HTTP to a sidecar but the sidecar process itself is not launched or verified within any OS-level sandbox. The runtime trusts that the sidecar enforces its own isolation.

### Finding 6: No User-Scoped Isolation (MEDIUM)

The security model is per-runtime-instance, not per-user or per-session. All tool executions share the same sandbox configuration and policy. There is no mechanism for per-user isolation boundaries.

## Test Coverage Assessment

### Well-Covered

- `policy.rs`: Extensive tests (80+ tests) — command allowlists, risk classification, path validation, rate limiting, injection attacks
- `detect.rs`: Backend detection, fallback behavior, config mapping (12 tests)
- `audit.rs`: Event serialization, log rotation, structured entries (8 tests)
- `shell.rs`: Command execution, blocking, rate limiting, env sanitization (15 tests)

### Gaps

- No tests verifying that `wrap_command()` is actually called during tool execution (because it isn't)
- No tests for fail-closed behavior when sandbox backend unavailable
- No integration tests connecting sandbox detection → tool execution → audit logging
- No tests for computer-use sidecar sandbox enforcement
- No tests for `SandboxConfig.enabled = Some(true)` with `backend = Auto` failing to find any backend

## Recommendations for Hardening

1. **Wire sandbox into execution path**: `ShellTool` (and potentially other tools) must call `wrap_command()` on the selected sandbox backend before executing commands.

2. **Add fail-closed mode**: New config option `sandbox.require = true|false` — when true, if no OS-level backend is available, refuse to execute rather than falling back to NoopSandbox.

3. **Tool-category sandbox requirements**: Define which tool categories (shell, file_write, computer_use) must have OS-level sandboxing vs. application-layer only.

4. **Propagate sandbox to audit**: Ensure `AuditEvent.security.sandbox_backend` reflects the actual sandbox used for each execution.

5. **Computer-use sidecar verification**: Add optional sidecar health/capability check to verify sidecar itself runs in appropriate isolation.

6. **Config documentation**: Make the isolation contract explicit so operators can reason about their security posture.

## Key Questions for Design Phase

1. Should `wrap_command()` be called for ALL shell executions, or only high/medium-risk ones?
2. Should fail-closed be the default, or opt-in?
3. How should the sandbox interact with `RuntimeAdapter.build_shell_command()`?
4. Should computer-use sidecar actions require sandbox wrapping of the sidecar process?
5. What is the minimum acceptable isolation when `backend = Auto` finds nothing?
