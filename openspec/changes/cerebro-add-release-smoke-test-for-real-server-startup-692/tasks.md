# Tasks: Add release smoke test for real Cerebro server startup

## Phase 1: Workflow smoke scaffold

- [x] 1.1 Update `.github/workflows/_build-cerebro-binaries.yml` to replace the Linux release `cerebro --help` smoke check with a dedicated real-startup smoke step that runs only for the Linux release artifact.
- [x] 1.2 In `.github/workflows/_build-cerebro-binaries.yml`, write the temporary `cerebro-smoke.toml` CI config with explicit `host`, `port`, `storage_mode = "in_memory"`, `auth_token`, and `[tui].enabled = false` values.
- [x] 1.3 In `.github/workflows/_build-cerebro-binaries.yml`, add inline shell setup for probe payload files, background `cerebro serve --config` launch, PID capture, and `trap`-based cleanup that always stops the server and removes temp files.

## Phase 2: Health, readiness, and MCP assertions

- [x] 2.1 In `.github/workflows/_build-cerebro-binaries.yml`, implement bounded startup polling that repeatedly checks process liveness and `GET /healthz` until HTTP 200 or timeout.
- [x] 2.2 In `.github/workflows/_build-cerebro-binaries.yml`, add the readiness assertion that `GET /readyz` returns HTTP 200 for the generated in-memory config and fails the step otherwise.
- [x] 2.3 In `.github/workflows/_build-cerebro-binaries.yml`, add the unauthenticated `POST /mcp` probe and assert the request is rejected when no Bearer token is sent.
- [x] 2.4 In `.github/workflows/_build-cerebro-binaries.yml`, add the authenticated `POST /mcp` `tools/list` probe with `Authorization: Bearer <token>` and validate a minimal JSON-RPC success shape: `jsonrpc = "2.0"`, expected `id`, no `error`, and `result.tools` as an array.

## Phase 3: Failure diagnostics and verification

- [x] 3.1 In `.github/workflows/_build-cerebro-binaries.yml`, ensure every timeout, early exit, or failed probe prints the captured Cerebro server log before the job fails.
- [x] 3.2 Verify `.github/workflows/_build-cerebro-binaries.yml` shell logic remains strict and reviewable by checking quoting, exit handling, cleanup order, and Linux-only scoping for the new smoke step.
- [x] 3.3 Run a local workflow-equivalent smoke check from `crates/cerebro` or the built release binary path: start `cerebro serve` with the temporary config, confirm `/healthz` and `/readyz` return 200, confirm `/mcp` rejects without auth, and confirm authenticated `tools/list` succeeds.
