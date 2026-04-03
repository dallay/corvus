# Tasks: Runtime Sandbox Hardening

**Change:** runtime-sandbox-hardening
**Issue:** #37
**Date:** 2026-04-02

## Task Order

Tasks are ordered by dependency. Each is independently testable and committable.

---

### [x] T1: Add `require` field to `SandboxConfig`

**Description:** Add `require: bool` field to `SandboxConfig` with `#[serde(default)]` defaulting to
`false`. Update `Default` impl. Add serde round-trip tests.

**Files:**

- `src/config/schema.rs` — add field to `SandboxConfig`, update `Default`
- `src/config/mod.rs` — no change expected (field auto-exported)

**Spec scenarios:** R6-S1, R6-S3

**Tests:**

- Serde round-trip: TOML with `require = true` deserializes correctly
- Backward compat: TOML without `require` field defaults to `false`
- Default `SandboxConfig` has `require = false`

**Dependencies:** None

---

### [x] T2: Change `create_sandbox()` to return `Result` with fail-closed support

**Description:** Change `create_sandbox()` signature from `Arc<dyn Sandbox>` to
`Result<Arc<dyn Sandbox>, anyhow::Error>`. When `require = true` and no OS-level backend is
available, return `Err`. When `require = true` and `backend = None` or `enabled = false`, return
`Err` (contradiction). When `require = false`, preserve existing NoopSandbox fallback. Update
`detect_best_sandbox()` similarly.

**Files:**

- `src/security/detect.rs` — change signatures, add require logic, update all fallback paths
- `src/security/mod.rs` — update re-export

**Spec scenarios:** R2-S1, R2-S2, R2-S3, R2-S4, R2-S5, R2-S6

**Tests:**

- `require = true` + explicit unavailable backend → `Err`
- `require = true` + auto finds nothing → `Err`
- `require = true` + `backend = None` → `Err`
- `require = true` + `enabled = false` → `Err`
- `require = false` + no backend → `Ok(NoopSandbox)` (existing behavior)
- `require = false` + available backend → `Ok(backend)`
- All existing `detect.rs` tests adapted to unwrap `Result`

**Dependencies:** T1

---

### [x] T3: Wire `Arc<dyn Sandbox>` into `ShellTool` and call `wrap_command()`

**Description:** Add `sandbox: Arc<dyn Sandbox>` field to `ShellTool`. Update `ShellTool::new()` to
accept sandbox. In `execute()`, call `self.sandbox.wrap_command(&mut cmd)?` after env sanitization
and before `cmd.output()`. If `wrap_command()` returns `Err`, return
`ToolResult { success: false, error: sandbox_error }`. Update tool factory functions in
`tools/mod.rs` to accept and pass `Arc<dyn Sandbox>`.

**Files:**

- `src/tools/shell.rs` — add field, update new(), call wrap_command() in execute()
- `src/tools/mod.rs` — update factory signatures to accept sandbox parameter

**Spec scenarios:** R1-S1, R1-S2, R1-S3, R1-S4

**Tests:**

- MockSandbox: verify `wrap_command()` is called on every execution
- MockSandbox returning Err: verify command NOT spawned, ToolResult.success = false
- NoopSandbox: verify identical behavior to pre-change (passthrough)
- Execution ordering: wrap_command after env_clear (verify via mock that inspects cmd state)
- All existing `shell.rs` tests updated to pass NoopSandbox

**Dependencies:** T1, T2

---

### [x] T4: Add NoopSandbox warning for non-read-only commands

**Description:** After `wrap_command()` call in `ShellTool::execute()`, check if
`self.sandbox.name() == "none"` and the command risk level is Medium or High. If so, emit
`tracing::warn!("OS-level sandbox not active; running with application-layer policy only", command = %command)`.
Low-risk (read-only) commands do not trigger the warning.

**Files:**

- `src/tools/shell.rs` — add warning logic after wrap_command()

**Spec scenarios:** R3-S1, R3-S2, R3-S3

**Tests:**

- NoopSandbox + mutating command (e.g., `touch file`) → warning emitted (use `tracing-test` or log
  capture)
- NoopSandbox + read-only command (e.g., `ls`) → no warning
- Real sandbox (mock with name != "none") + any command → no warning

**Dependencies:** T3

---

### [x] T5: Propagate sandbox backend name to audit events

**Description:** In `ShellTool::execute()`, pass `Some(self.sandbox.name().to_string())` to the
audit event via `with_security()`. Ensure every `CommandExecution` audit event has `sandbox_backend`
populated. This requires `ShellTool` to have access to `AuditLogger` or return sandbox info so the
caller can log it.

**Files:**

- `src/tools/shell.rs` — capture sandbox name, include in ToolResult or audit call
- Potentially `src/security/audit.rs` — no structural changes, field already exists

**Spec scenarios:** R4-S1, R4-S2, R4-S3

**Tests:**

- After execution with NoopSandbox: audit event has `sandbox_backend = Some("none")`
- After execution with mock sandbox named "firejail": audit event has
  `sandbox_backend = Some("firejail")`
- `sandbox_backend` is never `None` for executed commands

**Dependencies:** T3

---

### [x] T6: Add `create_sandbox()` to startup path and thread to tools

**Description:** Find where tool factories (`all_tools`, `default_tools`, `all_tools_with_runtime`)
are called in the startup/initialization path (likely `main.rs` or agent module). Call
`create_sandbox(&security_config)?` and pass the result to tool factories. Handle the `Result` — if
`Err` (require mode failed), log error and exit.

**Files:**

- `src/main.rs` or wherever tools are created — add `create_sandbox()` call
- `src/tools/mod.rs` — factory signatures already updated in T3

**Spec scenarios:** R2-S1 through R2-S6 (integration), R6-S4

**Tests:**

- Integration: default config → startup succeeds with NoopSandbox
- Integration: `require = true` on system without backends → startup fails with clear error

**Dependencies:** T2, T3

---

### [x] T7: Add computer-use sidecar health-check

**Description:** Add a lazy async health-check in `BrowserTool` that calls `GET {endpoint}/health` (
derived from the configured computer-use endpoint) on first computer-use action. Parse the response
for `isolation.type` and `isolation.runtime`. Log as `SecurityEvent` audit entry. On failure: warn
and continue if `sandbox.require = false`; reject action if `sandbox.require = true`. Add
`sidecar_verified: AtomicBool` or `OnceCell` to avoid repeated checks.

**Files:**

- `src/tools/browser.rs` — add health-check logic, lazy init, audit logging
- `src/config/schema.rs` — no changes (endpoint already configurable)

**Spec scenarios:** R5-S1, R5-S2, R5-S3, R5-S4

**Tests:**

- Mock HTTP server returning healthy response → audit event logged with isolation info
- Mock HTTP server returning error → warning logged, tool continues (require=false)
- Mock HTTP server returning error + require=true → action rejected
- No health endpoint (404) → treated as failure, warning

**Dependencies:** T1 (needs `require` field), T5 (audit pattern)

---

### [x] T8: Documentation — sidecar isolation contract

**Description:** Write operator-facing documentation explaining the sandbox isolation model: what
backends are supported, what `require` does, how to verify isolation is active, and what the
computer-use sidecar isolation contract expects from operators.

**Files:**

- `docs/sandbox-isolation.md` — new file

**Spec scenarios:** R5 (documentation aspect), R6 (operator expectations)

**Tests:** N/A (documentation only)

**Dependencies:** T1-T7 (write after implementation is stable)

---

## Summary

| Task | Title                          | Files                  | Scenarios    | Depends |
|------|--------------------------------|------------------------|--------------|---------|
| T1   | Add `require` to SandboxConfig | schema.rs              | R6-S1, R6-S3 | —       |
| T2   | Fail-closed `create_sandbox()` | detect.rs, mod.rs      | R2-*         | T1      |
| T3   | Wire sandbox into ShellTool    | shell.rs, tools/mod.rs | R1-*         | T1, T2  |
| T4   | NoopSandbox warning            | shell.rs               | R3-*         | T3      |
| T5   | Audit propagation              | shell.rs               | R4-*         | T3      |
| T6   | Startup path wiring            | main.rs, tools/mod.rs  | R2-*, R6-S4  | T2, T3  |
| T7   | Sidecar health-check           | browser.rs             | R5-*         | T1, T5  |
| T8   | Documentation                  | docs/                  | R5, R6       | T1-T7   |

**Critical path:** T1 → T2 → T3 → T4/T5 (parallel) → T6 → T7 → T8
