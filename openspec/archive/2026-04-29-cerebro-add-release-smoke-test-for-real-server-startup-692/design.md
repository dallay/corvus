# Design: Add release smoke test for real Cerebro server startup

## Technical Approach

Add a Linux-only release smoke step to `.github/workflows/_build-cerebro-binaries.yml` that runs the built `cerebro` release binary as a background process using a temporary CI config. The step will avoid default startup assumptions by writing an explicit TOML config with loopback bind, fixed CI port, `storage_mode = "in_memory"`, TUI disabled, and a deterministic `auth_token`.

After launch, the workflow will poll for process liveness and `/healthz` readiness within a bounded timeout, then assert `/readyz`, unauthenticated MCP rejection, and authenticated `tools/list` success against the real HTTP server. The step will always dump captured logs on failure and always terminate the background process before exit.

## Architecture Decisions

### Decision: Keep smoke validation inside the existing Linux release workflow

**Choice**: Extend `.github/workflows/_build-cerebro-binaries.yml` with a Linux-only real-startup smoke step after build.
**Alternatives considered**: Add a separate integration workflow; add a new Rust integration test harness; expand smoke validation to every platform.
**Rationale**: The approved scope is release validation for the packaged Linux binary. Reusing the existing release job keeps the check close to the artifact being published and avoids introducing a broader test framework.

### Decision: Generate an explicit temporary TOML config in CI

**Choice**: Write a short temporary config file during the workflow step with explicit `host`, `port`, `storage_mode = "in_memory"`, `auth_token`, and `tui.enabled = false`.
**Alternatives considered**: Rely on defaults; rely on environment overrides only; use embedded Surreal configuration.
**Rationale**: Current startup defaults can select `EmbeddedSurreal`, which has additional credential and bind requirements. An explicit CI config makes the smoke test deterministic and directly exercises the intended release path without Surreal-specific setup.

### Decision: Use HTTP polling plus minimal JSON-RPC assertions

**Choice**: Poll `/healthz` with `curl` until success or timeout, then validate `/readyz`, unauthenticated `POST /mcp`, and authenticated `tools/list`.
**Alternatives considered**: Sleep-based startup waits; probing only the TCP port; deeper tool execution via `tools/call`.
**Rationale**: `/healthz` is the lightest real-service startup signal, `/readyz` confirms storage readiness for the CI config, and `tools/list` is the least flaky authenticated MCP probe because it does not depend on memory mutation semantics.

### Decision: Prefer inline shell logic with strict cleanup and log capture

**Choice**: Implement the smoke logic directly in the workflow step using `bash`, `trap`, `curl`, and JSON body files.
**Alternatives considered**: Add a reusable external script or custom test utility.
**Rationale**: The change should stay narrow. Inline shell keeps the implementation local to release CI while still allowing robust cleanup, bounded polling, and actionable diagnostics.

## Data Flow

```text
GitHub Actions Linux job
        |
        v
build release binary
        |
        v
write temporary cerebro-smoke.toml + JSON probe bodies
        |
        v
start `cerebro serve --config <temp-config>` in background
        |
        v
poll `/healthz` + check process still alive
        |
        v
assert `/readyz` == HTTP 200
        |
        +--> POST /mcp without Authorization -> rejected
        |
        +--> POST /mcp with Bearer token + `tools/list` -> JSON-RPC success
        |
        v
on any failure: emit captured server log
        |
        v
always terminate background server and exit deterministically
```

### Sequence

```text
workflow step -> write temp config
workflow step -> launch cerebro serve
workflow step -> poll GET /healthz
cerebro -> bind listener + initialize storage
workflow step -> GET /readyz
workflow step -> POST /mcp (no auth)
workflow step -> POST /mcp (Bearer token, tools/list)
workflow step -> kill process + remove temp files
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `.github/workflows/_build-cerebro-binaries.yml` | Modify | Replace Linux release smoke coverage from `--help`-only validation to real server startup validation with background process handling, probes, logs, and cleanup. |
| `openspec/changes/cerebro-add-release-smoke-test-for-real-server-startup-692/design.md` | Create | Technical design for the release smoke workflow change. |

## Interfaces / Contracts

No product API changes are required. The workflow will consume existing contracts already implemented by `clients/cerebro`:

- `cerebro serve --config <path>` starts the HTTP server with config from TOML.
- `GET /healthz` returns HTTP `200` when the process is serving requests.
- `GET /readyz` returns HTTP `200` for a ready in-memory service.
- `POST /mcp` without valid Bearer auth yields a rejected JSON-RPC response.
- `POST /mcp` with valid Bearer auth and a `tools/list` request yields JSON-RPC `2.0` with a non-error `result.tools` array.

Expected temporary config shape:

```toml
host = "127.0.0.1"
port = 4040
storage_mode = "in_memory"
auth_token = "ci-smoke-token"

[tui]
enabled = false
```

Expected authenticated minimal probe body:

```json
{
  "jsonrpc": "2.0",
  "id": "smoke-tools-list",
  "method": "tools/list"
}
```

Validation rules inside the workflow step:

- Startup succeeds only if `/healthz` returns `200` before timeout and the process stays alive.
- Readiness succeeds only if `/readyz` returns `200` for the generated in-memory config.
- Unauthenticated MCP succeeds only if rejection is observed; acceptance is a failure.
- Authenticated MCP succeeds only if the response contains `jsonrpc = "2.0"`, the expected `id`, no `error`, and a `result.tools` array.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Workflow smoke | Real Linux release binary startup | Run in GitHub Actions after `cargo build --release` for Linux target. |
| HTTP probe validation | Health and readiness behavior under CI config | Use `curl` status checks against `/healthz` and `/readyz` with bounded retries. |
| MCP auth validation | Reject missing auth, accept valid token | Send one unauthenticated and one authenticated JSON-RPC request to `/mcp`. |
| Failure diagnostics | Startup failures remain actionable | On timeout, probe failure, or early exit, print captured server log before failing. |

## Migration / Rollout

No migration required.

Rollout is limited to the Linux branch of the existing release workflow. Other OS artifact smoke checks remain unchanged for this change.

## Open Questions

- [ ] Confirm whether the Linux release matrix can safely reserve a single fixed port (default `4040`) or whether the workflow should choose a deterministic high port value specific to the smoke step.
- [ ] Confirm whether JSON validation should use Python (available on GitHub runners) or remain shell-only; Python is more robust for parsing the authenticated MCP response but adds a small amount of inline script logic.
