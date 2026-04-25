# Release Decoupling Migration Plan

## Goal

Define a phased rollout from the current repo-wide release train to component-scoped release
versioning and publish orchestration.

## Rollout phases

### Phase 0: Baseline freeze and observability

- Preserve current stable/beta release behavior.
- Require `component-inventory.md` and `impact-map.md` to be current before automation changes.
- Confirm existing manifest/config/workflow state is healthy enough to serve as rollback baseline.

### Phase 1: Documentation-first contract

- Add canonical inventory, impact map, and migration plan docs.
- Do not change tag format, manifest shape, or publish gating yet.
- Success signal: maintainers can determine release scope from docs alone.

### Phase 2: Pilot component versioning

- Select one pilot component with the smallest shipped surface and lowest transversal dependency load.
- Introduce component-scoped version state for that pilot while preserving repo-wide behavior for
  non-pilot components.
- Success signal: pilot release PR/tag/release output is correct and readable.
- Failure signal: release-please cannot reconcile state, or publish scope becomes ambiguous.

### Phase 3: Pilot selective publish

- Gate stable/beta publish validation and artifact publication using the impact rules.
- Support single-component and shared-change multi-component paths.
- Success signal: changed component releases validate and publish without touching unaffected
  components.
- Failure signal: missing artifacts, over-publishing, or skipped validation.

### Phase 4: Expand component coverage

- Repeat the pilot pattern for remaining components.
- Remove repo-wide assumptions only after each component has a proven release contract.

## Rollback rules

- Roll back the smallest failing scope only: docs, config, manifest, workflow, or publish gating.
- Revert `release-please` config + manifest files together when version-state changes are involved.
- Revert workflow callers and `_publish.yml` together when publish gating changes are involved.
- Do not mint competing tags to recover from a bad rollout; restore the last known-good config
  first.
- Do not advance to the next phase until the previous phase has explicit verification evidence.

## Rollback procedures for current rollout slices

Use `git revert` to preserve audit history. Prefer reverting the smallest failing slice that restores a
known-good release posture.

### Slice A — Rook pilot component decoupling

Current commit:

- `b7444cbd` — `feat(release): start component-scoped release decoupling with rook pilot`

Use when:

- the pilot resolver chooses the wrong component set,
- pilot `rook` release PR output becomes unreadable,
- or stable/beta release-please behavior regresses immediately after the pilot rollout.

Rollback command:

```bash
git revert b7444cbd
```

Expected scope restored:

- pilot-only component release behavior is removed,
- prior repo-wide assumptions resume for the affected files touched by that slice.

### Slice B — Centralized publish gating refactor

Current commit:

- `b3841bc5` — `refactor(release): centralize component gating in publish workflows`

Use when:

- publish jobs skip unexpectedly,
- the component-flag outputs do not match resolver results,
- or workflow/job dependency changes break artifact publication order.

Rollback command:

```bash
git revert b3841bc5
```

Expected scope restored:

- `_publish.yml` returns to the previous repeated-gating posture,
- centralized component flags are removed.

### Slice C — Expand release-managed components to corvus-runtime and cerebro

Current commit:

- `43266d2f` — `feat(release): expand component versioning to corvus-runtime and cerebro`

Use when:

- `release-please` cannot reconcile the expanded manifest state,
- component release PR generation becomes ambiguous,
- or non-rook component version updates fan out incorrectly.

Rollback command:

```bash
git revert 43266d2f
```

Expected scope restored:

- `release-please-config.json` and `release-please-beta-config.json` return to the prior component set,
- `.release-please-manifest.json` and `.release-please-beta-manifest.json` return to the prior state,
- `_publish.yml` no longer validates `corvus-runtime` and `cerebro` version surfaces added by that slice.

Critical pairing rule:

- revert `release-please` config and manifest files together,
- never leave config expanded while manifests remain on the older component set.

### Slice D — Keep gradle-kmp validate-only

Current commit:

- `1ff3d95d` — `refactor(release): keep gradle-kmp validate-only`

Use when:

- Gradle version-alignment checks are too strict for the current rollout,
- or the validate-only posture needs to be relaxed temporarily while investigating Gradle publication
  state.

Rollback command:

```bash
git revert 1ff3d95d
```

Expected scope restored:

- `_publish.yml` stops checking `gradle.properties` and `gradle/build-logic/gradle.properties`,
- migration docs no longer call out `gradle-kmp` validate-only as the current explicit policy.

### Paired rollback rules by file group

If a rollback must be performed manually instead of by commit, keep these file groups aligned:

- Version-state group:
  - `release-please-config.json`
  - `release-please-beta-config.json`
  - `.release-please-manifest.json`
  - `.release-please-beta-manifest.json`
- Publish-gating group:
  - `.github/workflows/release-please.yml`
  - `.github/workflows/release-please-beta.yml`
  - `.github/workflows/_publish.yml`
- Policy/docs group:
  - `openspec/specs/release-management/component-versioning.md`
  - `openspec/specs/release-management/pipeline-gating.md`
  - `openspec/specs/release-management/migration-plan.md`

### Post-rollback verification checklist

After any rollback:

- run `actionlint` on the touched workflow files,
- confirm the expected affected component set is recorded in the rollback PR description,
- verify config/manifest pairs were reverted together when version state changed,
- verify no compensating or alternate release tags were created,
- and stop rollout progression until the restored state is understood and validated.

## Pilot selection rule

Choose the first pilot component using all of these constraints:

1. smallest shipped artifact set
2. minimal dependency on shared version roots
3. clear operator-facing release outcome
4. easy rollback if tag or publish behavior is wrong

## Verification evidence required per phase

- docs/spec diff merged
- exact files changed listed in PR
- expected affected component set recorded
- stable/beta workflow behavior observed or dry-run validated
- rollback command/document steps prepared before merge
