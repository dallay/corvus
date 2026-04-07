# Proposal: Productize Model Routing (Phase 1)

## Intent

Corvus has a solid model routing and query classification engine (30+ unit tests, clean trait-based architecture) but zero operator-facing documentation, incomplete config validation, and silent failure modes that make the feature effectively invisible to anyone who doesn't read Rust source. Operators cannot discover, configure, or troubleshoot routing without diving into code.

This change closes the product maturity gap for Phase 1: documentation, config validation hardening, silent failure fixes, and a formal openspec specification. The goal is that an operator can configure, validate, and understand model routing using only docs and `corvus doctor` — no source code reading required.

Tracked by DALLAY-173 (GitHub #269). Related: DALLAY-174 (#270, operator UX/docs), DALLAY-175 (#271, next-stage capabilities).

## Scope

### In Scope

1. **Operator documentation** — dedicated guide in the docs site (`clients/web/apps/docs/`) covering `[[model_routes]]` and `[query_classification]` TOML configuration, hint flow explanation, example configs for common scenarios (fast/reasoning split, code model, vision routing), and troubleshooting section.
2. **Config validation hardening** — new doctor checks:
   - Classification rule hint references a hint that exists in `model_routes` (warn if orphaned)
   - `query_classification.enabled = true` with zero rules triggers a warning
   - Classification enabled but zero `model_routes` configured triggers a warning
   - Classification rule with empty `keywords` AND empty `patterns` triggers a warning (rule can never match)
3. **Silent failure fixes** — improve error/warning messages:
   - Unknown hint fallback: log a clear warning when `hint:X` doesn't match any route, instead of silently passing `"hint:X"` as a model name
   - Skipped provider: when a non-primary provider fails to initialize, log which routes will be affected
4. **Formal openspec spec** — routing behavior contract in `openspec/specs/model-routing/spec.md` covering route resolution, classification, fallback behavior, and image routing gating

### Out of Scope

- Diagnostic CLI commands (`corvus route test`, `corvus route list`) — Phase 2
- Structured observability events and metrics for routing decisions — Phase 2
- Onboarding wizard integration for multi-provider routing setup — Phase 3
- Breaking validation changes (fail-hard on orphaned hints) — future major version
- Changes to the routing/classification runtime logic itself — engine is solid as-is

## Approach

**Documentation-first with additive validation.** No runtime behavior changes, only:

1. Write a comprehensive operator guide as a new docs page with TOML examples and a "how hints flow" diagram
2. Add doctor check functions in `doctor/mod.rs` for classification ↔ route integrity (all as warnings, not errors)
3. Add `tracing::warn!` calls in `router.rs` and `providers/mod.rs` for silent failure paths that currently produce confusing downstream errors
4. Write the formal spec using Given/When/Then scenarios and RFC 2119 keywords per openspec conventions

All changes are additive. No existing behavior is modified. Warnings are non-breaking.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/web/apps/docs/src/content/docs/` | New | Operator guide for model routing and query classification |
| `clients/agent-runtime/src/doctor/mod.rs` | Modified | New validation checks for classification ↔ route integrity |
| `clients/agent-runtime/src/providers/router.rs` | Modified | Warning log for unknown hint fallback (no logic change) |
| `clients/agent-runtime/src/providers/mod.rs` | Modified | Warning log listing affected routes when provider init fails |
| `openspec/specs/model-routing/` | New | Formal routing behavior spec |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Doctor warnings on existing configs with orphaned hints | Medium | Warnings only, never errors — operator sees info but nothing breaks |
| Docs become stale if routing internals change | Low | Spec provides the contract; docs reference spec behavior, not code internals |
| Additional `tracing::warn!` calls add log noise | Low | Warnings only fire on actual misconfigurations, not on normal operation |

## Rollback Plan

All changes are additive and independent:

- **Docs**: revert the docs page commit — zero runtime impact
- **Doctor checks**: revert the doctor changes — existing checks unaffected, new checks simply disappear
- **Warning logs**: revert the two log additions in router.rs and providers/mod.rs — returns to silent fallback behavior
- **Spec**: revert the spec file — no runtime dependency on specs

Each deliverable can be reverted independently without affecting the others. No migration, no data changes, no API contract changes.

## Dependencies

- None. All changes are additive to the existing codebase.
- Docs site build pipeline (`clients/web/apps/docs/`) must be functional (it is).
- No new crate dependencies required for doctor checks or warning logs.

## Success Criteria

- [ ] Operator can configure `[[model_routes]]` and `[query_classification]` using only the docs guide (no source code reading)
- [ ] `corvus doctor` reports warnings for orphaned classification hints, empty rules with classification enabled, and never-matching rules
- [ ] Unknown hint fallback produces a clear warning log naming the hint and the fallback behavior
- [ ] Failed provider init log names the affected routes
- [ ] Formal spec exists at `openspec/specs/model-routing/spec.md` with Given/When/Then scenarios covering route resolution, classification, fallback, and image gating
- [ ] All new doctor checks have corresponding unit tests
- [ ] Existing 30+ routing/classification tests continue to pass
