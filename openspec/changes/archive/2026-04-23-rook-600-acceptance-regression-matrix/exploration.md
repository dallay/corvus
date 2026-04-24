## Exploration: rook-600-acceptance-regression-matrix

### Current State
Rook already has strong slice-by-slice verification evidence across the M2 and M3 work, but that evidence is fragmented across archived `verify-report.md` files rather than consolidated into one acceptance/regression matrix. For the operator surfaces specifically, archived reports for #592-#599 already establish real command coverage over the embedded dashboard, the TUI shell, and the security/audit hardening slices. The common pattern is: each slice records targeted build/test commands, then maps those commands to a spec-compliance matrix. There is not yet a single repository artifact that tells an operator or reviewer, in one place, which commands cover dashboard overview/accounts, dashboard pools/routes/health, dashboard usage/settings, TUI status/providers/pools/health/routes, TUI dashboard-bridge behavior, loopback/auth secret-safety posture, and audit persistence.

Evidence found in archived Rook verification reports:

- `rook-592-dashboard-overview-providers-accounts` runs `pnpm --dir "clients/web" --filter @corvus/rook-dashboard run build`, `pnpm --dir "clients/web" --filter @corvus/rook-dashboard test`, `pnpm --dir "clients/web" --filter @corvus/rook-dashboard run test:e2e`, plus focused Rust admin tests.
- `rook-593-dashboard-pools-routes-health-ops` records passing `pnpm check`, focused Vitest for pools/routes features, and `pnpm test:e2e`, plus embedded asset packaging validation.
- `rook-594-dashboard-usage-settings` records `pnpm --filter @corvus/rook-dashboard run build`, `run test`, `run test:e2e`, and `run check`.
- `rook-595-tui-status-providers-pools-health` records `cargo fmt --manifest-path "clients/rook/Cargo.toml" --check`, full `cargo test --manifest-path "clients/rook/Cargo.toml"`, `cargo clippy --manifest-path "clients/rook/Cargo.toml" --all-targets -- -D warnings`, plus focused `tui::` and entrypoint tests.
- `rook-596-tui-route-inspection-recent-logs` records `cargo clippy --manifest-path clients/rook/Cargo.toml --all-targets -- -D warnings` and `cargo test --manifest-path clients/rook/Cargo.toml tui::`, but leaves manual interactive verification incomplete.
- `rook-597-tui-setup-troubleshooting` records `cargo test --manifest-path clients/rook/Cargo.toml tui::`, focused TUI entrypoint tests, and clippy.
- `rook-598-security-defaults-and-secret-protection` records targeted cargo tests for loopback bind defaults, explicit override behavior, inbound/outbound auth separation, admin secret redaction, and structured-log secret safety, plus clippy.
- `rook-599-observability-usage-health-audit` records targeted cargo tests for audit migrations, audit storage/service wiring, handler emission, bounded audit reads, and preservation of usage/health honesty, plus clippy.

Repo-level verification entrypoints already exist and are sufficient to anchor a bounded matrix without inventing new runtime features:

- Root `Makefile`: `dashboard-build`, `dashboard-check`, `dashboard-test`, `web-test-all`, `web-check-all`, `rust-test`, `rust-clippy`, `rust-fmt`, `check`, `check-all`.
- Direct web app scripts in `clients/web/apps/rook-dashboard/package.json`: `build`, `check`, `test`, `test:e2e`.
- Direct Rust crate entrypoint in `clients/rook/Cargo.toml`: `cargo test --manifest-path clients/rook/Cargo.toml ...`, `cargo clippy --manifest-path clients/rook/Cargo.toml --all-targets -- -D warnings`, `cargo fmt --manifest-path clients/rook/Cargo.toml --check`.

There is no existing consolidated Rook acceptance matrix/checklist artifact in `openspec`, docs, or `tmp/`. Searches for “acceptance matrix”, “regression matrix”, “acceptance checklist”, “regression checklist”, and similar terms did not find a Rook-specific consolidated matrix. What does exist is the repeated per-change `Verification Report` + `Spec Compliance Matrix` pattern in the archived changes.

The canonical spec surface that already carries most Rook HTTP acceptance posture is `openspec/specs/gateway/spec.md`. It already includes explicit acceptance/non-goal language for security and transport slices such as inbound auth, transport middleware, rate limits, idempotency, streaming, and baseline separation from prior archived work. The current Rook TUI contract lives in `openspec/specs/rook-tui/spec.md`, and the Rook dashboard contract currently lives inside `openspec/specs/dashboard/spec.md` under the Rook operator shell requirements.

### Affected Areas
- `openspec/changes/archive/2026-04-22-rook-592-dashboard-overview-providers-accounts/verify-report.md` — establishes dashboard overview/accounts verification commands and scenario mapping.
- `openspec/changes/archive/2026-04-22-rook-593-dashboard-pools-routes-health-ops/verify-report.md` — establishes pools/routes/health dashboard checks plus packaging validation.
- `openspec/changes/archive/2026-04-22-rook-594-dashboard-usage-settings/verify-report.md` — establishes usage/settings dashboard build/check/test/e2e coverage.
- `openspec/changes/archive/2026-04-23-rook-595-tui-status-providers-pools-health/verify-report.md` — establishes baseline TUI verification commands.
- `openspec/changes/archive/2026-04-23-rook-596-tui-route-inspection-recent-logs/verify-report.md` — establishes route-inspection TUI coverage and documents one remaining manual-check warning.
- `openspec/changes/archive/2026-04-23-rook-597-tui-setup-troubleshooting/verify-report.md` — establishes the dashboard-bridge/read-only TUI boundary.
- `openspec/changes/archive/2026-04-23-rook-598-security-defaults-and-secret-protection/verify-report.md` — establishes security/default-bind/secret-safety regression commands.
- `openspec/changes/archive/2026-04-23-rook-599-observability-usage-health-audit/verify-report.md` — establishes audit persistence and bounded admin-audit coverage.
- `Makefile` — existing repo-level verification entrypoints that a new matrix can reference instead of inventing commands.
- `clients/web/apps/rook-dashboard/package.json` — exact dashboard-local verification scripts.
- `clients/rook/Cargo.toml` — exact Rust crate verification target for Rook acceptance/regression commands.
- `openspec/specs/gateway/spec.md` — strongest existing source-of-truth domain for Rook HTTP acceptance posture and non-goal boundaries.
- `openspec/specs/rook-tui/spec.md` — source-of-truth for terminal acceptance boundaries.
- `openspec/specs/dashboard/spec.md` — source-of-truth for the embedded Rook dashboard operator surface.

### Approaches
1. **Documentation-only consolidated regression matrix** — Add a single change that documents the smallest meaningful acceptance matrix by mapping already-proven commands to the shipped Rook slices.
   - Pros: Smallest scope; uses existing evidence; no new runtime/test harness APIs; aligns with “evidence first”; directly solves the discoverability gap.
   - Cons: Does not itself execute commands automatically; can drift if later slices change coverage.
   - Effort: Low.

2. **Documentation plus one thin verification entrypoint** — Add the matrix plus a small Makefile target or script that runs the already-selected bounded commands for dashboard, TUI, and security/audit slices.
   - Pros: Improves repeatability; keeps automation thin by composing existing commands; still avoids new runtime features.
   - Cons: Slightly larger scope; must decide command budget carefully so it stays fast and meaningful; risks broadening into general CI design.
   - Effort: Low/Medium.

3. **Full end-to-end acceptance suite across all Rook surfaces** — Create a new integrated automation layer spanning web, TUI, gateway, security, and audit in one runner.
   - Pros: Stronger long-term automation story.
   - Cons: Too large for #600; invents new orchestration expectations; duplicates existing slice tests; high risk of violating the “smallest meaningful slice” constraint.
   - Effort: High.

### Recommendation
Use **Approach 1** as the core of #600, with a narrow allowance to promote it toward **Approach 2** only if an existing entrypoint can be added without inventing any new coverage semantics.

The smallest meaningful slice for #600 is a **consolidated acceptance/regression matrix artifact** that groups existing verification into a few operator-facing lanes:

- **Dashboard lane**: #592 overview/accounts, #593 pools/routes/health, #594 usage/settings via existing `@corvus/rook-dashboard` build/check/test/e2e commands.
- **TUI lane**: #595 status/providers/pools/health, #596 routes, #597 dashboard-bridge via existing `cargo test ... tui::`, focused entrypoint tests, and clippy/fmt where already used.
- **Security lane**: #598 loopback-first bind, inbound/outbound auth separation, secret-redaction/log safety via existing targeted cargo tests.
- **Audit/observability lane**: #599 audit migration/storage/router-read coverage plus preservation of usage placeholder and runtime-only health semantics.

That slice should explicitly avoid:

- inventing new runtime capabilities,
- redefining acceptance for already archived slices,
- requiring a new giant end-to-end harness,
- expanding into unrelated gateway features like streaming/idempotency/rate-limit revalidation unless they are deliberately referenced as already-covered earlier work.

For spec domain, **`gateway` is the best recommended domain for #600**. The reason is not that #600 changes gateway runtime behavior, but that the existing source-of-truth for Rook HTTP bind posture, auth boundary posture, transport baseline, and acceptance/non-goal framing already lives in `openspec/specs/gateway/spec.md`. A #600 delta under `gateway` can document the acceptance/regression matrix as the operator/reviewer verification boundary over the shipped Rook surfaces, while referencing `dashboard` and `rook-tui` as dependent covered surfaces. Creating a brand-new spec domain just for the matrix would add needless fragmentation.

### Risks
- The archived evidence is uneven in depth: #596 still has a documented manual-interactive warning, so a consolidated matrix must not overstate that slice as fully visually verified.
- A matrix can become stale if it copies commands mechanically instead of clearly marking which commands are canonical repo entrypoints versus historical one-off focused commands.
- If #600 tries to automate every archived command, scope will sprawl quickly and duplicate existing slice verification rather than summarizing it.
- The dashboard requirements currently live inside `openspec/specs/dashboard/spec.md`, while TUI uses `openspec/specs/rook-tui/spec.md`; the matrix must be explicit about cross-spec coverage so “gateway” ownership does not look like a claim that all behavior is gateway-only.

### Ready for Proposal
Yes — propose #600 as a tight acceptance/regression documentation slice, centered on a consolidated matrix of already-proven commands and expected coverage, with `gateway` as the spec domain and optional minimal automation only if it is strictly compositional over existing entrypoints.
