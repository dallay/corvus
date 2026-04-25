# Component-Scoped Pipeline Gating Design

## Status

Design-only for release decoupling work associated with #652 and #653. This document describes the
intended migration from repo-wide release gating toward component-aware validation and publication.
It does not change any live workflows, required checks, or release automation in this step.

## Problem

Current release and CI documentation largely describe pipeline behavior at repository scope. That
makes it difficult to distinguish between:

- checks that must always gate every merge,
- checks that gate only workflow changes,
- checks that should gate stable release for specific managed components,
- and checks that should remain informational for excluded or non-published components.

Without component-aware gating, the repository risks either over-blocking releases with unrelated
components or under-specifying which validations protect shipped artifacts.

## Goals

- Define a future gating model based on component release eligibility and policy.
- Separate component-scoped stable release validation from general merge-blocking CI.
- Keep canonical release authority repo-wide while allowing validation/publish decisions per
  component.
- Provide an operator-facing migration target before workflow changes are implemented.

## Non-Goals

- Changing current GitHub Actions required checks.
- Editing workflow files or branch protection.
- Introducing dynamic per-PR required-check mutation.
- Replacing the existing repo-wide `CI Required Gate` today.

## Current vs Target Model

### Current Model

Today, the repository primarily thinks about gating in two buckets:

1. merge-blocking checks for normal development, and
2. release/publish workflows triggered after the canonical release event.

That model is simple, but it does not express which validations matter for which release-managed
component.

### Target Model

The target model adds component awareness to release-time decisions:

1. **Merge gate** remains small, deterministic, and repo-safe.
2. **Component validation gate** is computed from release-eligible managed components.
3. **Component publish gate** is computed from components marked publishable for that cycle.
4. **Excluded components** are documented as intentionally outside stable publish scope.

## Proposed Gating Layers

### 1. Repository Merge Gate

This layer continues to protect normal development and should stay broadly applicable. It covers:

- baseline CI,
- workflow sanity for workflow edits,
- and other deterministic checks needed for branch health.

This gate is intentionally not responsible for expressing the full stable publish contract.

### 2. Component Validation Gate

This future layer evaluates only the components that are release-eligible for the stable cycle.
Validation scope should be derived from component state, not from the existence of unrelated code in
the monorepo.

Expected outcomes:

- publishable components must satisfy their required validation policy,
- validate-only components may still run release checks,
- excluded components do not become stable release blockers merely by existing.
- `gradle-kmp` currently belongs in the validate-only bucket: version alignment and publish
  credentials may be checked, but release state is not yet managed as its own `release-please`
  component.

### 3. Component Publish Gate

This future layer decides whether a component can proceed to publication after canonical tag
creation. It is narrower than validation because not every validated component is necessarily
published.

Expected outcomes:

- only components marked `publish` enter publication,
- validate-only components stop before publication,
- excluded components are skipped by policy with explicit operator visibility.

## Decision Inputs

The target component-aware gating model should derive from component-scoped state including:

- component identifier,
- release eligibility,
- publish policy,
- validation policy,
- and any documented exception notes.

No future gate should rely on implicit path heuristics alone when canonical component state is
available.

## Operator Interpretation Rules

When the design is implemented, operators should be able to answer the following quickly:

- Is this check protecting general branch health or a component release decision?
- Which components are in stable release scope for this cycle?
- Which components are publishable versus validate-only?
- Which components are intentionally excluded?

A component-aware gating system is successful only if those answers are visible without reading raw
workflow YAML.

## Documentation Migration Direction

Operator-facing CI documentation should gradually move from a repo-wide-only description to a model
that explains:

- current merge-blocking checks,
- current non-blocking release workflows,
- and the planned component-aware release gating direction.

That documentation change is part of this design step; live workflow behavior remains unchanged.

## Risks and Mitigations

### Risk: Confusing merge gating with release gating

If component-aware release validation is described as if it already blocks every PR, developers may
misunderstand the current workflow contract.

**Mitigation:** docs must clearly distinguish current behavior from migration direction.

### Risk: Component policies drift from actual workflow behavior

Design documents can become aspirational if they are not anchored to implementation sequencing.

**Mitigation:** explicitly mark this as design-only and defer live workflow changes until state and
policy mechanisms exist.

### Risk: Excessive fragmentation of checks

Too many tiny gates can reduce operator clarity.

**Mitigation:** keep a small repo-wide merge gate and reserve component granularity for release-time
validation/publish decisions.

## Expected Follow-On Work

Later implementation work is expected to:

- encode component-scoped release eligibility and policy in release state,
- map validation jobs to managed components,
- classify publish jobs by component contract,
- and update workflow/reporting surfaces to show component-aware status.

Those changes are intentionally out of scope for this design phase.
