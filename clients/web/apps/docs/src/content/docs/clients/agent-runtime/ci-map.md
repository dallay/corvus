---
title: "CI Workflow Map"
description: Reference map of GitHub workflows, their responsibilities, trigger conditions, and whether they should block merges.
owner: team-runtime
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: reference
---

# CI Workflow Map

This document explains what each GitHub workflow does, when it runs, and whether it should block
merges.

## Merge-Blocking vs Optional

Merge-blocking checks should stay small and deterministic. Optional checks are useful for automation
and maintenance, but should not block normal development.

### Merge-Blocking

- `.github/workflows/ci.yml` (`CI`)
  - Purpose: Rust validation (`fmt`, `clippy`, `test`, release build smoke)
  - Merge gate: `CI Required Gate`
- `.github/workflows/workflow-sanity.yml` (`Workflow Sanity`)
  - Purpose: lint GitHub workflow files (`actionlint`, tab checks)
  - Recommended for workflow-changing PRs

### Non-Blocking but Important

- `.github/workflows/docker.yml` (`Docker`)
  - Purpose: PR docker smoke check and publish images on `main`/tag pushes
- `.github/workflows/security.yml` (`Security Audit`)
  - Purpose: dependency advisories (`cargo audit`) and policy/license checks (`cargo deny`)
- `.github/workflows/publish-release.yml` (`Publish Release`)
  - Purpose: publish stable artifacts after the canonical GitHub Release is published
- `.github/workflows/release-please-beta.yml` (`Release Please Beta`)
  - Purpose: create beta prerelease PRs, tags, GitHub Releases, and beta artifact publication from the `beta` branch

### Optional Repository Automation

- `.github/workflows/labeler.yml` (`PR Labeler`)
  - Purpose: path labels + size labels
- `.github/workflows/auto-response.yml` (`Auto Response`)
  - Purpose: first-time contributor onboarding messages
- `.github/workflows/stale.yml` (`Stale`)
  - Purpose: stale issue/PR lifecycle automation
- `.github/workflows/pr-hygiene.yml` (`PR Hygiene`)
  - Purpose: nudge stale-but-active PRs to rebase/re-run required checks before queue starvation

## Trigger Map

- `CI`: push to `main`/`develop`, PRs to `main`
- `Docker`: push to `main`, tag push (`v*`), PRs touching docker/workflow files, manual dispatch
- `Publish Release`: `release.published` after `release-please` creates the canonical GitHub Release
- `Release Please Beta`: push to `beta`, manual dispatch

## Stable Release Governance Note

- `release-please` owns the stable release PR, canonical `vX.Y.Z` tag, canonical GitHub Release, and release notes.
- `publish-release.yml` and `_publish.yml` start from `release.published` and attach artifacts to the existing GitHub Release.
- This keeps `release-please` as the only canonical release-note authority while still letting asset publication run after the release exists.
- `release-please-beta.yml` owns the beta release PR, canonical `vX.Y.Z-beta.N` tag, beta GitHub Release, and beta release notes.
- `_publish.yml` publishes beta artifacts only when `release-please-beta.yml` calls it with `prerelease: true`.
- `Security Audit`: push to `main`, PRs to `main`, weekly schedule
- `Workflow Sanity`: PR/push when `.github/workflows/**`, `.github/*.yml`, or `.github/*.yaml`
  change
- `PR Labeler`: `pull_request_target` lifecycle events
- `Auto Response`: issue opened, `pull_request_target` opened
- `Stale`: daily schedule, manual dispatch
- `PR Hygiene`: every 12 hours schedule, manual dispatch

## Migration Direction: Component-Aware Gating

Current live workflow behavior remains repo-scoped: the merge gate is still the small deterministic
set described above, and stable publication still begins only after the canonical repo-wide release
exists.

The documented migration direction for release decoupling is to add component-aware release gating
without changing canonical release ownership:

- keep `release-please` as the repo-wide authority for the stable release PR, tag, GitHub Release,
  and release notes,
- introduce component-scoped release state so each managed component has explicit version,
  eligibility, and publish-policy metadata,
- derive future stable-release validation from the subset of components that are release-eligible
  for that cycle,
- classify managed components as `publish`, `validate-only`, or `excluded`,
- and avoid treating unrelated private or excluded components as automatic stable-release blockers
  solely because they live in the repository.

This is a documentation and design direction only. It does **not** mean current required checks,
workflow triggers, or publish logic are already component-aware.

## Fast Triage Guide

1. `CI Required Gate` failing: start with `.github/workflows/ci.yml`.
2. Docker failures on PRs: inspect `.github/workflows/docker.yml` `pr-smoke` job.
3. Stable release failures: inspect `.github/workflows/release-please.yml` and `.github/workflows/publish-release.yml`.
4. Beta release failures: inspect `.github/workflows/release-please-beta.yml`.
5. Security failures: inspect `.github/workflows/security.yml` and `deny.toml`.
6. Workflow syntax/lint failures: inspect `.github/workflows/workflow-sanity.yml`.

## Maintenance Rules

- Keep merge-blocking checks deterministic and reproducible (`--locked` where applicable).
- Prefer explicit workflow permissions (least privilege).
- Use path filters for expensive workflows when practical.
- Avoid mixing onboarding/community automation with merge-gating logic.
- When updating CI documentation, distinguish current live gates from planned component-aware
  release gating so operators do not infer behavior that workflows do not yet implement.
