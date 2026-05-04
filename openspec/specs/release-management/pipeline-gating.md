# Component-Scoped Pipeline Gating Design

## Status

Implemented in part for release decoupling work associated with #652 and #653. The repository already uses component-aware release scope resolution in `release-please.yml`, `release-please-beta.yml`, and `publish-release.yml`, with `_publish.yml` consuming `affected_components` to gate validation/publication by component. This document defines the intended steady-state gating model once release scope is driven by one canonical release-component graph.

## Problem

Current release and CI documentation still describe portions of pipeline behavior at repository scope or as workflow-local resolver logic. That makes it difficult to distinguish between:

- checks that must always gate every merge,
- checks that gate only workflow or release-infrastructure changes,
- checks that should gate stable or beta release for specific managed components,
- and checks that should remain informational for non-release surfaces.

Without graph-backed gating, the repository risks either over-blocking releases with unrelated components or under-specifying which validations protect shipped artifacts.

## Goals

- Define a gating model based on the canonical release-component graph.
- Separate component-scoped stable/beta release validation from general merge-blocking CI.
- Keep canonical release authority repo-wide while allowing validation and publication decisions per component.
- Make direct, shared-infra, and transitive inclusion reasons operator-visible.
- Keep validate-only and publishable components distinguishable in release summaries.

## Non-Goals

- Changing live workflows in this document alone.
- Making every repository surface release-blocking.
- Promoting non-release surfaces into semantic artifact release without explicit future decisions.

## Intended gating model

### 1. General merge-blocking CI

These checks remain repository-wide and are not themselves evidence that a component should release:

- lint and formatting
- unit/integration tests unrelated to release authority
- docs quality checks
- dependency/security checks
- general PR policy checks

### 2. Release-scope resolution gate

Before stable or beta release planning proceeds, the system MUST resolve `affected_components` from the canonical release-component graph.

That gate MUST:

- classify changed paths as release-owned, shared release infrastructure, non-release, or ignored;
- determine directly affected components;
- expand transitive dependency edges;
- emit a stable sorted component set;
- emit direct vs transitive reasons;
- and fail closed on unmapped release-relevant paths.

### 3. Component-scoped validation gate

After scope resolution, validation MUST run according to component participation.

For publishable components:

- version surface alignment checks MUST run;
- package-specific validation MUST run;
- publish prerequisites MUST be enforced.

For validate-only components:

- validation MUST run when the component is in scope;
- the component MUST appear in summaries;
- but it MUST NOT be treated as independent publish authority unless explicitly promoted.

### 4. Publish gate

Publication MUST consume graph-derived `affected_components` and publish only the publishable affected set.

The publish gate MUST reject publication when:

- version surfaces for an affected component disagree;
- cross-component dependency pins are inconsistent with the release plan;
- or scope resolution evidence is missing or contradictory.

## Stable and beta parity

Stable and beta release flows MUST share the same graph semantics:

- the same owned-path rules,
- the same shared release infrastructure fan-out,
- the same transitive dependency expansion,
- the same publish-policy classification.

Only channel-specific differences such as prerelease versioning and prerelease metadata should vary.

## Operator-facing release evidence

The release pipeline MUST make it easy to understand why a component participated.

Expected summary elements:

- directly affected components,
- transitively affected components,
- validate-only components,
- publishable components,
- and any shared release infrastructure path that caused broad fan-out.

This helps reviewers distinguish a true component code change from a release-infrastructure change. The graph resolver MUST emit inclusion reasons alongside the affected component set so operators can verify that release scope matches intent.

## Example gating outcomes

### Scenario: `rook`-only change

- owned paths resolve to `rook`
- no transitive dependency expands scope
- validation/publish gates should run only for `rook`

### Scenario: `cerebro` change with downstream runtime dependency

- owned paths resolve directly to `cerebro`
- dependency graph expands scope to `corvus-runtime`
- validation gates should run for both
- publish gate should explain `cerebro` as direct and `corvus-runtime` as transitive

### Scenario: shared release workflow change

- shared infrastructure path fans out to all declared managed components
- summaries should identify shared-infra fan-out as the reason
- validate-only `gradle-kmp` should remain distinguishable from publishable components

### Scenario: web-only change

- changes resolve to non-release
- semantic artifact release gates should not mint component release scope on their own
- general CI may still run independently
