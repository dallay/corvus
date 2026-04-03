# Exploration: Enhance Auto-Update System

### Current State

Auto-update behavior is centralized in `clients/agent-runtime/src/update/mod.rs` and currently
covers three surfaces: CLI startup notices, daemon background polling, and in-conversation channel
nudges.

- **Release detection** uses GitHub `releases/latest` endpoints (`profiletailors/corvus`, fallback
  `dallay/corvus`) with a 2s HTTP timeout and a 24h cache TTL, persisted in
  `workspace/state/version_check.json`.
- **CLI visibility** is limited to `agent` and `status` command paths (
  `clients/agent-runtime/src/main.rs`), where a best-effort bounded check prints a banner with
  manual update commands.
- **Daemon visibility** runs a supervisor-managed updater worker (
  `clients/agent-runtime/src/daemon/mod.rs`) that periodically checks and pushes channel
  notifications when destinations are configured.
- **Channel flow** supports opportunistic in-conversation update mentions and nonce-based
  confirmation (`corvus update confirm <nonce>`) before attempting auto-install.
- **Auto-install execution** is currently a minimal strategy: try `npm`/`pnpm`/`yarn`/`bun` global
  install commands, otherwise return manual instructions. No install-method detection exists.
- **Config surface** has `updates.enabled`, `updates.check_interval_minutes`,
  `updates.confirmation_ttl_minutes`, and `updates.notify_destinations` in
  `clients/agent-runtime/src/config/schema.rs`; no dedicated env overrides for these fields exist
  yet. Only `CORVUS_DISABLE_UPDATE_CHECK` globally disables checks.
- **Security posture** is mixed: install script (`clients/web/apps/marketing/public/install`)
  verifies SHA-256 and stages binary writes, but runtime auto-install path does not verify artifacts
  itself and relies on package manager behavior.

### Key Touchpoints

- `clients/agent-runtime/src/update/mod.rs` — core update detection, notice text, daemon polling,
  nonce confirmation, and auto-install behavior.
- `clients/agent-runtime/src/main.rs` — CLI routing and startup banner trigger points; location for
  new `update` subcommands.
- `clients/agent-runtime/src/channels/mod.rs` — pre-memory nonce interception and opportunistic
  in-conversation update mention.
- `clients/agent-runtime/src/daemon/mod.rs` — updater worker supervision and daemon lifecycle
  coupling.
- `clients/agent-runtime/src/config/schema.rs` — update config schema/defaults and env-override
  extension point.
- `clients/agent-runtime/src/service/mod.rs` — service restart/start/stop hooks relevant to safe
  post-install daemon handling.
- `clients/agent-runtime/src/gateway/admin.rs` and
  `clients/web/apps/dashboard/src/types/admin-config.ts` — API/UI extension points for visible
  update indicators/configuration in dashboard clients.
- `clients/agent-runtime/npm/corvus-cli/lib/install.js` and
  `clients/web/apps/marketing/public/install` — installation-channel behavior differences (npm
  wrapper installer vs shell installer).
- `.github/workflows/_publish.yml` — release asset checksum generation; base for stronger artifact
  verification policy.

### Risks

- **Process safety/races**: update state locking is in-process (`OnceLock<Mutex<()>>`) only;
  concurrent CLI + daemon processes can still race on `version_check.json`.
- **Non-atomic persistence**: update state writes use direct file writes, unlike the config path's
  temp-file + rename strategy.
- **Install-method ambiguity**: runtime cannot reliably determine whether user installed via
  npm/pnpm/yarn/bun, direct binary, script, cargo, or homebrew.
- **Security gap in auto-install**: runtime-side auto-install does not perform explicit artifact
  integrity verification for binary/script paths.
- **Operational disruption risk**: applying updates while daemon/service is active can leave mixed
  binary/runtime state without coordinated restart/session handling.
- **Version/source drift**: mixed org/repo/package references (`profiletailors` vs `dallay`)
  increase risk of wrong source selection.

### Open Questions and Assumptions

- **Client scope**: "client UI indicators" is assumed to include CLI + channel conversations +
  dashboard/web admin surfaces, not native mobile/desktop apps in this repository.
- **Auto-install policy default**: assumed default should remain safe/explicit (check + notify by
  default, auto-install opt-in).
- **Trust model**: need confirmation whether checksum-only verification is acceptable, or whether
  signed provenance (e.g., Sigstore/GPG) is required for runtime auto-install.
- **Install methods in execution scope**: requirement includes detecting `npm/pnpm/yarn/bun`,
  binary/script, `homebrew`, and `cargo`; assumption is execution MAY be supported for subset
  initially, with graceful/manual fallback for unsupported methods.
- **Daemon handling contract**: need product decision on whether updater should auto-restart managed
  services or stage update and require explicit `corvus service restart`.
- **Channel confirmation UX**: assumption is nonce confirmation remains mandatory for
  channel-initiated install unless a strict local policy setting allows unattended updates.

### Recommended Scope Boundaries

- **In scope (phase 1)**
    - Add a first-class `corvus update` command group (`check`, `install`, `status`, and
      confirmation
      plumbing as needed).
    - Introduce install-method detection and persistence (detected + user-overridable) with a safe
      fallback matrix.
    - Expand update config with explicit policy knobs (auto-check cadence, auto-install mode,
      restart
      behavior, visibility channels) plus env overrides.
    - Implement process-safe/atomic update state and install transaction guards.
    - Unify notification payloads across CLI banner, in-conversation mention, and machine-readable
      indicator endpoints.
    - Add focused tests for detection, policy gating, atomic state transitions, confirmation safety,
      and command UX.
- **Out of scope (phase 1)**
    - Re-architecting release pipeline/package ecosystem beyond verification metadata consumption.
    - Building a full standalone update UI in unrelated clients; expose API/typed fields first, then
      incremental frontend adoption.
    - Force-updating running sessions without explicit restart strategy and rollback semantics.

### Ready for Proposal

Yes. The codebase already has a clear update nucleus and insertion points for proactive visibility,
safe auto-install policy, install-method detection, and client-facing indicators. Proposal should
lock security invariants first (verification + atomicity + restart safety), then define phased UX
rollout.
