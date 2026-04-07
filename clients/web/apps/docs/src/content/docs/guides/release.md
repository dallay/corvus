---
title: Release Process
description: Canonical Corvus stable and snapshot release runbook for release-please, publish workflows, and GitHub Releases.
owner: team-platform
status: canonical
lastReviewed: 2026-04-07
appliesTo: main
docType: runbook
---

This runbook defines the canonical Corvus release contract.

- `release-please.yml` owns the repo-wide release PR and the canonical `vX.Y.Z` tag.
- `publish-release.yml` and `_publish.yml` own artifact publication plus the final GitHub Release.
- GitHub Releases are the canonical stable release notes surface.
- `publish-snapshot.yml` is a snapshot-only Gradle/Maven path and does not own stable release notes.

## Prerequisites

Before you publish, confirm:

1. **Repository access**
   - You are a maintainer for `dallay/corvus`.
   - `RELEASE_PLEASE_TOKEN` is configured so release-please can open PRs and create canonical tags.
2. **Gradle/Maven release credentials**
   - `SIGNING_IN_MEMORY_KEY`
   - `SIGNING_IN_MEMORY_KEY_PASSWORD`
   - `MAVEN_CENTRAL_USERNAME`
   - `MAVEN_CENTRAL_PASSWORD`
3. **Stable release channel credentials**
   - `CARGO_REGISTRY_TOKEN`
   - `NPM_TOKEN`
   - `DOCKERHUB_USERNAME`
   - `DOCKERHUB_TOKEN`

## Stable Release Contract

### What ships in a stable `vX.Y.Z` release

Stable publish automation validates and publishes only shipped artifacts:

- Gradle/KMP artifacts, including build-logic publication
- `clients/agent-runtime` crate
- `modules/cerebro` release assets
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
5. release-please creates the canonical `vX.Y.Z` tag.
6. `publish-release.yml` runs from that tag and calls `_publish.yml`.
7. `_publish.yml` validates shipped artifact versions, publishes artifacts, and creates or updates the GitHub Release.

The GitHub Release is the canonical public release record. The root `CHANGELOG.md` is only a pointer.

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

### `_publish.yml`

Check the workflow summary for:

- incoming tag
- shipped-artifact version check table
- optional credential warnings
- publish outcome per release surface
- npm policy notes confirming `corvus-cli` is internal/private and Windows ARM64 is unsupported
- GitHub Release publication result

## Manual Baseline Recovery

Baseline recovery is an operator action. The workflows in this change do **not** create or rewrite live tags or releases automatically. Treat this as manual recovery, not workflow-owned repair.

Use this procedure when the manifest, tags, or GitHub Releases drift:

1. Verify `.release-please-manifest.json`, `version.txt`, Gradle properties, Cargo manifests, and shipped npm package versions agree on the expected stable version.
2. Verify the intended release commit SHA.
3. Verify whether the canonical `vX.Y.Z` tag already exists.
4. Verify whether the GitHub Release already exists.
5. If the release commit and version files are correct but the tag or release is missing, backfill the missing tag and rerun the stable publish workflow as a manual operator action.
6. If evidence conflicts, stop and choose a new forward release baseline instead of rewriting history.

## Troubleshooting

### No release PR

- Confirm `RELEASE_PLEASE_TOKEN` is present.
- Confirm commits follow Conventional Commits.
- Review the `release-please.yml` summary before changing config.

### Release PR merged but no stable publish run

- Confirm the canonical `vX.Y.Z` tag exists.
- Confirm the tag was created by release-please on the repository with the expected permissions.

### Stable publish failed

- Review the shipped-artifact version check table in `_publish.yml`.
- Review credential warnings for Maven, Cargo, npm, and Docker.
- Repair forward from the failed publish stage. Do not cut a competing tag for the same version.

### GitHub Release missing after artifacts published

- Rerun or repair the GitHub Release portion of `_publish.yml`.
- Do not treat `CHANGELOG.md` as the source of truth.

## Canonical References

- [GitHub Releases for dallay/corvus](https://github.com/dallay/corvus/releases)
- [GitHub Actions workflow guide](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [GPG setup guide](./gpg-setup/)
