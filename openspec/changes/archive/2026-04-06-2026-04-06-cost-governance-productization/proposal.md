# Proposal: Cost Governance Productization

## Intent

Corvus has foundational cost governance infrastructure spread across three disconnected subsystems —
`CostTracker`, `MissionCoordinator`, and `SecurityPolicy` — but none are wired together into a
coherent, enforceable platform feature. `CostTracker` is dead code in production (never instantiated
outside tests), `MissionCoordinator` tracks per-mission cost in isolation, and
`SecurityPolicy.max_cost_per_day_cents` is misleadingly named (it controls action-rate, not
token-spend). The dashboard shows config but no usage data, and the CLI exposes only the action-rate
field.

This change productizes cost governance as a first-class platform feature: unified budget
enforcement that is visible, configurable, and enforceable across all runtime surfaces (agent loop,
CLI, gateway API, dashboard).

## Scope

### In Scope

- **Budget types and enforcement semantics**: Per-session, daily, and monthly budget scopes with
  warning thresholds (soft limit at `warn_at_percent`) and hard blocks (reject LLM calls when
  exceeded). Grace behavior: in-flight requests complete, next request blocked. `BudgetCheck` result
  enum: `Allowed`, `Warning(remaining_usd, percent_used)`, `Exceeded(limit, spent)`.
- **Override policy and audit**: `allow_override` config enables CLI `--override-budget` flag. Every
  override logged as `ObserverEvent::BudgetOverride` with operator identity. Gateway admin can
  temporarily raise limits via PATCH (also audited). No silent overrides.
- **Unification of parallel cost models**: `CostTracker` becomes single source of truth for
  token-cost budgets. `MissionCoordinator.accumulated_cost_cents` delegates to `CostTracker`.
  `SecurityPolicy.max_cost_per_day_cents` renamed to `max_actions_per_hour` to eliminate naming
  confusion. Action-rate limiting remains separate (different concern).
- **Agent loop integration**: Pre-flight `check_budget()` before each LLM call; post-call
  `record_usage()` from provider `TokenUsage` response. Behind `cost.enabled` feature flag.
- **CLI cost surface**: `corvus cost` subcommand (summary, history, reset), session cost summary on
  exit, `--override-budget` flag, warning output when approaching limits.
- **Gateway API cost endpoints**: `GET /cost/summary` (current spend), `GET /cost/history` (time
  series), `POST /cost/reset` (admin-only), `PATCH /admin/config` for `CostConfig` fields.
- **Dashboard cost UX**: Live usage display in `CostOverview.vue`, spending chart, budget progress
  bar, alert indicators.
- **Cost telemetry**: Populate `cost_usd` in `AgentEnd` observer events (currently always `None`),
  emit `BudgetWarning` and `BudgetExceeded` events, OTel span attributes for cost observability.

### Out of Scope

- Multi-tenant cost isolation (future architecture decision)
- Per-user budgets within a shared instance (future)
- Billing integration with external payment systems (future)
- Cost optimization recommendations or model routing by cost (future)
- Float-to-integer-cents migration for internal accounting precision (future iteration)

## Approach

Incremental, bottom-up implementation behind the existing `cost.enabled` feature flag (defaults to
`false`), making rollout safe and reversible.

**Implementation order:**

1. **Wire CostTracker to Agent Loop** (Issue A) — Instantiate `CostTracker` in the runtime
   bootstrap, pipe provider `TokenUsage` into `record_usage()`, add pre-flight `check_budget()` gate
   before each LLM call. This is the foundational wiring that activates the dead code.
2. **CLI Cost Surface** (Issue B) — `corvus cost` subcommand with summary/history/reset, session
   cost summary on exit, `--override-budget` flag with audit logging.
3. **Cost API Endpoints** (Issue C) — REST endpoints for cost data (`/cost/summary`,
   `/cost/history`, `/cost/reset`), admin PATCH for `CostConfig` fields on `/admin/config`.
4. **Dashboard Cost UX** (Issue D) — Live usage in `CostOverview.vue`, spending chart, budget
   progress bar, alert indicators consuming the new API endpoints.
5. **Unify Cost Models** (Issue E) — `MissionCoordinator` delegates cost recording to `CostTracker`,
   rename `SecurityPolicy.max_cost_per_day_cents` to `max_actions_per_hour`, add deprecation path
   for existing config files.
6. **Cost Telemetry** (Issue F) — Populate `cost_usd` in observer events, OTel span attributes,
   audit log for budget overrides.

Each issue is independently shippable. Issues A-B are runtime-only. Issue C bridges runtime and
gateway. Issue D is web-only. Issues E-F are cleanup and observability.

## Affected Areas

| Area                          | Impact   | Description                                                    |
|-------------------------------|----------|----------------------------------------------------------------|
| `src/cost/tracker.rs`         | Modified | Instantiate in runtime bootstrap; currently dead code          |
| `src/agent/loop_.rs`          | Modified | Add pre-flight `check_budget()` and post-call `record_usage()` |
| `src/agent/mission.rs`        | Modified | Delegate `accumulated_cost_cents` to `CostTracker`             |
| `src/config/schema.rs`        | Modified | Ensure `CostConfig` fields are runtime-accessible              |
| `src/security/policy.rs`      | Modified | Rename `max_cost_per_day_cents` → `max_actions_per_hour`       |
| `src/cli/`                    | New      | `corvus cost` subcommand, `--override-budget` flag             |
| `src/gateway/admin.rs`        | Modified | Cost data endpoints, admin PATCH for CostConfig                |
| `src/observability/`          | Modified | Populate `cost_usd` in events, new budget events               |
| `clients/web/apps/dashboard/` | Modified | `CostOverview.vue` live usage, charts, alerts                  |

## Risks

| Risk                                                                                                                  | Likelihood | Mitigation                                                                                   |
|-----------------------------------------------------------------------------------------------------------------------|------------|----------------------------------------------------------------------------------------------|
| Naming confusion between action-rate (`max_cost_per_day_cents`) and token-spend (`daily_limit_usd`) during transition | High       | Addressed directly by rename in Issue E; deprecation warning for old config key              |
| Provider gaps — some providers may not return token counts                                                            | Medium     | Add estimation fallback based on model pricing table; log when estimation is used            |
| JSONL append-only storage growth without rotation                                                                     | Medium     | Add log pruning/rotation in Issue A or dedicated follow-up; configurable retention window    |
| `f64` precision drift for USD amounts over long aggregation periods                                                   | Low        | Acceptable for current scale; integer-cents migration deferred to future iteration           |
| Config migration — existing `autonomy.max_cost_per_day_cents` in user files                                           | Medium     | Deprecation path with clear warning message; old key continues to work for one release cycle |
| Pre-flight budget check adds latency to every LLM call                                                                | Low        | `CostTracker` is in-memory with JSONL flush; check is O(1) lookup against cached aggregates  |

## Rollback Plan

1. **Feature flag**: Set `cost.enabled = false` (the default) to disable all budget enforcement
   immediately. No code revert needed.
2. **Per-issue revert**: Each issue is independently deployable. If Issue A (agent loop wiring)
   causes problems, revert that PR alone — downstream issues (B-F) degrade gracefully to showing
   config-only data.
3. **Config rename**: If the `max_actions_per_hour` rename (Issue E) breaks existing deployments,
   the deprecation alias ensures the old key still works during the transition period.
4. **Dashboard**: `CostOverview.vue` already shows config-only; if API endpoints (Issue C) are
   unavailable, it falls back to current behavior.

## Dependencies

- `CostTracker` (`src/cost/tracker.rs`) must be functional — it exists and passes tests but needs
  runtime instantiation
- `CostConfig` (`src/config/schema.rs`) fields must be loaded into runtime config — they exist but
  `cost.enabled` defaults to `false`
- Provider `TokenUsage` must be returned from LLM calls — verify which providers already return this
  data
- Agent loop spec (see `openspec/specs/agent-loop/spec.md`) defines the canonical loop contract
  where budget checks integrate

## Success Criteria

- [ ] `CostTracker` is instantiated in runtime and enforces daily/monthly budgets when
  `cost.enabled = true`
- [ ] LLM calls are blocked with a structured `BudgetExceeded` result when budget is exceeded
- [ ] `corvus cost` CLI subcommand shows current spend, history, and supports reset
- [ ] Budget overrides are logged as auditable `ObserverEvent::BudgetOverride` events
- [ ] `GET /cost/summary` returns current spend data; `GET /cost/history` returns time series
- [ ] `CostOverview.vue` displays live usage data, budget progress, and alert indicators
- [ ] `cost_usd` field in `AgentEnd` observer events is populated (no longer always `None`)
- [ ] `SecurityPolicy.max_cost_per_day_cents` is renamed to `max_actions_per_hour` with deprecation
  alias
- [ ] All enforcement is gated behind `cost.enabled` flag — disabled by default, zero behavior
  change for existing users
- [ ] Three parallel cost models consolidated into one unified model with `CostTracker` as source of
  truth
