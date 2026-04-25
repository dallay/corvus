# Component-Scoped Versioning Design

## Status

Design-only for release decoupling work associated with #652 and #653. This document defines the
intended direction for component-aware release state without changing live workflows,
`release-please` manifests, or publish automation yet.

## Problem

The current release contract is expressed primarily at repo scope. That is sufficient for the
canonical stable tag and release-note authority, but it is too coarse for a repository that
contains multiple release-managed components with different publication policies.

A repo-wide release train alone cannot clearly answer:

- which component version is the latest published version for that component,
- which components are release-eligible for the next stable cycle,
- which components are validate-only versus publishable,
- and which components are intentionally excluded from stable publication.

That ambiguity creates coupling between version state, validation scope, and publish behavior.

## Goals

- Preserve `release-please` as the canonical owner of the repo-wide stable PR, tag, and release
  notes.
- Introduce a component-scoped release-state model that can coexist with the canonical repo-wide
  release contract.
- Make publish policy explicit per component.
- Support future workflow changes where validation and publication can be decided per component.

## Non-Goals

- Changing any live GitHub workflow behavior in this design step.
- Editing `release-please` configuration, manifests, or workflow YAML.
- Introducing component-specific tags as an alternate stable release authority.
- Replacing the canonical repo-wide `vX.Y.Z` tag.

## Design Principles

1. **Repo-wide authority remains singular.** The canonical stable release still flows from the
   repo-wide `release-please` PR and `vX.Y.Z` tag.
2. **Component state is descriptive and operational.** Component records explain what each managed
   component last released, what it intends to release next, and whether it should publish.
3. **Publish policy is explicit, never inferred by omission.** If a component is excluded, the
   state must say so.
4. **Validation scope follows component eligibility.** Future gates should derive from the managed
   components selected for the release, not from every package in the monorepo.

## Proposed Component State Model

Each release-managed component should have a canonical state entry with fields equivalent to the
following logical model:

```text
component_id
component_path
component_kind
latest_released_version
pending_version
release_eligibility
publish_policy
validation_policy
notes
```

### Field Intent

- `component_id`: stable identifier used across docs, validation, and publish planning.
- `component_path`: repository location anchoring the component to real files.
- `component_kind`: package/app/runtime/library classification for operator understanding.
- `latest_released_version`: the most recent version actually released for this component.
- `pending_version`: planned next version for the component when a release is in progress.
- `release_eligibility`: whether the component is in scope for the current stable cycle.
- `publish_policy`: whether the component is published, validate-only, or excluded.
- `validation_policy`: the level of validation the component must satisfy for stable release.
- `notes`: operator-facing rationale for exceptions or transitional handling.

## State Semantics

### Published Component

A published component is expected to:

- maintain its own latest released version state,
- participate in release planning when eligible,
- and become publishable after the canonical tag exists.

### Validate-Only Component

A validate-only component is in managed release state but does not publish artifacts in the stable
contract. It may still require checks because it affects shipped components, shared tooling, or
consumer safety.

### Excluded Component

An excluded component remains visible in release state but is not considered part of stable publish
scope. Exclusion must be explicit and documented so that omission is interpreted as policy, not as
state drift.

## Relationship to Repo-Wide Versioning

The repo-wide stable release version still exists and remains canonical for the stable PR/tag flow.
Component-scoped version state does not replace that authority. Instead, it adds a second layer of
state needed to manage release eligibility and publication decisions in a mixed-scope repository.

The key design constraint is:

- **repo-wide version authority answers "what stable release happened?"**
- **component-scoped state answers "what does this component do in that release?"**

This preserves one stable release authority while enabling component-aware release operations.

## Migration Direction

The intended migration path is:

1. Document the component-scoped model in spec and design.
2. Update operator-facing docs to explain that gating will move toward component-aware scope.
3. Introduce implementation later in release state, validation selection, and publish planning.
4. Only after the model is implemented should workflows and `release-please` integration be changed.

## Risks and Mitigations

### Risk: Dual state becomes contradictory

If repo-wide and component-scoped state diverge, operators may not know which value is authoritative.

**Mitigation:** document that repo-wide state owns canonical stable release identity, while
component-scoped state owns component participation and policy.

### Risk: Hidden exclusions create release surprises

If excluded components are omitted from state entirely, missing publication may look like a bug.

**Mitigation:** require explicit excluded entries with rationale.

### Risk: Component gating becomes too granular too early

Overly aggressive decoupling could weaken release confidence before the model is mature.

**Mitigation:** this step is design-only; keep current live gating unchanged until component state
and validation contracts are implemented and verified.

## Expected Follow-On Implementation Areas

When implementation begins, likely touch points include:

- release state source-of-truth definitions,
- validation planning logic,
- publish planning logic,
- operator documentation for release triage,
- and workflow surfaces that currently assume repo-wide all-or-nothing gating.

No such live changes are part of this design document itself.
