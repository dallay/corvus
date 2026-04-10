# Proposal: Dashboard Memory Graph Explorer

## Intent

Issue #363 is the follow-up to `session-memory-visibility`: operators can already browse local memory and sessions in the dashboard, but the local memory experience is still a flat list. The dashboard needs a focused visualization layer that helps operators understand memory over time, see category distribution, and navigate how local memory clusters around sessions without blurring the boundary between local SQLite-backed data and the existing remote-only Cerebro panels.

This change adds a **local memory visualization tab/page** inside the dashboard memory experience, using existing admin APIs where possible so v1 stays implementation-realistic and does not depend on remote Cerebro semantics or a large backend contract expansion.

**GitHub Issue:** #363 — Memory graph visualization for dashboard

## Scope

### In Scope

- Add a dedicated **local memory visualization** page/tab within the dashboard memory area.
- Show a **chronological timeline** of local memory entries, visually grouped by `session_id`.
- Show a **category breakdown chart** for local memory using `/web/admin/memory/stats`.
- Add a **navigable relationship explorer** for local memory that lets operators move between session groups, category facets, and the memory entries belonging to them.
- Reuse existing `/web/admin/memory` and `/web/admin/memory/stats` endpoints as the primary data source.
- Keep **Local Memory** and **Cerebro Memory** clearly separated in copy, layout, and interaction model.
- Allow a small visualization dependency only if it meaningfully reduces implementation complexity and remains isolated to the dashboard app.

### Out of Scope

- Adding or depending on remote Cerebro timeline, ontology, or relationship semantics for the local explorer.
- Building a full editable graph studio, force-directed knowledge graph, or ontology editor.
- Introducing cross-session semantic edges, similarity scoring, or AI-generated relationship inference.
- Replacing the existing local memory list, filters, or stats cards.
- Changing runtime session lifecycle, local memory storage semantics, or the existing admin auth model.
- Mobile/KMP dashboard parity or end-user chat memory visualization.
- Bulk memory editing, drag-and-drop graph manipulation, or export/share workflows.

## Approach

Use a **local-first visualization layer** in the dashboard, built on top of the current admin contracts.

### V1 relationship model decision

**Decision:** v1 relationships will be **inferred-only**, with **no new local memory contract extension**.

### Justification

- The current local contract already provides the only trustworthy structural signal needed for a realistic first release: `session_id` on memory entries plus category aggregation from `/web/admin/memory/stats`.
- Explicit relationship edges do not exist today, and inventing them now would either require speculative backend logic or a new data model that is not required to satisfy issue #363.
- A session-centric inferred model is understandable, testable, and cheap to ship: operators can navigate **session → entries**, **category → entries**, and **session ↔ category intersections** without pretending local SQLite memory has Cerebro-style graph semantics.
- This preserves the product boundary: local visualization stays operational and concrete, while remote Cerebro relationship views remain a separate, explicitly remote enhancement path.

### V1 interaction model

1. **Timeline lane view**
   - Fetch local memory entries from `/web/admin/memory`.
   - Render entries in chronological order.
   - Group them visually by `session_id`, with a fallback bucket for entries without a session.

2. **Category chart**
   - Use `/web/admin/memory/stats` for category totals.
   - Clicking a category filters/highlights the timeline and relationship explorer.

3. **Relationship explorer**
   - Represent relationships as navigable clusters, not as semantic truth claims.
   - Primary relationships are:
     - **Session ↔ Memory Entry** via shared `session_id`
     - **Category ↔ Memory Entry** via entry `category`
     - **Session ↔ Category** as an aggregated derived view from entries in that session
   - The UI MAY use a lightweight interactive chart or node-link view, but the fallback SHOULD be a simpler grouped explorer if a library adds too much cost.

4. **Clear local/remote boundary**
   - The new visualization is explicitly labeled as **Local Memory Visualization**.
   - Existing Cerebro panels remain separate and are not reused as the local relationship model.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/web/apps/dashboard/src/App.vue` | Modified | Add navigation/state wiring for the local memory visualization page/tab and preserve local-vs-Cerebro separation |
| `clients/web/apps/dashboard/src/composables/useAdmin.ts` | Modified | Add view-friendly fetch helpers and state handling for visualization data derived from existing local admin APIs |
| `clients/web/apps/dashboard/src/types/admin-sessions.ts` | Modified | Add dashboard-local view models for timeline grouping and inferred relationship explorer state |
| `clients/web/apps/dashboard/src/components/memory/MemoryStats.vue` | Modified | Reuse or extend stats presentation to feed the category visualization workflow |
| `clients/web/apps/dashboard/src/components/memory/MemoryList.vue` | Modified | Support drill-in/navigation handoff between the existing list and the new visualization view |
| `clients/web/apps/dashboard/src/components/memory/` (new visualization components) | New | Add focused components for timeline rendering, category chart, and inferred relationship explorer |
| `clients/web/apps/dashboard/src/components/memory/CerebroTimelinePanel.vue` and related Cerebro panels | Possible copy/state touch | Clarify remote-only semantics so local visualization does not conflate with Cerebro timeline/relationship views |
| `clients/web/apps/dashboard/src/**/*.spec.ts` | Modified | Add component/composable tests for grouping, filtering, and local/remote mode boundaries |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Operators confuse local inferred relationships with Cerebro semantic relationships | Medium | Use explicit “Local Memory” labeling, explanatory copy, and keep the explorer physically separate from Cerebro panels |
| Visualization scope expands into a full graph product | High | Lock v1 to inferred session/category navigation only; push explicit edges, editing, and semantic inference out of scope |
| Large local datasets make visualization slow or noisy | Medium | Reuse pagination/filtering, cap rendered points when necessary, and default to session/category slices rather than rendering everything at once |
| Third-party visualization library adds maintenance cost | Medium | Prefer native/Vue rendering first; allow a small library only if it materially simplifies implementation and remains isolated |
| Existing local APIs prove awkward for one-screen visualization | Low | Keep v1 data shaping in the dashboard layer first; only consider a minimal backend extension in a later change if proven necessary during design |

## Rollback Plan

1. Remove the new visualization tab/page wiring from the dashboard memory flow.
2. Remove visualization-specific components and view models.
3. Keep existing Memory list, filters, stats, Sessions views, and Cerebro panels unchanged.
4. If a visualization dependency was introduced, remove it from the dashboard workspace manifest.

Rollback is safe because this proposal is intentionally dashboard-local and additive; it does not require a local memory schema migration or a new required gateway contract.

## Dependencies

- Existing local admin endpoints: `/web/admin/memory` and `/web/admin/memory/stats`
- Existing dashboard memory and session surfaces in `clients/web/apps/dashboard`
- Archived `session-memory-visibility` change as the local baseline
- Existing Cerebro dashboard panels only as a boundary reference, not as the local data model
- Optional lightweight visualization library, only if design confirms the value exceeds the maintenance cost

## Success Criteria

- [ ] Dashboard exposes a dedicated local memory visualization page/tab for operators.
- [ ] Timeline renders local memory entries chronologically and groups them by session.
- [ ] Category distribution is visually represented and can drive filtering/highlighting.
- [ ] Operators can navigate inferred local relationships between sessions, categories, and memory entries.
- [ ] Local visualization remains clearly separated from remote Cerebro panels and terminology.
- [ ] The feature ships without requiring a new explicit local-memory edge contract in v1.
- [ ] Existing memory list/stats behavior continues to work unchanged as the non-visual fallback.
- [ ] Dashboard tests cover timeline grouping, category interactions, and local/remote boundary behavior.
