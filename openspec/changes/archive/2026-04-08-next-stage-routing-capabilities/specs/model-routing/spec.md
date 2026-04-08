# Delta for Model Routing

## ADDED Requirements

### Requirement: Covered Routing UX Closure

The decision record for this change MUST treat DALLAY-175 / GitHub #271 as already satisfied by the delivered `productize-model-routing` artifacts and SHALL close it as covered work.

This change MUST NOT create new v1.0.0 implementation scope for the operator UX and documentation outcomes already captured by `productize-model-routing` and `openspec/specs/model-routing/spec.md`.

#### Scenario: Archive review closes the covered issue

- GIVEN the `next-stage-routing-capabilities` change is being reviewed for archival
- WHEN maintainers evaluate DALLAY-175 / GitHub #271
- THEN they MUST identify `productize-model-routing` as the delivered source of truth
- AND they MUST close DALLAY-175 / GitHub #271 as covered work

#### Scenario: Reviewer asks whether #271 still needs new v1.0.0 work

- GIVEN a reviewer questions whether operator routing UX or documentation remains unfinished for v1.0.0
- WHEN they consult this change record
- THEN the record MUST state that DALLAY-175 / GitHub #271 is already satisfied by existing routing artifacts
- AND the record MUST NOT require additional implementation work for that issue

### Requirement: v1.0.0 Deferral of Next-Stage Routing Capabilities

For v1.0.0, the product MUST NOT require embedding routes as a first-class routing feature, and managed route updates MUST remain out of scope.

For v1.0.0, config-file-driven routing SHALL remain the approved operating model. Embedding routes and managed route updates are deferred capabilities, not rejected capabilities, and MAY be reconsidered in a future change when operator or product demand exists.

#### Scenario: v1.0.0 scope is reviewed

- GIVEN the v1.0.0 routing scope is being reviewed
- WHEN maintainers assess next-stage routing capabilities
- THEN embedding routes MUST be treated as deferred for v1.0.0
- AND managed route updates MUST remain out of scope for v1.0.0
- AND config-file-driven routing SHALL remain the approved routing model

#### Scenario: Future demand emerges after v1.0.0 planning

- GIVEN operators or product stakeholders later identify demand for embedding routes or managed route updates
- WHEN a future routing change is proposed
- THEN this decision record MAY be used as the baseline for reconsideration
- AND the deferred capabilities MUST be treated as revisitable rather than rejected
