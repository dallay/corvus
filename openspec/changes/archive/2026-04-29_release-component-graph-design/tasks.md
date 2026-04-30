# Implementation Tasks: release-component-graph-design

## Phase 1 — OpenSpec release-management contract alignment

1.1 Update `openspec/specs/release-management/spec.md` so the canonical release contract defines graph-backed component scope resolution for externally versioned artifacts.

1.2 Update `openspec/specs/release-management/component-versioning.md` to evolve the current component-scoped versioning model into an explicit release-component graph design with ownership, dependency, and publish-policy fields.

1.3 Update `openspec/specs/release-management/component-inventory.md` to record the canonical managed component set, publish policy, and version-surface expectations for `rook`, `cerebro`, `corvus-runtime`, and `gradle-kmp`.

1.4 Update `openspec/specs/release-management/impact-map.md` to distinguish release-owned paths, shared release infrastructure fan-out, non-release paths, and transitive dependency-driven downstream release inclusion.

1.5 Update `openspec/specs/release-management/pipeline-gating.md` so gating language aligns with graph-derived `affected_components`, direct/transitive inclusion reasons, and validate-only versus publishable posture.

1.6 Update `openspec/specs/release-management/migration-plan.md` with phased rollout from workflow-local resolver maps to one canonical executable release graph.

## Phase 2 — Change artifact completeness

2.1 Add `openspec/changes/release-component-graph-design/proposal.md` describing intent, scope, risks, and rollback posture.

2.2 Add `openspec/changes/release-component-graph-design/design.md` documenting the release-component graph architecture, decisions, invariants, and rollout strategy.

2.3 Add `openspec/changes/release-component-graph-design/tasks.md` with phased implementation guidance for graph definition, workflow adoption, publish validation, and contract tests.

2.4 Add `openspec/changes/release-component-graph-design/state.yaml` so the change tracks proposal/design/tasks/spec progress consistently with other OpenSpec changes.

## Phase 3 — Follow-up implementation planning

3.1 Identify the intended file location and format for the executable release graph source of truth.

3.2 Define the first implementation slice that extracts the current workflow-local resolver logic into a reusable graph-backed resolver.

3.3 Define the publish-validation follow-up that extends `_publish.yml` and `scripts/release-contract.test.mjs` to enforce graph/config/manifest alignment and transitive dependency pins.

3.4 Confirm whether stable multi-component handoff should remain release-body based or move to a stronger machine-generated metadata contract.

## Phase 4 — Verification readiness

4.1 Review all updated OpenSpec artifacts for consistency of terminology across “release-managed”, “publishable”, “validate-only”, “shared release infrastructure”, and “non-release” classifications.

4.2 Review all updated requirement scenarios to ensure they use RFC 2119 language and Given/When/Then structure where normative behavior is specified.

4.3 Confirm that rollback posture remains documented for follow-up implementation work that changes live workflows.
