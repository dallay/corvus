---
title: Release Process
description: Canonical release procedure for publishing Corvus artifacts through GitHub Actions, signing, and Maven Central workflows.
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: runbook
---

This guide explains how to publish a full Corvus release (KMP + Rust + web artifacts) using
GitHub Actions.

## Prerequisites

Before you can publish, ensure you have:

1. **GPG Key configured**: Follow the [GPG Setup Guide](./gpg-setup/) to create and configure your
   signing key
2. **Maven Central access**: Repository secrets configured:

- `SIGNING_IN_MEMORY_KEY`: Your GPG private key
- `SIGNING_IN_MEMORY_KEY_PASSWORD`: GPG key passphrase
- `MAVEN_CENTRAL_USERNAME`: Maven Central username
- `MAVEN_CENTRAL_PASSWORD`: Maven Central password

3. **Release channel secrets** for non-Gradle artifacts:

- `CARGO_REGISTRY_TOKEN`: crates.io publishing token for `clients/agent-runtime`
- `NPM_TOKEN`: npm token for `@dallay/corvus`
- `DOCKERHUB_USERNAME`: Docker Hub account username
- `DOCKERHUB_TOKEN`: Docker Hub access token

4. **Release Please token**: `RELEASE_PLEASE_TOKEN` with repo write access so release-please can
   open PRs and create tags that trigger publishing workflows
5. **Write permissions**: You must be a maintainer of the repository

### What gets released

When `publish-release.yml` runs from a `vX.Y.Z` tag, it publishes:

- **KMP/Gradle artifacts** to Maven Central (`publishToMavenCentral`)
- **Build logic plugin artifacts** to Maven Central on stable release tags
- **Rust crate** (`clients/agent-runtime`) to crates.io
- **npm runtime packages** (`clients/agent-runtime/npm/*`) to npm, including
  `@dallay/corvus` plus platform-specific packages
- **Container images** to Docker Hub and GHCR
- **Native binaries + checksums** attached to the GitHub Release

Web apps (`clients/web/apps/docs`, `clients/web/apps/marketing`,
`clients/web/apps/dashboard`) are built in their own workflows and are not published to Maven,
crates.io, or npm by `publish-release.yml`.

## Understanding the Branch Model

Releases are cut from `main`. All release-ready changes must be merged into `main` before the
release-please PR is merged.

## Publishing a Release

### Step 1: Merge changes to `main`

Make sure all changes you want to ship are merged into `main`.

### Step 2: Release Please opens the release PR

On every push to `main`, the Release Please workflow opens or updates a release PR that:

- Bumps versions across Gradle, Cargo, npm, and web packages
- Updates optionalDependencies in `clients/agent-runtime/npm/corvus/package.json`
- Generates release notes based on Conventional Commits

If you need to control the version bump, use Conventional Commits:

- `fix:` -> patch
- `feat:` -> minor
- `feat!:` or `BREAKING CHANGE:` -> major

### Step 3: Review and merge the release PR

Review the release PR for version alignment and merge it when ready.

### Step 4: Tag and publish

When the release PR merges, Release Please creates a `vX.Y.Z` tag. That tag triggers
`publish-release.yml`, which runs `_publish.yml` to publish all artifacts.

### Step 5: Monitor the workflow

1. Go to **Actions** tab in GitHub
2. Click on the **Publish Release** workflow
3. Wait for completion (usually 5-10 minutes)

The workflow will:

- Build and publish Gradle/KMP artifacts to Maven Central
- Publish the Rust crate to crates.io
- Publish the npm CLI package
- Build and publish Docker images (Docker Hub + GHCR)
- Build native binaries for Linux, macOS, and Windows, generate SHA256 checksums, and attach them to
  GitHub Release
- Generate a changelog and create/update the GitHub Release notes

After the GitHub Release is published, `deploy-docs.yml` may also deploy docs to GitHub Pages.

## Publishing a Snapshot

Snapshots are published automatically daily, but this applies to the Gradle/Maven channel only.

### Automatic (Daily)

The `publish-snapshot.yml` workflow runs daily at 02:12 UTC.

### Manual

1. Go to **Actions** tab → **Publish Snapshot**
2. Click **Run workflow**
3. Select the branch (usually `main`)
4. Click **Run workflow**

Snapshots use the version defined in your Gradle build files with a `-SNAPSHOT` suffix.
Rust crates, npm package, Docker images, and GitHub Release assets are only published on
stable `vX.Y.Z` releases.

## Troubleshooting

### Release workflow failed

1. Check the workflow logs in GitHub Actions
2. Common issues:

- **Signing failed**: Check GPG secrets are correctly configured
- **Maven Central auth failed**: Verify credentials haven't expired
- **Build failed**: Ensure all tests pass locally with `./gradlew check`
- **Version mismatch**: Release Please should keep versions aligned. If it fails, check
  `release-please-config.json` and the release PR diff
- **Release PR not created**: Missing `RELEASE_PLEASE_TOKEN`, or commits are not Conventional
- **Missing release secret**: `CARGO_REGISTRY_TOKEN`, `NPM_TOKEN`,
  `DOCKERHUB_USERNAME`, or `DOCKERHUB_TOKEN`

### Version already exists

Maven Central doesn't allow overwriting releases. If you need to fix something:

1. Use a new patch version (e.g., `v1.2.4` instead of `v1.2.3`)
2. Never delete and recreate tags with the same version

### Snapshot not updating

Snapshots can be cached by Maven/Gradle. Force an update:

```bash
./gradlew build --refresh-dependencies
```

## Release Checklist

Use this checklist before publishing:

- [ ] All tests pass locally (`./gradlew check`)
- [ ] Release PR is up to date and merged
- [ ] Versions are aligned in the release PR diff
- [ ] GPG key is valid and not expired
- [ ] Maven Central credentials are current
- [ ] crates.io, npm, and Docker Hub release secrets are set
- [ ] Release tag `vX.Y.Z` was created by Release Please

## See Also

- [GPG Setup Guide](./gpg-setup/)
- [GitHub Workflows](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [Contributing Guide](https://github.com/dallay/corvus/blob/main/.github/CONTRIBUTING.md)
