---
title: Release Process
description: Canonical Corvus stable, beta, and snapshot release runbook for release-please, publish workflows, and GitHub Releases.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: runbook
---

This runbook defines the canonical Corvus release contract.

- `release-please.yml` owns the repo-wide release PR, the canonical `vX.Y.Z` tag, the canonical GitHub Release, and the canonical stable release notes.
- `release-please-beta.yml` owns the repo-wide beta prerelease PR, the canonical `vX.Y.Z-beta.N` tag, the canonical beta GitHub Release, and the canonical beta release notes from the beta branch (`beta`).
- `publish-release.yml` and `_publish.yml` only own artifact publication after `release-please` publishes the GitHub Release.
- GitHub Releases are the canonical stable release notes surface.
- `publish-snapshot.yml` is a snapshot-only Gradle/Maven path and does not own stable release notes.

## Prerequisites

Before you publish, confirm:

1. **Repository access**
   - You are a maintainer for `dallay/corvus`.
   - `APP_ID` and `APP_PRIVATE_KEY` are configured so release-please can mint a GitHub App token with permission to open PRs and create canonical tags plus the canonical GitHub Release.
2. **Gradle/Maven release credentials**
   - `SIGNING_IN_MEMORY_KEY`
   - `SIGNING_IN_MEMORY_KEY_PASSWORD`
   - `MAVEN_CENTRAL_USERNAME`
   - `MAVEN_CENTRAL_PASSWORD`
3. **Release channel credentials**
   - `CARGO_REGISTRY_TOKEN`
   - `NPM_TOKEN`
   - `DOCKERHUB_USERNAME`
   - `DOCKERHUB_TOKEN`

## Stable Release Contract

### Canonical release scope graph and resolvers

- `config/release-components.json` is the canonical managed component graph.
- `scripts/release-components.mjs` loads and validates that graph before workflows consume it.
- `scripts/resolve-release-components.mjs` resolves changed-file scope for `release-please.yml` and `release-please-beta.yml`.
- `scripts/resolve-release-from-tag.mjs` resolves stable publish scope from the release tag and optional `affected_components:` override in the release body.

### What ships in a stable `vX.Y.Z` release

Stable publish automation validates and publishes only shipped artifacts:

- Gradle/KMP artifacts, including build-logic publication
- `clients/agent-runtime` crate
- `clients/cerebro` release assets
- npm runtime packages:
  - `@dallay/corvus`
  - `@dallay/corvus-darwin-x64`
  - `@dallay/corvus-darwin-arm64`
  - `@dallay/corvus-linux-x64`
  - `@dallay/corvus-linux-arm64`
  - `@dallay/corvus-windows-x64`
- Docker images
- Native archives and checksums attached to the GitHub Release

### Intentional exclusions

- Private web packages are excluded from stable repo-wide version churn.
- `clients/web/**/package.json` is not part of stable release-please fan-out.
- `clients/agent-runtime/npm/corvus-cli/package.json` is internal/private and is excluded from stable release fan-out and stable npm publishing.
- Windows ARM64 is intentionally unsupported for stable npm publication right now. `@dallay/corvus-windows-arm64` is not published and is not referenced by `@dallay/corvus` optional dependencies.

## Stable Release Flow

1. Merge release-ready work into `main`.
2. `release-please.yml` opens or updates one repo-wide release PR.
3. Review the release PR diff. Only shipped stable artifacts should be version-bumped.
4. Merge the release PR.
5. release-please creates the canonical `vX.Y.Z` tag and canonical GitHub Release.
6. `publish-release.yml` runs from `release.published` and passes explicit `release_tag` / `release_id` context into `_publish.yml`.
7. `_publish.yml` validates shipped artifact versions, publishes artifacts, and attaches assets to the existing GitHub Release.

The GitHub Release created by `release-please` is the canonical public release record. The root `CHANGELOG.md` is only a pointer.

## Beta Release Contract

### What ships in a beta `vX.Y.Z-beta.N` release

Beta publish automation ships the same artifact surfaces as the stable channel:

- Gradle/KMP artifacts, including build-logic publication
- `clients/agent-runtime` crate
- `clients/cerebro` release assets
- the shipped runtime npm packages
- Docker images
- Native archives and checksums attached to the GitHub Release

### Beta channel rules

- `release-please-beta.yml` runs from the `beta` branch and owns beta prerelease PRs, tags, GitHub Releases, and release notes.
- Beta releases use tags in the form `vX.Y.Z-beta.N`.
- The GitHub Release must stay marked as a prerelease.
- Beta releases use the npm `beta` dist-tag and must not overwrite `latest`.
- Beta Docker publication uses the exact prerelease version plus the moving `beta` tag and must not overwrite stable aliases like `latest`, `X`, or `X.Y`.

## Beta Release Flow

1. Cut or refresh the `beta` branch from `minor` when a prerelease candidate is ready for wider validation.
2. Merge release-ready fixes into `beta`.
3. `release-please-beta.yml` opens or updates one repo-wide beta prerelease PR.
4. Review the prerelease PR diff. Only shipped beta artifacts should be version-bumped.
5. Merge the beta release PR.
6. `release-please-beta.yml` creates the canonical `vX.Y.Z-beta.N` tag and canonical beta GitHub Release.
7. `_publish.yml` publishes beta artifacts in prerelease mode, using the npm `beta` dist-tag and beta-safe Docker tags.

### Governance rule

- Release automation changes must land through a pull request, not a direct push to `main`.
- Treat direct pushes to `main` as emergency-only recovery and document the reason when they happen.
- If release infrastructure is broken, repair it on a short-lived branch, open a PR, and let normal branch protection plus checks validate the fix.

## Snapshot Flow

`publish-snapshot.yml` is manual or scheduled and covers the Gradle/Maven snapshot channel only.

- No canonical stable tag is created.
- No GitHub Release is created.
- No stable release notes are published.

## Diagnostics to Check During a Release

### `release-please.yml`

Check the workflow summary for:

- manifest baseline version from `.release-please-manifest.json`
- candidate version output
- whether a release PR was created in that run
- whether tag/release outputs were emitted
- raw release-please action outputs for drift diagnosis

### `release-please-beta.yml`

Check the workflow summary for:

- manifest baseline version from `.release-please-beta-manifest.json`
- candidate beta version output
- whether a beta prerelease PR was created in that run
- whether beta tag/release outputs were emitted
- raw release-please action outputs for drift diagnosis

### `_publish.yml`

Check the workflow summary for:

- incoming tag
- existing GitHub Release id / asset upload target
- shipped-artifact version check table
- optional credential warnings
- publish outcome per release surface
- npm policy notes confirming `corvus-cli` is internal/private and Windows ARM64 is unsupported
- GitHub Release asset upload result
- confirmation that release-please owns canonical stable release notes
- confirmation that release-please-beta.yml owns canonical beta release notes

## Manual Baseline Recovery

Baseline recovery is an operator action. The workflows in this change do **not** create or rewrite live tags or releases automatically. Treat this as manual recovery, not workflow-owned repair.

Use this procedure when the manifest, tags, or GitHub Releases drift:

1. Verify `.release-please-manifest.json` or `.release-please-beta-manifest.json`, `version.txt`, Gradle properties, Cargo manifests, and shipped npm package versions agree on the expected release version for the channel you are repairing.
2. Verify the intended release commit SHA.
3. Verify whether the canonical `vX.Y.Z` tag already exists.
4. Verify whether the GitHub Release already exists.
5. If the release commit and version files are correct but the tag or release is missing, backfill the missing GitHub Release authority first and rerun the stable publish workflow from `release.published` as a manual recovery.
6. If evidence conflicts, stop and choose a new forward release baseline instead of rewriting history.

## Troubleshooting

### No release PR

- Confirm `APP_ID` and `APP_PRIVATE_KEY` are present and the GitHub App is installed on `dallay/corvus`.
- Confirm commits follow Conventional Commits.
- Review the `release-please.yml` summary before changing config.

### Release PR merged but no stable publish run

- Confirm the canonical GitHub Release exists and was published.
- Confirm `release-please` created the release with the expected permissions and token.
- Confirm the `publish-release.yml` trigger saw `release.published` for the same `vX.Y.Z` tag.

### Release PR merged but no beta publish run

- Confirm the canonical beta GitHub Release exists and is marked as a prerelease.
- Confirm `release-please-beta.yml` created the release with the expected permissions and token.
- Confirm `_publish.yml` was called from `release-please-beta.yml` with `prerelease: true`.

### `release-please` fails with `Resource not accessible by integration`

- Verify the workflow token can call the GitHub Releases API before changing app permissions.
- Check the merged release PR for stale `release-please` labels.
- If the merged release PR still has `autorelease: pending`, remove that label and rerun `release-please`.
- Treat a stale `autorelease: pending` label as release state drift, not as proof of a GitHub App permission problem.

### Stable publish failed

- Review the shipped-artifact version check table in `_publish.yml`.
- Review the release id / tag context passed from `publish-release.yml`.
- Review credential warnings for Maven, Cargo, npm, and Docker.
- Repair forward from the failed publish stage. Do not cut a competing tag for the same version.

### GitHub Release missing after artifacts published

- Repair the `release-please` GitHub Release first, because `_publish.yml` only attaches assets to the existing release.
- Rerun the stable publish handoff from `release.published` after the canonical release exists again.
- Do not treat `CHANGELOG.md` as the source of truth.

### Release notes drift from published assets

- Treat `release-please` as the only canonical release-note authority.
- Treat `release-please-beta.yml` as the only canonical beta release-note authority.
- `_publish.yml` may attach assets to the existing GitHub Release, but it must not replace the canonical notes body.

## Canonical References

- [GitHub Releases for dallay/corvus](https://github.com/dallay/corvus/releases)
- [GitHub Actions workflow guide](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [GPG setup guide](./gpg-setup)
