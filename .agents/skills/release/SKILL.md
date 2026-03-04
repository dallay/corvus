---
name: release
description:
  Use this skill when asked to create a release, publish to Maven Central,
  create GitHub releases, or manage version tagging. It guides through the
  complete release workflow for this Gradle project.
---

# Release Manager

This skill guides release and snapshot operations for Corvus across Gradle/KMP,
Rust, npm, Docker images, and GitHub Releases.

## Prerequisites

Before publishing, ensure the user has:

1. **GPG Key configured**: Must have a signing key set up
2. **Maven Central access**: Repository secrets configured:

- `SIGNING_IN_MEMORY_KEY`: GPG private key
- `SIGNING_IN_MEMORY_KEY_PASSWORD`: GPG key passphrase
- `MAVEN_CENTRAL_USERNAME`: Maven Central username
- `MAVEN_CENTRAL_PASSWORD`: Maven Central password

3. **Release channel secrets** (required for stable release tags):

- `CARGO_REGISTRY_TOKEN`: crates.io token
- `NPM_TOKEN`: npm token for `@dallay/corvus`
- `DOCKERHUB_USERNAME`: Docker Hub username
- `DOCKERHUB_TOKEN`: Docker Hub token

4. **Write permissions**: Must be a maintainer of the repository

If any prerequisite is missing, inform the user and guide them to the [GPG Setup Guide](https://github.com/dallay/corvus/blob/main/clients/web/apps/docs/src/content/docs/en/guides/gpg-setup.md) first.

## Release Surface

For `publish-release.yml` (tag `vX.Y.Z`), expect:

- Gradle/KMP artifacts to Maven Central
- Build logic plugin publication to Maven Central (release mode)
- Rust crate publication from `clients/agent-runtime` to crates.io
- npm runtime publication for `@dallay/corvus` and platform packages under `clients/agent-runtime/npm/*`
- Docker image publication to Docker Hub and GHCR
- Native binaries (Linux, macOS, Windows) + SHA256 checksums attached to GitHub Release

For `publish-snapshot.yml`:

- Snapshot publication is Gradle/Maven-only
- No crates.io, npm, Docker, or GitHub Release asset publication

## Branch Model

This project uses a two-branch model:

- **`main`**: Stable releases (bug fixes, non-breaking changes)
- **`minor`**: Next minor version development (features)

Ask the user which type of release they want to make:

- **Patch release** (e.g., 1.2.3 → 1.2.4): Changes in `main`
- **Minor release** (e.g., 1.2.3 → 1.3.0): Changes in `minor`

## Publishing a Release

Follow these steps:

### Step 1: Verify Changes

Ensure all changes to be released are in the correct branch:

- Patch: `main`
- Minor: `minor`

Help the user verify with:

```bash
git checkout main  # or minor
git pull origin main  # or minor
git log --oneline -10
```

### Step 2: Update Version

The user has two options for version management:

**Option A - Manual (update code first, then tag):**

1. Update version in `gradle.properties`:
   ```
   VERSION=1.2.3
   ```
2. Update `gradle/build-logic/gradle.properties`:
   ```
   VERSION=1.2.3
   ```
3. Update web monorepo versions:
   - `clients/web/package.json`
   - `clients/web/apps/*/package.json`
   - `clients/web/packages/*/package.json`
4. Update `clients/agent-runtime/Cargo.toml` `version` field
5. **Regenerate `Cargo.lock`** — the CI build uses `--locked`, so the lockfile must be
   committed and in sync with `Cargo.toml` before pushing the tag:
   ```bash
   cd clients/agent-runtime
   cargo generate-lockfile
   cd -
   git add clients/agent-runtime/Cargo.lock
   ```
   > **Why this matters**: `cargo build --locked` fails with exit code 101 if `Cargo.lock`
   > is stale. Always regenerate and commit it as part of the version bump commit.
6. Update runtime npm package versions:
   - `clients/agent-runtime/npm/corvus-cli/package.json`
   - `clients/agent-runtime/npm/corvus/package.json`
   - `clients/agent-runtime/npm/corvus-darwin-arm64/package.json`
   - `clients/agent-runtime/npm/corvus-darwin-x64/package.json`
   - `clients/agent-runtime/npm/corvus-linux-arm64/package.json`
   - `clients/agent-runtime/npm/corvus-linux-x64/package.json`
   - `clients/agent-runtime/npm/corvus-windows-arm64/package.json`
   - `clients/agent-runtime/npm/corvus-windows-x64/package.json`
7. In `clients/agent-runtime/npm/corvus/package.json`, keep `optionalDependencies` versions aligned with the same release version

**Option B - Sync from Git tag (if tag exists first):**

```bash
# Sync version files to the latest tag
make sync-version

# Review changes
git diff gradle.properties gradle/build-logic/gradle.properties clients/web/package.json clients/web/apps/*/package.json clients/web/packages/*/package.json clients/agent-runtime/Cargo.toml clients/agent-runtime/npm/corvus-cli/package.json clients/agent-runtime/npm/corvus/package.json clients/agent-runtime/npm/corvus-*/package.json
```

The `make sync-version` command runs `./sync-version-with-tag.sh` which:

- Finds the latest semantic Git tag (`vX.Y.Z`)
- Extracts the numeric version (drops leading `v`)
- Updates all tracked version targets (Gradle, web monorepo, Cargo, npm)

### Step 3: Create and Push Tag

```bash
# Create annotated tag matching the pattern vX.Y.Z
git tag -a v1.2.3 -m "Release version 1.2.3"

# Push the tag (triggers release workflow)
git push origin v1.2.3
```

**Important**: Tag must match `^v[0-9]+\.[0-9]+\.[0-9]+$` (e.g., `v1.2.3`)

### Step 4: Monitor Workflow

1. Go to **Actions** tab in GitHub
2. Click on **Publish Release** workflow
3. Wait for completion (5-10 minutes)

The workflow will:

- Build the project
- Generate changelog from conventional commits
- Publish to Maven Central
- Create GitHub release with changelog

## Publishing a Snapshot

Snapshots are useful for testing changes before a stable release.

### Automatic (Daily)

The `publish-snapshot.yml` workflow runs daily at 02:12 UTC.

### Manual Trigger

1. Go to **Actions** → **Publish Snapshot**
2. Click **Run workflow**
3. Select branch (`main` or `minor`)
4. Click **Run workflow**

Snapshots use version with `-SNAPSHOT` suffix (e.g., `1.2.3-SNAPSHOT`).

## Troubleshooting

### Release workflow failed

1. Check workflow logs in GitHub Actions
2. Common issues:

- **Signing failed**: GPG secrets not configured correctly
- **Maven Central auth failed**: Credentials expired
- **Build failed**: Run `./gradlew check` locally first
- **Version mismatch**: Git tag must match Gradle, web monorepo, Cargo, and all runtime npm package versions (`clients/agent-runtime/npm/*`)
- **Missing release secret**: cargo/npm/docker secrets missing for release workflow
- **`cargo build --locked` fails (exit 101)**: `Cargo.lock` is stale — run
  `cargo generate-lockfile` inside `clients/agent-runtime/`, commit the result, and
  push **before** creating the release tag. See [Step 2](#step-2-update-version) for
  the full procedure.

### Version already exists

Maven Central doesn't allow overwriting releases:

1. Use new patch version (e.g., `v1.2.4` instead of `v1.2.3`)
2. Never delete and recreate tags

### Snapshot not updating

Snapshots can be cached. Force update:

```bash
./gradlew build --refresh-dependencies
```

## Release Checklist

Before publishing, verify:

- [ ] All tests pass locally (`./gradlew check`)
- [ ] Version updated in all release targets (Gradle, web monorepo, Cargo, runtime npm package matrix)
- [ ] **`Cargo.lock` regenerated and committed** (`cd clients/agent-runtime && cargo generate-lockfile`)
- [ ] CHANGELOG.md updated (if maintained)
- [ ] GPG key valid and not expired
- [ ] Maven Central credentials current
- [ ] crates.io, npm, and Docker Hub secrets configured
- [ ] Tag follows `vX.Y.Z` format
- [ ] Working on correct branch (main for patches, minor for features)

## Commands Reference

```bash
# Sync version from latest Git tag
make sync-version

# Verify current version
rg '^VERSION=' gradle.properties

# Regenerate Cargo.lock after bumping Cargo.toml version (REQUIRED before tagging)
cd clients/agent-runtime && cargo generate-lockfile && cd -
git add clients/agent-runtime/Cargo.lock

# Run all checks before release
./gradlew check

# Create annotated tag
git tag -a v1.2.3 -m "Release v1.2.3"

# Push tag (triggers release)
git push origin v1.2.3
```

## See Also

- [Release Process Documentation](../release/)
- [GPG Setup Guide](../gpg-setup/)
- [GitHub Workflows](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
