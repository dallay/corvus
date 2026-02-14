# GitHub Actions Workflows

This directory contains all GitHub Actions workflows for the starter-gradle project. Workflows are organized by purpose: CI/CD, security scanning, publishing, repository automation, and maintenance.

## 📋 Quick Reference

| Category        | Workflow                             | Purpose                              | Trigger                                 |
| --------------- | ------------------------------------ | ------------------------------------ | --------------------------------------- |
| **CI/CD**       | `pull-request-check.yml`             | Main CI checks for PRs and pushes    | Push to any branch, PR to main/minor/\* |
| **CI/CD**       | `pull-request-check-build-logic.yml` | Checks for build-logic changes       | Changes to `gradle/build-logic/**`      |
| **CI/CD**       | `deploy-docs.yml`                    | Deploy documentation to GitHub Pages | Push to `main`                          |
| **Security**    | `codeql-analysis.yml`                | Security scanning with CodeQL        | Push to main/minor, daily schedule      |
| **Publishing**  | `publish-release.yml`                | Publish release to Maven Central     | Tag push `v*.*.*`                       |
| **Publishing**  | `publish-snapshot.yml`               | Publish snapshot versions            | Manual, daily schedule                  |
| **Publishing**  | `_publish.yml`                       | Reusable publish workflow            | Called by other workflows               |
| **Automation**  | `auto-fix-lockfile.yml`              | Auto-update lockfiles                | Daily schedule, manual                  |
| **Automation**  | `fix-renovate.yml`                   | Fix lockfiles for Renovate PRs       | Comment `/fix-lock` on PR               |
| **Repo Mgmt**   | `git-issue-labeled.yml`              | Auto-comments/closes labeled issues  | Issue labeled                           |
| **Repo Mgmt**   | `git-issue-auto-close.yml`           | Close inactive issues                | Weekly schedule                         |
| **Repo Mgmt**   | `git-sync-labels.yml`                | Sync labels from config              | Push to `labels.yml`                    |
| **Quality**     | `semantic-pull-request.yml`          | Lint PR titles                       | PR open/edit                            |
| **Quality**     | `pull-request-limit.yml`             | Block changes to restricted files    | PR touching CODEOWNERS/workflows        |
| **Quality**     | `lychee-links.yml`                   | Check project links with Lychee      | Push, PR, daily schedule, manual        |
| **Maintenance** | `cleanup-cache.yml`                  | Clean up Action caches               | PR closed                               |
| **Maintenance** | `stale.yml`                          | Mark stale issues/PRs                | Daily schedule                          |
| **Reporting**   | `contributor-report.yml`             | PR contributor reports               | PR events                               |
| **Community**   | `greetings.yml`                      | Welcome new contributors             | PR/issue created                        |

---

## 🔧 CI/CD Workflows

### `pull-request-check.yml` - Main CI Pipeline

**Purpose**: Runs the primary CI checks for all pushes and pull requests.

**Triggers**:

- Push to any branch (except tags)
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
- Full fetch-depth (0) for proper commit message validation
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

- Push to `main` branch
- Manual trigger

**What it does**:

1. 📝 Checks out repository
2. 📦 Sets up pnpm 10
3. 📦 Sets up Node.js 24 with pnpm caching
4. 📥 Installs dependencies (`pnpm install`)
5. 🏗 Builds the site (`pnpm run build` in `website/docs`)
6. ⬆️ Uploads artifact for deployment
7. 🚀 Deploys to GitHub Pages

**Key Points**:

- Requires `contents: read`, `pages: write`, `id-token: write` permissions
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

## 📦 Publishing Workflows

### `publish-release.yml` - Release Publishing

**Purpose**: Publishes a release version to Maven Central and creates GitHub release.

**Triggers**:

- Tag push matching `v[0-9]+.[0-9]+.[0-9]+` (e.g., `v1.2.3`)

**What it does**:
Calls the reusable `_publish.yml` workflow with:

- `release: true` - Creates GitHub release
- `changelog: true` - Generates changelog

**Restrictions**:

- Only runs on `dallay/starter-gradle` repository
- Requires tag starting with `v`

---

### `publish-snapshot.yml` - Snapshot Publishing

**Purpose**: Publishes snapshot versions to Maven Central.

**Triggers**:

- Manual trigger (`workflow_dispatch`)
- Daily schedule (cron: `12 2 * * *`)

**What it does**:
Calls the reusable `_publish.yml` workflow with:

- `release: false` - No GitHub release created
- `changelog: false` - No changelog generated

**Restrictions**:

- Only runs on `dallay/starter-gradle` repository

---

### `_publish.yml` - Reusable Publishing Workflow

**Purpose**: Internal reusable workflow for publishing artifacts.

**Called by**: `publish-release.yml`, `publish-snapshot.yml`

**Inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `release` | boolean | required | Whether to create GitHub release |
| `changelog` | boolean | false | Whether to generate changelog |

**Secrets Required**:

- `SIGNING_IN_MEMORY_KEY` - GPG signing key
- `SIGNING_IN_MEMORY_KEY_PASSWORD` - GPG key password
- `MAVEN_CENTRAL_USERNAME` - Maven Central username
- `MAVEN_CENTRAL_PASSWORD` - Maven Central password

**What it does**:

1. 📦 Sets up build environment
2. 🌿 Generates changelog (if enabled) using `release-changelog-builder-action`
3. 👻 Publishes to Maven Central using Gradle
   - For `dallay/starter-gradle`: also publishes build-logic
   - For forks: only publishes main project
4. 🚀 Creates GitHub release (if enabled)

**Key Points**:

- ⚠️ Warning: Do not use never-expiring User Token for Maven Central
- Uses commit-based changelog generation
- Changelog config: `.github/config/changelog.json`

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

1. 📡 Validates PR is from a bot (github-actions, dependabot, or renovate)
2. 📡 Gets PR branch info
3. ✈ Checks out the PR branch
4. 📦 Sets up build environment
5. 🔏 Writes build-logic locks (if exists)
6. 🔒 Writes global locks
7. 💾 Commits and pushes changes directly to PR branch
8. 💬 Adds 👍 reaction to the comment

**Key Points**:

- Only works on PRs from automation bots
- Directly commits to the PR branch
- Fails if PR is not from an allowed bot

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

**Purpose**: Validates project links with Lychee to catch broken references early.

**Triggers**:

- Push to any branch (excluding tags)
- Pull requests to `main`, `minor`, `fix/**`, `feat/**`, `patch/**`
- Daily schedule (cron: `0 6 * * *`)
- Manual trigger

**What it does**:

1. ✈ Checks out the repository
2. 🔗 Runs `lycheeverse/lychee-action` with retries and private-link exclusions
3. Fails the job when broken links are detected

**Config files**:

- `.lycheeignore` for ignored URLs/patterns

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
- [Starter-Gradle Makefile](../Makefile) - Local development commands
- [AGENTS.md](../../AGENTS.md) - Agent development guidelines

---

## 🔄 Workflow Dependencies

```
publish-release.yml ─┐
                     ├──> _publish.yml (reusable)
publish-snapshot.yml ┘

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

1. Check the [Actions tab](https://github.com/dallay/starter-gradle/actions) for failed runs
2. Review workflow logs for error details
3. Check [GitHub Status](https://www.githubstatus.com/) for service disruptions
4. Open an issue with the `ci|actions` label
