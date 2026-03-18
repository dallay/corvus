# Exploration: First-Run Web Dashboard Activation

## Current Findings

### Relevant entry points and behavior today

- `clients/agent-runtime/src/main.rs`
  - `onboard --interactive` dispatches to `onboard::run_wizard()` on a blocking thread.
  - No dashboard-specific onboarding flag/flow exists.
- `clients/agent-runtime/src/onboard/wizard.rs`
  - `run_wizard()` builds config, saves it, prints final summary/next steps, and optionally asks to
    launch channels.
  - Final onboarding output includes provider/model/memory/channels/gateway summary, but no web
    dashboard activation prompt.
  - Quick setup prints `Gateway: pairing required (127.0.0.1:8080)` while schema default gateway
    port is 3000 (stale UX detail).
- `clients/agent-runtime/src/config/schema.rs`
  - Gateway defaults are secure by default: host `127.0.0.1`, port `3000`, `require_pairing = true`.
- `clients/agent-runtime/src/gateway/mod.rs`
  - When `corvus gateway` runs, it prints local URL, `/pair`, `/web/admin/config`,
    `/web/admin/options`, and pairing instructions.
  - Pairing code is shown only when gateway is running and no paired tokens exist.
  - `GET /health` exposes paired status (`paired: bool`) without secrets.
- `clients/agent-runtime/src/security/pairing.rs`
  - Pairing tokens are generated once, returned once, and stored as SHA-256 hashes.
  - `require_pairing` governs auth checks and must remain preserved by default.
- `clients/agent-runtime/src/gateway/utils.rs`
  - Admin endpoints require local origin/referer (`localhost` or `127.0.0.1`) and bearer token when
    pairing is enabled.
  - Direct curl calls to `/web/admin/*` without Origin/Referer are intentionally rejected.
- `clients/web/apps/dashboard/src/composables/useConfig.ts`
  - Dashboard defaults to `/api` as proxied gateway base URL under `http://corvus.localhost`.
  - Pair flow: `POST /pair` with `X-Pairing-Code`, then bearer-based `GET /web/admin/options` +
    `GET /web/admin/config`.
  - Failure messaging is coarse (`auth.loadError`), not deterministic diagnosis.
- `clients/web/apps/dashboard/package.json`, `clients/web/README.md`, `Makefile`
  - Proxied dev entrypoint is `http://corvus.localhost`.
  - Launch commands exist (`make dev-up` / `./dev/cli.sh up-dashboard`).

### Operational command surface currently relevant

- `corvus onboard --interactive`
- `corvus gateway`
- `corvus status`
- `corvus doctor`
- `make dev-up`
- `./dev/cli.sh up-dashboard`

## Gap Analysis (RF1-RF5 + Scenarios A-D)

### RF1: Final onboarding prompt in interactive onboarding

- **Gap:** Missing. No final prompt asks whether to activate web dashboard now.

### RF2: If accepted, guide web activation (URL, pairing status, pairing token usage, optional browser open)

- **Partially available elsewhere, not wired into onboarding:**
  - URL and pairing guidance exist only in gateway startup logs (`corvus gateway`).
  - Dashboard base URL conventions exist in web app (`/api` behind `http://corvus.localhost`).
- **Gap in onboarding:**
  - No accept branch in onboarding.
  - No pairing status summary at onboarding end.
  - No explicit token-usage walkthrough for dashboard auth.
  - No optional browser-open step from onboarding.

### RF3: Keep CLI-only flow unchanged when declined

- **Current state:** Trivially true because the prompt does not exist.
- **Potential regression risk:** If adding prompt, ensure decline path does not alter existing next
  steps, channel autostart prompt, or env signaling (`CORVUS_AUTOSTART_CHANNELS`).

### RF4: Deterministic diagnosis on web availability failure + exact fallback commands

- **Gap:** Missing in onboarding.
- **Current diagnostics are fragmented:**
  - `corvus doctor` validates config/workspace/daemon freshness but does not provide
    dashboard-specific activation diagnosis.
  - Dashboard frontend uses generic load errors.
- **Additional constraint:** Manual fallback must respect admin origin guard; direct `/web/admin/*`
  curl guidance is unsafe/incompatible unless origin constraints are explicitly handled.

### RF5: Quick resume path later

- **Gap:** No dedicated resume command or dashboard-specific status guidance from onboarding.
- **Current fallback path:** generic commands (`corvus status`, `corvus gateway`, `make dev-up`,
  `./dev/cli.sh up-dashboard`) exist but are not packaged as a "resume dashboard setup" flow.

### Scenario mapping

- **Scenario A (accept + web available):** Not implemented in onboarding.
- **Scenario B (decline):** Existing behavior continues, but no explicit decline branch yet.
- **Scenario C (accept + web unavailable):** No deterministic branching/diagnosis or exact fallback
  sequence.
- **Scenario D (resume later):** No dedicated path; only implicit/manual commands.

## Risks and Constraints

- **Security invariants (must preserve):**
  - Keep `gateway.require_pairing = true` default semantics and no silent weakening.
  - Avoid logging or re-printing bearer tokens beyond explicit controlled pairing flow.
  - Do not expose secrets in onboarding output.
- **Compatibility constraints:**
  - Existing CLI UX flow should remain stable (especially channel autostart prompt sequence).
  - Onboarding runs in a blocking thread; any availability probes must be bounded/time-limited to
    avoid UX stalls.
- **Architecture constraints:**
  - Pairing code/token generation currently occurs in live gateway process (`PairingGuard`), not
    during onboarding config generation.
  - Admin API origin guard requires browser-origin traffic for `/web/admin/*`; fallback guidance
    should direct users to dashboard UI flow, not raw API calls.
- **Browser open constraints:**
  - Existing `browser_open` tool is agent-tooling scoped, HTTPS-only, and blocks localhost/private
    hosts; it is not a drop-in for opening local dashboard URL from onboarding.

## Suggested Scope Boundaries

### In scope (explore -> proposal candidate)

- Add final onboarding prompt in `run_wizard()` asking to activate web dashboard now.
- Add explicit accept/decline branching:
  - Decline = preserve current CLI-only behavior and next steps.
  - Accept = print dashboard URL + gateway/pairing status + token usage instructions.
- Add deterministic availability diagnosis for accept flow (bounded checks) and emit exact fallback
  commands.
- Add clear resume-later guidance in onboarding output (at minimum command-level guidance).

### Out of scope (for this change)

- Reworking pairing protocol, token format, or auth model.
- Relaxing admin origin guard or changing `/web/admin/*` security policy.
- Building a full new dashboard orchestration service/process manager.
- Broad dashboard frontend refactors unrelated to first-run activation guidance.

## Implementation Hotspots (for proposal/design)

- `clients/agent-runtime/src/onboard/wizard.rs`
  - Primary hotspot: append final prompt and branch logic after summary.
  - Add helper(s) for deterministic diagnosis and rendering exact fallback commands.
- `clients/agent-runtime/src/main.rs`
  - Likely unchanged unless introducing a dedicated resume command.
- `clients/agent-runtime/src/gateway/mod.rs` and `clients/agent-runtime/src/security/pairing.rs`
  - Reference-only for preserving pairing semantics and startup messaging consistency.
- `clients/web/apps/dashboard/src/composables/useConfig.ts` (optional follow-up)
  - Not required for onboarding prompt itself, but relevant if alignment with deterministic
    diagnosis messaging is desired later.

## Ready for Proposal

Yes. The repository already has all core primitives (gateway pairing, dashboard URLs, startup
commands), but onboarding lacks orchestration. Proposal should formalize the accept/decline decision
flow, deterministic diagnosis matrix, secure fallback commands, and resume path wording while
preserving existing security defaults.
