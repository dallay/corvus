# Specification: Component-Aware Release Graph for Versioned Artifacts

## Overview

This specification defines the requirements for a canonical release-component graph that drives component-scoped release planning, validation, and publication for externally versioned artifacts in the Corvus monorepo.

The graph formalizes which components are release-managed, how repository paths map to components, which components depend on each other for release coordination, and which components are publishable versus validate-only.

## Scope

This specification covers:

- The canonical release-component graph model and its required fields
- Path-to-component ownership resolution rules
- Transitive dependency expansion semantics
- Publish policy classification (publishable vs validate-only)
- Version surface alignment requirements
- Stable and beta release flow parity expectations
- Operator-facing release evidence and traceability

This specification does NOT cover:

- Implementation details of the graph resolver
- Specific file format or storage location for the graph definition
- Changes to live workflow behavior (covered in follow-up implementation)
- Promotion of non-release surfaces into semantic artifact release

## Requirements

### REQ-1: Canonical Release-Component Graph Definition

The system MUST provide one canonical release-component graph that serves as the executable source of truth for release scope resolution.

**Rationale**: Workflow-local resolver maps are duplicated and drift easily. A single canonical graph ensures stable and beta flows use identical semantics.

#### REQ-1.1: Required Graph Fields

Each release component in the graph MUST define:

- `id`: stable component identifier used across workflows, manifests, tests, and docs
- `publish_policy`: whether the component is `publishable` or `validate-only`
- `owned_paths`: repository paths whose direct modification makes the component affected
- `version_surfaces`: canonical files whose values must stay aligned when the component releases
- `published_artifacts`: externally visible release outputs (crates, npm packages, binaries, Maven publications)
- `release_channels`: supported channels (`stable`, `beta`, `snapshot`)

#### REQ-1.2: Optional Graph Fields

Each release component SHOULD define:

- `kind`: component classification (runtime/crate/npm/gradle) for operator understanding
- `shared_infra_paths`: paths that fan out to a declared set of components
- `depends_on_release_of`: upstream components whose release-relevant change requires downstream participation
- `non_release_paths`: owned repository surfaces intentionally outside semantic artifact release
- `notes`: operator-facing rationale for transitional or exceptional handling

#### Scenario: Graph provides complete component metadata

- GIVEN an operator or workflow needs to understand release scope
- WHEN it reads the canonical release-component graph
- THEN it MUST be able to determine which components are release-managed
- AND whether each component is publishable or validate-only
- AND which paths directly affect each component
- AND which version surfaces must align for each component
- AND which artifacts are published by each component
- AND which dependency edges can expand release scope transitively

### REQ-2: Path Classification and Ownership Resolution

The system MUST classify every changed repository path into exactly one of these categories:

1. **release-owned**: path belongs to a managed component's `owned_paths`
2. **shared-release-infra**: path belongs to `shared_infra_paths` and fans out to multiple components
3. **non-release**: path is intentionally outside semantic artifact release
4. **ignored**: path is operationally irrelevant to release planning

**Rationale**: Deterministic path classification is the foundation for correct component scope resolution.

#### REQ-2.1: Direct Ownership Resolution

When a changed path matches a component's `owned_paths`:

- The component MUST be marked as directly affected
- The reason MUST be recorded as "direct ownership"

#### REQ-2.2: Shared Infrastructure Fan-out

When a changed path matches `shared_infra_paths`:

- All components declared in the fan-out set MUST be marked as affected
- The reason MUST be recorded as "shared release infrastructure"

#### REQ-2.3: Non-release Path Handling

When a changed path is classified as non-release:

- It MUST NOT mint semantic artifact release scope on its own
- It MAY trigger independent deploy or validation workflows outside the release train

#### Scenario: Single-component change resolves correctly

- GIVEN only paths under `clients/rook/**` have changed
- AND `clients/rook/**` is owned by the `rook` component
- WHEN release scope is resolved
- THEN `rook` MUST be included in `affected_components`
- AND the reason MUST be "direct ownership"
- AND unrelated components MUST NOT be included

#### Scenario: Shared infrastructure fans out correctly

- GIVEN `release-please-config.json` has changed
- AND it is declared as shared release infrastructure for `[rook, cerebro, corvus-runtime, gradle-kmp]`
- WHEN release scope is resolved
- THEN all four components MUST be included in `affected_components`
- AND the reason MUST be "shared release infrastructure"

#### Scenario: Non-release change does not mint release scope

- GIVEN only paths under `clients/web/**` have changed
- AND `clients/web/**` is classified as non-release
- WHEN release scope is resolved
- THEN `affected_components` MUST be empty for semantic artifact release
- AND the resolver MUST NOT fail

### REQ-3: Transitive Dependency Expansion

After direct ownership and shared infrastructure resolution, the system MUST expand release scope through declared `depends_on_release_of` edges.

**Rationale**: Components that ship versioned dependencies on other managed components must release together to preserve version consistency across published artifacts.

#### REQ-3.1: Transitive Closure

The resolver MUST:

1. Start with the set of directly affected components
2. For each affected component, check if any downstream component declares a `depends_on_release_of` edge to it
3. Add downstream components to the affected set
4. Repeat until no new components are added (transitive closure)

#### REQ-3.2: Reason Preservation

For each transitively affected component, the resolver MUST record:

- The upstream component that triggered inclusion
- The dependency edge that caused the expansion

#### Scenario: Transitive dependency expands scope correctly

- GIVEN only paths under `clients/cerebro/**` have changed
- AND `cerebro` is directly affected
- AND the graph declares that `corvus-runtime` depends on release of `cerebro`
- WHEN release scope is resolved
- THEN `cerebro` MUST be included with reason "direct ownership"
- AND `corvus-runtime` MUST be included with reason "transitive dependency on cerebro"

#### Scenario: No transitive expansion when upstream is not affected

- GIVEN only paths under `clients/rook/**` have changed
- AND `rook` is directly affected
- AND no component declares a `depends_on_release_of` edge to `rook`
- WHEN release scope is resolved
- THEN only `rook` MUST be included in `affected_components`
- AND no transitive expansion MUST occur

### REQ-4: Fail-Closed for Unmapped Paths

Once the canonical graph is fully adopted, the system MUST fail closed when a release-relevant changed path is not classified by the graph.

**Rationale**: Silent fallback can hide missing ownership rules and cause incorrect release scope.

#### REQ-4.1: Unmapped Path Detection

The resolver MUST fail when:

- A changed path is not matched by any `owned_paths`, `shared_infra_paths`, `non_release_paths`, or ignored patterns
- AND the path is determined to be release-relevant (not docs-only, not test-only, not CI-only)

#### REQ-4.2: Operator Feedback

When failing on an unmapped path, the resolver MUST:

- Report the unmapped path clearly
- Suggest whether it should be added to a component's `owned_paths`, `shared_infra_paths`, or `non_release_paths`

#### Scenario: Unmapped release-relevant path fails resolution

- GIVEN a new file `clients/new-component/src/main.rs` has been added
- AND no component's `owned_paths` matches `clients/new-component/**`
- AND the path is not classified as non-release or ignored
- WHEN release scope is resolved
- THEN the resolver MUST fail
- AND it MUST report the unmapped path `clients/new-component/src/main.rs`

### REQ-5: Publish Policy Enforcement

The system MUST distinguish between publishable and validate-only components throughout the release pipeline.

**Rationale**: Some components participate in validation and version alignment without being independent publish authorities.

#### REQ-5.1: Publishable Components

Components with `publish_policy: publishable` MUST:

- Participate in release PR creation
- Have their artifacts published to external registries
- Appear in release tags and changelogs

#### REQ-5.2: Validate-Only Components

Components with `publish_policy: validate-only` MUST:

- Participate in validation when in scope
- Appear in release summaries
- NOT be treated as independent publish authorities
- NOT publish artifacts to external registries

#### Scenario: Publishable component publishes artifacts

- GIVEN `rook` is in `affected_components`
- AND `rook` has `publish_policy: publishable`
- WHEN the publish workflow runs
- THEN `rook` artifacts MUST be published to crates.io and npm
- AND `rook` MUST appear in the release changelog

#### Scenario: Validate-only component does not publish

- GIVEN `gradle-kmp` is in `affected_components`
- AND `gradle-kmp` has `publish_policy: validate-only`
- WHEN the publish workflow runs
- THEN `gradle-kmp` validation MUST run
- AND `gradle-kmp` MUST appear in the validation summary
- BUT `gradle-kmp` artifacts MUST NOT be published to external registries

### REQ-6: Version Surface Alignment

Before publication, the system MUST verify that all version surfaces for an affected component agree on the release version.

**Rationale**: Version drift across manifests, wrappers, and dependency pins causes broken releases.

#### REQ-6.1: Component Version Consistency

For each affected publishable component, the system MUST verify:

- All files listed in `version_surfaces` contain the same version
- Wrapper packages reference the same version
- Cross-component dependency pins are consistent with the release plan

#### REQ-6.2: Validation Failure

If version surfaces disagree, the system MUST:

- Fail validation before publication
- Report which surfaces are misaligned
- Report the expected version and the actual values found

#### Scenario: Aligned version surfaces pass validation

- GIVEN `corvus-runtime` is in `affected_components` with version `1.2.3`
- AND `version.txt` contains `1.2.3`
- AND `clients/agent-runtime/Cargo.toml` contains `version = "1.2.3"`
- AND `clients/agent-runtime/npm/corvus/package.json` contains `"version": "1.2.3"`
- WHEN version validation runs
- THEN validation MUST succeed

#### Scenario: Misaligned version surfaces fail validation

- GIVEN `rook` is in `affected_components` with version `2.0.0`
- AND `version.txt` contains `2.0.0`
- BUT `clients/rook/Cargo.toml` contains `version = "1.9.0"`
- WHEN version validation runs
- THEN validation MUST fail
- AND it MUST report that `clients/rook/Cargo.toml` is misaligned

### REQ-7: Stable and Beta Parity

Stable and beta release flows MUST use identical graph semantics for component resolution.

**Rationale**: Divergent resolution logic between channels creates confusion and increases maintenance burden.

#### REQ-7.1: Shared Graph Semantics

Both stable and beta flows MUST use:

- The same `owned_paths` rules
- The same `shared_infra_paths` fan-out
- The same `depends_on_release_of` expansion
- The same `publish_policy` classification

#### REQ-7.2: Channel-Specific Differences

Only these aspects MAY differ between stable and beta:

- Prerelease version suffixes (e.g., `-beta.1`)
- Prerelease tag naming conventions
- Prerelease manifest files (`.release-please-beta-manifest.json`)

#### Scenario: Stable and beta resolve same component set

- GIVEN the same repository diff is evaluated for both stable and beta
- WHEN each resolver computes `affected_components`
- THEN both MUST produce the same component membership
- AND both MUST produce the same inclusion reasons (direct/shared/transitive)

### REQ-8: Operator-Facing Release Evidence

The system MUST make it easy for operators to understand why each component participated in a release.

**Rationale**: Opaque release scope decisions erode trust and make debugging difficult.

#### REQ-8.1: Inclusion Reasons

For each component in `affected_components`, the system MUST emit:

- Whether it was directly affected, shared-infra affected, or transitively affected
- The specific path(s) or dependency edge(s) that caused inclusion
- Whether it is publishable or validate-only

#### REQ-8.2: Summary Format

The release summary MUST clearly distinguish:

- Directly affected components
- Transitively affected components
- Validate-only components
- Publishable components

#### Scenario: Release summary explains component participation

- GIVEN `cerebro` changed directly and `corvus-runtime` was included transitively
- WHEN the release workflow completes
- THEN the summary MUST show:
  - `cerebro` as directly affected (reason: owned path `clients/cerebro/src/...`)
  - `corvus-runtime` as transitively affected (reason: depends on release of `cerebro`)
  - Both as publishable components

## Initial Managed Component Set

The canonical graph MUST initially model these components:

### `rook`

- `publish_policy`: publishable
- `owned_paths`: `clients/rook/**`
- `published_artifacts`: crate, npm wrapper/platform packages, release binaries
- `release_channels`: stable, beta

### `cerebro`

- `publish_policy`: publishable
- `owned_paths`: `clients/cerebro/**`
- `published_artifacts`: crate, binaries, release assets
- `release_channels`: stable, beta

### `corvus-runtime`

- `publish_policy`: publishable
- `owned_paths`: `clients/agent-runtime/**`
- `published_artifacts`: crate, npm wrapper/platform packages, release binaries
- `release_channels`: stable, beta
- `depends_on_release_of`: `[cerebro]`

### `gradle-kmp`

- `publish_policy`: validate-only
- `owned_paths`: `gradle/**`, `gradle.properties`, `modules/agent-core-kmp/**`
- `published_artifacts`: none (validate-only)
- `release_channels`: stable, beta (validation only)

## Non-Release Surfaces

The following surfaces MUST be classified as non-release unless explicitly promoted:

- `clients/web/**`
- `clients/androidApp/**`
- `clients/composeApp/**`
- docs-only content
- marketing-only content

## Conformance

An implementation conforms to this specification if:

1. It provides a canonical release-component graph with all required fields
2. It correctly classifies paths and resolves direct ownership
3. It correctly expands transitive dependencies
4. It fails closed on unmapped release-relevant paths
5. It enforces publish policy distinctions
6. It validates version surface alignment before publication
7. It uses identical graph semantics for stable and beta flows
8. It emits operator-facing inclusion reasons for all affected components

## References

- `openspec/specs/release-management/spec.md` - canonical release-management contract
- `openspec/specs/release-management/component-versioning.md` - component-scoped versioning design
- `openspec/specs/release-management/component-inventory.md` - managed component inventory
- `openspec/specs/release-management/impact-map.md` - path ownership and fan-out rules
- `openspec/specs/release-management/pipeline-gating.md` - component-scoped gating model
- `openspec/specs/release-management/migration-plan.md` - phased rollout strategy
