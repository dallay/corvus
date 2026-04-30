# Delta for Repository Quality Workflows

## ADDED Requirements

### Requirement: Canonical Local Sonar Review Workflow

The repository MUST provide one canonical local entry point for the recurring monthly Sonar review workflow.

At minimum, that local workflow MUST be invokable through a documented repository-level command and MUST orchestrate the required local coverage generation and scanner invocation steps in a repeatable order.

The local workflow MUST be designed to mirror the existing hosted Sonar analysis job closely enough for contributors to reproduce expected analysis inputs before relying on CI results.

#### Scenario: Contributor runs one documented local Sonar workflow

- GIVEN a contributor is preparing or reviewing the monthly Sonar maintenance slice
- WHEN the contributor runs the documented local Sonar command
- THEN the repository MUST execute the required local Sonar preparation and analysis steps through one canonical workflow
- AND the contributor MUST NOT need to manually reconstruct the sequence from multiple unrelated commands.

### Requirement: Local Sonar Workflow Parity with Hosted Coverage Inputs

The local Sonar workflow MUST reproduce the expected coverage artifact inputs used by the hosted Sonar analysis workflow for the directly supported repository surfaces in this slice.

At minimum, the local workflow MUST generate or validate the expected Kotlin coverage inputs, dashboard web LCOV inputs, and Rust LCOV inputs before invoking the scanner.

The system MUST treat the hosted Sonar workflow configuration as the authoritative parity reference for coverage artifact expectations in this slice.

#### Scenario: Local workflow prepares the same coverage classes of inputs as hosted analysis

- GIVEN the repository’s hosted Sonar analysis workflow expects Kotlin, dashboard web, and Rust coverage artifacts
- WHEN a contributor runs the local Sonar workflow successfully
- THEN the local workflow MUST generate or validate those same classes of coverage inputs before scanner execution
- AND the workflow MUST NOT silently skip a required supported coverage input.

### Requirement: Fail-Closed Prerequisite and Coverage Validation

The local Sonar workflow MUST fail closed when required credentials, scanner prerequisites, or expected supported coverage artifacts are unavailable.

Failure messaging MUST be operator-facing and explicit enough for a contributor to understand which prerequisite is missing or which expected workflow input could not be produced.

The system MUST NOT continue into scanner execution when required prerequisites for this slice are absent.

#### Scenario: Missing Sonar token stops local workflow before analysis

- GIVEN a contributor attempts to run the local Sonar workflow without the required Sonar token configured
- WHEN the workflow validates prerequisites
- THEN the workflow MUST fail before scanner execution
- AND the failure output MUST explain that the token is required for local Sonar analysis.

#### Scenario: Missing supported coverage artifact stops local workflow before analysis

- GIVEN a required supported coverage generation step fails or does not produce the expected artifact path
- WHEN the local Sonar workflow validates analysis inputs
- THEN the workflow MUST fail before scanner execution
- AND the failure output MUST identify the missing or failed coverage prerequisite.

### Requirement: Contributor-Facing Sonar Workflow Documentation

The repository MUST document the local Sonar review entry point, its prerequisites, and its relationship to hosted Sonar analysis in contributor-facing documentation.

That documentation MUST explain that local execution is intended to improve reproducibility and faster iteration, not to replace the hosted Sonar workflow as the repository’s final shared quality signal.

#### Scenario: Contributor reads local Sonar guidance

- GIVEN a contributor reads the repository’s development or workflow documentation
- WHEN the contributor looks for Sonar review instructions
- THEN the documentation MUST describe how to run the local Sonar workflow and what prerequisites it requires
- AND the documentation MUST clarify how local execution relates to hosted Sonar results.

### Requirement: Bounded Monthly Sonar Remediation Scope

When the monthly Sonar workflow exposes actionable quality issues during this slice, remediation MUST remain bounded to directly affected files and locally reproducible or strongly evidenced issues related to the workflow’s purpose.

The system MUST NOT treat this maintenance slice as authorization for open-ended repository-wide cleanup.

#### Scenario: Quality remediation remains narrow and directly related

- GIVEN the monthly Sonar workflow identifies a small set of actionable quality issues in touched or directly affected files
- WHEN remediation is performed as part of this slice
- THEN the fixes MUST remain limited to directly affected files and workflow-adjacent quality issues
- AND unrelated repository-wide cleanup MUST remain out of scope.
