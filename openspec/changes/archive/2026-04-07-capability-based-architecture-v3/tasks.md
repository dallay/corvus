# Tasks: Capability-Based Architecture v3 (M1 Design Contract)

## Phase 1: Canonical Spec Promotion

- [x] 1.1 Reviewed
  `openspec/changes/capability-based-architecture-v3/specs/capability-architecture/spec.md` against
  current `openspec/specs/*` usage and decided canonical promotion is **not** needed before verify.
- [x] 1.2 Kept `openspec/specs/capability-architecture/spec.md` deferred to archive so M1 stays
  change-scoped and does not imply runtime adoption.
- [x] 1.3 Recorded the canonical-promotion decision in `proposal.md` and `design.md` to remove
  wording ambiguity before verify.

## Phase 2: Artifact Consistency Alignment

- [x] 2.1 Reviewed `proposal.md`, `specs/capability-architecture/spec.md`, and `design.md` for
  consistent M1-only wording: design/spec-only, no runtime behavior changes, no
  registry/resolution/execution implementation.
- [x] 2.2 Aligned proposal/design wording so capability taxonomy, migration boundaries, security
  semantics, and phased roadmap describe the same M1 contract.
- [x] 2.3 Added only one minimal reference artifact, `verify-checklist.md`, because verify
  preparation required an explicit review checklist.

## Phase 3: M1 Readiness Validation

- [x] 3.1 Validated that `exploration.md`, `proposal.md`, `specs/capability-architecture/spec.md`,
  `design.md`, `tasks.md`, and `state.yaml` all exist for M1.
- [x] 3.2 Confirmed the spec remains scenario-based, contract-focused, and free of hidden
  implementation commitments for M2-M5.
- [x] 3.3 Confirmed the design preserves current runtime seams (`bootstrap`, factories, dispatcher,
  channels, gateway) as the M1 compatibility baseline and does not imply runtime inversion.

## Phase 4: Verify Preparation

- [x] 4.1 Prepared `openspec/changes/capability-based-architecture-v3/verify-checklist.md`, mapping
  proposal success criteria to artifacts and key spec requirement groups.
- [x] 4.2 Updated `openspec/changes/capability-based-architecture-v3/state.yaml` so this non-code
  apply step completes with `next: verify`.
- [x] 4.3 Prepared the artifact set for `/sdd-verify`, explicitly treating M1 as a
  documentation/spec contract change with no runtime code changes.
