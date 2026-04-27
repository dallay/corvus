# Proposal: Add config export and environment-based configuration overrides #678

## Intent

Rook needs a first-class, inspectable configuration model instead of relying primarily on CLI flags and partial scaffolding. This change establishes a stable operator-facing configuration surface that behaves predictably across local development, containers, and automation while preserving the existing gateway safety posture and secret redaction requirements.

## Scope

### In Scope
- Introduce a first-class `RookConfig` model for Rook runtime configuration.
- Implement layered configuration loading with explicit precedence across defaults, config file inputs, `ROOK_*` environment overrides, and CLI flags.
- Add `rook config export` to print the effective configuration with secret values redacted or reduced to presence-only state.
- Validate effective configuration at startup and config export time so invalid configuration fails closed with clear operator-facing errors.
- Document and test precedence, environment override behavior, and redaction semantics.
- Align the proposal with the `gateway` spec domain as the source of truth for bind posture and operator-visible secret handling.

### Out of Scope
- Redesigning provider credential storage, encryption, or secret management backends.
- Expanding gateway/admin API behavior beyond configuration loading, validation, and export surfaces.
- Introducing a new remote configuration service, hot reload system, or live config mutation workflow.
- Reworking unrelated CLI commands or changing established gateway semantics outside the configuration path.

## Approach

Define a single `RookConfig` model under `clients/rook/src/config/` and route startup through one layered config loader that derives the effective configuration from defaults, file state, `ROOK_*` environment variables, and explicit CLI flags. The loader will validate the final resolved configuration before server startup or export, emit deterministic precedence behavior, and reuse the `gateway` domain requirements for loopback-first bind defaults, inbound-auth fail-closed behavior, and operator-visible secret redaction.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/config/` | Modified | Add the first-class `RookConfig` model, layered loaders, environment override parsing, validation, and redaction/export helpers. |
| `clients/rook/src/main.rs` | Modified | Route CLI startup and `rook config export` through the shared effective-config loading and validation flow. |
| `clients/rook/src/...` CLI wiring | Modified | Connect CLI flags to the final precedence layer without bypassing centralized config validation. |
| `clients/rook/tests/` and/or config-focused test modules | Modified | Add coverage for precedence ordering, `ROOK_*` overrides, invalid config failures, and redacted export output. |
| Operator docs for Rook configuration | Modified | Document supported config inputs, precedence rules, environment variable naming, and safe export expectations. |
| `openspec/specs/gateway/spec.md` and related gateway artifacts | Referenced | Treat gateway requirements as the source of truth for bind posture, inbound auth config expectations, and operator-visible secret protection. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Precedence behavior becomes ambiguous across file, env, and CLI inputs | Medium | Centralize resolution in one loader and add explicit precedence tests and operator documentation. |
| Config export accidentally exposes secrets | Medium | Use redaction/presence-only rendering aligned to gateway secret-handling requirements and verify with tests. |
| Stricter validation breaks existing startup paths unexpectedly | Medium | Keep validation scoped to clearly invalid states, preserve existing safe defaults, and provide clear actionable error messages. |
| Bind or inbound-auth behavior drifts from gateway source-of-truth | Low | Treat `openspec/specs/gateway/spec.md` as authoritative for bind defaults and secret-related config behavior. |

## Rollback Plan

Revert the new centralized config loader and `rook config export` command wiring, restoring the prior CLI-driven startup path while preserving any unaffected documentation changes only if they still describe shipped behavior. If rollout reveals regressions, disable the new export path and revert precedence enforcement in `clients/rook/src/config/` and `clients/rook/src/main.rs` together so startup semantics return to the previous implementation as one unit.

## Dependencies

- Existing gateway spec requirements for loopback-first bind posture, inbound auth configuration, and operator-visible secret protection.
- Current Rook CLI/config scaffolding in `clients/rook/src/config/` and `clients/rook/src/main.rs`.
- Test coverage updates for config loading, validation, and export behavior.

## Success Criteria

- [ ] `rook config export` outputs the effective configuration without leaking secrets.
- [ ] A first-class `RookConfig` model defines the resolved runtime configuration for Rook.
- [ ] `ROOK_*` environment overrides are implemented, documented, and covered by tests.
- [ ] Invalid configuration fails closed with clear operator-facing validation messages.
- [ ] Config precedence across defaults, file, environment, and CLI is explicit and verified by tests.
- [ ] The change preserves gateway source-of-truth requirements for bind posture and secret redaction.
