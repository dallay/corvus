# Proposal: Runtime Sandbox Hardening

## Intent

The Corvus agent-runtime has a well-designed `Sandbox` trait with four OS-level backends (Landlock, Firejail, Bubblewrap, Docker) and auto-detection logic — but **none of it is ever called during tool execution**. The `wrap_command()` method is dead code. Every shell command runs with application-layer policy only (`SecurityPolicy`), and operators who configure `sandbox.backend = Landlock` get a silent fallback to `NoopSandbox` with no indication that OS-level isolation is absent.

This change wires the existing sandbox infrastructure into the actual execution path, adds a fail-closed option, propagates sandbox context to audit events, and establishes the isolation contract for the computer-use sidecar.

**GitHub Issue:** #37

## Scope

### In Scope

1. **Wire `Sandbox` into `ShellTool` execution**: Inject `Arc<dyn Sandbox>` into `ShellTool` and call `wrap_command()` for ALL shell executions (defense in depth).
2. **Add `sandbox.require` config option**: When `true`, `create_sandbox()` returns an error instead of silently falling back to `NoopSandbox`. Defaults to `false` for backward compatibility.
3. **Warning on NoopSandbox for mutating operations**: When `sandbox.require = false` and `NoopSandbox` is active, log a warning for non-read-only command executions.
4. **Propagate sandbox backend to audit events**: Ensure `AuditEvent.security.sandbox_backend` reflects the actual sandbox used per execution, not always `None`.
5. **Computer-use sidecar isolation contract**: Add a startup health-check that queries and logs the sidecar's isolation level in the audit log. Document the expected isolation contract.
6. **Focused tests for new security boundaries**: Integration tests verifying sandbox wiring, fail-closed behavior, audit propagation, and NoopSandbox warning paths.

### Out of Scope

- Replacing or modifying the existing `SecurityPolicy` model (it is solid and well-tested)
- Adding new sandbox backends beyond the existing four
- Web application security (web clients use the HTTP gateway, not the runtime directly)
- Per-user or per-session isolation boundaries (noted as future work)
- Sandbox wrapping for file-system tools (evaluated during design; may be deferred)

## Approach

### Phase 1: Fail-Closed Infrastructure

- Add `require: bool` field to `SandboxConfig` (default `false`).
- Change `create_sandbox()` signature to return `Result<Arc<dyn Sandbox>, anyhow::Error>` so it can fail when `require = true` and no backend is available.
- Update all call sites (currently only `security/mod.rs` re-export).

### Phase 2: Wire Sandbox into ShellTool

- Add `sandbox: Arc<dyn Sandbox>` field to `ShellTool`.
- Update `ShellTool::new()` to accept the sandbox.
- In `ShellTool::execute()`, after `validate_command_execution()` succeeds and before `cmd.output()`, call `self.sandbox.wrap_command(&mut cmd)?`.
- Update `tools/mod.rs` factory (`add_shell_tool` / `create_tools`) to pass the sandbox through.

### Phase 3: Audit Propagation

- In `ShellTool::execute()`, pass `self.sandbox.name()` to the audit event builder via `with_security(Some(sandbox_name))`.
- Ensure the sandbox backend name appears in every `CommandExecution` audit event.

### Phase 4: NoopSandbox Warning

- When `NoopSandbox` is the active backend and a non-read-only command executes, emit a `tracing::warn!` with a clear message: "OS-level sandbox is not active; running with application-layer policy only".
- Read-only detection reuses the existing `SecurityPolicy::risk_level()` classification.

### Phase 5: Computer-Use Sidecar Contract

- Add an optional health-check call to the sidecar endpoint at `BrowserTool` initialization that queries isolation capabilities.
- Log the sidecar's reported isolation level as an audit event (`SecurityEvent` type).
- Document the expected isolation contract in `docs/` — what operators SHOULD ensure about sidecar deployment.

### Phase 6: Tests

- Unit tests: `ShellTool` calls `wrap_command()` on the injected sandbox.
- Unit tests: `create_sandbox()` returns error when `require = true` and backend unavailable.
- Unit tests: `NoopSandbox` warning fires for mutating commands, not for read-only.
- Unit tests: audit events contain `sandbox_backend` after execution.
- Integration test: end-to-end sandbox detection → tool execution → audit log entry.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/security/detect.rs` | Modified | `create_sandbox()` returns `Result`, respects `require` flag |
| `src/security/traits.rs` | Modified | Minor — may add `is_noop()` helper to `Sandbox` trait |
| `src/security/mod.rs` | Modified | Re-export updated signature |
| `src/config/schema.rs` | Modified | Add `require: bool` to `SandboxConfig` |
| `src/tools/shell.rs` | Modified | Accept and use `Arc<dyn Sandbox>`, call `wrap_command()` |
| `src/tools/mod.rs` | Modified | Pass sandbox to `ShellTool` in factory functions |
| `src/tools/browser.rs` | Modified | Add sidecar health-check at init, log isolation level |
| `src/security/audit.rs` | Unchanged | Already has `sandbox_backend` field — just needs to be populated |
| `docs/` | New | Sidecar isolation contract documentation |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Breaking existing deployments that rely on silent NoopSandbox fallback | Low | `sandbox.require` defaults to `false`; existing behavior preserved unless operator opts in |
| `wrap_command()` backend errors blocking all shell execution | Medium | Errors from `wrap_command()` are surfaced clearly; `NoopSandbox.wrap_command()` always succeeds; operators can set `backend = None` to explicitly opt out |
| Performance overhead of sandbox wrapping on every shell call | Low | `wrap_command()` is synchronous command mutation (no I/O); `NoopSandbox` is a no-op; real backends add CLI prefix only |
| Sidecar health-check adding startup latency | Low | Health-check is async with a short timeout; failure is logged as warning, not fatal (unless `require = true`) |
| Test complexity for OS-specific backends | Medium | Use mock `Sandbox` implementations in tests; real backend tests remain behind feature flags |

## Rollback Plan

1. **Config-level rollback**: Set `sandbox.require = false` and `sandbox.backend = None` to restore exact pre-change behavior (NoopSandbox, no wrapping, no warnings).
2. **Code-level rollback**: Revert the PR. The change is additive — `ShellTool` gains a field and a call; removing it restores the original execution path. No data migrations, no schema changes, no external API changes.
3. **Feature flag**: The `sandbox.require` config option itself acts as a feature flag. Operators can deploy the code without enabling enforcement.

## Dependencies

- No new crate dependencies expected.
- Existing sandbox backends (`landlock`, `firejail`, `bubblewrap`, `docker`) are already implemented and tested in isolation.
- Computer-use sidecar health-check depends on the existing HTTP client infrastructure in `browser.rs`.

## Success Criteria

- [ ] `ShellTool::execute()` calls `wrap_command()` on the injected sandbox for every shell execution
- [ ] `create_sandbox()` returns an error (not NoopSandbox) when `sandbox.require = true` and no backend is available
- [ ] `AuditEvent.security.sandbox_backend` is populated with the actual backend name for every `CommandExecution` event
- [ ] A warning is logged when `NoopSandbox` is used for non-read-only operations
- [ ] Computer-use sidecar reports its isolation level at startup in an audit event
- [ ] All existing `SecurityPolicy` tests continue to pass (no regressions)
- [ ] New tests cover: sandbox wiring, fail-closed, audit propagation, NoopSandbox warning, sidecar health-check
- [ ] `cargo test`, `cargo clippy`, and `cargo fmt --check` pass cleanly
