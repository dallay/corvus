# Tasks: Dashboard Memory Graph Explorer

## Phase 1: Data contracts and explorer state

- [x] 1.1 RED: extend `clients/web/apps/dashboard/src/composables/useAdmin.spec.ts` for `listMemoryEntries()` return values, `fetchMemoryStats()` return values, pagination params, and non-breaking reactive refs.
- [x] 1.2 GREEN: update `clients/web/apps/dashboard/src/composables/useAdmin.ts` to expose response-returning local memory helpers while preserving existing loading buckets and list/stats refs.
- [x] 1.3 GREEN: extend `clients/web/apps/dashboard/src/types/admin-sessions.ts` with `LocalMemorySubview`, explorer selection, timeline groups, category facets, relationship clusters, and snapshot types.
- [x] 1.4 RED: add `clients/web/apps/dashboard/src/composables/useLocalMemoryExplorer.spec.ts` covering chronological grouping, no-session fallback, category focus clear, session/category intersections, truncation, and zero Cerebro dependency.
- [x] 1.5 GREEN: create `clients/web/apps/dashboard/src/composables/useLocalMemoryExplorer.ts` to page local memory (`200` per request), cap explorer entries, and derive timeline/facet/cluster/visible-entry state.

## Phase 2: Local explorer components

- [x] 2.1 RED: add `LocalMemoryExplorerPanel.spec.ts` for loading, empty, error, truncated, and local-only explanatory states.
- [x] 2.2 GREEN: create `clients/web/apps/dashboard/src/components/memory/LocalMemoryExplorerPanel.vue` to coordinate explorer state, notices, and child component wiring.
- [x] 2.3 RED: add `LocalMemoryTimeline.spec.ts`, `LocalMemoryCategoryChart.spec.ts`, and `LocalMemoryRelationshipExplorer.spec.ts` for ordering, active selection, clear-focus flow, and intersection drill-in.
- [x] 2.4 GREEN: create `LocalMemoryTimeline.vue`, `LocalMemoryCategoryChart.vue`, and `LocalMemoryRelationshipExplorer.vue` with native Vue/CSS/SVG rendering, inferred-local labeling, and a navigable no-session lane.

## Phase 3: Dashboard wiring and fallback preservation

- [x] 3.1 RED: extend `clients/web/apps/dashboard/src/components/memory/MemoryStats.spec.ts` and `MemoryList.spec.ts` for category/session drill-in events that keep the browse list as the fallback.
- [x] 3.2 GREEN: update `clients/web/apps/dashboard/src/components/memory/MemoryStats.vue` and `MemoryList.vue` to emit local explorer handoff events without changing remote Cerebro behavior.
- [x] 3.3 RED: extend `clients/web/apps/dashboard/src/App.spec.ts` for `browse|explorer` subview switching, filter preservation, session handoff, and local-vs-Cerebro separation.
- [x] 3.4 GREEN: update `clients/web/apps/dashboard/src/App.vue` and, if needed, `CerebroTimelinePanel.vue` copy to wire the explorer subview and keep remote semantics visibly separate.

## Phase 4: Verification

- [x] 4.1 Verify composables with `pnpm --dir clients/web --filter @corvus/dashboard test -- src/composables/useAdmin.spec.ts src/composables/useLocalMemoryExplorer.spec.ts`.
- [x] 4.2 Verify components and app wiring with `pnpm --dir clients/web --filter @corvus/dashboard test -- src/components/memory/LocalMemoryExplorerPanel.spec.ts src/components/memory/LocalMemoryTimeline.spec.ts src/components/memory/LocalMemoryCategoryChart.spec.ts src/components/memory/LocalMemoryRelationshipExplorer.spec.ts src/components/memory/MemoryStats.spec.ts src/components/memory/MemoryList.spec.ts src/App.spec.ts`.
- [x] 4.3 Run dashboard-local static verification with `pnpm --dir clients/web --filter @corvus/dashboard run check` (no build).
