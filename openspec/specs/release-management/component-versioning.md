# Component-Scoped Versioning Design

## Status

Design-only for release decoupling work associated with #652 and #653. This document defines the intended direction for component-aware release state without changing live workflows, `release-please` manifests, or publish automation yet.

## Problem

The current release contract is expressed primarily at repo scope. That is sufficient for canonical stable release authority, but it is too coarse for a monorepo that contains multiple release-managed components with different publication policies and dependency relationships.

A repo-wide release train alone cannot clearly answer:

- which component version is the latest published version for that component,
- which components are release-eligible for the next stable cycle,
- which components are validate-only versus publishable,
- which changed paths should directly affect a component,
- and which components must join a release transitively because a published dependency changed.

That ambiguity creates coupling between version state, validation scope, and publish behavior.

## Goals

- Preserve `release-please` as the canonical owner of the repo-wide stable PR, tag, and release notes.
- Introduce a release-component graph that can coexist with the canonical repo-wide release contract.
- Make publish policy explicit per component.
- Make direct ownership and transitive release dependencies explicit and machine-consumable.
- Support future workflow changes where validation and publication can be decided per component from one source of truth.

## Non-Goals

- Changing live GitHub workflow behavior in this design document alone.
- Editing `release-please` configuration, manifests, or workflow YAML in this design document alone.
- Introducing component-specific tags as an alternate stable release authority.
- Replacing the canonical repo-wide `vX.Y.Z` tag.
- Adding non-published surfaces such as web/docs/mobile into semantic artifact release by default.

## Design Principles

1. **Repo-wide authority, component-aware planning**
   - The repository keeps one canonical release authority while component participation is resolved independently.
2. **Published artifacts only by default**
   - Only externally versioned/published artifacts join the semantic release train unless explicitly promoted later.
3. **One executable source of truth**
   - Ownership rules, dependency edges, version surfaces, and publish policy should live in one canonical graph definition.
4. **Explainable release fan-out**
   - Every component in `affected_components` should have a direct, shared-infra, or transitive dependency reason.
5. **Fail closed for unknown release impact**
   - Missing graph coverage is safer when surfaced as an error than when silently ignored.

## Canonical release-component graph

The intended model is an explicit release-component graph with fields conceptually shaped like:

```text
release_component
  id
  kind
  owned_paths
  shared_infra_paths
  version_surfaces
  published_artifacts
  release_channels
  publish_policy
  depends_on_release_of
  non_release_paths
  notes
```

### Field meaning

- `id`: stable component identifier used in workflows, manifests, tests, and docs.
- `kind`: runtime/crate/npm/gradle classification for operator understanding.
- `owned_paths`: direct repository ownership for release scope resolution.
- `shared_infra_paths`: paths whose change fans out to a declared component set.
- `version_surfaces`: canonical files whose versions must remain aligned.
- `published_artifacts`: externally visible outputs such as crates, npm packages, binaries, or Maven publications.
- `release_channels`: allowed channels such as `stable`, `beta`, and `snapshot`.
- `publish_policy`: whether the component is publishable or validate-only.
- `depends_on_release_of`: upstream components whose release-relevant change requires downstream release participation.
- `non_release_paths`: owned repository surfaces intentionally outside semantic artifact release.
- `notes`: operator-facing rationale for transitional handling or exceptions.

### Graph-backed component scope resolution

The release-component graph serves as the executable source of truth for determining which components participate in a release. When changes land, the system resolves `affected_components` by:

1. Classifying each changed path as release-owned, shared release infrastructure, non-release, or ignored.
2. Deriving directly affected components from owned paths and shared infrastructure fan-out.
3. Expanding transitive dependency edges until closure is reached.
4. Preserving direct and transitive inclusion reasons for operator visibility.
5. Emitting a deterministic, stable-sorted affected component set.

This graph-driven resolution replaces workflow-local resolver maps and ensures stable and beta release flows use identical component membership semantics.

## Initial managed component set

The initial graph should reflect the repository's current release reality:

### `rook`

- publishable
- owns `clients/rook/**`
- ships crate, npm wrapper/platform packages, and release binaries

### `cerebro`

- publishable
- owns `clients/cerebro/**`
- ships crate, binaries, and release assets through the shared publish flow

### `corvus-runtime`

- publishable
- owns `clients/agent-runtime/**`
- ships crate, npm wrapper/platform packages, and release binaries
- has a published dependency relationship with `cerebro` that may require transitive release participation

### `gradle-kmp`

- validate-only for now
- owns `gradle/**`, `gradle.properties`, and related Gradle version surfaces
- participates in validation posture and version alignment without yet becoming independent `release-please` manifest authority

## Direct ownership versus transitive dependency

The graph must distinguish two different reasons a component enters release scope:

### Direct ownership

A component is directly affected when a changed path belongs to its owned paths or declared shared release infrastructure.

Examples:

- `clients/rook/src/**` -> `rook`
- `clients/cerebro/src/**` -> `cerebro`
- `release-please-config.json` -> all declared shared-infra components

### Transitive release dependency

A component is transitively affected when a published dependency relationship requires it to join the release set after another component changes.

Example:

- `cerebro` changes directly
- `corvus-runtime` depends on release of `cerebro`
- final scope includes `cerebro` directly and `corvus-runtime` transitively

This distinction is essential because workflows and operators must understand not only *which* components were released, but *why*.

## Publish policy

Each managed component should declare one of these policies:

- **publishable**: participates in release PR/version/changelog/publish flows for external artifacts
- **validate-only**: participates in validation and impact reasoning, but not yet as independent publish authority

`gradle-kmp` remains validate-only in the intended near-term model. This keeps Gradle version alignment visible without forcing premature manifest authority changes.

## Relationship to `release-please`

The release-component graph does not replace `release-please`. Instead:

- `release-please` remains canonical owner of release PRs, versions, tags, and changelogs;
- the graph decides which components are affected by a change;
- publish workflows validate graph-derived version surfaces and dependency pins before artifact publication.

## Relationship to non-release surfaces

Web apps, docs, Android, and Compose clients remain outside semantic artifact release unless they later become externally versioned artifacts. They may still have deploy, preview, CI, or packaging workflows, but they do not join the component release graph by default.

## Expected benefits

A graph-backed versioning model should make it possible to answer, deterministically:

- which component versions are authoritative,
- which components should release for a given change set,
- which components are publishable versus validate-only,
- which downstream components joined transitively,
- and why a release was intentionally limited or expanded.
