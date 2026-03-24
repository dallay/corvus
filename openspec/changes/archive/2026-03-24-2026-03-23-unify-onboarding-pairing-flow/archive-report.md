# Archive Report: 2026-03-23-unify-onboarding-pairing-flow

## Status

- status: archived
- archive_mode: openspec
- verification_verdict: PASS WITH WARNINGS
- archived_at: 2026-03-24

## Executive Summary

Completed change `2026-03-23-unify-onboarding-pairing-flow` was archived after syncing the accepted
OpenSpec delta into the main source of truth. No critical verification issues remained, so archive
proceeded. The new product-level onboarding spec was created, and the governing `client-surfaces`
and `dashboard` specs were updated to align transport, activation, terminology, and recovery-state
expectations. Verification warnings about incomplete default verify coverage and repo-dirtying
verification side effects were carried forward for follow-up.

## Specs Synced

| Domain | Action | Details |
|--------|--------|---------|
| onboarding | Created | Main spec did not exist; copied the approved onboarding spec into `openspec/specs/onboarding/spec.md`. |
| client-surfaces | Updated | Merged 2 added requirements (`Onboarding Contract Alignment`, `Cross-Surface Recovery State Coverage`) and 1 modified requirement (`Transport Invariant`) into `openspec/specs/client-surfaces/spec.md`. |
| dashboard | Updated | Merged 1 added requirement (`Dashboard Onboarding Boundary`) and 2 modified requirements (`Accepted-Path Activation Guidance`, `Deterministic Diagnosis and Fallback Commands`) into `openspec/specs/dashboard/spec.md`. |

## Archive Operation

- Moved `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/` to
  `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/`.
- Preserved exploration, proposal, delta specs, design, tasks, verification report, and this archive
  report in the audit trail.
- Preserved the completed task ledger (`23/23`) and PASS WITH WARNINGS verification evidence.

## Warnings Carried Forward

1. `openspec/config.yaml` still points verification testing at `make test`, which does not cover the
   Rust cargo suite or web Vitest suites touched by this change; full verification still requires
   supplemental `make rust-test` and `make web-test-all` runs.
2. Verification commands still dirty the working tree by emitting coverage `.profraw` files under
   `clients/agent-runtime/`, and `make build` runs `agentsyncApply`, which can update managed MCP
   config files and `.gitignore` during validation.
3. Suggested follow-up remains to adopt a non-mutating repo-wide verify target and route LLVM
   coverage scratch output to ignored temp storage.

## Artifacts

- `openspec/specs/onboarding/spec.md`
- `openspec/specs/client-surfaces/spec.md`
- `openspec/specs/dashboard/spec.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/exploration.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/proposal.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/design.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/tasks.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/specs/onboarding/spec.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/specs/client-surfaces/spec.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/specs/dashboard/spec.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/verify-report.md`
- `openspec/changes/archive/2026-03-24-2026-03-23-unify-onboarding-pairing-flow/archive-report.md`

## Next Recommended

- Update `openspec/config.yaml` verify commands to cover Gradle, Rust, and web suites without
  supplemental archive-time commands.
- Redirect coverage scratch artifacts away from tracked paths so verification no longer dirties the
  working tree.

## Risks

- Remaining verification-stack hygiene warnings can still reduce confidence in future PASS results if
  the default verify target is used without the supplemental Rust and web coverage.
- Validation side effects from coverage emission and `agentsyncApply` can obscure unrelated changes in
  the worktree during future archive cycles.
