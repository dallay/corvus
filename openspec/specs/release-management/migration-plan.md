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
