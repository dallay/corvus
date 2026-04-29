# Proposal: Internal Release Dependency Sync for Release PRs

## Intent

Prevent release PRs from entering an invalid state where `release-please` updates release-managed crate versions but leaves versioned internal path dependencies out of sync. The repository currently releases `corvus-runtime`, `cerebro`, and `rook` as independent components, but `corvus-runtime` also ships a versioned path dependency on `cerebro`. When those version surfaces drift, Cargo resolution fails before lockfile regeneration, coverage, or release validation can complete.

This change introduces a narrow, executable contract for versioned internal release dependencies so release PR automation can both validate and automatically repair those pins before heavier CI jobs run.

## Scope

### In Scope

1. **Canonical internal release dependency contract**
   - Define the initial set of versioned internal release dependencies that MUST stay aligned.
   - Record which upstream component version owns each downstream dependency pin.
   - Clarify how this contract relates to the broader release-component graph.

2. **Executable sync and validation behavior**
   - Specify a canonical script interface for `--check` and `--write` modes.
   - Specify fail-closed behavior when declared dependencies drift or when new unmanaged versioned internal release dependencies appear.
   - Specify machine-readable and operator-readable output expectations.

3. **Release PR and CI workflow integration**
   - Require release PR maintenance automation to run the sync in write mode before lockfile regeneration.
   - Require a lightweight validation step to run before heavier workflows such as Sonar or coverage.
   - Require stable and beta release paths to share the same synchronization semantics.

4. **Lockfile and publish contract alignment**
   - Clarify that lockfile regeneration is downstream of dependency sync, not the primary drift detector.
   - Clarify that publish validation must treat internal release dependency drift as a contract violation.

### Out of Scope

- Replacing `release-please` with a custom release engine.
- General dependency management for external crates.
- Promoting additional non-publishable surfaces into the semantic release train.
- Reworking the full release-component graph beyond the minimum data needed for internal dependency alignment.

## Affected Areas

- `openspec/specs/release-management/spec.md`
- `openspec/specs/release-management/component-versioning.md`
- `openspec/specs/release-management/component-inventory.md`
- `config/release-components.json`
- `scripts/sync-internal-release-deps.mjs`
- `.github/workflows/release-please.yml`
- `.github/workflows/release-please-beta.yml`
- `.github/workflows/sync-cargo-lockfiles.yml`
- `.github/workflows/pull-request-check.yml`
- `clients/web/apps/docs/src/content/docs/guides/release.md`
- `clients/web/apps/docs/src/content/docs/es/guides/release.md`

## Risks

1. **Contract drift between config and manifests**
   - If the declared internal dependency graph and the Cargo manifests diverge, automation could rewrite the wrong fields or miss required pins.

2. **Over-coupling release workflows to implementation details**
   - If workflow steps assume too much about one specific dependency layout, the automation may become brittle when more components are added.

3. **False confidence from partial coverage**
   - If the first slice only models `corvus-runtime -> cerebro` but the repository later gains more versioned internal release dependencies without updating the contract, failures could reappear.

## Rollback Plan

If the automation proves unstable:

- disable the write-mode workflow integration,
- keep the check-mode validation available for diagnosis,
- fall back to current manual/lockfile-based drift discovery,
- and preserve the documented contract so the implementation can be retried incrementally.

Because this change is contract-first, rollback should remove live workflow coupling before removing the contract definition itself.

## Success Signals

- Release PR automation updates internal versioned path dependencies before lockfile regeneration.
- CI fails early with a clear error when internal release dependency pins drift.
- `cargo generate-lockfile` no longer becomes the first detector for this class of release mismatch.
- Stable and beta release automation behave consistently for internal dependency alignment.
