---
name: release
description:
  Use this skill when asked to create a release, publish to Maven Central,
  create GitHub releases, or manage version tagging. It guides through the
  complete release workflow for this Gradle project.
---

# Release Manager

This skill guides the creation of releases and snapshots for this Gradle-based
project, including publishing to Maven Central and GitHub Releases.

## Prerequisites

Before publishing, ensure the user has:

1. **GPG Key configured**: Must have a signing key set up
2. **Maven Central access**: Repository secrets configured:
  - `SIGNING_IN_MEMORY_KEY`: GPG private key
  - `SIGNING_IN_MEMORY_KEY_PASSWORD`: GPG key passphrase
  - `MAVEN_CENTRAL_USERNAME`: Maven Central username
  - `MAVEN_CENTRAL_PASSWORD`: Maven Central password
3. **Write permissions**: Must be a maintainer of the repository

If any prerequisite is missing, inform the user and guide them to the
[GPG Setup Guide](../gpg-setup/) first.

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
3. Update `docs/website/package.json` version field

**Option B - Sync from Git tag (if tag exists first):**

```bash
# Sync version files to the latest tag
make sync-version

# Review changes
git diff gradle.properties gradle/build-logic/gradle.properties docs/website/package.json
```

The `make sync-version` command runs `./sync-version-with-tag.sh` which:

- Finds the latest semantic Git tag (`vX.Y.Z`)
- Extracts the numeric version (drops leading `v`)
- Updates all version targets

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

Snapshots are useful for testing changes before a正式 release.

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
  - **Version mismatch**: Git tag must match code version (e.g., `v1.2.3` = `1.2.3`)

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
- [ ] Version updated in build files
- [ ] CHANGELOG.md updated (if maintained)
- [ ] GPG key valid and not expired
- [ ] Maven Central credentials current
- [ ] Tag follows `vX.Y.Z` format
- [ ] Working on correct branch (main for patches, minor for features)

## Commands Reference

```bash
# Sync version from latest Git tag
make sync-version

# Verify current version
cat gradle.properties | grep VERSION

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
- [GitHub Workflows](https://github.com/dallay/starter-gradle/blob/main/.github/workflows/README.md)
