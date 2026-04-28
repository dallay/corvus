# Release Decoupling Migration Plan

## Goal

Define a phased rollout from the current repo-wide release train and workflow-local component resolver logic toward a canonical release-component graph that drives component-scoped release planning, validation, and publication for externally versioned artifacts.

## Rollout phases

### Phase 0: Baseline freeze and observability

- Preserve current stable/beta release behavior.
- Require `component-inventory.md` and `impact-map.md` to be current before automation changes.
- Confirm existing manifest/config/workflow state is healthy enough to serve as rollback baseline.

### Phase 1: Documentation-first graph contract

- Add the canonical release-component graph design and align release-management specs.
- Define managed components, publish policy, direct ownership, shared release infrastructure, and known transitive dependency edges.
- Do not change live workflow behavior yet.
- Success signal: maintainers can determine release participation and exclusions from the documented contract alone.

### Phase 2: Canonical executable graph definition

- Choose the file location and format for the executable release graph source of truth.
- Encode the documented ownership rules, shared-infra fan-out, publish policy, version surfaces, and dependency edges in that source.
- Add contract tests that compare graph data with existing `release-please` config/manifests.
- Success signal: one canonical model can answer release-scope questions without reading workflow-local maps.
- Failure signal: graph data cannot faithfully represent current release behavior or drifts from manifests/config.

### Phase 3: Shared resolver adoption

- Extract current workflow-local resolver logic into one reusable graph-backed resolver.
- Update `release-please.yml` and `release-please-beta.yml` to consume the shared resolver.
- Keep output parity for direct component scope while adding explicit reasoning for direct/shared/transitive inclusion.
- Success signal: stable and beta workflows compute the same component membership for the same diff, apart from channel-specific prerelease behavior.
- Failure signal: mismatched scope results, hidden regressions, or ambiguous workflow summaries.

### Phase 4: Publish validation hardening

- Update `publish-release.yml` and `_publish.yml` to consume graph-backed metadata consistently.
- Validate version surfaces and cross-component dependency pins for affected components before publication.
- Keep validate-only components visible in validation posture without treating them as standalone publish authorities.
- Success signal: affected publishable components validate and publish without touching unaffected components.
- Failure signal: missing artifacts, over-publishing, or dependency/version drift.

### Phase 5: Fail-closed hardening

- Remove permissive fallback behavior once the canonical graph covers all release-relevant paths.
- Make unmapped release-relevant changes fail closed.
- Add operator-facing guidance for interpreting direct, shared-infra, and transitive release reasons.
- Success signal: missing ownership data is surfaced immediately and corrected before release automation proceeds.
- Failure signal: maintainers still need undocumented tribal knowledge to understand release scope.

### Phase 6: Future component promotion

- Promote additional surfaces into the release-component graph only when they become externally versioned/published artifacts.
- Introduce new `release-please` package authority only after publish policy, version surfaces, and dependency posture are documented.
- Examples of future candidates may include currently non-release surfaces only if they later acquire true external artifact/version ownership.

## Rollback posture

If any implementation phase introduces ambiguity or destabilizes release behavior:

- revert to the last known-good workflow-local resolver behavior,
- preserve existing `release-please` manifests/config as the release authority baseline,
- and keep the documented graph contract for a later retry rather than forcing partially trusted automation into production use.

## Exit criteria for steady state

The migration should be considered complete only when:

- one canonical executable release graph defines managed components and release-scope semantics,
- stable and beta release planning use that graph consistently,
- publish validation enforces graph-derived version and dependency invariants,
- non-release surfaces are explicitly excluded unless promoted later,
- and maintainers can explain every component in a release by direct, shared-infra, or transitive reasoning.
