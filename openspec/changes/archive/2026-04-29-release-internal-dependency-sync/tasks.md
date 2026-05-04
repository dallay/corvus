# Implementation Tasks: release-internal-dependency-sync

## Phase 1 — Contract definition

1.1 Update `openspec/specs/release-management/spec.md` to define internal release dependency synchronization as part of the canonical release contract.

1.2 Update `openspec/specs/release-management/component-versioning.md` to document versioned internal path dependencies as release-managed invariants.

1.3 Update `openspec/specs/release-management/component-inventory.md` to record the initial `corvus-runtime -> cerebro` dependency alignment rule and any future release-managed internal edges.

1.4 Extend `config/release-components.json` with a canonical `internalReleaseDependencies` section covering the initial managed edge and required metadata for validation/sync.

## Phase 2 — Executable sync and validation

2.1 Implement `scripts/sync-internal-release-deps.mjs` with a `--check` mode that fails on drift, missing dependency entries, path mismatches, unmanaged internal release edges, and unreadable upstream versions.

2.2 Implement `scripts/sync-internal-release-deps.mjs` with a `--write` mode that safely rewrites stale internal dependency version pins while leaving external dependencies untouched.

2.3 Add automated tests for aligned state, drift detection, drift rewrite, path mismatch, missing entry, and unmanaged edge scenarios.

## Phase 3 — Workflow integration

3.1 Update stable and beta release PR maintenance workflows so internal dependency sync runs in write mode after release-please version bumps and before lockfile regeneration.

3.2 Update `sync-cargo-lockfiles.yml` so lockfile regeneration assumes synchronized manifests and no longer acts as the primary drift detector.

3.3 Add an early pull-request validation step that runs the sync script in check mode before Sonar, coverage, or other heavier Rust validation.

## Phase 4 — Documentation and verification

4.1 Update release runbook documentation in English and Spanish to explain internal dependency sync behavior, failure diagnostics, and expected remediation.

4.2 Verify stable and beta workflows share the same dependency sync semantics and produce actionable summaries on failure.

4.3 Review the contract, script behavior, and workflow summaries to ensure new versioned internal release dependencies cannot be added silently without config coverage.
