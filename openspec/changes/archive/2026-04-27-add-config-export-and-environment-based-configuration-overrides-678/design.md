# Design: Add config export and environment-based configuration overrides #678

## Technical Approach

This change formalizes Rook's operator-facing configuration path around a single effective-config assembly pipeline in `clients/rook/src/config/`. The implementation keeps `gateway` as the source-of-truth domain for bind posture and secret-handling requirements, and treats the existing `127.0.0.1:4141` default plus inbound-auth fail-closed behavior as contracts that the new loader must preserve.

The core approach is:

1. keep `RookConfig` as the resolved runtime configuration model
2. introduce explicit partial overlay types for file, environment, and CLI inputs
3. assemble effective configuration through one layered pipeline: defaults → file → env → CLI
4. validate only after the final effective config is assembled
5. expose a dedicated redacted export view instead of serializing raw config structs
6. convert the validated `RookConfig` into the existing `ServerConfig` for runtime startup

This keeps `main.rs` thin, avoids duplicating precedence logic across commands, and ensures `rook serve` and `rook config export` report the same effective values under the same inputs.

## Architecture Decisions

### Decision: Keep `RookConfig` as the canonical resolved model

**Choice**: Treat `clients/rook/src/config/mod.rs::RookConfig` as the single canonical effective configuration type, and continue using `ServerConfig` only as the server runtime wiring type.

**Alternatives considered**:
- Expand `ServerConfig` to absorb file/env/export concerns.
- Keep separate ad hoc config assembly in `main.rs` for each command.

**Rationale**:
- The repository already separates operator config concerns from HTTP runtime concerns.
- `server/mod.rs` already consumes `ServerConfig`; changing that boundary is unnecessary for this scoped change.
- A distinct `RookConfig` supports export, validation, and precedence testing without coupling those concerns to server startup internals.

### Decision: Represent overlays as partial typed structures, not immediate mutation from every source

**Choice**: Introduce or formalize partial overlay structs for each input layer, with nested partials mirroring the shape of `RookConfig`, then merge them into defaults in precedence order.

**Alternatives considered**:
- Parse TOML straight into `RookConfig` and mutate it field-by-field for env and CLI.
- Store env and CLI overrides as untyped string maps until the end.

**Rationale**:
- The file loader already uses `PartialRookConfig` and nested partial structs; extending that pattern is consistent with current code.
- Matching partial shapes across file/env/CLI makes precedence behavior explicit and testable.
- Typed partials allow parse failures to be attributed to exact operator inputs before runtime conversion.

### Decision: Centralize assembly in one config loader API

**Choice**: Add a single assembly entrypoint in `clients/rook/src/config/` that accepts optional file path, environment map, and CLI override struct, returns a resolved `RookConfig`, and performs validation after all overlays are applied.

**Alternatives considered**:
- Keep `from_sources_with_path_unvalidated` for startup and a different path for export.
- Validate after each layer.

**Rationale**:
- The change requires deterministic precedence across `serve` and `config export`.
- Final-state validation is less error-prone than per-layer validation because partial overlays may be intentionally incomplete.
- A single assembly API keeps command code in `main.rs` focused on parsing and dispatch.

### Decision: Parse `ROOK_*` overrides into typed partial config rather than mutating live config directly

**Choice**: Move environment interpretation toward an env-overlay builder that maps supported `ROOK_*` names into a partial typed overlay, then merges it like any other layer.

**Alternatives considered**:
- Keep the existing `apply_env_overrides(&mut self, env)` mutation style as the long-term interface.

**Rationale**:
- The current mutation path works but hides the overlay boundary and makes CLI/env behavior less symmetric.
- Converting env into a typed overlay simplifies precedence reasoning, error reporting, and future documentation generation.
- This also makes it easier to test env parsing in isolation from file loading and validation.

### Decision: Redacted export must be a dedicated view model

**Choice**: Continue using a dedicated export struct family such as `RookConfigExportView`, but shape it as an explicitly operator-safe rendering layer derived from validated effective config.

**Alternatives considered**:
- Derive `Serialize` on `RookConfig` and rely on field-level redaction hacks.
- Print debug output with custom `Debug` impls.

**Rationale**:
- The gateway spec requires operator-visible secret protection.
- Existing code already uses dedicated export structs and redaction helpers; this is the safest and least disruptive pattern.
- A dedicated export view prevents future secret-bearing fields from being leaked through accidental generic serialization.

### Decision: CLI flags remain the highest-precedence additive overlay

**Choice**: Keep CLI flags as the final overlay layer, represented as a partial `ServeOverrides`-like structure that only sets fields explicitly supplied by the operator.

**Alternatives considered**:
- Give CLI flags defaults at clap parse time and always populate a full config.
- Let CLI bypass config loading for some fields.

**Rationale**:
- Explicit CLI precedence is required by the proposal.
- Boolean flags such as `--tui` and `--inbound-auth-enabled` already behave as additive-only overrides; representing them as partials preserves that intent.
- Avoiding clap defaults prevents accidental masking of file or environment settings.

## Data Flow

### Effective configuration assembly

```text
Operator
  |
  | rook serve / rook config export
  v
main.rs
  |
  | parse CLI -> partial CLI overrides
  v
config::load_effective_config(...)
  |
  +--> Defaults: RookConfig::default()
  |
  +--> File overlay: parse TOML into PartialRookConfig
  |
  +--> Env overlay: parse ROOK_* into PartialRookConfig-like overlay
  |
  +--> CLI overlay: apply explicit command flags only
  |
  +--> validate final RookConfig
  |
  +--> on serve: to_server_config()
  |
  +--> on export: RookConfigExportView::from_config()
```

### Validation and export flow

```text
config file/env/CLI inputs
        |
        v
partial overlays
        |
        v
merged RookConfig
        |
        +--> validate host/db/auth/transport/rate-limit/idempotency
        |        |
        |        +--> invalid => operator-facing RookError::Config
        |
        +--> valid =>
                 |-- serve: ServerConfig
                 '-- export: redacted JSON view
```

### Environment parsing shape

The environment parser maps flat `ROOK_*` variables into the nested config model. Representative mappings stay aligned to the current config schema:

- `ROOK_HOST` → `host`
- `ROOK_PORT` → `port`
- `ROOK_ENABLE_TUI` → `enable_tui`
- `ROOK_DB_PATH` → `db_path`
- `ROOK_INBOUND_AUTH_ENABLED` → `inbound_auth.enabled`
- `ROOK_INBOUND_AUTH_TOKEN` → `inbound_auth.bearer_token`
- `ROOK_TRANSPORT_REQUEST_ID_*` → `transport.request_id.*`
- `ROOK_TRANSPORT_TRUSTED_PROXY_*` → `transport.trusted_proxy.*`
- `ROOK_API_RATE_LIMIT_*` / `ROOK_V1_MODELS_RATE_LIMIT_*` / `ROOK_V1_CHAT_RATE_LIMIT_*` → `rate_limits.*`
- `ROOK_CHAT_IDEMPOTENCY_*` → `idempotency.chat_completions.*`

Numeric, boolean, comma-delimited list, and string parsing remain field-specific and fail closed with variable-specific messages.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/add-config-export-and-environment-based-configuration-overrides-678/design.md` | Create | Technical design for this OpenSpec change. |
| `clients/rook/src/config/mod.rs` | Modify | Centralize effective-config assembly, add overlay merge helpers, formalize env parsing, keep validation entrypoint, and refine export view/redaction helpers. |
| `clients/rook/src/main.rs` | Modify | Replace direct config mutation in command handlers with calls into the shared config assembly API; keep CLI parsing and output behavior only. |
| `clients/rook/src/server/mod.rs` | Possible narrow touch | Consume unchanged `ServerConfig`; only adjust if conversion or tests need minor updates. No new server behavior is introduced by design. |
| `clients/rook/src/lib.rs` | Possible narrow touch | Only if new config submodules or re-exports are introduced under `config/`. |
| `clients/rook/README.md` or equivalent operator docs | Modify | Document precedence, default config path, supported `ROOK_*` names, export safety, and validation behavior. |
| `clients/rook/src/config/mod.rs` tests | Modify | Add unit coverage for merge order, env parsing, validation failures, and redaction semantics. |
| `clients/rook/src/main.rs` tests | Modify | Verify CLI integration uses shared precedence and export paths rather than bespoke mutation logic. |

## Interfaces / Contracts

### Effective config assembly API

The implementation should converge on a single entrypoint in `clients/rook/src/config/` with a shape along these lines:

```rust
pub struct CliRookConfigOverlay {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub enable_tui: Option<bool>,
    pub db_path: Option<PathBuf>,
    pub inbound_auth: Option<PartialInboundAuthConfig>,
    pub rate_limits: Option<PartialRateLimitConfig>,
    pub idempotency: Option<PartialIdempotencyConfig>,
}

pub struct LoadRookConfigInput<'a> {
    pub file_path: Option<&'a Path>,
    pub env: &'a HashMap<String, String>,
    pub cli: Option<CliRookConfigOverlay>,
}

pub fn load_effective_config(input: LoadRookConfigInput<'_>) -> Result<RookConfig, RookError>;
```

The exact names may differ, but the contract should be:

- accept all input layers explicitly
- parse file and env into partial overlays
- merge in precedence order
- validate only once on the final resolved config
- return `RookError::Config` with operator-readable messages on failure

### Partial overlay merge contract

Each overlay type should follow the same semantic rule:

```rust
trait ApplyOverlay<T> {
    fn apply_to(self, target: &mut T);
}
```

This does not need to be a literal public trait, but the internal contract should remain:

- `None` means "no override at this layer"
- `Some(value)` means "replace the current resolved value"
- nested `Some(partial)` means recursively apply only the explicitly set nested fields

That rule is what makes defaults < file < env < CLI deterministic.

### Export contract

`rook config export` should continue to emit a dedicated redacted JSON representation. The export contract for secrets in this change is:

- inbound auth token is never emitted raw
- when inbound auth is enabled and a token is configured, export shows redacted or presence-only state
- when inbound auth is enabled but token is blank/missing, export still fails validation rather than printing an invalid success view
- non-secret scalar fields such as host, port, db path, rate limits, and request-id headers are emitted as effective values

Representative view shape:

```rust
#[derive(Serialize)]
pub struct RookConfigExportView {
    pub host: String,
    pub port: u16,
    pub enable_tui: bool,
    pub db_path: String,
    pub inbound_auth: InboundAuthExportView,
    pub transport: TransportExportView,
    pub rate_limits: RateLimitExportView,
    pub idempotency: IdempotencyExportView,
}
```

### Validation contract

Validation remains anchored in config and must cover the final effective `RookConfig` for both startup and export:

- `host` must not be blank
- `db_path` must not be blank
- inbound auth enabled requires a non-blank token
- request-id headers must be syntactically valid header names
- trusted proxy enabled requires at least one valid CIDR
- rate-limit windows and request counts must be greater than zero
- idempotency replay window must be greater than zero

This preserves existing validation behavior while ensuring invalid file/env/CLI combinations fail before startup or export output.

## Module Impacts

### `clients/rook/src/config/`

This is the main implementation surface.

Planned changes:

- keep `RookConfig` as the effective model
- retain or refine `PartialRookConfig` and nested partial structs as the merge vocabulary
- add overlay application helpers to remove repeated field-by-field mutation logic
- introduce an env-to-partial parser instead of burying env semantics in direct mutation
- expose one public assembly function for command integration
- keep `to_server_config()` as the conversion seam into existing runtime wiring
- preserve and strengthen `RookConfigExportView` as the only export serializer

This module becomes the single source of truth for:

- config discovery
- file parsing
- environment parsing
- precedence
- validation
- redacted export rendering

### `clients/rook/src/main.rs`

`main.rs` should stop manually assembling final configs field-by-field after calling an unvalidated loader.

Planned changes:

- keep clap command definitions largely intact for this change
- translate `serve` flags into a partial CLI overlay
- call the shared config assembly API for both `Serve` and `Config::Export`
- convert the resulting `RookConfig` into `ServerConfig` only for `serve`
- keep JSON formatting for export in the CLI layer, but keep redaction policy in `config/`

This keeps `main.rs` responsible for:

- parsing
- command dispatch
- stdout rendering
- process exit behavior

and removes responsibility for config precedence or validation internals.

### Existing runtime conversion

The change should preserve the current runtime boundary:

```text
RookConfig --to_server_config()--> ServerConfig --server::run(...)--> HTTP runtime
```

No new runtime config type is needed beyond possibly cleaner conversion helpers. That keeps the change bounded and avoids incidental churn in `server/mod.rs`.

### Docs and tests

Docs should explain:

- default config path discovery (`XDG_CONFIG_HOME` / `HOME`)
- exact precedence order
- supported `ROOK_*` overrides
- that export is safe for operator inspection because secrets are redacted
- that invalid effective config fails before startup or export succeeds

Tests should be concentrated in config and CLI integration layers, not server behavior unrelated to config.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Default path discovery | Test `XDG_CONFIG_HOME` and `HOME` resolution cases. |
| Unit | TOML partial parsing | Parse partial config files with nested sections and unknown-field rejection. |
| Unit | Overlay merge precedence | Verify defaults < file < env < CLI for shared fields like host, port, db path, inbound auth, and rate limits. |
| Unit | Env parsing | Verify booleans, numerics, CIDR lists, and variable-specific failures for invalid `ROOK_*` inputs. |
| Unit | Validation | Verify invalid final combinations fail closed with actionable `RookError::Config` messages. |
| Unit | Export redaction | Verify raw secrets never appear in `RookConfigExportView` JSON output. |
| Integration-ish (`main.rs` tests) | `serve` CLI integration | Parse CLI input, assemble effective config with env/file inputs, and confirm CLI has final precedence. |
| Integration-ish (`main.rs` tests) | `config export` integration | Confirm export uses the same assembly pipeline as serve and prints redacted effective config. |
| Regression | Gateway bind posture contract | Preserve default `127.0.0.1:4141` and explicit non-loopback override behavior consistent with `openspec/specs/gateway/spec.md`. |

## Migration / Rollout

No migration required.

This change is an in-place refactor and hardening of Rook's configuration path. Existing config files continue to be read from the same discovered location, existing defaults remain in place, and runtime startup still receives a `ServerConfig`. The rollout concern is behavioral consistency: the shared assembly path must match current safe defaults while making precedence and validation explicit.

## Open Questions

- [ ] Should `rook config export` continue returning JSON only, or should the implementation leave room for a future TOML/YAML format flag without changing the internal export view model?
- [ ] Should the environment parser accept any additional aliases for current variables, or should this change strictly preserve the already-implemented `ROOK_*` names to avoid undocumented compatibility surface?
- [ ] Should additive-only CLI flags such as `--tui` and `--inbound-auth-enabled` remain one-way overrides in the shared overlay model, or should the model be generalized now to support explicit disable flags later? For this change, the design assumes current one-way behavior is preserved.
