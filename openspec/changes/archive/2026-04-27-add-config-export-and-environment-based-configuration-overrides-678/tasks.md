# Tasks: Add config export and environment-based configuration overrides #678

## Phase 1: Foundation and shared config assembly

- [x] 1.1 In `clients/rook/src/config/mod.rs`, formalize `RookConfig` as the canonical effective model and align `PartialRookConfig`/nested partials so file, env, and CLI overlays share the same shape.
- [x] 1.2 RED: add config-module tests in `clients/rook/src/config/mod.rs` covering defaults < file < env < CLI precedence for host, port, db path, inbound auth, and one nested rate-limit field.
- [x] 1.3 In `clients/rook/src/config/mod.rs`, implement a single `load_effective_config(...)` entrypoint that reads defaults, parses file overlay, applies env overlay, applies CLI overlay, and validates only the final resolved config.
- [x] 1.4 REFACTOR: remove or narrow old ad hoc config-loading/mutation paths in `clients/rook/src/config/mod.rs` so `serve` and export cannot bypass shared assembly.

## Phase 2: Environment overrides and validation

- [x] 2.1 RED: add unit tests in `clients/rook/src/config/mod.rs` for supported `ROOK_*` mappings, including booleans, numerics, CIDR lists, and unsupported variables being ignored.
- [x] 2.2 In `clients/rook/src/config/mod.rs`, implement an env-to-partial parser for documented overrides such as `ROOK_HOST`, `ROOK_PORT`, `ROOK_ENABLE_TUI`, `ROOK_DB_PATH`, inbound auth, transport, rate-limit, and idempotency fields.
- [x] 2.3 In `clients/rook/src/config/mod.rs`, make env parse failures return variable-specific `RookError::Config` messages and keep overlay semantics deterministic (`None` = no override, `Some` = replace).
- [x] 2.4 RED/GREEN: add and implement final-config validation tests in `clients/rook/src/config/mod.rs` for blank host/db path, enabled inbound auth without token, invalid header names, missing trusted proxy CIDRs, and zero-valued limits/windows.

## Phase 3: CLI integration and safe config export

- [x] 3.1 RED: add `clients/rook/src/main.rs` tests proving `serve` and `rook config export` resolve the same effective values from identical file/env/CLI inputs.
- [x] 3.2 In `clients/rook/src/main.rs`, translate command flags into a typed CLI overlay and route both `Serve` and `Config::Export` through `config::load_effective_config(...)` before runtime conversion or printing.
- [x] 3.3 RED: add export tests in `clients/rook/src/config/mod.rs` and/or `clients/rook/src/main.rs` asserting raw tokens, API keys, auth headers, and cookies never appear in export output.
- [x] 3.4 In `clients/rook/src/config/mod.rs`, finalize `RookConfigExportView` redaction/presence-only rendering for secret-bearing fields while preserving effective non-secret values such as bind target and db path.

## Phase 4: Documentation and automated verification

- [x] 4.1 Update `clients/rook/README.md` or the operator-facing Rook config doc to document default config discovery, supported `ROOK_*` names, precedence order, validation failures, and redacted export behavior.
- [x] 4.2 Add or update automated tests under `clients/rook/src/config/mod.rs` and `clients/rook/src/main.rs` to cover gateway default bind posture (`127.0.0.1:4141`), explicit non-loopback overrides, invalid-config fail-closed behavior, and documented precedence regression cases.
