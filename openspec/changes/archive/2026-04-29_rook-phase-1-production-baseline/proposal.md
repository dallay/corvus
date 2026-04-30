# Proposal: Rook Phase 1 Production Baseline

## Intent

Rook already has the core pieces of a local gateway product — HTTP serving, admin and gateway routes, SQLite-backed state, structured logs, and middleware seams — but its production baseline is still incomplete. Operators cannot yet inspect the effective configuration, validate a deployment before serving traffic, distinguish process liveness from service readiness, or scrape a stable metrics surface. Several of the key operational entrypoints are still placeholders.

This change delivers the Phase 1 production baseline for the existing gateway surface by making configuration assembly explicit and exportable, turning `rook doctor` into a real diagnostic command, separating readiness from liveness, and exposing a first observability/metrics baseline. The goal is to make Rook operable in development and early production-style environments without changing its existing gateway contract or expanding into later-phase auth, billing, or remote dependency management.

## Scope

### In Scope

1. **Effective config model and export**
   - Introduce a first-class Rook config model for the runtime concerns Rook already owns.
   - Support explicit precedence across built-in defaults, config file values, `ROOK_*` environment overrides, and CLI flags.
   - Implement `rook config export` so operators can inspect the effective configuration used by `serve`.
   - Redact sensitive values in exported output.

2. **`rook doctor` diagnostics**
   - Replace the placeholder command with deterministic operational diagnostics.
   - Validate that config loads and passes validation.
   - Check that the database path is usable and startup migrations can be opened/applied.
   - Verify embedded dashboard assets and inbound auth configuration are internally consistent.
   - Return machine-readable pass/warn/fail results and a non-zero exit code when critical checks fail.

3. **Readiness and liveness health reporting**
   - Preserve a simple liveness signal for orchestration and uptime checks.
   - Add a distinct readiness signal that reflects whether critical local dependencies required to serve traffic are available.
   - Base readiness on local startup/runtime prerequisites such as validated config, successful DB initialization, and available application assets/router state.
   - Keep readiness independent from transient upstream provider health in Phase 1 so the process does not flap on remote failures.

4. **Observability and metrics baseline**
   - Add a minimal scrape-friendly metrics surface for production operations.
   - Expose request counts and latency for core gateway/admin surfaces.
   - Track baseline operational counters for rate-limit rejections, idempotency outcomes, and upstream request result classes where those hooks already exist.
   - Prefer middleware-level and shared helper instrumentation over handler-by-handler duplication.

### Out of Scope

- Admin auth, pairing, or broader exposure hardening beyond the current local-first posture.
- Provider-specific remote health probes or readiness checks against all upstream vendors.
- Billing, usage accounting, or advanced observability dashboards.
- Large-scale configuration expansion beyond fields already needed by startup and current runtime behavior.
- Phase 2/3 operator tooling beyond `config export`, `doctor`, health, and baseline metrics.

## Approach

This proposal keeps the gateway domain as the source of truth and treats Phase 1 as an operational hardening pass over the existing Rook server. The implementation should centralize config loading and validation so `serve`, `rook doctor`, and `rook config export` all use the same effective configuration path. Health semantics should be expressed as explicit liveness and readiness contracts rather than a single coarse endpoint, with readiness limited to local critical dependencies. Metrics should be introduced through stable middleware and shared runtime hooks so the baseline can expand later without reshaping handlers.

The recommended delivery order remains:
1. config export + `ROOK_*` overrides
2. `rook doctor`
3. readiness/liveness reporting
4. observability/metrics baseline

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/main.rs` | Modified | Replace placeholder CLI branches for `rook config export` and `rook doctor`; route all operational commands through the shared config/diagnostic path. |
| `clients/rook/src/config/` | Modified/New | Own the first-class config model, file loading, `ROOK_*` environment overrides, validation, and redacted export formatting. |
| `clients/rook/src/server/mod.rs` | Modified | Consume the shared effective config and expose the runtime state needed for readiness and metrics bootstrap. |
| `clients/rook/src/admin/mod.rs` and `clients/rook/src/admin/handlers.rs` | Modified | Expose explicit liveness/readiness endpoints on the existing admin/API surface and preserve stable health response shapes. |
| `clients/rook/src/doctor/` | New | Implement deterministic operator diagnostics and pass/warn/fail reporting. |
| `clients/rook/src/health/` | New | Define shared health evaluation types and local dependency checks used by readiness reporting. |
| `clients/rook/src/observability/` | New | Host metrics registry/bootstrap, shared counters/histograms, and any scrape handler wiring. |
| Existing middleware and gateway hooks (`transport`, rate-limit, idempotency, upstream request paths) | Modified | Emit baseline metrics from stable shared seams rather than individual handlers. |
| `clients/rook/README.md` | Modified (after implementation) | Update operator documentation for `config export`, `ROOK_*` overrides, `rook doctor`, readiness/liveness endpoints, and metrics exposure. |
| `openspec/specs/client-surfaces/gateway-api.md` and related gateway spec artifacts | Modified | Record the new operational expectations against the existing gateway spec domain rather than creating a separate domain. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Config drift between `serve`, `doctor`, and `config export` | Medium | Centralize effective config assembly and validation in one reusable path with precedence tests. |
| Secret leakage through exported config | Medium | Redact secrets by default and add explicit export tests covering sensitive fields. |
| Readiness becomes too brittle by depending on external providers | Medium | Limit Phase 1 readiness to local critical dependencies only; leave upstream availability for metrics/logging and later health work. |
| Metrics instrumentation becomes fragmented across handlers | Medium | Prefer middleware and shared helper hooks; keep the first metric set intentionally small. |
| New health/metrics endpoints alter existing local operator expectations | Low | Add endpoints additively, preserve existing admin/gateway routing posture, and document stable response shapes. |

## Rollback Plan

Phase 1 should be structured so each operational slice can be reverted with minimal blast radius:

1. **Config/export rollback** — remove the new export command path and env override loader, and fall back to the prior startup assembly. Because this work is additive around configuration entrypoints, rollback should not require changing gateway request handling.
2. **Doctor rollback** — revert the new doctor module and CLI wiring, restoring the previous placeholder command without affecting serving behavior.
3. **Health rollback** — remove explicit readiness/liveness route additions and restore the previous coarse health surface if the new semantics cause orchestration issues.
4. **Metrics rollback** — disable router/middleware metrics exposure and revert the observability module without touching core request routing.

No data migration is required for this phase, and rollback should not require undoing the existing `/v1` gateway or `/api` admin surfaces.

## Dependencies

- Existing Rook server composition and gateway/admin routing remain the runtime host for all Phase 1 changes.
- Existing registry, DB startup, inbound auth, rate-limit, idempotency, and upstream request paths provide the integration seams for diagnostics, readiness, and metrics.
- Gateway spec artifacts remain the correct spec domain for documenting the new operational baseline.
