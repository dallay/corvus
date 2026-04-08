# Proposal: Professionalize Release Please Monorepo

## Problem

Corvus currently operates as a single repo-wide release train, but the release-please setup has drifted from reality. The manifest says `1.0.0`, the repository history shows a merged `chore: release v1.0.0` commit, and the visible tag history still tops out at `v0.5.0`. At the same time, release orchestration, publish workflows, release notes, and version fan-out are only partially aligned:

- release-please is configured as one root `simple` package, but it fans version bumps across Gradle, Cargo, runtime npm packages, and private web package manifests.
- publish automation is triggered by one repo-wide `vX.Y.Z` tag and validates one global version.
- GitHub release-note ownership is split between release-please settings, `_publish.yml`, docs, and a stale root `CHANGELOG.md`.
- the npm publish matrix does not fully match the set of runtime packages being version-bumped.

The result is an unprofessional release surface: baseline recovery is unclear, ownership boundaries are blurred, and unnecessary version churn increases risk.

## Goals

- Re-establish a trustworthy release-please baseline so manifest state, git tags, and GitHub releases agree.
- Formalize release-please as the canonical orchestrator for one repo-wide Corvus release train.
- Define a clean contract between release-please, tag-triggered publish workflows, and GitHub release notes.
- Reduce version bump scope to shipped artifacts and explicitly handle intentional exclusions.
- Reconcile the runtime npm publish contract so versioned packages are either published or clearly excluded by policy.

## Non-Goals

- Converting Corvus to true multi-component or independently versioned monorepo releases.
- Redesigning the snapshot channel beyond clarifying its relationship to stable releases.
- Changing the repo-wide version model for shipped Gradle, Rust, runtime npm, and Cerebro artifacts.
- Adding new distribution channels or materially changing artifact contents.

## Scope

### In Scope

- Recover and document release-please baseline/state drift, including manifest, tags, bootstrap residue, and release history expectations.
- Tighten `release-please-config.json` so repo-wide version fan-out matches the intended shipped artifact set.
- Clarify ownership between `.github/workflows/release-please.yml`, `.github/workflows/publish-release.yml`, and `.github/workflows/_publish.yml`.
- Standardize GitHub release-note/changelog ownership and retire contradictory behavior or documentation.
- Reconcile runtime npm package versioning versus publish matrix coverage, including documenting intentional omissions such as unpublished targets.
- Update release runbook/docs so operators have one accurate mental model for stable releases.

### Out of Scope

- Splitting tags, manifests, or workflows into per-component release trains.
- Reworking unrelated CI/CD pipelines outside the release orchestration path.
- Broad web-app packaging changes beyond excluding private apps from unnecessary release version churn.

## Approach

Keep the current single-version architecture and professionalize it instead of replacing it. The implementation should proceed in this order:

1. **Recovery first**: repair baseline drift so release-please state matches real repository release state.
2. **Canonical orchestration**: make release-please responsible for release PRs, version updates, and canonical repo-wide tagging.
3. **Clean publish contract**: keep publish workflows tag-driven, but make release creation/notes ownership explicit and consistent.
4. **Version scope control**: keep global versioning for shipped artifacts, remove or justify unnecessary bumps for private/non-shipped packages.
5. **Documented exceptions**: where an artifact is versioned but intentionally not published, capture that policy in workflow/docs rather than leaving silent gaps.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `.github/workflows/release-please.yml` | Modified | Clarify release-please role and any needed recovery/output handling |
| `release-please-config.json` | Modified | Reduce drift, remove stale bootstrap config, narrow extra-file fan-out |
| `.release-please-manifest.json` | Modified | Repair baseline to match canonical release state |
| `.github/workflows/publish-release.yml` | Modified | Preserve one repo-wide tag trigger while clarifying orchestration contract |
| `.github/workflows/_publish.yml` | Modified | Align publish matrix, release-note generation, and release asset creation with the new contract |
| `clients/web/apps/docs/src/content/docs/guides/release.md` | Modified | Document the canonical stable release flow and exclusions accurately |
| `CHANGELOG.md` | Modified/Removed | Retire or realign stale changelog ownership |
| `clients/web/**/package.json` | Modified | Stop unnecessary repo release churn for private web apps if they are not shipped artifacts |
| `clients/agent-runtime/npm/*/package.json` | Modified | Align versioned runtime packages with publish intent and documented policy |

## Expected Outcome

After this change, Corvus will have one explicit and reliable release train:

- release-please opens and maintains the single repo-wide release PR.
- the canonical `vX.Y.Z` tag reflects real release state and correctly triggers publish automation.
- publish workflows and GitHub release creation operate with a clear division of responsibilities.
- only shipped artifacts are forced through repo-wide version churn unless an exclusion is intentionally documented.
- operators can follow one release runbook without conflicting sources of truth.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Baseline repair may be more complex than the visible drift suggests | Medium | Treat recovery as a first-class deliverable and verify against tags, releases, and workflow behavior before steady-state cleanup |
| Token/permission issues may be the real cause of missing tags/releases | Medium | Include workflow/auth contract review in scope, not just manifest edits |
| Removing private web apps from version bumps may break hidden assumptions | Medium | Audit version checks/docs and keep exclusions explicit |
| Changelog ownership changes could create duplicate or missing release notes | Medium | Choose one canonical source and remove contradictory paths/documentation |
| Unpublished runtime npm targets may represent a product gap, not just a docs gap | Medium | Force an explicit keep/publish/remove decision during implementation |

## Rollback Plan

If the professionalized release flow causes instability, revert the workflow/config/doc changes together and restore the prior release-please and publish configuration from git history. Do not attempt partial rollback of manifest/workflow/docs independently; the recovery path should restore the last known working orchestration contract as a unit.

## Dependencies

- Access to repository tag/release history and GitHub Actions behavior for baseline verification.
- Agreement on which artifacts are part of the canonical shipped release set.

## Success Criteria

- [ ] release-please manifest/baseline is reconciled with the repository's actual release state.
- [ ] one repo-wide `vX.Y.Z` release contract is explicit across config, workflows, and docs.
- [ ] publish workflows and GitHub release-note ownership are consistent and non-duplicative.
- [ ] private web apps are no longer version-bumped by default unless intentionally required.
- [ ] runtime npm package versioning matches the actual publish matrix or documented exclusions.
