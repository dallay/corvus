# Cost Governance Specification

## Purpose

This specification defines Corvus cost governance as a product capability rather than an isolated
runtime mechanism. It establishes the canonical budget model, enforcement semantics, override and
audit rules, required operator and admin surfaces, and the separation between token-spend
governance and action-rate governance.

## Current Baseline

- Issue A (`Wire CostTracker to agent loop`) is already merged and is part of the baseline for this
  change.
- The runtime can instantiate cost tracking for the canonical agent loop and use that wiring behind
  the existing `cost.enabled` gate.
- CLI, gateway/admin API, dashboard, session reporting, and observability product surfaces are NOT
  yet complete and remain required work for this change.
- `SecurityPolicy.max_cost_per_day_cents` MUST be treated as legacy action-rate terminology, not as
  the canonical token-spend budget model.

## Requirements

### Requirement: Budget Scope Model

The system MUST govern token spend using four explicit budget scopes: `session`, `daily`,
`monthly`, and `mission`.

- `session` MUST represent spend accumulated within one canonical session identity.
- `daily` MUST represent spend accumulated within the active UTC calendar day.
- `monthly` MUST represent spend accumulated within the active UTC calendar month.
- `mission` MUST represent spend accumulated within a single mission identity, independent from the
  broader session total.
- The system MUST evaluate all configured token-spend scopes that apply to a request before allowing
  a metered model call.
- A scope MAY be absent or disabled, but the system MUST NOT infer one scope from another.

#### Scenario: Multiple configured scopes are evaluated together

- GIVEN token-spend budgets are configured for `session`, `daily`, and `monthly`
- WHEN the runtime evaluates a metered model call for an active session
- THEN the system MUST check all three configured scopes before the call is admitted
- AND the most restrictive resulting decision MUST govern the call outcome.

#### Scenario: Mission budget remains independent from session budget

- GIVEN a session contains two separate missions
- AND the first mission has already consumed its configured mission budget
- WHEN a second mission starts within the same session
- THEN the system MUST evaluate the second mission against its own mission scope
- AND the system MUST continue to evaluate the enclosing session, daily, and monthly scopes
  independently.

### Requirement: Warning and Hard-Block Semantics

The system MUST distinguish warning semantics from hard-block semantics for token-spend budgets.

- A warning MUST occur when spend reaches or exceeds the configured warning threshold for a scope
  but
  has not yet exceeded the hard limit for that same scope.
- A hard block MUST occur when a scope's hard limit is exceeded or would be exceeded by the next
  metered model call.
- In-flight model calls MUST be allowed to complete once admitted.
- After a hard-block condition is reached, the next metered model call MUST be rejected until spend
  returns below an active limit or an authorized override is applied.
- Warning states MUST be visible to operators and admins; hard-block states MUST be visible and
  actionable.

#### Scenario: Warning is emitted before limit is exceeded

- GIVEN a daily budget has a hard limit of 100 USD and a warning threshold of 80 percent
- WHEN accumulated daily spend reaches 82 USD after a completed model call
- THEN the system MUST classify the daily scope as warning state
- AND the next metered model call MUST still be eligible to run if no hard limit is exceeded.

#### Scenario: Hard block applies on the next metered call

- GIVEN a session budget hard limit is 25 USD
- AND an admitted model call completes with total session spend now at 26 USD
- WHEN the agent loop attempts the next metered model call for that session
- THEN the system MUST reject that call as budget-exceeded
- AND the system MUST NOT cancel or retroactively fail the already completed prior call.

### Requirement: Override Policy and Audit Trail

The system MUST support explicit, auditable override behavior for token-spend hard blocks.

- Overrides MUST require an explicit operator or admin action; silent overrides MUST NOT exist.
- The product model MUST support both local operator overrides and remote admin overrides.
- Every override MUST record who performed it, which scope or limit was overridden, when the
  override occurred, and the justification or source context when supplied.
- Override records MUST be append-only audit events.
- The system SHOULD support temporary overrides with explicit expiry or replacement conditions.

#### Scenario: Local operator override is audited

- GIVEN a CLI operator encounters a budget-exceeded condition
- WHEN the operator uses an approved override control to continue execution
- THEN the system MUST permit execution only if overrides are enabled by policy
- AND the system MUST append an audit record containing the operator identity, affected budget
  scope, timestamp, and override action.

#### Scenario: Remote admin override is visible after application

- GIVEN a gateway-admin surface raises a monthly budget for a running deployment
- WHEN the override is accepted by the authorized admin path
- THEN the new limit MUST become the active limit for subsequent evaluations
- AND the system MUST persist an audit event that can be queried independently of the runtime
  process that applied it.

### Requirement: Required Product Surfaces

The system MUST expose cost-governance information and controls across the required product
surfaces: CLI, gateway/admin API, dashboard, session reporting, and observability.

- The CLI MUST expose current budget status, warning or blocked state, and operator-approved
  override flow when policy allows it.
- The gateway/admin API MUST expose current spend, budget status, history, and authorized
  administrative controls for reset or limit adjustment.
- The dashboard MUST expose live or near-live budget status, warning or blocked indicators, and
  historical spend views derived from gateway-safe APIs.
- Session reporting MUST expose session-level spend totals and budget outcomes for the completed or
  active session.
- Observability MUST expose structured events and metrics for spend, warnings, hard blocks, and
  overrides.

#### Scenario: Operator surfaces show the same budget state

- GIVEN a session is in warning state for the daily budget
- WHEN an operator checks the CLI and an admin checks the dashboard through the gateway
- THEN both surfaces MUST report the same warning classification for that active budget scope
- AND neither surface MUST require direct access to runtime-internal storage.

#### Scenario: Session reporting includes budget outcome

- GIVEN a session finishes after spending against active token budgets
- WHEN the session summary or history is requested
- THEN the system MUST include the session spend total
- AND the system MUST include whether the session ended within budget, in warning state, or blocked
  by budget governance.

#### Scenario: Observability records warning and override lifecycle

- GIVEN a session first crosses a warning threshold and later receives an approved override
- WHEN observability data is emitted for that session
- THEN the system MUST emit structured records for both the warning and the override
- AND those records MUST be correlatable to the same session or mission identity without exposing
  secrets.

### Requirement: Separation of Governance Domains

The system MUST keep token-spend governance separate from action-rate governance.

- Token-spend governance MUST apply to model usage cost and spend budgets.
- Action-rate governance MUST apply to action frequency or action-count controls.
- Configuration, policy names, user-facing labels, and audit records MUST NOT present action-rate
  controls as token-spend budgets.
- A request MAY be denied by either governance domain, but the denial reason MUST identify the
  governing domain.
- Legacy names that imply token spend for action-rate controls MUST be removed or clearly
  deprecated.

#### Scenario: Action-rate denial is not reported as token budget exhaustion

- GIVEN an agent has exhausted its action-rate allowance but has token budget remaining
- WHEN a request is denied by policy
- THEN the system MUST report the denial as an action-rate governance outcome
- AND the system MUST NOT present the denial as daily or monthly token spend exhaustion.

#### Scenario: Token budget denial leaves action-rate accounting unchanged

- GIVEN an agent is blocked because its monthly token-spend budget is exceeded
- WHEN the denial is recorded
- THEN the system MUST classify the outcome under token-spend governance
- AND the system MUST NOT mutate action-rate counters solely because of that token-budget denial.

### Requirement: Baseline Truthfulness and Remaining Productization Work

The specification for this change MUST describe the already-merged runtime baseline truthfully while
also defining the remaining productization work required to complete the feature.

- The merged agent-loop cost wiring MUST be treated as current baseline behavior for this change.
- Remaining work MUST include the operator and admin product surfaces, audit completeness,
  observability completeness, and governance-domain cleanup needed for a production-ready feature.
- Product documentation and acceptance reviews MUST distinguish between baseline behavior already on
  `main` and required surfaces that are still pending.
- The system MUST NOT claim full cost-governance product completion until the required surfaces and
  governance separation in this specification are delivered.

#### Scenario: Baseline includes runtime wiring but not surface completion

- GIVEN an implementation review is performed after Issue A has merged
- WHEN the reviewer evaluates this change against the specification
- THEN the reviewer MUST treat agent-loop cost wiring as already satisfied baseline behavior
- AND the reviewer MUST still mark CLI, gateway/admin API, dashboard, session reporting, and
  observability requirements as pending until delivered.

#### Scenario: Product completion cannot be claimed from runtime-only wiring

- GIVEN cost tracking is wired in the runtime and can enforce limits internally
- WHEN no dashboard, CLI summary, admin API, or override audit surface is available to operators
- THEN the system MUST be considered partially productized
- AND the feature MUST NOT be represented as complete cost governance.
