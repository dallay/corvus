# Tasks: Next-Stage Routing Capabilities

_Decision-only change. No runtime implementation, config/schema edits, or automated tests are in scope._

## Phase 1: Artifact Alignment

- [x] 1.1 Confirm `openspec/changes/next-stage-routing-capabilities/proposal.md`, `design.md`, and `specs/model-routing/spec.md` all describe this change as archival and non-code.
- [x] 1.2 Confirm `openspec/specs/model-routing/spec.md` remains reference-only source of truth for shipped request-time routing behavior.

## Phase 2: Decision Recording

- [x] 2.1 Record that DALLAY-175 / GitHub `#271` is already covered by archived `productize-model-routing`, with no new v1.0.0 implementation scope.
- [x] 2.2 Record that embedding routes and managed route updates are deferred for v1.0.0, while `config.toml` remains the approved routing model.

## Phase 3: Verify Preparation

- [x] 3.1 Prepare verification to review artifact consistency against both delta spec scenarios in `openspec/changes/next-stage-routing-capabilities/specs/model-routing/spec.md`.
- [x] 3.2 Note in verification handoff that no runtime validation or test execution is required because no code changes are in scope.

## Apply Notes for Verify

- Verify against both delta requirements/scenarios in `openspec/changes/next-stage-routing-capabilities/specs/model-routing/spec.md` only.
- Confirm `proposal.md`, `design.md`, and this `tasks.md` consistently describe the change as decision-only, archival, and non-code.
- Confirm `openspec/specs/model-routing/spec.md` is referenced as the source of truth for shipped request-time routing behavior and is not modified by this change.
- Confirm the recorded decisions remain: DALLAY-175 / GitHub `#271` is already covered by archived `productize-model-routing`; embedding routes and managed route updates are deferred for v1.0.0; `config.toml` remains the approved routing model.
- Do not require runtime validation, automated tests, or build execution during verify because no application/runtime code changed in this apply phase.
