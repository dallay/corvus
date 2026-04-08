# Verify Checklist: capability-based-architecture-v3

## Proposal Success Criteria Mapping

- [ ] Success Criterion 1 -> Confirm `proposal.md`, `specs/capability-architecture/spec.md`, and
  `design.md` together define taxonomy, descriptor contract, dependency semantics, migration
  boundaries, and security attachment points.
- [ ] Success Criterion 2 -> Confirm all three artifacts consistently state M1 is design/spec only
  with no runtime behavior changes.
- [ ] Success Criterion 3 -> Confirm proposal/design reference current compatibility baselines and
  parity constraints for agent, channels, and gateway.
- [ ] Success Criterion 4 -> Confirm roadmap/design split later work into distinct M2, M3, M4, and
  M5 phases.
- [ ] Success Criterion 5 -> Confirm proposal/spec/design explicitly forbid fake plugin
  architecture, dynamic plugin loading in M1, and broad runtime inversion.

## Spec Requirement Group Mapping

- [ ] Taxonomy and boundaries -> Verify executable vs descriptive distinction, family boundaries,
  and "what a capability is not" are explicit.
- [ ] Descriptor contract -> Verify required shared fields, namespacing, auditability, and
  family-specific extension constraints are defined.
- [ ] Dependency semantics -> Verify required vs optional dependencies, compatibility constraints,
  and deterministic validation expectations are defined without requiring resolution in M1.
- [ ] Migration boundaries -> Verify trait/factory/dispatcher runtime remains the M1 compatibility
  baseline and that runtime inversion is prohibited.
- [ ] Security attachment points -> Verify approval, policy strength, namespacing, and audit
  continuity are preserved or strengthened.
- [ ] Anti-pattern constraints -> Verify dynamic plugin loading, registry-only claims, and
  undifferentiated capability typing are prohibited in M1.
- [ ] Roadmap constraints -> Verify M2 registry, M3 resolution, M4 execution, and M5
  tests/docs/adoption are separated.

## Artifact Completeness

- [ ] `exploration.md` exists and supports the design-first recommendation.
- [ ] `proposal.md` exists and matches M1 scope.
- [ ] `specs/capability-architecture/spec.md` exists and is scenario-based.
- [ ] `design.md` exists and maps contract design to current runtime hotspots.
- [ ] `tasks.md` exists and contains only M1 artifact-completion work.
- [ ] `state.yaml` advances this change to `next: verify` after apply completion.

## Verify Notes

- This change contains no runtime code changes.
- Verification should be performed as documentation/spec contract review plus artifact consistency
  review.
- Canonical spec promotion is deferred to archive unless verify finds a blocking reason to promote
  earlier.
