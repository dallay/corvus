# GitHub Actions Workflows

This directory contains all GitHub Actions workflows for the Corvus monorepo. Workflows are organized by purpose: CI/CD, security scanning, publishing, repository automation, and maintenance.

## 📋 Quick Reference

| Category       | Workflow                             | Purpose                                                           | Trigger                                 |
| -------------- | ------------------------------------ | ----------------------------------------------------------------- | --------------------------------------- |
| **CI/CD**      | `pull-request-check.yml`             | Main CI checks for PRs and protected pushes                       | Push to main/minor, PR to main/minor/\* |
| **CI/CD**      | `pull-request-check-build-logic.yml` | Checks for build-logic changes                                    | Changes to `gradle/build-logic/**`      |
| **CI/CD**      | `deploy-docs.yml`                    | Deploy documentation to GitHub Pages                              | Push docs to `main`, Release published  |
| **Security**   | `codeql-analysis.yml`                | Security scanning with CodeQL                                     | Push to main/minor, daily schedule      |
| **Security**   | `snyk-security.yml`                  | Snyk SAST/SCA/Container/IaC scans                                 | Push/PR to main/minor, manual           |
| **Publishing** | `publish-release.yml`                | Attach stable artifacts to canonical GitHub Release               | `release.published`                     |
| **Publishing** | `publish-snapshot.yml`               | Publish Gradle/Maven snapshots only                               | Manual, daily schedule                  |
| **Publishing** | `release-please.yml`                 | Create repo-wide release PRs, tags, and canonical GitHub Releases | Push to `main`, manual                  |
| **Publishing** | `release-please-beta.yml`            | Create beta prerelease PRs, tags, and canonical GitHub Releases   | Push to `beta`, manual                  |
| **Publishing** | `_publish.yml`                       | Reusable stable/beta/snapshot publish workflow                    | Called by other workflows               |

## Release Scope Resolution

- `config/release-components.json` defines the canonical managed component graph.
- `scripts/resolve-release-components.mjs` is the shared changed-file resolver for release-please stable and beta.
- `scripts/resolve-release-from-tag.mjs` is the shared stable publish resolver for canonical release tags and `affected_components:` overrides.
- GitHub Releases remain the canonical stable and beta release notes surface.
- `publish-release.yml` and `_publish.yml` never author canonical release notes.
- `publish-snapshot.yml` never owns stable release notes.

| **Automation** | `auto-fix-lockfile.yml` | Auto-update lockfiles | Daily schedule, manual |
| **Automation** | `fix-renovate.yml` | Fix lockfiles for Renovate PRs | Comment `/fix-lock` on PR |
| **Repo Mgmt** | `git-issue-labeled.yml` | Auto-comments/closes labeled issues | Issue labeled |
| **Repo Mgmt** | `git-issue-auto-close.yml` | Close inactive issues | Weekly schedule |
| **Repo Mgmt** | `git-sync-labels.yml` | Sync labels from config | Push to `labels.yml` |
| **Quality** | `semantic-pull-request.yml` | Lint PR titles | PR open/edit |
| **Quality** | `pull-request-limit.yml` | Block changes to restricted files | PR touching CODEOWNERS/workflows |
| **Quality** | `detekt.yml` | Kotlin static analysis for KMP surfaces | Kotlin/Gradle changes, weekly, manual |
| **Quality** | `lychee-links.yml` | Check project links with Lychee (full repo) | Daily schedule (4am), manual |
| **Maintenance** | `cleanup-cache.yml` | Clean up Action caches | PR closed |
| **Maintenance** | `stale.yml` | Mark stale issues/PRs | Daily schedule |
| **Reporting** | `contributor-report.yml` | PR contributor reports | PR events |
| **Community** | `greetings.yml` | Welcome new contributors | PR/issue created |

---

## 🔧 CI/CD Workflows

### `pull-request-check.yml` - Main CI Pipeline

**Purpose**: Runs the primary CI checks for protected branch pushes and pull requests.

**Triggers**:

- Push to `main` and `minor` (except tags)
- Pull requests to `main`, `minor`, `fix/**`, `feat/**`, `patch/**`
- Manual trigger (`workflow_dispatch`)

**What it does**:

1. ✅ Validates commit messages (skips for bot PRs)
2. 📦 Sets up Node.js 24
3. ☕ Sets up Java 25 (Corretto)
4. 🐘 Sets up Gradle with caching
5. 🏗 Runs `./gradlew check` (includes tests, linting, formatting checks)
6. 🛠 Generates CycloneDX BOM (on push only)
7. 📈 Generates code coverage report (on push only)
8. 📤 Uploads coverage to Codecov (on push only, skips dependabot)

**Key Points**:

- Uses concurrency control to cancel in-progress runs
- Shallow checkout (`fetch-depth: 1`) because only the latest commit message is validated
- 60-minute timeout

---

### `pull-request-check-build-logic.yml` - Build Logic Checks

**Purpose**: Validates changes to the build-logic module and Gradle configuration.

**Triggers**:

- Push/PR with changes to:
  - `gradle/*.toml` (version catalogs)
  - `gradle/build-logic/**`

**What it does**:

1. Same setup as main PR check (Node, Java, Gradle)
2. Checks if build-logic exists (`:build-logic:help`)
3. Runs `:build-logic:check` if build-logic exists

**Key Points**:

- Path-filtered to only run when build-logic changes
- Conditional execution based on build-logic existence

---

### `deploy-docs.yml` - Documentation Deployment

**Purpose**: Deploys the documentation website to GitHub Pages.

**Triggers**:

- Push to `main` branch (docs-related paths only)
- Release published
- Manual trigger

**What it does**:

1. 📝 Checks out repository
2. 📦 Sets up pnpm 10
3. 📦 Sets up Node.js 24 with pnpm caching
4. 📥 Installs dependencies (`pnpm install`)
5. 🏗 Builds the site (`pnpm run build` in `clients/web/apps/docs`)
6. ⬆️ Uploads artifact for deployment
7. 🚀 Deploys to GitHub Pages

**Key Points**:

- Requires `contents: read`, `pages: write`, `id-token: write` permissions
- Uses path filters on push to avoid unnecessary deployments
- Also deploys on release publication to keep release docs aligned
- Uses GitHub Pages artifact upload and deployment actions
- Output available at GitHub Pages URL

---

## 🔒 Security Workflows

### `codeql-analysis.yml` - Security Scanning

**Purpose**: Performs security analysis using GitHub CodeQL.

**Triggers**:

- Push to `main` or `minor`
- Daily schedule (cron: `16 5 * * *`)
- Manual trigger

**What it does**:

1. ✈ Checks out repository
2. 📦 Sets up build environment (Node, Java, Gradle)
3. ⚙️ Initializes CodeQL with configuration from `.github/config/codeql.yml`
4. 🏗 Builds Java/Kotlin code manually
5. 🔍 Runs CodeQL analysis for both Actions and Java/Kotlin

**Languages Scanned**:

- `actions` - GitHub Actions workflows
- `java-kotlin` - Java and Kotlin source code

**Key Points**:

- Uses manual build mode for Java/Kotlin
- Requires `security-events: write` permission
- Config file: `.github/config/codeql.yml`

---

### `detekt.yml` - Kotlin Static Analysis

**Purpose**: Runs Detekt against Kotlin and Gradle-related surfaces and uploads SARIF to GitHub
Code Scanning.

**Triggers**:

- Push to `main` when Kotlin/Gradle-related files change
- Pull request to `main` when Kotlin/Gradle-related files change
- Weekly schedule
- Manual trigger

**What it does**:

1. Checks out the repository with shallow history
2. Resolves a pinned Detekt release asset
3. Downloads the Detekt CLI
4. Runs static analysis and uploads SARIF results

**Key Points**:

- Path-filtered to avoid running on docs/web/Rust-only changes
- Uses concurrency cancellation to avoid redundant scans
- Uploads findings via GitHub Code Scanning

---

## 📦 Publishing Workflows

release-please is the canonical owner of the stable GitHub Release and release notes.
\_publish.yml exists to attach artifacts to that existing release after `release.published`.
`release-please-beta.yml` owns the canonical beta branch prerelease PRs, tags, GitHub Releases,
and beta release notes.

### `publish-release.yml` - Release Publishing

**Purpose**: Publishes stable artifacts after `release-please` publishes the canonical GitHub
Release, then attaches assets to that existing release.

**Triggers**:

- `release.published` for the canonical stable GitHub Release

**What it does**:
Calls the reusable `_publish.yml` workflow with explicit release context:

- `release: true` - Enables stable publication mode
- `release_tag` - Canonical component-scoped stable tag from the GitHub Release event (for example `rook-vX.Y.Z`, `corvus-runtime-vX.Y.Z`, or `cerebro-vX.Y.Z`)
- `release_id` - Existing GitHub Release identifier used for asset upload
- `affected_components` - Component set resolved from the published stable tag namespace, or overridden for multi-component stable handoff with an `affected_components:` line in the GitHub Release body, then passed into `_publish.yml`

**Stable contract**:

- `release-please.yml` is the canonical owner of the stable release PR, tag, GitHub Release, and release notes.
- `publish-release.yml` starts only after `release-please` publishes that GitHub Release.
- `_publish.yml` attaches artifacts to the existing GitHub Release and must not replace canonical release notes.
- Stable version checks only cover shipped artifacts.
- Private web packages are excluded from stable version churn.
- `corvus-cli` is internal/private and is not a stable publish target.
- Windows ARM64 is intentionally unsupported and is not part of the stable npm publish matrix.

**Restrictions**:

- Only runs on `dallay/corvus` repository
- Requires a published, non-draft, non-prerelease GitHub Release with a supported component-scoped stable tag (`rook-vX.Y.Z`, `corvus-runtime-vX.Y.Z`, or `cerebro-vX.Y.Z`)
- Multi-component stable handoff may override the tag-implied component set by adding a release body line like `affected_components: rook, corvus-runtime`

---

### `publish-snapshot.yml` - Snapshot Publishing

**Purpose**: Publishes snapshot versions to Maven Central only.

**Triggers**:

- Manual trigger (`workflow_dispatch`)
- Daily schedule (cron: `12 2 * * *`)

**What it does**:
Calls the reusable `_publish.yml` workflow with:

- `release: false` - No stable GitHub Release context is required

**Snapshot contract**:

- Snapshot publishing does not create the canonical stable tag.
- Snapshot publishing does not own GitHub Release creation.
- Snapshot publishing does not own stable release notes.
- Snapshot publishing does not participate in the `release.published` stable handoff.

**Restrictions**:

- Only runs on `dallay/corvus` repository

---

### `release-please-beta.yml` - Beta Release PR Automation

**Purpose**: Opens or updates the single repo-wide beta prerelease PR from `beta` and owns the
canonical beta prerelease tag, GitHub Release, and release notes.

**Triggers**:

- Push to `beta`
- Manual trigger (`workflow_dispatch`)

**What it does**:

- Runs release-please with `release-please-beta-config.json`
- Creates or updates a beta prerelease PR with shipped-artifact beta version bumps
- Writes diagnostics to the workflow summary, including the beta manifest baseline and action outputs
- On merge, creates the canonical `vX.Y.Z-beta.N` tag and canonical GitHub prerelease
- Publishes canonical beta GitHub Release notes, then hands beta artifact publication to `_publish.yml`

**Beta contract**:

- `release-please-beta.yml` is the canonical owner of beta release PRs, beta tags, beta GitHub Releases, and beta notes.
- Beta releases publish the same shipped artifact set as stable releases.
- npm beta releases use the `beta` dist-tag instead of `latest`.
- Beta Docker publication uses the exact prerelease version plus the moving `beta` tag, and never overwrites stable aliases like `latest`.

---

### `release-please.yml` - Release PR Automation

**Purpose**: Opens or updates the single repo-wide stable release PR from `main` and owns the
canonical stable tag, GitHub Release, and release notes.

**Triggers**:

- Push to `main`
- Manual trigger (`workflow_dispatch`)

**What it does**:

- Runs release-please with `release-please-config.json`
- Creates/updates a release PR with shipped-artifact version bumps
- Checks out repository metadata before writing the workflow summary so diagnostics do not fail on missing manifest files
- Writes diagnostics to the workflow summary, including manifest baseline and action outputs
- On merge, creates the canonical `vX.Y.Z` tag and canonical GitHub Release
- Publishes the canonical release notes that downstream workflows treat as the single source of truth
- Hands off stable artifact publication through `release.published`

**Governance note**:

- Changes to release automation should land through a pull request, not by bypassing `main` protections.
- Reserve direct pushes to `main` for explicit emergency recovery only.
- If `release-please` fails with `Resource not accessible by integration`, inspect the merged release PR for a stale `autorelease: pending` label before rotating tokens or changing GitHub App permissions.

**Secrets Required**:

- `APP_ID` / `APP_PRIVATE_KEY` - GitHub App credentials used to mint the release automation token. The app installation must have at least: Contents: Read and write, Pull requests: Read and write, Issues: Read and write. This avoids relying on the default `GITHUB_TOKEN` when the repository or organization blocks release creation for the built-in integration.

---

### `_publish.yml` - Reusable Publishing Workflow

**Purpose**: Internal reusable workflow for publishing stable, beta, and snapshot artifacts.

**Called by**: `publish-release.yml`, `publish-snapshot.yml`, `release-please-beta.yml`

**Inputs**:

| Input         | Type    | Default  | Description                                           |
| ------------- | ------- | -------- | ----------------------------------------------------- |
| `release`     | boolean | required | Whether the workflow is in release mode               |
| `prerelease`  | boolean | `false`  | Whether the release is a beta prerelease              |
| `release_tag` | string  | empty    | Canonical release tag to validate and publish against |
| `release_id`  | string  | empty    | Existing GitHub Release id for asset upload           |

**Secrets Required**:

- `SIGNING_IN_MEMORY_KEY` - GPG signing key
- `SIGNING_IN_MEMORY_KEY_PASSWORD` - GPG key password
- `MAVEN_CENTRAL_USERNAME` - Maven Central username
- `MAVEN_CENTRAL_PASSWORD` - Maven Central password
- `CARGO_REGISTRY_TOKEN` - crates.io publish token (release)
- `NPM_TOKEN` - npm publish token (release)
- `DOCKERHUB_USERNAME` - Docker Hub namespace/user (release)
- `DOCKERHUB_TOKEN` - Docker Hub access token (release)

**What it does**:

1. 📦 Sets up build environment
2. 🧭 Validates explicit stable release context (`release_tag`, `release_id`) against the existing GitHub Release
3. 👻 Publishes to Maven Central using Gradle
4. 🦀 Publishes Rust crate to crates.io (release only)
5. 📦 Publishes shipped runtime npm packages to npm using `latest` for stable or `beta` for prereleases
6. 🐳 Builds and publishes multi-arch runtime Docker image to Docker Hub + GHCR with stable or beta-safe tags
7. 📊 Builds and publishes multi-arch dashboard Docker image to Docker Hub + GHCR with stable or beta-safe tags
8. 🚀 Attaches assets to the existing canonical GitHub Release

**Key Points**:

- ⚠️ Warning: Do not use never-expiring User Token for Maven Central
- `release-please` owns the canonical public GitHub Release and canonical stable release notes
- `release-please-beta.yml` owns the canonical public GitHub prerelease and canonical beta release notes
- `_publish.yml` only uploads assets with `gh release upload --clobber`
- Stable publication fans out from `release.published`
- Beta publication fans out from `release-please-beta.yml`
- Stable npm publishing excludes `corvus-cli` because it is internal/private
- Windows ARM64 is intentionally unsupported for stable npm publication

---

## 🤖 Automation Workflows

### `auto-fix-lockfile.yml` - Automatic Lockfile Updates

**Purpose**: Automatically updates Gradle lockfiles and creates a PR.

**Triggers**:

- Daily schedule (cron: `27 3 * * *`)
- Manual trigger

**What it does**:

1. 📦 Sets up build environment
2. 🔧 Checks if build-logic exists
3. 🔏 Writes build-logic locks (if exists)
4. 🔒 Writes global locks
5. 💾 Creates PR with lockfile changes using `create-pull-request`

**PR Details**:

- Branch: `auto-pr/fix-lockfile`
- Label: `pr|chore`
- Commit message: `chore: auto fix lockfile [skip ci]..`

---

### `fix-renovate.yml` - Renovate Lockfile Fix

**Purpose**: Allows maintainers to fix lockfiles for Renovate PRs via comment command.

**Triggers**:

- Comment on PR containing `/fix-lock`

**Who can use**:

- Repository OWNER
- Repository MEMBER
- Repository COLLABORATOR

**What it does**:

1. Historical workflow for maintainers to request a Renovate lockfile refresh from a PR comment
2. It is currently disabled pending a safer replacement because privileged `issue_comment` workflows must not execute PR code

**Key Points**:

- Disabled for security hardening
- Keep `auto-fix-lockfile.yml` as the safe lockfile refresh path
- Do not directly commit to untrusted PR branches from `issue_comment`

---

## 🏷️ Repository Management Workflows

### `git-issue-labeled.yml` - Issue Label Automation

**Purpose**: Automatically responds to issues based on labels added.

**Triggers**:

- Issue labeled event

**Actions by Label**:

| Label                          | Action                                                            |
| ------------------------------ | ----------------------------------------------------------------- |
| `status\|waiting-reproduction` | Comments asking for minimal reproduction with link to explanation |
| `close\|stackoverflow`         | Comments redirecting to StackOverflow and closes the issue        |

---

### `git-issue-auto-close.yml` - Auto-Close Inactive Issues

**Purpose**: Closes issues that have been inactive for 7 days with specific labels.

**Triggers**:

- Weekly schedule (cron: `0 0 */7 * *`)

**What it does**:

1. Finds open issues with labels:
   - `status|waiting-reproduction`
   - `status|waiting-feedback`
2. Checks if last update was > 7 days ago
3. Comments explaining closure
4. Closes the issue

---

### `git-sync-labels.yml` - Label Synchronization

**Purpose**: Keeps GitHub labels in sync with configuration file.

**Triggers**:

- Push to `main` that modifies `.github/config/labels.yml`
- Manual trigger

**What it does**:

1. ☯ Syncs labels using `EndBug/label-sync`
2. Deletes labels not in config (`delete-other-labels: true`)

**Config file**: `.github/config/labels.yml`

---

## ✅ Quality Workflows

### `semantic-pull-request.yml` - PR Title Linting

**Purpose**: Ensures PR titles follow conventional commit format.

**Triggers**:

- PR events: opened, edited, synchronize

**What it does**:
Calls the shared workflow from `dallay/common-actions/.github/workflows/semantic-pr.yml@main`

---

### `pull-request-limit.yml` - Restricted File Protection

**Purpose**: Prevents unauthorized changes to sensitive repository files.

**Triggers**:

- PRs modifying:
  - `.github/CODEOWNERS`
  - `.github/workflows/**`

**Who is blocked**:

- Anyone who is not OWNER, MEMBER, or COLLABORATOR

**What it does**:

1. Adds `close|invalid` label
2. Comments explaining the restriction
3. Closes the PR

---

### `lychee-links.yml` - Link Validation

**Purpose**: Runs a best-effort full-repo link audit with Lychee without adding noisy PR blockers.

**Triggers**:

- Daily schedule (cron: `0 4 * * *` - 4am UTC)
- Manual trigger

**What it does**:

1. ✈ Checks out the repository
2. 🔗 Runs `lycheeverse/lychee-action` using `lychee.toml`
3. 📝 Creates a PR with the report if broken links are found
4. 💾 Maintains lychee cache for faster subsequent runs

**Config files**:

- `.lycheeignore` for ignored URLs/patterns
- `lychee.toml` for lychee configuration
- `gradle/configs/git/hooks/pre-commit.sh` for offline staged-file checks before commit
- `Makefile` targets `make link-check` and `make link-check-local` for local reproduction

---

## 🧹 Maintenance Workflows

### `cleanup-cache.yml` - Cache Cleanup

**Purpose**: Cleans up GitHub Actions caches when PRs are closed.

**Triggers**:

- Pull request closed

**What it does**:
Calls the shared workflow from `dallay/common-actions/.github/workflows/cleanup-cache.yml@main`

---

### `stale.yml` - Stale Issue/PR Management

**Purpose**: Marks and closes stale issues and pull requests.

**Triggers**:

- Daily schedule (cron: `0 2 * * *`)

**What it does**:
Calls the shared workflow from `dallay/common-actions/.github/workflows/stale.yml@main`

---

## 📊 Reporting Workflows

### `contributor-report.yml` - Contributor Reports

**Purpose**: Generates reports for PR contributors.

**Triggers**:

- Pull request events: opened, reopened, synchronize, edited

**What it does**:
Calls the shared workflow from `dallay/common-actions/.github/workflows/contributor-report.yml@main`

---

## 👋 Community Workflows

### `greetings.yml` - Contributor Greetings

**Purpose**: Automatically greets new contributors.

**Triggers**:

- Pull request created
- Issue created

**What it does**:
Calls the shared workflow from `dallay/common-actions/.github/workflows/greetings.yml@main`

---

## 🔐 Security Best Practices

### Pinned Action Versions

All workflows use **pinned commit SHAs** for action references instead of tags:

```yaml
# ✅ Good - Pinned by SHA
uses: actions/checkout@8e8c483db84b4bee98b60c0593521ed34d9990e8 # v6.0.1

# ❌ Bad - Mutable tag
uses: actions/checkout@v6
```

This prevents supply chain attacks where a malicious actor could republish a tag with compromised code.

### Permissions

Workflows follow the principle of least privilege:

```yaml
permissions:
  contents: read # Only read repository contents
  issues: write # Only write to issues
  pull-requests: write # Only write to PRs
```

### Secrets

- Secrets are never logged or exposed
- Publishing workflows require repository-level secrets
- Forks cannot access secrets from upstream

---

## 📝 Workflow Development Guidelines

### Adding a New Workflow

1. Create file in `.github/workflows/<name>.yml`
2. Follow naming convention: lowercase with hyphens
3. Add schema declaration for IDE support:
   ```yaml
   # yaml-language-server: $schema=https://json.schemastore.org/github-workflow.json
   ```
4. Pin all action references by SHA
5. Document triggers, inputs, and outputs
6. Add entry to this README

### Testing Workflows

1. Use `workflow_dispatch` trigger for manual testing
2. Test in a fork first for destructive operations
3. Use `continue-on-error: true` for experimental steps
4. Add concurrency control to prevent conflicts:
   ```yaml
   concurrency:
     group: ${{ github.workflow }}-${{ github.ref }}
     cancel-in-progress: true
   ```

### Common Patterns

**Environment Setup**:

```yaml
- name: 📦 Setup Node
  uses: actions/setup-node@395ad3262231945c25e8478fd5baf05154b1d79f # v6.1.0
  with:
    node-version: "24"

- name: ☕ Setup Java
  uses: actions/setup-java@dded0888837ed1f317902acf8a20df0ad188d165 # v5.0.0
  with:
    java-version: "25"
    distribution: "corretto"

- name: 🐘 Setup Gradle
  uses: gradle/actions/setup-gradle@4d9f0ba0025fe599b4ebab900eb7f3a1d93ef4c2 # v5.0.0
  with:
    gradle-version: wrapper
    cache-read-only: false
```

**Bot Detection**:

```yaml
if: >
  github.event.pull_request.user.login != 'github-actions[bot]' &&
  github.event.pull_request.user.login != 'dependabot[bot]' &&
  github.event.pull_request.user.login != 'renovate[bot]'
```

---

## 📚 Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)
- [Security Hardening](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)
- [Reusable Workflows](https://docs.github.com/en/actions/using-workflows/reusing-workflows)
- [Starter-Gradle Makefile](../../Makefile) - Local development commands
- [AGENTS.md](../../AGENTS.md) - Agent development guidelines

---

## 🔄 Workflow Dependencies

```
publish-release.yml ────────┐
release-please-beta.yml     ├──> _publish.yml (reusable)
publish-snapshot.yml ───────┘

Other workflows call dallay/common-actions:
- cleanup-cache.yml
- contributor-report.yml
- semantic-pull-request.yml
- stale.yml
- greetings.yml
```

---

## 📞 Support

For issues with GitHub Actions:

1. Check the [Actions tab](https://github.com/dallay/corvus/actions) for failed runs
2. Review workflow logs for error details
3. Check [GitHub Status](https://www.githubstatus.com/) for service disruptions
4. Open an issue with the `ci|actions` label
