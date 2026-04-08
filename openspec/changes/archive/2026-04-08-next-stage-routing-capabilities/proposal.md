# Proposal: Next-Stage Routing Capabilities

## Intent

Formalize the routing follow-up as an archive-friendly decision-only change so the explored
conclusions can move through the normal SDD flow without introducing implementation work.

This proposal records that DALLAY-175 / GitHub #271 is already fully satisfied by the archived
`productize-model-routing` change, and that DALLAY-174 / GitHub #270 should be closed with explicit
v1.0.0 decisions: embedding routes are deferred, and managed route updates are deferred.

## Scope

### In Scope

- Record the closure decision that DALLAY-175 / GitHub #271 is already covered by
  `productize-model-routing`.
- Record the closure decision that embedding routes are not needed for v1.0.0 and are deferred
  rather than rejected.
- Record the closure decision that managed route updates are not needed for v1.0.0 and are deferred
  rather than rejected.
- Preserve the decision rationale, rollback plan, risks, and success criteria needed for clean
  archival.

### Out of Scope

- Any runtime routing, embedding, memory, admin API, or configuration-schema implementation.
- New OpenSpec delta requirements, design work, or task breakdown for product changes.
- Reopening or replacing the delivered `productize-model-routing` scope.

## Approach

Adopt the exploration recommendation as the final proposal scope.

This change remains documentation and decision tracking only:

1. Treat `openspec/changes/next-stage-routing-capabilities/exploration.md` as the evidence base.
2. Capture the explicit closure decisions in this proposal.
3. Keep downstream work minimal and archive-oriented, with no code or runtime behavior changes.

## Affected Areas

| Area                                                              | Impact     | Description                                                                                               |
|-------------------------------------------------------------------|------------|-----------------------------------------------------------------------------------------------------------|
| `openspec/changes/next-stage-routing-capabilities/proposal.md`    | New        | Formalizes the decision-only change for normal SDD handling and archival.                                 |
| `openspec/changes/next-stage-routing-capabilities/exploration.md` | Referenced | Provides the evidence and rationale for the closure decisions.                                            |
| `openspec/specs/model-routing/spec.md`                            | Referenced | Existing productized routing behavior remains the source of truth for #271 coverage; no changes proposed. |
| `DALLAY-174` / GitHub `#270`                                      | Decision   | Close with explicit deferral of embedding routes and managed route updates for v1.0.0.                    |
| `DALLAY-175` / GitHub `#271`                                      | Decision   | Close as already completed by archived `productize-model-routing`.                                        |

## Risks

| Risk                                                                            | Likelihood | Mitigation                                                                                                                         |
|---------------------------------------------------------------------------------|------------|------------------------------------------------------------------------------------------------------------------------------------|
| Decision-only closure could be mistaken for rejection of future capabilities    | Low        | State explicitly that embedding routes and managed updates are deferred, not rejected.                                             |
| #271 could be reopened if coverage is not linked back to shipped artifacts      | Low        | Reference `productize-model-routing` and the existing `openspec/specs/model-routing/spec.md` coverage when closing the issue.      |
| Future teams may assume runtime follow-up was intentionally omitted by accident | Low        | Keep the proposal explicit that no runtime work is in scope for v1.0.0 and that follow-up can be proposed later if demand appears. |

## Rollback Plan

If the decision is found to be incorrect, revert by reopening DALLAY-174 / GitHub #270 and/or
DALLAY-175 / GitHub #271 and starting a new implementation-focused SDD change. No runtime rollback
is required because this change introduces no product behavior changes.

## Dependencies

- `openspec/changes/next-stage-routing-capabilities/exploration.md`
- Archived `productize-model-routing` change and `openspec/specs/model-routing/spec.md`
- Final issue/decision communication in Linear and GitHub

## Success Criteria

- [ ] `proposal.md` exists for `next-stage-routing-capabilities` and records this as a decision-only
  change.
- [ ] The proposal explicitly states that DALLAY-175 / GitHub #271 is already covered by
  `productize-model-routing`.
- [ ] The proposal explicitly states that embedding routes are not needed for v1.0.0 and are
  deferred.
- [ ] The proposal explicitly states that managed route updates are not needed for v1.0.0 and are
  deferred.
- [ ] The proposal introduces no runtime implementation scope and is suitable for clean archival
  once the closure communication is completed.
