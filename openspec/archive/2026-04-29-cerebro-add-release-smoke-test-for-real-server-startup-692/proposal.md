# Proposal: Add release smoke test for real Cerebro server startup

## Intent

Strengthen Cerebro release validation by proving the packaged binary can start a real HTTP/MCP server in CI, not just print `--help`. This closes the gap where a broken runtime startup path, config parsing issue, bind failure, readiness regression, or MCP auth failure could still pass the release workflow.

## Scope

### In Scope
- Add a Linux release-workflow smoke step that starts the built `cerebro` binary in the background with a CI-specific temporary config.
- Validate basic server startup behavior through `GET /healthz`, `GET /readyz`, and minimal `POST /mcp` probes with and without Bearer auth.
- Ensure the workflow polls for startup, captures logs, and always cleans up the background server process on success or failure.

### Out of Scope
- Building a broader integration-test harness or replacing existing Rust test coverage.
- Expanding release smoke validation to other operating systems, packaging formats, or non-release workflows.
- Deep behavioral testing of Cerebro tools, persistence semantics, or end-to-end memory workflows beyond a minimal MCP handshake.

## Approach

Update the Linux release workflow path in `.github/workflows/_build-cerebro-binaries.yml` to run a real startup smoke test after the binary is built. The workflow will generate a temporary config tailored for CI instead of relying on the default configuration, explicitly setting loopback host/bind values, a fixed test port, a non-embedded storage mode suitable for CI (`in_memory` preferred), and a known `auth_token`.

The smoke test will launch `cerebro serve --config <temp-config>` in the background, poll until either `/healthz` responds or startup times out, then verify:
- `/healthz` returns HTTP 200
- `/readyz` returns the expected readiness result for the temporary config
- `POST /mcp` without auth is rejected
- `POST /mcp` with `Authorization: Bearer <token>` returns a minimal valid JSON-RPC response for a safe probe such as `tools/list`

On failure, the workflow will dump captured server logs before exiting. On completion, it will terminate the background process so the release job remains deterministic and leak-free.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `.github/workflows/_build-cerebro-binaries.yml` | Modified | Add real startup smoke validation for the Linux release build artifact. |
| `openspec/specs/gateway/spec.md` | Modified | Capture the release-smoke expectation for real HTTP bind, health/readiness, and MCP auth posture as part of the source-of-truth domain. |
| `crates/cerebro/` | Indirectly validated | Existing `serve`, config loading, HTTP endpoints, and MCP auth paths are exercised by release CI without broad product-surface changes. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Smoke test flakes due to startup timing or port contention | Medium | Use explicit polling with timeout, bind to loopback with a fixed CI-owned port, and fail with logs for diagnosis. |
| Default config assumptions make CI startup non-deterministic | High | Generate a dedicated temporary config with explicit storage mode, port, host, and auth token. |
| MCP probe asserts the wrong failure/success shape | Medium | Keep the probe minimal (`tools/list`) and verify only the contractually stable auth/reply expectations needed for smoke coverage. |

## Rollback Plan

Revert the workflow smoke-test additions and corresponding spec delta if the release path becomes unstable or blocks urgent publishing. This restores the previous `cerebro --help`-only validation while preserving existing build artifact generation.

## Dependencies

- Existing Linux release build job in `.github/workflows/_build-cerebro-binaries.yml`
- Current `cerebro serve` support for `--config`, `/healthz`, `/readyz`, `/mcp`, and Bearer-token auth
- CI runner support for launching a background process and issuing HTTP requests with standard shell tooling

## Success Criteria

- [ ] The Linux release workflow starts the built `cerebro` binary with a CI-specific temporary config rather than validating only `--help`.
- [ ] The release smoke step passes only when `/healthz` succeeds, `/readyz` matches expected readiness, unauthenticated MCP is rejected, and authenticated `tools/list` returns a minimal valid JSON-RPC response.
- [ ] Failures surface actionable diagnostics by dumping startup logs and cleaning up the background process before job exit.
