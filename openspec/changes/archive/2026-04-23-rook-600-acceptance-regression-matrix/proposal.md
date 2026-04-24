# Proposal: Rook Acceptance and Regression Matrix

## Intent

Rook's M2/M3 operator work from #592 through #599 already shipped with meaningful verification, but the proof is fragmented across archived `verify-report.md` files. Operators and reviewers currently have to reconstruct acceptance coverage slice-by-slice to understand which existing commands cover the dashboard, TUI, security posture, and audit behavior.

This change creates the smallest meaningful M3 follow-up: a consolidated acceptance/regression matrix under the `gateway` spec domain that maps already-shipped commands and evidence to the implemented Rook operator slices. The value is consolidation, traceability, and regression discipline now that the feature slices have landed, not new runtime behavior or a new end-to-end platform.

## Scope

### In Scope

- Add a bounded Rook acceptance/regression matrix artifact that consolidates existing verification evidence for #592-#599.
- Map existing repo commands and focused slice commands to shipped operator lanes: dashboard, TUI, security, and audit/observability.
- Document explicit placeholders, partial/manual verification, and deferred areas honestly so the matrix does not overstate coverage.
- Optionally add one thin automation entrypoint only if it strictly reuses existing commands without introducing new coverage semantics or a new harness.

### Out of Scope

- Any new Rook runtime features, APIs, routes, transport behavior, or operator workflows.
- Any new large integrated acceptance harness spanning dashboard, TUI, and gateway surfaces.
- Re-defining acceptance criteria for archived slices beyond consolidating and referencing their already-proven evidence.
- Expanding into unrelated gateway concerns such as new streaming, idempotency, rate-limit, or TLS work.

## Approach

Use the archived verification reports for #592-#599 as the source evidence, then normalize that evidence into one matrix that answers three questions for each shipped slice: what surface is covered, which existing command(s) prove it, and what remains intentionally deferred or manually verified.

The proposal anchors the matrix in the `gateway` spec domain because `openspec/specs/gateway/spec.md` already carries the strongest source-of-truth for Rook HTTP bind posture, auth boundary posture, and acceptance/non-goal framing. The matrix will reference `openspec/specs/dashboard/spec.md` and `openspec/specs/rook-tui/spec.md` for surface-specific behavior, but it will avoid creating a new spec domain purely for reporting.

The preferred implementation is documentation-first. If a small automation helper is added, it must only compose existing entrypoints such as `make dashboard-build`, `make dashboard-check`, `make dashboard-test`, `make rust-test`, `make rust-clippy`, direct `@corvus/rook-dashboard` scripts, or targeted `cargo test --manifest-path clients/rook/Cargo.toml ...` commands already used by the archived slices.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/rook-600-acceptance-regression-matrix/proposal.md` | New | Defines the intent, bounded scope, and rationale for the consolidated matrix change. |
| `openspec/specs/gateway/spec.md` | Modified | Adds the acceptance/regression matrix requirements or traceability language under the recommended `gateway` domain. |
| `openspec/specs/dashboard/spec.md` | Referenced | Supplies the dashboard operator-shell contract that the matrix maps to #592-#594 verification commands. |
| `openspec/specs/rook-tui/spec.md` | Referenced | Supplies the TUI read-only/operator-boundary contract that the matrix maps to #595-#597 verification commands. |
| `openspec/changes/archive/2026-04-22-rook-592-dashboard-overview-providers-accounts/verify-report.md` | Referenced | Source evidence for dashboard overview/providers/accounts verification. |
| `openspec/changes/archive/2026-04-22-rook-593-dashboard-pools-routes-health-ops/verify-report.md` | Referenced | Source evidence for dashboard pools/routes/health verification and packaging checks. |
| `openspec/changes/archive/2026-04-22-rook-594-dashboard-usage-settings/verify-report.md` | Referenced | Source evidence for dashboard usage/settings verification. |
| `openspec/changes/archive/2026-04-23-rook-595-tui-status-providers-pools-health/verify-report.md` | Referenced | Source evidence for baseline TUI verification. |
| `openspec/changes/archive/2026-04-23-rook-596-tui-route-inspection-recent-logs/verify-report.md` | Referenced | Source evidence for route inspection coverage and partial manual verification caveat. |
| `openspec/changes/archive/2026-04-23-rook-597-tui-setup-troubleshooting/verify-report.md` | Referenced | Source evidence for the dashboard-bridge and read-only TUI boundary. |
| `openspec/changes/archive/2026-04-23-rook-598-security-defaults-and-secret-protection/verify-report.md` | Referenced | Source evidence for loopback-first bind, auth separation, and secret-safety regressions. |
| `openspec/changes/archive/2026-04-23-rook-599-observability-usage-health-audit/verify-report.md` | Referenced | Source evidence for audit persistence, bounded audit reads, and runtime-honest usage/health coverage. |
| `Makefile` | Referenced/Maybe Modified | Existing verification entrypoints that the matrix may cite directly; only lightly modified if a thin compositional target is justified. |
| `clients/web/apps/rook-dashboard/package.json` | Referenced | Existing dashboard-local scripts the matrix can cite without inventing new commands. |
| `clients/rook/Cargo.toml` | Referenced | Existing Rust crate entrypoint used by the matrix for targeted regression commands. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| The matrix overstates coverage for slices with incomplete manual verification, especially #596. | Medium | Mark partial/manual coverage explicitly and preserve caveats from the archived reports instead of flattening them away. |
| The matrix drifts from reality as commands evolve. | Medium | Prefer canonical repo entrypoints where available and keep focused historical commands clearly labeled as slice-specific evidence. |
| Scope expands into building a new test platform. | Medium | Keep automation optional and compositional only; reject any work that invents new orchestration or runtime verification semantics. |
| `gateway` ownership could confuse readers about dashboard/TUI source-of-truth boundaries. | Low | State directly that `gateway` owns the matrix framing while dashboard and TUI specs remain authoritative for their behavior. |

## Rollback Plan

If the consolidated matrix proves misleading or too noisy, revert the matrix artifact and any thin helper target introduced by this change. Because the proposal does not add new runtime behavior or APIs, rollback is documentation- and command-surface-only and restores the prior state where verification evidence remains in archived per-slice reports.

## Dependencies

- Archived verification reports for `rook-592` through `rook-599` must remain available as source evidence.
- Existing verification entrypoints in `Makefile`, `clients/web/apps/rook-dashboard/package.json`, and `clients/rook/Cargo.toml` must remain the basis for command mapping.
- `openspec/specs/gateway/spec.md`, `openspec/specs/dashboard/spec.md`, and `openspec/specs/rook-tui/spec.md` provide the behavioral contracts the matrix traces to.

## Success Criteria

- [ ] A single acceptance/regression matrix artifact exists for Rook #592-#599 and is anchored under the `gateway` spec domain.
- [ ] The matrix maps existing commands to the shipped dashboard, TUI, security, and audit/observability slices without inventing new runtime features or APIs.
- [ ] The matrix explicitly calls out placeholders, partial/manual verification, and deferred areas so acceptance claims remain honest.
- [ ] Any automation added by this change is strictly a thin composition of existing commands rather than a new generalized test harness.
