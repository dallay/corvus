# Design: Rook Phase 1 Production Baseline

## Technical Approach

This change adds the first production-operations baseline to the existing Rook gateway without
changing its domain ownership or request-routing contract. The design keeps `gateway` as the
source-of-truth spec domain and treats Phase 1 as an additive hardening pass over the current
`clients/rook` binary.

The implementation should centralize effective configuration assembly first, then reuse that same
path for startup, diagnostics, and operator-visible export. Health semantics should remain local
and deterministic, with readiness focused on startup-critical dependencies already inside the Rook
process. Metrics should attach to existing transport, rate-limit, idempotency, and upstream helper
seams rather than being copy-pasted into individual handlers.

Phase 1 is intentionally bounded to:

- config export with `ROOK_*` environment overrides
- deterministic `rook doctor` diagnostics
- readiness/liveness health reporting
- a scrape-friendly observability baseline

This design avoids speculative work such as upstream reachability probing, broad config expansion,
or any change to the local-first admin/gateway bind posture.

## Architecture

### Current architectural grounding

Rook already has the core seams needed for Phase 1:

- `clients/rook/src/main.rs` owns CLI parsing and currently dispatches `serve`, `tui`, `doctor`,
  and `config export`
- `clients/rook/src/server/mod.rs` owns `ServerConfig`, app assembly, registry opening, and HTTP
  lifecycle
- `clients/rook/src/admin/mod.rs` exposes the `/api/*` operator/admin routes, including the current
  compatibility `GET /api/health`
- `clients/rook/src/config/mod.rs` already owns validation for inbound auth, transport,
  rate-limits, and idempotency, but does not yet provide first-class effective config loading
- middleware seams already exist for transport, rate limiting, and idempotency
- gateway helper boundaries already exist for upstream request execution and outcome handling

Phase 1 should extend these seams rather than introduce a parallel runtime model.

### Target module layout

The design assumes the following additive structure inside `clients/rook/src/`:

- `config/mod.rs`
  - own the `RookConfig` model, shared validation entrypoint, config discovery, TOML loading, env overrides, and operator-safe export rendering
  - expose effective config assembly from defaults, file, env, and CLI overlays
  - provide conversion into `ServerConfig`
- `doctor.rs`
  - command entrypoint and deterministic local checks for config, DB, assets, and auth consistency
- `health.rs`
  - readiness/liveness domain types and evaluation logic
- `observability.rs`
  - metrics bootstrap, counter/histogram definitions, and small emission helpers
  - back the admin-surface metrics endpoint through shared registry rendering

`main.rs`, `server/mod.rs`, and `admin/*` remain the main integration points.

### Runtime component model

```text
CLI / main.rs
   |
   v
EffectiveConfigAssembler (defaults -> file -> env -> CLI)
   |
   +--> config export (redacted output)
   +--> doctor (deterministic checks)
   +--> server startup
             |
             v
         ServerConfig / AppState
             |
             +--> registry open + startup dependency capture
             +--> admin router (/api/health, /api/health/live, /api/health/ready, /api/metrics)
             +--> gateway router (/v1/*)
             +--> middleware instrumentation
```

### Shared runtime state

Phase 1 should add a small shared runtime state object rather than scattering readiness and metrics
state across handlers.

Recommended shape:

- effective config operator summary or snapshot metadata
- startup dependency results
  - config validated
  - registry/database opened
  - dashboard assets/router available
- metrics registry/handle
- existing registry handle

This state should be created once during startup and injected into admin handlers and middleware via
existing Axum state/layer patterns. The goal is to reuse the current composition style in
`server/mod.rs`, not to replace it with a new app framework.

### Sequence diagram: effective config assembly

```text
Operator -> CLI: rook serve | rook doctor | rook config export
CLI -> ConfigAssembler: load_effective_config(command inputs)
ConfigAssembler -> Defaults: apply built-in defaults
ConfigAssembler -> FileLoader: load config.toml if present
ConfigAssembler -> EnvLoader: apply ROOK_* overrides
ConfigAssembler -> CliOverlay: apply command-specific overrides
ConfigAssembler -> Validator: validate effective config
Validator -> CLI: validated RookConfig or readable error
```

### Sequence diagram: `rook doctor`

```text
Operator -> CLI: rook doctor
CLI -> ConfigAssembler: load_effective_config()
ConfigAssembler -> Doctor: validated config
Doctor -> Check(config): pass/fail/warn
Doctor -> Check(db): open registry / DB and verify startup readiness
Doctor -> Check(assets): verify embedded dashboard/admin assets available
Doctor -> Check(auth): verify inbound auth consistency
Doctor -> CLI: structured results + overall exit code
```

### Sequence diagram: readiness flow

```text
Client -> /api/health/ready: GET
AdminHandler -> HealthEvaluator: readiness_snapshot()
HealthEvaluator -> RuntimeState: inspect startup dependency results
RuntimeState -> HealthEvaluator: config/db/router/assets status
HealthEvaluator -> AdminHandler: ready or not-ready JSON response
AdminHandler -> Client: 200 or non-success status
```

### Sequence diagram: metrics emission

```text
Client -> Middleware stack: request
Transport middleware -> Metrics: start timer for route surface
Rate-limit middleware -> Metrics: increment rejection counter if denied
Idempotency middleware -> Metrics: increment replay/conflict/pass counter
Gateway helper -> Metrics: increment upstream outcome counter
Transport middleware -> Metrics: record status class + duration on response
Operator -> /api/metrics: scrape metrics endpoint
```

## Decisions

### Decision: Introduce a first-class `RookConfig` and keep `ServerConfig` as runtime wiring

**Choice**: Add a first-class `RookConfig` that owns effective configuration assembly and
validation, then convert it into the existing `ServerConfig` for runtime startup.

**Rationale**:

- `ServerConfig` in `server/mod.rs` is already the runtime wiring structure used by the HTTP server.
- `main.rs` currently assembles `ServerConfig` directly from CLI flags, which makes `doctor` and
  `config export` impossible to share without duplication.
- A separate `RookConfig` lets the code distinguish between operator-facing effective configuration
  and server-only runtime wiring while still reusing validation.

**Alternatives considered**:

- Expand `ServerConfig` directly into the full configuration system.
- Keep per-command config parsing in `main.rs`.

**Why not chosen**:

- Expanding `ServerConfig` directly would overload a runtime type with file/env/export concerns.
- Per-command parsing would violate the spec requirement that `serve`, `doctor`, and `config
  export` share one configuration path.

### Decision: Keep precedence explicit as defaults < file < env < CLI

**Choice**: Implement configuration assembly as a layered overlay pipeline with explicit precedence:

1. built-in defaults
2. config file values
3. `ROOK_*` environment overrides
4. CLI flag overrides

**Rationale**:

- This is the required Phase 1 contract.
- An explicit overlay model is easier to test than ad hoc conditional mutation.
- It keeps later config additions predictable.

**Implementation note**:

CLI input should be represented as a partial override object rather than forcing every command to
materialize a fully populated config. This prevents clap defaults from accidentally masking file/env
values where the operator did not intend to override them.

### Decision: Reuse existing config validation methods instead of duplicating doctor-specific rules

**Choice**: `doctor`, `serve`, and `config export` all call the same validation pipeline anchored in
`config/` and existing validation methods for inbound auth, transport, rate limits, and
idempotency.

**Rationale**:

- `clients/rook/src/config/mod.rs` already contains meaningful validators.
- Phase 1 must fail closed the same way across startup and diagnostics.
- Centralization prevents future drift and reduces maintenance cost.

### Decision: Add operator-safe redacted export, not raw serialized config

**Choice**: `rook config export` should serialize a redacted operator view, not the raw underlying
config object.

**Rationale**:

- The spec requires operator-visible safety.
- Existing account/admin surfaces already use the pattern of exposing configuration presence without
  leaking raw secret material.
- A dedicated export view avoids accidental leakage through generic `Debug` or `Serialize`
  implementations.

**Redaction rules for Phase 1**:

- inbound bearer tokens: redact to enabled/configured state
- provider credentials if present in the effective runtime config: redact to presence-only state
- any future secret-bearing headers/cookies/tokens: redact by default

### Decision: Implement doctor as deterministic local checks only

**Choice**: Phase 1 `rook doctor` runs only fast local checks against effective config and local
runtime dependencies.

Required checks:

- config load and validation
- DB path usability and registry open/migration readiness
- embedded asset availability needed for the dashboard/admin process
- inbound auth internal consistency

**Rationale**:

- The proposal and spec explicitly exclude live upstream probing in Phase 1.
- Operators need a reliable preflight check, not one that fails because a remote vendor is having a
  transient issue.
- Local checks align with what startup and readiness actually require.

### Decision: Keep readiness local and separate from liveness

**Choice**: Add `/api/health/live` and `/api/health/ready` while preserving `GET /api/health` for
compatibility.

**Semantics**:

- liveness: process is running and able to serve the event loop
- readiness: config validated, DB/registry opened, router assembled, and embedded assets/local
  runtime resources available

**Rationale**:

- `admin/mod.rs` currently exposes `/api/health` as a coarse compatibility endpoint returning `ok`.
- Phase 1 should add semantics, not remove compatibility.
- Local-only readiness avoids orchestration flapping when upstream providers are transiently down.

### Decision: Capture readiness as startup dependency state, not ad hoc handler logic

**Choice**: Readiness should be driven from a small shared health evaluation layer backed by startup
results captured during server initialization.

**Rationale**:

- `server/mod.rs` already performs startup work in a deterministic sequence: validate config, open
  registry, build app/router.
- Those steps can produce the canonical readiness inputs once, rather than recomputing them in every
  request handler.
- This keeps health handlers thin and testable.

**Operational behavior**:

- startup failure before the server binds still exits the process and therefore never serves a
  readiness endpoint
- readiness endpoint is primarily for the running process and should reflect whether the critical
  local runtime state remains available
- in Phase 1, this state can be mostly startup-derived plus any local dependency checks that are
  cheap to reevaluate

### Decision: Expose one scrape-friendly metrics endpoint on the admin surface

**Choice**: Add one explicit operator-facing metrics route, preferably `/api/metrics`, on the
existing admin surface.

**Rationale**:

- The admin surface already carries operator endpoints and is the least invasive place to add
  metrics.
- A scrape-friendly text endpoint matches common operator expectations.
- One endpoint keeps rollout simple while meeting the baseline observability requirement.

### Decision: Instrument existing middleware and gateway helper seams

**Choice**: Attach metrics at shared seams already present in `server/mod.rs` composition:

- transport middleware: request count, status class, latency
- rate-limit middleware: rejection counters
- idempotency middleware: replay/conflict/pass counters
- gateway upstream helper boundary: success/failure outcome counters

**Rationale**:

- These seams already see the relevant control points.
- This avoids per-handler duplication and keeps labels consistent.
- It matches the spec requirement to use stable middleware/helper boundaries.

### Decision: Preserve minimal invasive routing changes

**Choice**: Additive route changes only:

- preserve `GET /api/health`
- add `GET /api/health/live`
- add `GET /api/health/ready`
- add `GET /api/metrics`

**Rationale**:

- Keeps the existing gateway/admin contract stable.
- Respects the current Rook architecture and local-first posture.
- Minimizes regression risk for tests and existing operator workflows.

## Risks

### Risk: CLI defaults may override file/env unintentionally

If clap continues to inject hard defaults directly into command structs, CLI parsing may always win
and erase file/env values even when the operator did not intend an override.

**Mitigation**:

- represent CLI overrides as optional values where precedence matters
- only apply a CLI override when the operator actually supplied the flag
- keep precedence tests for representative fields such as port and DB path

### Risk: configuration logic drifts between `main.rs` and `server/mod.rs`

Startup currently validates pieces inside `build_app_with_registry`, while CLI assembly happens in
`main.rs`.

**Mitigation**:

- move effective config assembly into `config/`
- reduce `main.rs` to command parsing plus handoff
- keep `server/mod.rs` focused on runtime wiring from validated config

### Risk: secret leakage through export or diagnostic output

`doctor` and `config export` are operator-visible and could accidentally print raw tokens.

**Mitigation**:

- create explicit redacted export view types
- ensure doctor explanations report configuration state, not secret values
- add tests that assert raw token values never appear in rendered output

### Risk: readiness becomes brittle or too static

If readiness checks too many dependencies, the process may flap. If readiness only snapshots
startup and never reflects local runtime degradation, it may become misleading.

**Mitigation**:

- limit Phase 1 to local critical dependencies
- capture startup dependency state explicitly
- allow the health evaluator to reevaluate cheap local checks where helpful, without introducing
  remote probes

### Risk: metrics cardinality or placement becomes unstable

Route/endpoint labels can become too granular if emitted from arbitrary handler paths.

**Mitigation**:

- label by stable route surface and coarse endpoint class where possible
- define metric families centrally in `observability.rs`
- emit through middleware/helper wrappers only

### Risk: route additions accidentally change existing `/api/health` behavior

Existing tests assert a simple successful response for `/api/health`.

**Mitigation**:

- preserve the base endpoint as a compatibility route
- make any richer health detail live under `/api/health/live` and `/api/health/ready`
- update tests additively rather than rewriting the old contract in place

## Implementation Outline

### 1. Effective config assembly and export

Affected modules:

- `clients/rook/src/main.rs`
- `clients/rook/src/config/mod.rs`
- `clients/rook/src/server/mod.rs`

Approach:

1. Define `RookConfig` for the Phase 1 runtime concerns already represented in the codebase:
   - server host/port
   - DB path
   - TUI enablement if needed by runtime startup
   - inbound auth
   - transport
   - rate limits
   - idempotency
2. Add partial override structures for file values, env values, and CLI values.
3. Implement a single `load_effective_config(...)` entrypoint that:
   - starts from built-in defaults
   - loads TOML config if present
   - applies `ROOK_*` env overrides
   - applies command-specific CLI overrides
   - validates the final config
4. Add conversion from validated `RookConfig` to existing `ServerConfig`.
5. Implement redacted export rendering for operator-visible output.
6. Update `main.rs` so `serve`, `doctor`, and `config export` all call this one path.

Expected `ROOK_*` coverage for Phase 1 should mirror the same fields exposed in the effective
runtime config, not an unrelated or larger schema.

### 2. `rook doctor`

Affected modules:

- `clients/rook/src/main.rs`
- `clients/rook/src/doctor.rs`
- `clients/rook/src/config/mod.rs`
- `clients/rook/src/registry/*` via existing open helpers
- dashboard asset module(s) already used by the router

Approach:

1. Add a `DoctorCheckResult` model with:
   - `status`: `pass | warn | fail`
   - `name`
   - `message`
2. Implement deterministic checks in a fixed order:
   - effective config load/validation
   - inbound auth consistency
   - database/registry open readiness
   - embedded asset availability
3. Return a structured report to stdout/stderr with a non-zero process exit when any required check
   fails.
4. Reuse the same config assembly entrypoint used by `serve`.
5. Keep checks local and fast; do not add network probing in this phase.

### 3. Readiness/liveness health reporting

Affected modules:

- `clients/rook/src/server/mod.rs`
- `clients/rook/src/admin/mod.rs`
- `clients/rook/src/admin/handlers.rs`
- `clients/rook/src/health.rs`
- possibly `clients/rook/src/lib.rs` to export the new module

Approach:

1. Add shared health domain types for:
   - live status
   - ready/not-ready status
   - dependency entries for readiness results
2. Extend startup in `server/mod.rs` to capture the local dependency facts needed for readiness.
3. Inject a small health state/evaluator into admin handlers through existing Axum state.
4. Preserve `GET /api/health` compatibility.
5. Add:
   - `GET /api/health/live`
   - `GET /api/health/ready`
6. Return structured JSON with stable response semantics and dependency details for readiness
   failures.

Recommended response posture:

- `/api/health`: compatibility success response for a healthy running process
- `/api/health/live`: lightweight live JSON
- `/api/health/ready`: dependency-oriented JSON and non-success status if local critical
  prerequisites are unavailable

### 4. Observability and metrics baseline

Affected modules:

- `clients/rook/src/server/mod.rs`
- `clients/rook/src/admin/mod.rs`
- `clients/rook/src/admin/handlers.rs`
- `clients/rook/src/observability.rs`
- existing middleware and gateway helper boundaries

Approach:

1. Introduce a metrics registry/bootstrap module that is initialized during server startup.
2. Define baseline metric families for:
   - total requests by surface/endpoint/status class
   - request duration for core `/api/*`, `/v1/models`, and `/v1/chat/completions` paths
   - rate-limit rejections
   - idempotency replay/conflict/pass outcomes
   - upstream request outcomes by result type
3. Thread a shared metrics handle through middleware state or helper wrappers.
4. Instrument existing middleware:
   - transport records request start/end and status class
   - rate-limit increments rejection counters
   - idempotency increments outcome counters
5. Instrument the gateway upstream boundary for result-type counters.
6. Expose one scrape-friendly metrics endpoint on the admin surface.

## Rationale Summary

The design keeps the Phase 1 work grounded in the current Rook architecture:

- `main.rs` remains a command dispatcher, not a second runtime system
- `server/mod.rs` remains the startup and router composition owner
- `admin/*` remains the operator HTTP surface
- `config/*` becomes the single source of truth for effective config assembly
- new `doctor`, `health`, and `observability` modules isolate concerns without forcing broad
  rewrites

This is the least invasive route that still satisfies the gateway-domain spec requirements for
config export, deterministic doctor diagnostics, separate readiness/liveness semantics, and a
production metrics baseline.
