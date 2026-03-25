---
name: github-actions
description: >
  GitHub Actions CI/CD best practices for workflow design, security hardening, and pipeline optimization.
  Trigger: When creating, reviewing, or auditing GitHub Actions workflows (.github/workflows/*.yml),
  fixing CI/CD issues, or hardening pipeline security.
license: Apache-2.0
metadata:
  author: "@dallay"
  version: "1.0"
---

# GitHub Actions CI/CD Skill

Expert guidance for designing, securing, and optimizing GitHub Actions CI/CD pipelines.

## When to Use

- Creating or modifying `.github/workflows/*.yml` files
- Auditing existing CI/CD pipelines for security, performance, or reliability issues
- Fixing workflow triggers, permissions, or caching problems
- Setting up deployment pipelines (staging, production, rollback)
- Integrating security scanning (SAST, SCA, secret scanning) into CI
- Optimizing workflow execution time (caching, matrix, parallelism)

## Critical Patterns

### Security (Non-Negotiable)

- **Pin actions to full SHA**: Always use `uses: owner/action@<40-char-sha> # vX.Y.Z`. Tags
  (`@v4`, `@main`) are mutable and vulnerable to supply chain attacks. Add version as comment for
  readability.
- **Least-privilege permissions**: Set `permissions: {}` or `contents: read` at workflow level.
  Override per-job only when writes are needed. Never leave permissions at default (broad write).
- **Secrets via `secrets.*` only**: Never hardcode sensitive data. Use environment-specific secrets
  for deployment workflows with manual approval gates.
- **OIDC over static credentials**: For cloud auth (AWS, Azure, GCP), prefer OIDC short-lived
  tokens over long-lived access keys stored as secrets.
- **Audit third-party actions**: Prefer `actions/` org. Review source for external actions. Enable
  Dependabot for action version updates.
- **`pull_request_target` caution**: This trigger runs with write permissions against the base repo.
  Never checkout untrusted PR code and execute it in this context.

### Workflow Structure

- **Descriptive names**: Workflow `name` and step `name` must be clear for log readability.
- **Concurrency control**: Use `concurrency` with `cancel-in-progress: true` to prevent duplicate
  runs and waste.
- **Conditional execution**: Use `if` conditions for branch-specific, actor-specific, or
  event-specific logic.
- **Reusable workflows**: Extract common patterns into `workflow_call` workflows (prefix with `_`
  for internal ones, e.g., `_publish.yml`).
- **Timeout**: Set `timeout-minutes` on long-running jobs to prevent hung runners.

### Performance

- **Caching**: Use `actions/cache` (pinned SHA) with `hashFiles` keys for package managers and
  build outputs. Use `restore-keys` for fallback.
- **Shallow clone**: Use `fetch-depth: 1` unless full history is needed (release tagging, blame).
- **Matrix parallelization**: Use `strategy.matrix` with `fail-fast: false` for comprehensive
  multi-env testing.
- **Artifacts for inter-job data**: Use `actions/upload-artifact` / `actions/download-artifact`
  (pinned SHAs) with appropriate `retention-days`.

### Deployment

- **Environment protection**: Use GitHub `environment` with required reviewers, branch restrictions,
  and deployment URLs.
- **Rollback strategy**: Keep previous artifacts. Implement automated rollback on health check
  failure.
- **Version validation**: Verify version consistency across manifests before publishing.
- **Idempotent publishes**: Check if version already exists before publishing to registries.

## Decision Tables

### When to Use `fetch-depth: 0` (Full History)

| Use Case                    | `fetch-depth` |
|-----------------------------|---------------|
| Standard build/test         | `1`           |
| Release tagging/changelog   | `0`           |
| Commit message validation   | `0`           |
| CodeQL / deep analysis      | `0` or omit   |
| Dependency lockfile fix     | `0`           |

### Choosing Deployment Strategy

| Risk Tolerance | Strategy        | Rollback Speed |
|----------------|-----------------|----------------|
| Low            | Blue/Green      | Instant        |
| Medium         | Canary          | Fast           |
| High           | Rolling Update  | Moderate       |
| Feature flags  | Dark Launch     | Instant        |

### Permission Mapping

| Workflow Need                    | Permission Required          |
|----------------------------------|------------------------------|
| Read code only                   | `contents: read`             |
| Push commits / create tags       | `contents: write`            |
| Comment on / update PRs          | `pull-requests: write`       |
| Upload SARIF security results    | `security-events: write`     |
| Deploy to GitHub Pages           | `pages: write`, `id-token: write` |
| Publish to GitHub Packages       | `packages: write`            |
| Create/update issues             | `issues: write`              |
| Manage caches                    | `actions: write`             |

## Code Examples

### Secure Workflow Skeleton

```yaml
# yaml-language-server: $schema=https://json.schemastore.org/github-workflow.json
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
        with:
          fetch-depth: 1
      # ... build steps
```

### Effective Caching Pattern

```yaml
- name: Cache dependencies
  uses: actions/cache@0c907a75c2c80ebcb7f088228285e798b750cf8f # v4.2.1
  with:
    path: |
      ~/.npm
      ./node_modules
    key: ${{ runner.os }}-node-${{ hashFiles('**/package-lock.json') }}
    restore-keys: |
      ${{ runner.os }}-node-
```

### Environment-Protected Deployment

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: production
      url: https://prod.example.com
    permissions:
      contents: read
      id-token: write  # For OIDC
    steps:
      - name: Configure cloud credentials (OIDC)
        uses: aws-actions/configure-aws-credentials@7474bc4690e29a8392af63c5b98e7449536d5c3a # v4
        with:
          role-to-assume: arn:aws:iam::123456789:role/deploy
          aws-region: us-east-1
```

## Audit Checklist

When reviewing workflows, check:

1. **All `uses:` pinned to full SHA** with version comment
2. **`permissions:` explicitly set** (workflow + job level)
3. **`concurrency:` configured** for non-trivial workflows
4. **`timeout-minutes:` set** on long-running jobs
5. **Secrets accessed via `secrets.*`** only, never in logs
6. **`fetch-depth: 1`** unless full history needed
7. **Caching configured** for package manager dependencies
8. **`if` conditions** for conditional steps (bot exclusions, branch filters)
9. **Environment protection** for deployment jobs
10. **No mutable action refs** (`@v4`, `@main`, `@latest`)

## Troubleshooting Quick Reference

| Symptom                        | Likely Cause                         | Fix                                          |
|--------------------------------|--------------------------------------|-----------------------------------------------|
| Workflow not triggering        | Wrong `on` trigger or path filter    | Verify branch/path/event config               |
| `Resource not accessible`      | Missing `permissions`                | Add required permission at job level           |
| Cache miss every run           | Dynamic cache key                    | Use `hashFiles` for stable keys               |
| Slow checkout                  | `fetch-depth: 0` on large repo       | Use `fetch-depth: 1`                          |
| Flaky tests in CI              | Race conditions, env differences     | Use explicit waits, `services` for deps       |
| `pull_request_target` exploit  | Running untrusted code with perms    | Never checkout PR head in `_target` trigger   |

## Commands

```bash
# Validate workflow YAML syntax locally
npx yaml-lint .github/workflows/*.yml

# List action versions and check for updates
gh api repos/actions/checkout/releases/latest --jq '.tag_name'

# Resolve tag to immutable SHA (for pinning)
git ls-remote --tags https://github.com/actions/checkout.git | rg "refs/tags/v6.0.2(\^\{\})?$" | sort | tail -n 1 | awk '{print $1}'

# Check workflow run status
gh run list --workflow=pull-request-check.yml --limit=5
```

## Resources

- [GitHub Actions Security Hardening](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)
- [GitHub Actions Reusable Workflows](https://docs.github.com/en/actions/sharing-automations/reusing-workflows)
- See [pinned-tag skill](../pinned-tag/SKILL.md) for SHA resolution workflow
