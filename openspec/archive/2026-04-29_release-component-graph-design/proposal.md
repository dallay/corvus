# Proposal: Component-Aware Release Graph for Versioned Artifacts

## Intent

Define the next release-management slice that turns the current component-aware release pilot into an explicit, dependency-aware release graph for externally versioned artifacts. The repository already uses `release-please` plus `affected_components` gating for `rook`, `cerebro`, `corvus-runtime`, and `gradle-kmp`, but the rules that decide release scope still live largely as workflow-local maps and partial conventions.

This change makes release scope a first-class contract. It preserves `googleapis/release-please-action` as the canonical owner of release PRs, tags, version bumps, and changelog generation, while adding one explicit source of truth for:

- which components are release-managed,
- which paths directly affect those components,
- which shared release files fan out to multiple components,
- which components are publishable versus validate-only,
- and which transitive dependency edges require downstream release participation.

The goal is to support independent release PR creation for externally versioned artifacts without allowing dependency/version drift or accidental repo-wide release churn.

## Scope

### In Scope

1. **Canonical release-component graph contract**
   - Define a release-component graph model for externally versioned and published artifacts.
   - Distinguish direct path ownership from transitive release dependencies.
   - Record publish policy (`publishable` vs `validate-only`) per managed component.

2. **Release scope resolution rules**
   - Specify how changed paths resolve to directly affected components.
   - Specify how shared release infrastructure fans out to declared component sets.
   - Specify how transitive dependency edges expand the final affected release set.
   - Specify fail-closed behavior for unmapped release-relevant paths once the model is fully adopted.

3. **Managed component inventory clarification**
   - Clarify that the initial managed set is `rook`, `cerebro`, `corvus-runtime`, and `gradle-kmp`.
   - Clarify that `gradle-kmp` remains validate-only unless explicitly promoted later.
   - Clarify that web, docs, Android, and Compose surfaces remain outside the semantic artifact release train unless they become externally published artifacts.

4. **Workflow and publish contract alignment**
   - Specify that stable and beta workflows must use the same component graph semantics.
   - Specify that publish validation and artifact publication must consume graph-derived `affected_components`.
   - Specify version-consistency invariants for version files, wrapper packages, and cross-component dependency pins.

5. **Operator-facing changelog and traceability rules**
   - Clarify that changelog and release scope must stay component-distinguishable.
   - Clarify that direct and transitive inclusion reasons should be explainable from workflow summaries and release evidence.

### Out of Scope

- Replacing `release-please` with a different release engine.
- Making web apps, docs, Android, or Compose clients semantic-release participants in this slice.
- Promoting `gradle-kmp` to an independent `release-please` manifest authority in this slice.
- Changing live workflow behavior as part of this proposal artifact alone.
- Designing deploy-only workflows for non-release surfaces.

## Affected Areas

- `openspec/specs/release-management/spec.md`
- `openspec/specs/release-management/component-versioning.md`
- `openspec/specs/release-management/component-inventory.md`
- `openspec/specs/release-management/impact-map.md`
- `openspec/specs/release-management/pipeline-gating.md`
- `openspec/specs/release-management/migration-plan.md`
- future release graph configuration/resolver implementation
- `.github/workflows/release-please.yml`
- `.github/workflows/release-please-beta.yml`
- `.github/workflows/publish-release.yml`
- `.github/workflows/_publish.yml`
- `scripts/release-contract.test.mjs`

## Risks

1. **Config drift between graph, manifests, and workflows**
   - If the release graph contract diverges from `release-please` package definitions or publish logic, the wrong components could release.

2. **Over-broad transitive fan-out**
   - If dependency rules are too coarse, the repository could regress toward near repo-wide release scope.

3. **Hidden unmapped release surfaces**
   - If a release-relevant path is not modeled, required downstream releases could be missed.

4. **Operator confusion about validate-only components**
   - `gradle-kmp` could be mistaken for a publish authority if publish policy remains implicit.

## Rollback Plan

If follow-up implementation proves too risky or introduces release ambiguity:

- keep the current workflow-local resolver behavior as the operational fallback,
- preserve existing `release-please` configs/manifests as the stable authority,
- and roll back only the graph-consuming implementation while retaining the design/spec documentation for a later retry.

Because this proposal is documentation-first, rollback for this change artifact is limited to not applying the graph-backed implementation until confidence is established.

## Success Signals

- Maintainers can determine release participation from one explicit component graph contract instead of workflow-local maps.
- The release-management specs clearly distinguish direct ownership, shared release infra fan-out, and transitive release dependencies.
- The release contract explicitly excludes non-published surfaces from semantic artifact release unless promoted later.
- Follow-up workflow implementation can consume one canonical model for both stable and beta release scope.
