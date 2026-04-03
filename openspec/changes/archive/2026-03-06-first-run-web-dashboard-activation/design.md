# Technical Design: First-Run Web Dashboard Activation

## Status

Proposed

## Design Goals

- Implement RF1-RF5 with security-first defaults and deterministic behavior.
- Preserve existing onboarding UX for CLI-only users.
- Keep all checks local, bounded, and independent of external network.
- Avoid any changes to pairing/token protocol or admin endpoint security policy.

## Architecture and Flow

### Placement in Onboarding Flow (RF1, RF3)

`corvus onboard --interactive` remains the entry point (`clients/agent-runtime/src/main.rs`) and
continues to call `onboard::run_wizard()` (`clients/agent-runtime/src/onboard/wizard.rs`).

Design insertion point in `run_wizard()`:

1. Existing config questions and summary complete.
2. Existing channel autostart question (if present in current flow) remains in the same order.
3. New final optional prompt: "Activate web dashboard now?".
4. Branch to accept/decline handler.
5. Print completion and next-step messaging.

This preserves functional parity for non-web users: if user declines, no new mandatory steps or side
effects are introduced.

### Branching Model (RF2, RF4, RF5)

Introduce a dedicated internal flow layer under onboarding (design-level module boundary):

- `DashboardActivationDecision`: `Accept | Decline`
- `ActivationDiagnosis`:
    - `GatewayNotRunning`
    - `GatewayRunningPairingRequired`
    - `GatewayRunningAlreadyPaired`
    - `DashboardUiUnavailable`
    - `UnknownLocalFailure` (deterministic catch-all)

Proposed orchestrator method in onboarding domain (name illustrative):

- `run_dashboard_activation_flow(config, io, clock) -> ActivationOutcome`

`ActivationOutcome` includes:

- `decision`
- `diagnosis` (for accept path)
- `browser_open_attempted`
- `browser_open_result` (`Opened | Unsupported | FailedNonFatal`)
- `resume_block_rendered`

## Data and Control Flow by Path

### 1) Accept Path (Scenario A/C)

Control flow:

1. Render one-screen activation guide (3-5 steps):

- Proxied local entrypoint (`http://corvus.localhost`)
- Gateway URL/API base (`/api` under the same origin)
- Pairing instruction through existing proxied `/api/pair` flow
- Optional browser open attempt (non-fatal)

1. Run bounded local diagnosis checks (below).
2. Print diagnosed state and exact fallback command block.
3. Always print resume-later block.

Bounded checks (deterministic order):

1. Run a direct gateway probe (`gatewayHealthProbe`) against the gateway's native local health /
   pairing surface with a fixed timeout budget so gateway-only failures are distinguishable from
   proxy/UI failures.
2. If the direct gateway probe is healthy, evaluate `paired` status only (boolean, no token
   exposure).
3. Run a separate proxied/UI reachability probe against `http://corvus.localhost/api/health` to
   determine whether Caddy + dashboard entrypoint are reachable.
4. Determine dashboard UI availability from execution environment capability:
    - If CLI can launch/open browser in current runtime, mark available.
    - If unsupported/fails, classify as `DashboardUiUnavailable` for guidance only, not a hard
      onboarding error.
5. Combine direct gateway probe + proxied/UI probe results to classify `GatewayNotRunning`,
   `GatewayRunningPairingRequired`, `GatewayRunningAlreadyPaired`, or `DashboardUiUnavailable`
   deterministically.

Timeout defaults (design baseline):

- Per request timeout: 500 ms
- One retry after 150 ms jitter-free backoff
- Total diagnosis budget: <= 1.5 s

These values keep the CLI responsive while handling slower local startup conditions.

### 2) Decline Path (Scenario B)

Control flow:

1. User selects decline.
2. Skip all diagnosis and browser-open behavior.
3. Continue existing CLI-only completion messaging unchanged.
4. Add concise resume-later block only (no workflow mutation).

No config mutation, no daemon management, no pairing changes.

### 3) Unavailable/Fallback Path (Scenario C)

If accept path diagnosis finds missing prerequisites:

- Print a deterministic diagnosis line: `Dashboard activation status: <STATE_CODE>`
- Print state-specific recovery commands (copy/paste).
- Never suggest insecure direct calls to `/web/admin/*`.

State-to-fallback mapping:

- `GatewayNotRunning`
    - `corvus gateway`
    - `make dev-up`
    - `./dev/cli.sh up-dashboard`
    - Open `http://corvus.localhost`
- `GatewayRunningPairingRequired`
    - Open `http://corvus.localhost`
    - Complete pairing using gateway-provided code at `/api/pair`
- `GatewayRunningAlreadyPaired`
    - Open `http://corvus.localhost`
    - Continue to dashboard configuration view
- `DashboardUiUnavailable`
    - Start UI: `make dev-up`
    - Then run `./dev/cli.sh up-dashboard`
    - Open `http://corvus.localhost`
- `UnknownLocalFailure`
    - `corvus status`
    - `corvus doctor`
    - `corvus gateway`
    - `make dev-up`
    - `./dev/cli.sh up-dashboard`

### 4) Resume-Later Path (Scenario D)

Always render a compact resume block when user declines or accept path does not complete cleanly:

1. `corvus status`
2. `corvus gateway`
3. `make dev-up`
4. `./dev/cli.sh up-dashboard`
5. Open `http://corvus.localhost` and pair via `/api/pair`

This uses existing commands only, preserving backward compatibility and reducing scope risk.

## Interfaces and Touchpoints

### Runtime/CLI modules

- `clients/agent-runtime/src/onboard/wizard.rs`
    - Add final optional prompt and branch dispatcher.
    - Integrate rendering of guide, diagnosis, fallback, and resume blocks.
- `clients/agent-runtime/src/main.rs`
    - No behavior change expected; only reference for command entry continuity.
- `clients/agent-runtime/src/gateway/mod.rs`
    - Reuse canonical local URL and pairing messaging conventions for consistency.
- `clients/agent-runtime/src/config/schema.rs`
    - Source of truth for secure defaults (`require_pairing=true`; local entrypoint now proxied via
      `corvus.localhost`).
- `clients/agent-runtime/src/security/pairing.rs`
    - Security invariant reference only; no protocol/storage changes.
- `clients/agent-runtime/src/gateway/utils.rs`
    - Security invariant reference: origin/referer constraints remain unchanged.

### Web/dashboard touchpoints

- `clients/web/apps/dashboard/src/composables/useConfig.ts`
    - Align error display terminology with deterministic diagnosis labels in a follow-up (
      non-blocking).
- `clients/web/README.md` (and/or onboarding docs)
    - Document first-run activation and resume flow commands.

## Error Taxonomy and Deterministic Diagnostics

### User-facing status codes

Use stable, grep-friendly codes in onboarding output:

- `DASH-001 GatewayNotRunning`
- `DASH-002 GatewayRunningPairingRequired`
- `DASH-003 GatewayRunningAlreadyPaired`
- `DASH-004 DashboardUiUnavailable`
- `DASH-999 UnknownLocalFailure`

### Message format

Each diagnosis output block:

1. One-line status with code.
2. One-line cause (local and concrete).
3. One recovery command block.
4. One resume-later reminder.

### Determinism rules

- Fixed probe order and timeout values.
- No branching on non-deterministic external services.
- Fallback to `DASH-999` for unexpected local errors, with safe generic commands.

## Security Considerations (NFR-S1)

- Keep `gateway.require_pairing = true` default behavior unchanged.
- Do not print bearer tokens, token hashes, auth headers, or persisted secret paths.
- Pairing code display remains only within existing controlled gateway flow.
- Do not recommend bypass paths that violate origin/referer protections.
- Keep direct gateway probes local/private and read-only, and keep proxied entrypoint probes
  limited to `http://corvus.localhost/api/health` for UI/proxy reachability.
- Ensure diagnostic logs are sanitized and avoid embedding user-provided strings without escaping.

## Test Strategy and Requirement Mapping

### Unit tests

- Onboarding prompt order and optional wording (RF1).
- Decline branch parity snapshots/behavior (RF3, NFR-C1).
- Diagnosis mapper from probe outcomes to `DASH-00x` codes (RF4, NFR-R1).
- Output redaction assertions: no token/bearer leakage (NFR-S1).
- Resume block rendering triggers (RF5).

Potential file touchpoint:

- `clients/agent-runtime/src/onboard/wizard.rs` tests (or nearby onboarding test module)

### Integration tests

- Simulated local gateway states:
    - down -> `DASH-001`
    - up+unpaired -> `DASH-002`
    - up+paired -> `DASH-003`
- Browser-open unsupported/failure path remains non-fatal and yields guidance (`DASH-004` when
  applicable).
- Copy/paste command blocks are present and stable (RF4, RF5).

### E2E/CLI scenario tests

- Scenario A: accept + available path yields one-screen 3-5 steps and canonical URLs.
- Scenario B: decline yields functionally equivalent CLI-only completion.
- Scenario C: accept + unavailable shows deterministic diagnosis and exact fallback commands.
- Scenario D: resume commands succeed independently post-onboarding.

## Rollout, Compatibility, and Observability

### Rollout and compatibility

- Backward compatible by default: decline path mirrors current CLI flow.
- No migration required for config or tokens.
- Keep command examples aligned with project tooling (`make dev-up`, `./dev/cli.sh up-dashboard`,
  `corvus gateway`, `corvus status`, `corvus doctor`).

### Observability/logging guidance

- Emit structured debug logs (non-user-facing) for diagnosis internals:
    - probe start/end, timeout, result class, chosen `DASH` code.
- Do not include secrets or sensitive headers in logs.
- Keep user-facing output concise; reserve stack traces for verbose/debug mode only.

## Requirement-to-Design Traceability

- RF1 -> final optional prompt insertion in onboarding completion flow.
- RF2 -> one-screen 3-5 step guide + optional browser open.
- RF3 -> decline path skip-all behavior with CLI parity.
- RF4 -> fixed diagnosis state machine + command mapping + status codes.
- RF5 -> explicit resume block with existing commands.
- NFR-S1 -> token-safe output and unchanged pairing security model.
- NFR-U1 -> compact actionable messaging with canonical defaults.
- NFR-R1 -> bounded probe budget and non-fatal optional browser action.
- NFR-C1 -> no protocol/config breaking changes.

## Decisions and Open Items

Resolved in this design:

1. Resume guidance uses existing commands only (no new alias in this change).
2. Optional browser-open targets the proxied local entrypoint (`http://corvus.localhost`) only.
3. Deterministic bounded checks use 500 ms timeout, 1 retry, <= 1.5 s budget.
4. Diagnosis is implemented for onboarding output first, with extraction-ready interfaces for later
   reuse.

Unresolved requiring user input:

- None required to proceed with implementation based on current scope.
