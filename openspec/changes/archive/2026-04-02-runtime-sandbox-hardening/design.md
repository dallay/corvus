# Design: Runtime Sandbox Hardening

## Technical Approach

Wire the existing `Sandbox` trait infrastructure into the `ShellTool` execution path, add a
fail-closed config option, propagate sandbox identity to audit events, emit warnings when
`NoopSandbox` handles mutating operations, and establish a health-check contract for the
computer-use sidecar. This is defense-in-depth layered on top of the existing `SecurityPolicy`
application-layer controls.

The change is purely additive — no existing security policy logic is modified. Backward
compatibility is preserved by defaulting `sandbox.require = false`.

## Architecture Decisions

### Decision: Inject `Arc<dyn Sandbox>` into `ShellTool` via constructor

**Choice**: Add `sandbox: Arc<dyn Sandbox>` as a field on `ShellTool`, passed through `new()` and
the tool factory functions.
**Alternatives considered**: (1) Global/static sandbox singleton — rejected because it prevents
per-test injection and violates the trait-driven extension pattern used throughout the codebase. (2)
Resolve sandbox inside `execute()` on each call — rejected because sandbox creation involves
filesystem probes and should happen once at startup.
**Rationale**: Follows the existing dependency injection pattern (`security: Arc<SecurityPolicy>`,
`runtime: Arc<dyn RuntimeAdapter>`). Enables mock injection for testing. Single allocation shared
across all calls.

### Decision: `create_sandbox()` returns `Result<Arc<dyn Sandbox>, anyhow::Error>`

**Choice**: Change the return type from `Arc<dyn Sandbox>` to
`Result<Arc<dyn Sandbox>, anyhow::Error>`.
**Alternatives considered**: (1) Panic on failure — violates non-negotiable "no unwrap in
production". (2) Return `Option` — less ergonomic with `?` chains and doesn't carry error context.
**Rationale**: Matches Corvus conventions (`Result` + `anyhow` for fallible operations). Lets
callers distinguish "no sandbox available" from "sandbox creation succeeded with NoopSandbox". The
`require` flag determines whether "no backend found" is an error or a valid NoopSandbox result.

### Decision: Call `wrap_command()` for ALL shell executions, not just high-risk

**Choice**: Unconditionally call `self.sandbox.wrap_command(&mut cmd)` for every shell execution
after policy validation passes.
**Alternatives considered**: Only wrap medium/high-risk commands — rejected because (1) risk
classification is an application-layer concept and the OS sandbox should be orthogonal, (2)
`NoopSandbox.wrap_command()` is a no-op so there's zero overhead when no backend is active, (3)
defense-in-depth means even "low-risk" commands benefit from OS isolation.
**Rationale**: Simplest correct approach. The sandbox is either on or off — mixing risk levels with
sandbox decisions creates confusing security semantics.

### Decision: Detect NoopSandbox via `sandbox.name() == "none"` check

**Choice**: Use the existing `Sandbox::name()` method to detect NoopSandbox (`name() == "none"`)
rather than adding a new trait method.
**Alternatives considered**: (1) Add `Sandbox::is_noop()` with default implementation — adds a
method to the public trait that all backends must consider. (2) Use `std::any::TypeId` — requires
`'static` bound and is fragile.
**Rationale**: `NoopSandbox::name()` already returns `"none"` and is tested. The check is used in
exactly one place (warning logic). Adding a trait method for a single internal check is
over-engineering. If a future backend also returns `"none"`, that backend IS a noop by definition.

### Decision: Sidecar health-check is async, optional, and non-blocking

**Choice**: Add an async health-check call during `BrowserTool` first-use (lazy initialization), not
at construction time. Log the result as an audit event. Failure is a warning unless
`sandbox.require = true`.
**Alternatives considered**: (1) Check at `BrowserTool::new()` — rejected because construction is
synchronous in the current factory and adding async there would require significant refactoring. (2)
Skip health-check entirely — rejected because the proposal explicitly includes sidecar verification.
**Rationale**: Lazy health-check avoids startup latency and doesn't block tool registration. The
sidecar may not be running when the runtime starts (it may be launched on-demand). Logging the
isolation level in audit provides operator visibility.

## Data Flow

### Shell Command Execution (Updated)

```
ShellTool::execute(args)
    │
    ├── 1. rate limit check
    ├── 2. security.validate_command_execution()     [app-layer policy]
    ├── 3. security.record_action()
    ├── 4. runtime.build_shell_command()              [build Command]
    ├── 5. cmd.env_clear() + safe env vars
    ├── 6. self.sandbox.wrap_command(&mut cmd)?        [NEW: OS-level wrapping]
    │       ├── NoopSandbox: no-op, Ok(())
    │       ├── LandlockSandbox: apply LSM rules
    │       ├── FirejailSandbox: prefix with firejail
    │       ├── BubblewrapSandbox: prefix with bwrap
    │       └── DockerSandbox: rewrite as docker run
    ├── 7. if sandbox.name() == "none" && !is_read_only(command)
    │       └── tracing::warn!("OS-level sandbox not active...")  [NEW]
    ├── 8. tokio::time::timeout(cmd.output())         [execute]
    └── 9. return ToolResult with sandbox name in audit context
```

### Sandbox Injection Flow (Startup)

```
Config::load_or_init()
    │
    └── SecurityConfig.sandbox
            │
            ├── create_sandbox(&security_config)?     [NEW: returns Result]
            │       ├── require=true + no backend → Err(...)
            │       ├── require=false + no backend → Ok(NoopSandbox)
            │       └── backend available → Ok(Arc<dyn Sandbox>)
            │
            └── Arc<dyn Sandbox>
                    │
                    ├── default_tools_with_runtime(security, runtime, sandbox)
                    │       └── ShellTool::new(security, runtime, sandbox)
                    │
                    └── all_tools_with_runtime(... sandbox ...)
                            └── ShellTool::new(security, runtime, sandbox)
```

## File Changes

| File                           | Action    | Description                                                                                                                                                                                                                             |
|--------------------------------|-----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `src/config/schema.rs`         | Modify    | Add `require: bool` to `SandboxConfig` with `#[serde(default)]` (defaults to `false`)                                                                                                                                                   |
| `src/security/detect.rs`       | Modify    | Change `create_sandbox()` to return `Result<Arc<dyn Sandbox>>`. When `config.sandbox.require == true` and no real backend found, return `Err`. Rename internal `detect_best_sandbox()` to also return `Result` with `require` awareness |
| `src/security/traits.rs`       | Unchanged | No changes needed — `name() == "none"` check is sufficient                                                                                                                                                                              |
| `src/security/mod.rs`          | Modify    | Update `create_sandbox` re-export (signature changed)                                                                                                                                                                                   |
| `src/security/audit.rs`        | Unchanged | `SecurityContext.sandbox_backend` field already exists — just needs to be populated by callers                                                                                                                                          |
| `src/tools/shell.rs`           | Modify    | Add `sandbox: Arc<dyn Sandbox>` field. Update `new()` signature. Call `wrap_command()` in `execute()`. Add NoopSandbox warning. Pass `sandbox.name()` to audit context                                                                  |
| `src/tools/mod.rs`             | Modify    | Update `default_tools`, `default_tools_with_runtime`, `all_tools`, `all_tools_with_runtime` to accept and pass `Arc<dyn Sandbox>` to `ShellTool`                                                                                        |
| `src/tools/browser.rs`         | Modify    | Add lazy async health-check for sidecar isolation level. Log result as `SecurityEvent` audit entry                                                                                                                                      |
| Call sites of `create_sandbox` | Modify    | Update to handle `Result` (likely in `main.rs` or agent initialization)                                                                                                                                                                 |

## Interfaces / Contracts

### Updated `SandboxConfig`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing (None = auto-detect, Some = explicit)
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Sandbox backend to use
    #[serde(default)]
    pub backend: SandboxBackend,

    /// When true, refuse to start if no OS-level sandbox backend is available.
    /// When false (default), fall back to NoopSandbox with a warning.
    #[serde(default)]
    pub require: bool,

    /// Custom Firejail arguments (when backend = firejail)
    #[serde(default)]
    pub firejail_args: Vec<String>,
}
```

### Updated `create_sandbox` Signature

```rust
/// Create a sandbox based on auto-detection or explicit config.
///
/// Returns `Err` when `config.sandbox.require == true` and no real
/// OS-level backend is available. Returns `Ok(NoopSandbox)` when
/// `require == false` and no backend is found.
pub fn create_sandbox(config: &SecurityConfig) -> Result<Arc<dyn Sandbox>> {
    // ...
}
```

### Updated `ShellTool`

```rust
pub struct ShellTool {
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    sandbox: Arc<dyn Sandbox>,
    timeout: Duration,
}

impl ShellTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        sandbox: Arc<dyn Sandbox>,
    ) -> Self {
        Self {
            security,
            runtime,
            sandbox,
            timeout: Duration::from_secs(60),
        }
    }
}
```

### Updated Tool Factory Signatures

```rust
pub fn default_tools(
    security: Arc<SecurityPolicy>,
    sandbox: Arc<dyn Sandbox>,
) -> Vec<Box<dyn Tool>>

pub fn default_tools_with_runtime(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    sandbox: Arc<dyn Sandbox>,
) -> Vec<Box<dyn Tool>>

// all_tools and all_tools_with_runtime gain sandbox parameter similarly
```

### Config TOML Example

```toml
[security.sandbox]
enabled = true
backend = "auto"        # auto | landlock | firejail | bubblewrap | docker | none
require = true          # NEW: fail startup if no OS backend available

# Existing firejail customization
firejail_args = ["--net=none", "--no3d"]
```

### Sidecar Health-Check Response (Expected Contract)

```json
{
  "status": "healthy",
  "isolation": {
    "type": "container",
    "runtime": "docker",
    "version": "24.0.7"
  }
}
```

The sidecar health endpoint (`GET /v1/health`) is optional. If the endpoint is absent or returns an
error, the runtime logs a warning audit event but continues operating (unless
`sandbox.require = true`).

## Testing Strategy

| Layer       | What to Test                                                                   | Approach                                                                                                              |
|-------------|--------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| Unit        | `ShellTool` calls `wrap_command()` on injected sandbox                         | Mock `Sandbox` that records calls; assert `wrap_command` was invoked after `execute()`                                |
| Unit        | `create_sandbox()` returns `Err` when `require=true` + no backend              | Set `require=true`, `backend=Landlock` on non-Linux; assert `Err` returned                                            |
| Unit        | `create_sandbox()` returns `Ok(NoopSandbox)` when `require=false` + no backend | Existing test adapted to unwrap `Result`                                                                              |
| Unit        | NoopSandbox warning fires for mutating commands                                | Mock sandbox with `name() == "none"`, capture `tracing::warn!` via `tracing-test` subscriber, execute a write command |
| Unit        | NoopSandbox warning does NOT fire for read-only commands                       | Same setup, execute `ls` (read-only), assert no warning                                                               |
| Unit        | Audit event contains `sandbox_backend` after execution                         | Verify `AuditEvent.security.sandbox_backend == Some("none")` or backend name                                          |
| Unit        | `SandboxConfig` serde round-trip with `require` field                          | Serialize/deserialize TOML with `require = true`, assert field preserved                                              |
| Unit        | `SandboxConfig` backward compat (missing `require`)                            | Deserialize TOML without `require` field, assert defaults to `false`                                                  |
| Integration | End-to-end: sandbox detection → ShellTool → audit log                          | Create `ShellTool` with real `NoopSandbox`, execute allowed command, read audit log, verify `sandbox_backend` present |
| Unit        | `BrowserTool` sidecar health-check logs audit event                            | Mock HTTP endpoint, verify `SecurityEvent` audit entry emitted                                                        |
| Unit        | `BrowserTool` sidecar health-check failure is non-fatal                        | Mock failing endpoint, verify tool still initializes (when `require=false`)                                           |

### Mock Sandbox for Tests

```rust
#[cfg(test)]
struct MockSandbox {
    wrap_called: std::sync::atomic::AtomicBool,
    name: &'static str,
}

impl Sandbox for MockSandbox {
    fn wrap_command(&self, _cmd: &mut Command) -> std::io::Result<()> {
        self.wrap_called.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    fn is_available(&self) -> bool { true }
    fn name(&self) -> &str { self.name }
    fn description(&self) -> &str { "Mock sandbox for testing" }
}
```

## Migration / Rollout

No migration required. The change is additive and backward-compatible:

1. **Default behavior unchanged**: `sandbox.require` defaults to `false`. Existing deployments that
   don't set this field continue with the same NoopSandbox fallback.
2. **No data migration**: No schema changes, no persistent state changes.
3. **Operator opt-in**: Operators enable enforcement by adding `require = true` to their config.
4. **Instant rollback**: Set `sandbox.require = false` and `sandbox.backend = "none"` to restore
   exact pre-change behavior. Or revert the PR entirely — the change is contained.

## Open Questions

- [x] Should `wrap_command()` be called for ALL shell executions? → **Yes**, defense in depth.
  NoopSandbox is a no-op so zero overhead.
- [x] Should fail-closed be the default? → **No**, opt-in via `require = true` to preserve backward
  compat.
- [x] How does sandbox interact with `RuntimeAdapter.build_shell_command()`? → Sandbox wraps AFTER
  runtime builds the command. The runtime produces a `Command`, then `wrap_command()` modifies it (
  prefixes, environment, namespace flags).
- [x] Should computer-use sidecar actions require sandbox wrapping of the sidecar process? → **Not
  in this change**. The sidecar runs as a separate process. We add a health-check to verify and log
  its isolation level, but wrapping the sidecar process itself is out of scope (the sidecar is
  deployed externally).
- [x] Where exactly is `create_sandbox()` called in the startup path? → **Nowhere in production
  code.** `create_sandbox()` is defined in `detect.rs`, re-exported in `security/mod.rs`, but only
  called from tests within `detect.rs`. It is never invoked from `main.rs`, agent initialization, or
  tool factory code. This confirms the exploration finding that the sandbox infrastructure is
  completely dead code. The implementation must add the `create_sandbox()` call to the
  startup/initialization path (wherever `all_tools` or `all_tools_with_runtime` is called) and
  thread the result through to `ShellTool`.
