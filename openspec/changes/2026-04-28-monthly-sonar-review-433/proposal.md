# Proposal: Monthly Sonar review workflow and targeted quality remediation #433

## Intent

Close the operational gap between Corvus’s hosted SonarQube/SonarCloud analysis workflow and what a developer can run locally during the recurring monthly Sonar review. Today the repository has CI analysis wiring in `.github/workflows/sonarqube-analysis.yml`, but lacks one canonical local entry point that reproduces the same coverage artifact expectations, validates scanner prerequisites, and gives contributors a bounded path to review and remediate high-signal quality issues without widening into repository-wide cleanup.

This change should create the missing OpenSpec artifacts for a narrow, reviewable maintenance slice that:
- adds a documented local `make sonar` workflow;
- centralizes local scanner invocation and prerequisite validation;
- aligns local coverage generation with the existing CI Sonar workflow for Kotlin, dashboard web, and Rust coverage artifacts;
- allows bounded, directly related quality remediation where locally reproducible issues block or weaken monthly review confidence.

## Why

The repository already treats Sonar analysis as a required quality signal in CI, but the current developer experience makes monthly review slower, more error-prone, and harder to reproduce locally. Contributors need one repeatable command path that mirrors CI assumptions closely enough to validate expected coverage artifacts and catch missing credentials or tools early.

Without that parity:
- monthly review depends too heavily on hosted CI feedback loops;
- developers have no single documented local workflow for Sonar reproduction;
- quality remediation risks drifting into ad hoc cleanup rather than a bounded maintenance slice.

## Scope

### In Scope
- Define a canonical local Sonar workflow entry point (`make sonar`) for monthly review.
- Define fail-closed local prerequisite validation for scanner availability, credentials, and expected coverage inputs.
- Define parity expectations between local workflow behavior and `.github/workflows/sonarqube-analysis.yml` coverage artifact paths.
- Define narrow documentation requirements for contributor-facing local Sonar execution.
- Define bounded targeted remediation expectations for directly affected files and locally reproducible quality issues uncovered by the workflow.

### Out of Scope
- Changing hosted Sonar policy, organization ownership, or quality-gate thresholds.
- Broad repository-wide smell cleanup unrelated to the local Sonar workflow slice.
- Inventing a second source of truth for Sonar analysis arguments separate from CI.
- Replacing CI Sonar analysis with local-only verification.
- Expanding into unrelated operational or gateway runtime behavior.

## Affected Areas

### Affected modules/packages
- Repository root `Makefile`
- `scripts/sonar.sh`
- `.github/workflows/sonarqube-analysis.yml`
- `README.md` or nearest contributor workflow documentation
- Directly affected Kotlin/Rust/web files only if bounded remediation is required by locally reproducible findings

### Affected spec domains
- New domain introduced by this change: `repository-quality-workflows`

Rationale for new domain:
- This change is repository-tooling and contributor-workflow oriented rather than runtime-gateway behavior.
- Existing domains such as `gateway` and `multi-agent-orchestration` are not the right ownership home for a local Sonar workflow contract.
- A dedicated domain keeps the scope explicit and prevents repository maintenance workflow rules from being scattered across runtime/product specs.

## Success Criteria

- Contributors can run one documented local command to reproduce the expected Sonar workflow inputs.
- The local workflow fails closed with clear, operator-facing messages when prerequisites or coverage artifacts are missing.
- The change documents parity expectations against the existing CI Sonar workflow rather than introducing drift.
- Any quality remediation remains narrow, locally reproducible, and limited to directly affected files.

## Risks

- Local Sonar execution may drift from CI if scanner arguments or coverage paths are duplicated carelessly.
- The maintenance slice could widen into general cleanup if remediation boundaries are not explicit.
- Developers may infer that successful local Sonar execution replaces hosted Sonar analysis; this change must avoid that implication.

## Rollback Plan

If the local Sonar workflow proves too brittle or misleading:
- remove the `make sonar` entry point and supporting script changes;
- revert documentation that claims local parity;
- keep `.github/workflows/sonarqube-analysis.yml` as the sole supported Sonar path until a better local workflow is ready;
- revert any bounded remediation changes that were only justified by this maintenance slice.

This rollback is low risk because the change is operational and additive: it does not alter production runtime semantics or repository deployment behavior.
