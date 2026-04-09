# Proposal: Cerebro Memory Enhancement Layer

## Intent

Issue #361 is the approved Phase 2 follow-up to the archived `session-memory-visibility` change.
Corvus already exposes **local-first** session and memory visibility through SQLite-backed gateway
endpoints and dashboard views, but operators still cannot use Cerebro's richer long-term-memory
capabilities from the gateway or dashboard. Current runtime reality is partial: Cerebro is
configurable, `mem_save` / `mem_search` / `mem_delete` integrations exist, dashboard only shows a
basic `cerebro_configured` signal, and the docs still mark `mem_save_prompt`,
`mem_session_start`, `mem_session_end`, `mem_session_summary`, and `mem_context` as planned /
NotImplemented.

This change adds an **enhancement layer** on top of the existing local visibility baseline: the
gateway becomes a safe operator-only proxy for Cerebro memory tools, and the dashboard adds
Cerebro-aware search, drill-in, relationship, and session/context workflows. The proposal is
aggressive by design: it includes session/context tool support in the gateway and dashboard even
when Cerebro cannot fully execute those calls yet, so the product surface can feature-detect,
communicate readiness, and degrade gracefully instead of hiding the capability entirely.

## Scope

### In Scope

- Add **admin-only gateway proxy endpoints** for the Cerebro memory tool surface used by operators,
  with typed request/response contracts rather than raw JSON-RPC pass-through.
- Add a **Cerebro capability/status endpoint** that reports configured / reachable / available /
  planned-but-not-implemented states for the relevant memory tools.
- Proxy implemented Cerebro tools needed for operator memory workflows, including at minimum:
  `mem_search`, `mem_get_observation`, `mem_timeline`, `mem_stats`, and the existing write/update
  tools where appropriate for admin workflows.
- Include **session/context-oriented tool support** in the enhancement layer for
  `mem_session_start`, `mem_session_end`, `mem_session_summary`, `mem_context`, and
  `mem_save_prompt`, with normalized handling for `NotImplemented` and unavailable backend states.
- Enhance the dashboard memory experience with **Cerebro semantic search**, remote stats,
  observation detail drill-in, timeline/history exploration, and read-only relationship / ontology
  insights when returned by Cerebro.
- Enhance the dashboard session experience so operators can invoke or inspect Cerebro
  session/context workflows from the existing session detail flow without replacing the local
  SQLite session lifecycle as the source of truth.
- Preserve and clearly label the distinction between **Local Memory** (current SQLite visibility)
  and **Cerebro Memory** (remote long-term memory enhancement).
- Add focused runtime and dashboard tests for capability detection, proxy auth/validation,
  graceful degradation, and UI state handling.

### Out of Scope

- Implementing missing Cerebro tools inside the Cerebro service itself.
- Replacing the existing SQLite-backed session and memory visibility endpoints as the primary local
  source of truth.
- Changing end-user chat flows, mobile/KMP surfaces, or non-admin gateway endpoints.
- Building a full editable graph studio, ontology authoring system, or bulk remote memory
  management console.
- Automatic agent-loop behavior changes that make runtime context loading depend on new Cerebro
  session/context tools.
- Any relaxation of current gateway auth, pairing, or admin-role boundaries.

## Approach

Use a **local-first, remote-enhanced** architecture.

1. **Gateway enhancement layer**
   - Add a dedicated Cerebro admin proxy surface under `/web/admin/cerebro/*`.
   - Keep the proxy allowlisted and typed; do **not** expose arbitrary MCP tool execution.
   - Reuse existing admin bearer-token enforcement and Cerebro config/auth plumbing.
   - Normalize remote outcomes into stable gateway states such as `available`, `unconfigured`,
     `unreachable`, `unsupported`, and `not_implemented`.

2. **Capability-first integration**
   - Before the dashboard enables Cerebro actions, it queries a gateway status/capability endpoint.
   - The gateway determines tool readiness from configuration, connectivity, and tool-call results.
   - Session/context tools are surfaced even if backend execution returns `NotImplemented`; the UI
     must show that state explicitly instead of failing the entire memory experience.

3. **Dashboard enhancement of existing pages**
   - Extend the current Sessions and Memory dashboard sections rather than introducing a separate
     product surface.
   - Add Cerebro modes/panels alongside current local pages: semantic search, remote stats,
     observation detail, timeline, and relationship/ontology insights.
   - Add session-detail affordances for Cerebro context/session actions while keeping local session
     metadata authoritative.

4. **Safe graceful degradation**
   - If Cerebro is not configured, unreachable, or partially implemented, existing local SQLite
     views remain fully usable and visually primary.
   - Cerebro panels become read-only disabled states or informative empty states with actionable
     status messaging.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Register new `/web/admin/cerebro/*` routes and feature-detection wiring |
| `clients/agent-runtime/src/gateway/admin.rs` | Modified | Extend admin payloads and shared auth/error patterns for Cerebro enhancement endpoints |
| `clients/agent-runtime/src/gateway/` (new `cerebro.rs` or equivalent) | New/Modified | Typed handlers for capability/status and proxied Cerebro memory tool operations |
| `clients/agent-runtime/src/tools/mcp/cerebro.rs` | Modified | Reuse/build adapter helpers for gateway-side Cerebro tool calls |
| `clients/agent-runtime/src/tools/mcp/normalize.rs` | Modified | Expand canonical Cerebro tool constants / normalized status mapping |
| `clients/agent-runtime/src/agent/memory_loader.rs` | Possible follow-on touch | Align status assumptions and avoid conflicts with current session-scoped Cerebro limitations |
| `clients/web/apps/dashboard/src/composables/useAdmin.ts` | Modified | Add Cerebro capability + proxy API methods and UI state handling |
| `clients/web/apps/dashboard/src/types/admin-sessions.ts` | Modified | Add typed models for Cerebro capability, search, observation, timeline, and session/context results |
| `clients/web/apps/dashboard/src/components/memory/*` | Modified | Add Cerebro semantic search, remote stats, detail, and insight panels |
| `clients/web/apps/dashboard/src/components/sessions/*` | Modified | Add Cerebro session/context actions and readiness states in session detail UX |
| `clients/web/apps/dashboard/src/App.vue` | Modified | Add navigation/state wiring for enhanced local-vs-Cerebro operator flows |
| `clients/web/apps/docs/src/content/docs/cerebro/mcp-tools.md` and related docs | Modified | Document operator-facing enhancement behavior and capability states if contracts change |
| `clients/agent-runtime/tests/*` and `clients/web/apps/dashboard/src/**/*.spec.ts` | Modified | Add regression and feature-detection coverage |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Admin proxy accidentally broadens remote execution surface | Medium | Keep an explicit allowlist of Cerebro tool wrappers; no arbitrary MCP passthrough; reuse admin auth and input validation |
| Dashboard promises features that Cerebro still returns as `NotImplemented` | High | Make capability/status reporting first-class and render unsupported states explicitly in UI and API contracts |
| Operator confusion between local SQLite memory and Cerebro long-term memory | Medium | Keep separate labels, sections, and explanatory copy; local remains authoritative for current session visibility |
| Remote latency or outages degrade dashboard usability | Medium | Use bounded timeouts, non-blocking panels, cached readiness state where appropriate, and always preserve local views |
| Contract drift between gateway wrappers and evolving Cerebro tool schemas | Medium | Anchor gateway contracts to the current 13-tool inventory, add contract tests, and centralize normalization logic |
| Session/context tools need data that current runtime/dashboard flows do not yet model cleanly | Medium | Scope this phase to operator-triggered proxy workflows and explicit unsupported states, not automatic runtime behavior changes |

## Rollout Strategy

1. Ship the gateway Cerebro capability endpoint and hidden/disabled dashboard wiring first.
2. Enable implemented-tool panels (`mem_search`, observation drill-in, timeline, stats) behind
   runtime capability detection.
3. Add session/context UI affordances once normalized `not_implemented` / `unsupported` states are
   in place so aggressive scope does not break non-ready deployments.
4. Keep local visibility pages as the default operator fallback throughout rollout.

## Rollback Plan

1. Remove `/web/admin/cerebro/*` route registration and handler wiring.
2. Revert dashboard Cerebro panels/actions while preserving the existing Sessions and Memory local
   visibility views from `session-memory-visibility`.
3. Leave `memory.cerebro` configuration support untouched; rollback only removes the enhancement
   layer, not baseline Cerebro runtime configuration.
4. Because this change is additive and does not require new local database schema changes, rollback
   does not require data migration or cleanup.

## Dependencies

- Archived `session-memory-visibility` change as the baseline local session/memory UX and contracts.
- Existing Cerebro runtime configuration and MCP adapter plumbing in `clients/agent-runtime`.
- Current Cerebro 13-tool inventory defined in `openspec/specs/cerebro/spec.md` and documented in
  `clients/web/apps/docs/src/content/docs/cerebro/mcp-tools.md`.
- Existing dashboard session and memory views/composables introduced by the Phase 1 visibility
  work.
- Approved product direction from GitHub issue #361.

## Success Criteria

- [ ] The gateway exposes an admin-only Cerebro capability/status API that distinguishes
      configuration, reachability, and per-tool readiness.
- [ ] The gateway exposes typed proxy endpoints for the approved Cerebro memory workflows without
      allowing arbitrary MCP passthrough.
- [ ] Dashboard memory views support Cerebro semantic search and drill-in when Cerebro is available.
- [ ] Dashboard surfaces remote stats, observation detail, and timeline/history insights from
      Cerebro without regressing existing local memory visibility.
- [ ] Dashboard session detail exposes Cerebro session/context actions or status for
      `mem_session_start`, `mem_session_end`, `mem_session_summary`, and `mem_context`.
- [ ] When a planned tool returns `NotImplemented`, both gateway and dashboard surface a normalized,
      user-understandable unsupported state rather than a generic failure.
- [ ] Non-Cerebro deployments continue working unchanged with existing local session/memory views.
- [ ] All new Cerebro proxy routes require existing admin authentication and do not leak Cerebro
      auth tokens, raw MCP payloads, or sensitive memory data outside intended responses.
- [ ] Runtime and dashboard tests cover success, unavailable, unreachable, and not-implemented
      states for the enhancement layer.
