# Tasks: Cerebro Memory Enhancement Layer

## Phase 1: Gateway Contracts and Capability Foundation

- [x] 1.1 RED: add Rust tests for `clients/agent-runtime/src/tools/mcp/normalize.rs` covering `available`, `unconfigured`, `unreachable`, `unsupported`, and `not_implemented` mappings for the 12 allowlisted Cerebro tools.
- [x] 1.2 GREEN: extend `clients/agent-runtime/src/tools/mcp/normalize.rs` with canonical tool constants, planned-tool overrides, and shared gateway status classifiers used by status and action handlers.
- [x] 1.3 RED: add mock-driven tests for `clients/agent-runtime/src/tools/mcp/client.rs` and `clients/agent-runtime/src/tools/mcp/cerebro.rs` proving live `tools/list` discovery and safe auth-redacted failures.
- [x] 1.4 GREEN: implement the Cerebro inventory/helper path in `clients/agent-runtime/src/tools/mcp/client.rs` and `clients/agent-runtime/src/tools/mcp/cerebro.rs`, then align `clients/agent-runtime/src/agent/memory_loader.rs` with the local-first session limitation.

## Phase 2: Gateway Handlers and Typed Proxy Surface

- [x] 2.1 RED: add handler tests for `clients/agent-runtime/src/gateway/cerebro.rs` covering admin auth/origin enforcement, raw MCP payload rejection, and typed `/web/admin/cerebro/status` responses.
- [x] 2.2 GREEN: create `clients/agent-runtime/src/gateway/cerebro.rs` and wire `clients/agent-runtime/src/gateway/mod.rs` for `/web/admin/cerebro/status`, search, observation, timeline, stats, memory write, prompt, session, and context routes.
- [x] 2.3 GREEN: implement typed DTOs and allowlisted handler translation in `clients/agent-runtime/src/gateway/cerebro.rs`, reusing shared helpers from `clients/agent-runtime/src/gateway/admin.rs` or `clients/agent-runtime/src/gateway/utils.rs` where needed.
- [x] 2.4 RED/GREEN: add focused proxy tests proving success plus normalized `unconfigured`, `unreachable`, `unsupported`, and `not_implemented` action outcomes for search, stats, and session/context workflows.

## Phase 3: Dashboard Types, Composables, and UI Wiring

- [x] 3.1 RED: extend `clients/web/apps/dashboard/src/composables/useAdmin.spec.ts` for typed Cerebro status/actions, separate loading buckets, and normalized degraded-state parsing.
- [x] 3.2 GREEN: update `clients/web/apps/dashboard/src/types/admin-sessions.ts` and `clients/web/apps/dashboard/src/composables/useAdmin.ts` with Cerebro capability, search, observation, timeline, stats, and session/context contracts.
- [x] 3.3 GREEN: update `clients/web/apps/dashboard/src/App.vue`, `components/memory/MemoryStats.vue`, and `components/memory/MemoryList.vue`; add `CerebroStatusCard.vue`, `CerebroSearchPanel.vue`, `CerebroObservationDetail.vue`, and `CerebroTimelinePanel.vue` for local-vs-Cerebro memory flows.
- [x] 3.4 GREEN: update `clients/web/apps/dashboard/src/components/sessions/SessionDetail.vue`; add `CerebroSessionActions.vue` so local session facts stay primary while context/session tools render explicit readiness and results.

## Phase 4: Focused UI Verification and Graceful Degradation

- [x] 4.1 RED/GREEN: add `MemoryStats.spec.ts`, `MemoryList.spec.ts`, and new Cerebro memory panel specs covering available search, remote stats, relationship insights, and fallback when Cerebro is unconfigured or unreachable.
- [x] 4.2 RED/GREEN: add `SessionDetail.spec.ts` and `CerebroSessionActions` specs covering visible `not_implemented` session tools, available context lookup, and non-blocking local-session rendering.
- [x] 4.3 Verify the full slice with targeted Rust gateway tests and dashboard Vitest suites so gateway contracts, capability normalization, UI states, and graceful degradation all match the delta specs.
