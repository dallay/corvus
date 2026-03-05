---
title: Release Process
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

4. **Write permissions**: You must be a maintainer of the repository

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

This project uses a two-branch model for releases:

- **`main`**: Stable releases. Bug fixes and non-breaking changes land here
- **`minor`**: Next minor version development. Features land here

See [MAINTENANCE.md](https://github.com/dallay/corvus/blob/main/.github/MAINTENANCE.md) for the
complete workflow.

## Publishing a Release

### Step 1: Ensure all changes are merged

Make sure all changes you want to release are in the correct branch:

- **Patch release**: Changes should be in `main`
- **Minor release**: Changes should be in `minor`

### Step 2: Update the version

Update the version in all release version targets:

```text
gradle.properties
gradle/build-logic/gradle.properties
clients/web/package.json
clients/web/apps/*/package.json
clients/web/packages/*/package.json
clients/agent-runtime/Cargo.toml
clients/agent-runtime/npm/corvus-cli/package.json
clients/agent-runtime/npm/corvus/package.json
clients/agent-runtime/npm/corvus-darwin-arm64/package.json
clients/agent-runtime/npm/corvus-darwin-x64/package.json
clients/agent-runtime/npm/corvus-linux-arm64/package.json
clients/agent-runtime/npm/corvus-linux-x64/package.json
clients/agent-runtime/npm/corvus-windows-arm64/package.json
clients/agent-runtime/npm/corvus-windows-x64/package.json
```

### Automating version sync from Git tag

You can keep the project version in sync with a Git tag automatically using the helper script and
Make target included in this repository.

- `make sync-version` — runs `./sync-version-with-tag.sh` and syncs the latest semantic Git tag
  version (`vX.Y.Z`) into:
  - `gradle.properties` (`VERSION=`)
  - `gradle/build-logic/gradle.properties` (`VERSION=`)
  - `clients/web/package.json` (`"version"`)
  - every web app in `clients/web/apps/*/package.json` (`"version"`)
  - every web shared package in `clients/web/packages/*/package.json` (`"version"`)
  - `clients/agent-runtime/Cargo.toml` (`version = "..."`)
  - `clients/agent-runtime/npm/corvus-cli/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-darwin-arm64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-darwin-x64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-linux-arm64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-linux-x64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-windows-arm64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-windows-x64/package.json` (`"version"`)
- `./sync-version-with-tag.sh` — shell script that selects the globally latest semantic tag using
  `git tag --sort=-v:refname | grep -Em1 '^v[0-9]+\.[0-9]+\.[0-9]+$'` (not the nearest tag from
  `HEAD`), extracts the numeric version (drops the leading `v`), and updates all version targets
  listed above.

Usage patterns (pick one workflow):

1) Recommended (update code first, then tag)

```bash
# Update build files and commit
# bump version in gradle.properties or build.gradle.kts to 0.1.1
git add gradle.properties
git commit -m "chore: bump version to 0.1.1"

# Create an annotated tag matching the version
git tag -a v0.1.1 -m "Release v0.1.1"
# Push commit and tag
git push origin main
git push origin v0.1.1
```

2) If you created the tag first (what caused the CI failure you saw), sync the code to the tag
   locally and commit the change

```bash
# Ensure tag exists locally (or fetch it)
git fetch --tags

# Sync version files to the latest tag
make sync-version
# Review and commit the change
git add gradle.properties gradle/build-logic/gradle.properties clients/web/package.json clients/web/apps/*/package.json clients/web/packages/*/package.json clients/agent-runtime/Cargo.toml clients/agent-runtime/npm/corvus-cli/package.json clients/agent-runtime/npm/corvus/package.json clients/agent-runtime/npm/corvus-*/package.json
git commit -m "chore: sync version to $(awk -F= '/^VERSION=/{print $2; exit}' gradle.properties)"
# Push the commit (no need to recreate the tag)
git push origin main
```

Notes and caveats:

- The release CI enforces that the Git tag (e.g. `v0.1.1`) matches all tracked version files
  (Gradle + web monorepo + Cargo + runtime npm package matrix). If they do not match, the build
  fails.
- In `clients/agent-runtime/npm/corvus/package.json`, keep `optionalDependencies` versions aligned
  with the same release version.
- Prefer creating the commit that updates the version before creating the tag to avoid mismatches.
- The script only recognizes tags matching the regex `^v[0-9]+\.[0-9]+\.[0-9]+$`.

### Step 3: Create and push a tag

```bash
# Checkout the appropriate branch
git checkout main  # or git checkout minor

# Pull latest changes
git pull origin main

# Create an annotated tag
git tag -a v1.2.3 -m "Release version 1.2.3"

# Push the tag (this triggers the release workflow)
git push origin v1.2.3
```

**Important**: The tag must match the pattern `v[0-9]+.[0-9]+.[0-9]+` (e.g., `v1.2.3`)

### Step 4: Monitor the workflow

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
3. Select the branch (usually `main` or `minor`)
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
- **Version mismatch**: Tag version must match Gradle + web monorepo + Cargo + runtime npm package
  versions (`clients/agent-runtime/npm/*`)
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
- [ ] Version is synced in all targets (Gradle, web monorepo, Cargo, runtime npm package matrix)
- [ ] CHANGELOG.md is updated (if maintained manually)
- [ ] GPG key is valid and not expired
- [ ] Maven Central credentials are current
- [ ] crates.io, npm, and Docker Hub release secrets are set
- [ ] Tag follows the `vX.Y.Z` format
- [ ] Working on the correct branch (`main` for patches, `minor` for features)

## See Also

- [GPG Setup Guide](./gpg-setup/)
- [GitHub Workflows](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [Contributing Guide](https://github.com/dallay/corvus/blob/main/.github/CONTRIBUTING.md)
