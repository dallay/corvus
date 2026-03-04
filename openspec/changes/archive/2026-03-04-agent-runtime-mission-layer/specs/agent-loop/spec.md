# Delta for Agent Loop

## ADDED Requirements

### Requirement: Mission Lifecycle Contract

The runtime MUST provide a first-class mission lifecycle for autonomous objectives with
deterministic
state transitions across objective intake, plan creation, checkpoint progression, replan, and
terminal outcomes.

#### Scenario: Mission completes through planned checkpoints

- GIVEN a mission objective accepted by the runtime
- WHEN the mission plan is generated and checkpoints are executed in order
- THEN the runtime MUST transition the mission through objective, planning, active, and completed
  states
- AND each checkpoint MUST persist progress sufficient to resume from the latest successful
  checkpoint.

#### Scenario: Mission replans after checkpoint failure

- GIVEN an active mission with at least one failed checkpoint attempt
- WHEN the failure is classified as recoverable within mission governance limits
- THEN the runtime MUST trigger a replan transition before resuming execution
- AND the mission MUST preserve prior successful checkpoints and failure reason metadata.

### Requirement: Mission Governance and Fail-Closed Enforcement

The runtime MUST enforce mission-level governance controls for budget, SLA/time ceilings, and
termination reasons. Any unknown or invalid governance state MUST fail closed.

#### Scenario: Mission terminated by budget ceiling

- GIVEN a running mission with a configured budget ceiling
- WHEN cumulative mission execution exceeds the configured budget
- THEN the runtime MUST terminate the mission with a deterministic `budget_exhausted` reason
- AND the runtime MUST stop additional checkpoint or delegated execution.

#### Scenario: Mission terminated by SLA ceiling

- GIVEN a running mission with a configured SLA/time ceiling
- WHEN mission elapsed execution time exceeds the configured SLA threshold
- THEN the runtime MUST terminate the mission with a deterministic `sla_exceeded` reason
- AND the runtime MUST emit a mission-level termination event before exit.

### Requirement: Delegated Mission Orchestration Parity

Mission decomposition and delegated execution MUST route through existing dispatcher, policy, and
approval boundaries. Mission mode MUST NOT create a bypass path.

#### Scenario: Delegated mission step requires approval

- GIVEN a mission checkpoint that dispatches a delegated or risk-bearing tool action
- WHEN dispatcher risk classification evaluates the action
- THEN the runtime MUST require approval or deny execution according to existing policy behavior
- AND the mission step MUST remain blocked until an allowed decision is present.

#### Scenario: Delegated mission step denied by policy

- GIVEN a mission checkpoint that resolves to a denied action by current policy
- WHEN mission orchestration requests execution
- THEN the runtime MUST return a structured denial result into mission state
- AND the mission MUST transition using fail-closed governance semantics without unauthorized side
  effects.

### Requirement: Mission KPI Telemetry

The runtime MUST emit mission-level KPI telemetry for progress, outcomes, and guardrail violations
using existing observability infrastructure.

#### Scenario: Mission progress telemetry per checkpoint

- GIVEN an active mission with multiple checkpoints
- WHEN each checkpoint starts and completes
- THEN the runtime MUST emit telemetry fields that include mission ID, checkpoint index, and
  completion status
- AND telemetry MUST include timing fields suitable for latency and throughput KPIs.

#### Scenario: Guardrail violation telemetry on governance stop

- GIVEN a mission terminated due to governance or policy constraints
- WHEN the mission enters a terminal failure state
- THEN the runtime MUST emit a telemetry event describing the guardrail class and termination reason
- AND telemetry payloads MUST avoid exposing sensitive arguments or secret values.

### Requirement: Mission Integration and Backward Compatibility Coverage

The runtime MUST include integration tests for mission lifecycle, governance enforcement, delegated
security parity, and compatibility with non-mission loop behavior.

#### Scenario: Integration suite validates mission lifecycle and governance

- GIVEN mission mode enabled in runtime integration tests
- WHEN objective, checkpoint progression, replanning, and terminal governance paths are exercised
- THEN the integration suite MUST verify deterministic mission states and termination reasons
- AND the suite MUST cover both happy and failure paths.

#### Scenario: Legacy loop path remains behaviorally stable

- GIVEN mission mode disabled or not requested
- WHEN existing loop integration suites execute
- THEN prior loop behavior MUST remain stable for approval, timeout, and fallback semantics
- AND mission-layer additions MUST NOT change native non-mission tool execution semantics.

## MODIFIED Requirements

### Requirement: Entry Points Alignment

The system MUST provide a unified loop contract across all entry points (CLI, channels, gateway
webhook) for both standard turns and mission-managed execution. Any semantic differences MUST be
explicitly justified and narrow in scope.

(Previously: The system MUST provide a unified loop contract across all entry points (CLI,
channels, gateway webhook). Any semantic differences MUST be explicitly justified and narrow in
scope.)

#### Scenario: Mission behavior parity across entry points

- GIVEN equivalent mission objectives submitted through CLI, channel, and gateway paths
- WHEN mission lifecycle orchestration and governance checks are applied
- THEN each entry point MUST enforce equivalent mission transition and guardrail behavior
- AND no entry point MAY bypass dispatcher, approval, or policy evaluation.
