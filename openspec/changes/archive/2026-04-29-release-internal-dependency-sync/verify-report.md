# Verify Report: release-internal-dependency-sync

## Status

PASS

## Summary

The change is implemented and verified against the declared release-management behavior. The repository now defines a canonical `internalReleaseDependencies` contract, validates and rewrites managed internal release dependency pins with `scripts/sync-internal-release-deps.mjs`, persists synchronized manifests in release maintenance workflows, stages rewritten manifests during Cargo lockfile sync, and documents the behavior in release runbooks.

## Artifacts Reviewed

- `openspec/changes/release-internal-dependency-sync/proposal.md`
- `openspec/changes/release-internal-dependency-sync/design.md`
- `openspec/changes/release-internal-dependency-sync/tasks.md`
- `openspec/changes/release-internal-dependency-sync/specs/release-management/spec.md`
- `config/release-components.json`
- `scripts/release-components.mjs`
- `scripts/sync-internal-release-deps.mjs`
- `scripts/release-contract.test.mjs`
- `.github/workflows/pull-request-check.yml`
- `.github/workflows/release-please.yml`
- `.github/workflows/release-please-beta.yml`
- `.github/workflows/sync-cargo-lockfiles.yml`
- `clients/web/apps/docs/src/content/docs/guides/release.md`
- `clients/web/apps/docs/src/content/docs/es/guides/release.md`

## Scenario Compliance Matrix

### Requirement: Release-managed internal dependency synchronization

#### Scenario: Aligned internal release dependency passes validation
- Evidence: `scripts/release-contract.test.mjs`
- Result: PASS
- Notes: test `internal release dependency sync check passes when manifests are aligned` and direct script run in `--check` mode both succeed.

#### Scenario: Drifted internal release dependency is repaired in write mode
- Evidence: `scripts/release-contract.test.mjs`
- Result: PASS
- Notes: test `internal release dependency sync write mode rewrites version drift` confirms rewrite from stale version to upstream-selected version.

#### Scenario: Unmanaged internal release edge fails closed
- Evidence: `scripts/release-contract.test.mjs`, `scripts/sync-internal-release-deps.mjs`
- Result: PASS
- Notes: tests cover both `--check` and `--write` behavior for unmanaged internal path edges and assert `unmanaged-internal-release-edge` output.

#### Scenario: Path mismatch remains a hard failure
- Evidence: `scripts/release-contract.test.mjs`
- Result: PASS
- Notes: test `internal release dependency sync fails on path mismatch in both modes` confirms failure in both modes.

### Requirement: Release workflows persist synchronized pins

#### Scenario: Stable and beta release workflows commit synchronized manifests
- Evidence: `scripts/release-contract.test.mjs`, `.github/workflows/release-please.yml`, `.github/workflows/release-please-beta.yml`
- Result: PASS
- Notes: tests assert presence and order of the commit/push steps after sync in both release workflows.

#### Scenario: Lockfile sync workflow commits rewritten manifests with lockfiles
- Evidence: `scripts/release-contract.test.mjs`, `.github/workflows/sync-cargo-lockfiles.yml`
- Result: PASS
- Notes: test asserts the commit step stages `clients/agent-runtime/Cargo.toml` along with lockfiles.

## Verification Commands

Executed successfully:

- `node --test scripts/release-contract.test.mjs`
- `node scripts/sync-internal-release-deps.mjs --check`
- `node scripts/sync-internal-release-deps.mjs --write`
- `cargo metadata --manifest-path clients/agent-runtime/Cargo.toml --locked --format-version 1 --no-deps`
- `cargo metadata --manifest-path clients/cerebro/Cargo.toml --locked --format-version 1 --no-deps`

## Risks / Follow-up

- The current contract covers the initial `corvus-runtime -> cerebro` edge. Future release-managed internal path dependencies MUST be added to `internalReleaseDependencies` before they are introduced into release-managed manifests.
- The source-of-truth release-management spec has already been synchronized with this change, and the archive record is complete.
