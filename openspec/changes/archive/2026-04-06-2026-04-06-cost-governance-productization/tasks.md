# Tasks: Cost Governance Productization

## Phase 1: Runtime Cost Service and Override Flow

- [x] 1.1 Extend `clients/agent-runtime/src/cost/types.rs` with summary, history, audit, and override DTOs used by CLI, gateway, and dashboard.
- [x] 1.2 Create `clients/agent-runtime/src/cost/service.rs` to expose tracker-backed summary, history, reset, and scoped override operations.
- [x] 1.3 Update `clients/agent-runtime/src/cost/tracker.rs` and `clients/agent-runtime/src/cost/mod.rs` to support history windows, reset scopes, override expiry/scope, and shared exports.
- [x] 1.4 Update `clients/agent-runtime/src/agent/agent.rs` to emit first-class warning/exceeded outcomes and consume override decisions instead of ad hoc logging.
- [x] 1.5 Verification: add Rust unit tests for threshold math, history aggregation, reset behavior, and override scope expiry in `clients/agent-runtime/src/cost/*`.

## Phase 2: CLI and Gateway/Admin Surface

- [x] 2.1 Update `clients/agent-runtime/src/main.rs` to add `corvus cost` subcommands (`summary`, `history`, `reset`), exit summaries, and explicit `--override-budget` flow.
- [x] 2.2 Create `clients/agent-runtime/src/gateway/cost.rs` for `/api/web/cost/summary`, `/api/web/cost/history`, `/api/web/admin/cost/reset`, and `/api/web/admin/cost/override`.
- [x] 2.3 Update `clients/agent-runtime/src/gateway/admin.rs` and `clients/agent-runtime/src/gateway/mod.rs` to PATCH `cost` config separately from autonomy fields and route machine-readable budget errors.
- [x] 2.4 Verification: add runtime integration tests covering CLI override audit, gateway auth for reset/override, summary/history payloads, and blocked-request responses.

## Phase 3: Dashboard and Reporting UX

- [x] 3.1 Update `clients/web/apps/dashboard/src/types/admin-config.ts` and `src/composables/useAdmin.ts` for cost summary/history/deprecation metadata.
- [x] 3.2 Create `clients/web/apps/dashboard/src/composables/useCostGovernance.ts` to load live usage, alerts, and operator actions from gateway-safe APIs.
- [x] 3.3 Update `clients/web/apps/dashboard/src/components/config/CostOverview.vue` to show policy, live usage, warnings/blocks, overrides/resets, and reporting trends.
- [x] 3.4 Verification: extend `clients/web/apps/dashboard/src/components/config/CostOverview.spec.ts` for config-only fallback, warning/exceeded states, and live history rendering.

## Phase 4: Observability and Audit Completion

- [x] 4.1 Update `clients/agent-runtime/src/observability/traits.rs` with `BudgetWarning`, `BudgetExceeded`, and `BudgetOverride` event variants plus audit payloads.
- [x] 4.2 Update `clients/agent-runtime/src/observability/log.rs`, `otel.rs`, and `prometheus.rs` to emit cost lifecycle logs, spans, metrics, and `cost_usd` session data.
- [x] 4.3 Verification: add focused tests/assertions that warning, block, override, and reset events are emitted without leaking secrets.

## Phase 5: Governance Cleanup and Mission Alignment

- [x] 5.1 Update `clients/agent-runtime/src/agent/mission.rs` so mission spend is derived from runtime cost records instead of mission-local truth.
- [x] 5.2 Update `clients/agent-runtime/src/security/policy.rs`, `clients/agent-runtime/src/config/schema.rs`, and `clients/agent-runtime/src/config/mod.rs` to rename `max_cost_per_day_cents`, keep a one-release alias, and emit deprecation warnings.
- [x] 5.3 Verification: add tests for mission/session independence, action-rate vs token-spend denial labeling, and deprecated config normalization across CLI/admin responses.

## Phase 6: Verification Follow-up Gaps

- [x] 6.1 Add explicit `session` token-spend budget scope to runtime config, tracker evaluation, and exposed summaries.
- [x] 6.2 Add explicit `mission` token-spend budget scope to runtime evaluation so mission scope is enforced alongside session/day/month where applicable.
- [x] 6.3 Update budget evaluation to check all configured scopes before metered model calls and return the governing scope/result.
- [x] 6.4 Add spec-mapped tests for multi-scope evaluation, mission/session independence, governance-domain separation, and cross-surface consistency.
- [x] 6.5 Bring validation to green for verify expectations, including `cargo fmt --all -- --check` and relevant dashboard package-level checks or documented scoped exceptions.
