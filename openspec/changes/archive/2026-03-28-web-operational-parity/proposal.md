# Proposal: Web Operational Parity

## Intent

The Corvus runtime exposes ~60 admin/operator capabilities through its CLI, config schema, and
gateway API. The dashboard surfaces ~10 config sections. The chat app surfaces only pairing and
message send/receive. This creates a significant operational gap: operators must SSH into the host
and use the CLI for most administrative tasks, even when a web-accessible runtime is available.

This proposal closes the gap by:

1. **Expanding the dashboard** to surface Tier 1 and Tier 2 runtime capabilities through new config
   sections and operational views.
2. **Adding gateway endpoints** to expose runtime state that is currently only available via CLI.
3. **Enhancing the chat app** with end-user features (streaming, tool approval) without adding admin
   surfaces.
4. **Maintaining clear boundaries** — the dashboard is operator-only, the chat app is end-user-only,
   and CLI-only capabilities (Tier 3) stay in the CLI.

**Reference**: DALLAY-181, GitHub [#276](https://github.com/dallay/corvus/issues/276), parent
DALLAY-176.

## Scope

### In Scope

- **Dashboard: wire already-built views** — Provider account pools, update status, and health
  already have types/views in `gateway/admin.rs` and `types/admin-config.ts`. Create Vue components
  and wire them to the existing GET endpoints.

- **Dashboard: channel visibility** — Add `GET /web/admin/channels` endpoint returning status for
  all configured channels (Telegram, Discord, WhatsApp, Slack, webhook). Create a channel management
  dashboard section showing enabled/disabled status, health indicators, and configuration summaries.

- **Dashboard: operational visibility** — Surface autonomy policy details (auto_approve, always_ask
  lists), scheduler task list (beyond just settings), and skills inventory. Add new gateway
  endpoints
  where the runtime data exists but no HTTP surface exists yet.

- **Dashboard: extended config sections** — Add dashboard sections for web search, browser,
  composio, memory/cerebro, identity, and multimodal config. These already exist in
  `AdminConfigView` and `AdminConfigUpdateRequest` but have no Vue components.

- **Chat: end-user enhancements** — Streaming responses (SSE), tool approval UI, session
  persistence, health indicator, and file upload support. These are end-user features only.

- **Gateway: new admin endpoints** — Up to 14 new endpoints for capabilities that exist in the
  runtime but have no HTTP surface.

### Out of Scope

- **New runtime capabilities** — This change surfaces existing capabilities; it does not add new
  operator features to the runtime itself.
- **CLI changes** — The CLI interface is unchanged.
- **Admin surfaces in chat app** — The chat app boundary is end-user only. No config editing, no
  channel management, no provider management.
- **Tier 3 capabilities** — Onboarding, interactive sessions, OS service management,
  hardware/peripheral, migration, OAuth flows, sandbox config, Telegram binding, binary updates
  remain CLI-only.
- **Mobile clients** — `clients/composeApp` is not in scope for this change.
- **Authentication/authorization changes** — The existing pairing-based auth model is sufficient.
  RBAC or multi-user access control is a separate initiative.

## Approach

This is a **multi-phase, multi-layer change** spanning Rust gateway endpoints, TypeScript types, and
Vue components. The phases are designed to be independently shippable:

### Phase 1: Wire Already-Built (lowest risk, highest value)

Surface data that already flows through `AdminConfigView`:

- **Provider pools component** — `ProviderPoolsSettings.vue` using existing
  `GET/PUT /web/admin/provider-pools` endpoints and `AdminProviderPoolsView` types.
- **Update status component** — `UpdateSettings.vue` using the `updates` section of
  `AdminConfigView` (already serialized by the gateway).
- **Web search / Browser / Composio / Memory config components** — These fields already exist in
  `AdminConfigView` and `AdminConfigUpdateRequest`. Create Vue components + extend
  `AdminConfigForm`.

### Phase 2: Channel Visibility (new endpoint + UI)

- Add `GET /web/admin/channels` endpoint in `gateway/admin.rs` returning enabled channels with
  health status.
- Create `ChannelsOverview.vue` dashboard section.
- Extend `AdminConfigForm` and `ConfigSection` type.

### Phase 3: Operational Visibility (new endpoints + UI)

- Extend `SecuritySettings.vue` to show autonomy policy details (auto_approve, always_ask).
- Add `GET /web/admin/tasks` endpoint for scheduled task list.
- Add `GET /web/admin/health` aggregate health endpoint.
- Create skills inventory endpoint and component if runtime supports enumeration.

### Phase 4: Extended Config (new endpoints + UI)

- Add MCP config endpoint and component.
- Add tunnel config endpoint and component.
- Add cost tracking, model catalog, daemon status endpoints (Tier 2).

### Phase 5: Chat Enhancements (parallel)

- SSE streaming endpoint + chat app integration.
- Tool approval WebSocket/SSE flow.
- Session persistence (conversation history across page reloads).
- Health indicator (simple ping to gateway).

## Affected Areas

| Area                                                          | Impact            | Description                                                          |
|---------------------------------------------------------------|-------------------|----------------------------------------------------------------------|
| `clients/agent-runtime/src/gateway/admin.rs`                  | Modified          | New view structs, new handler functions for ~14 endpoints            |
| `clients/agent-runtime/src/gateway/mod.rs`                    | Modified          | Route registration for new endpoints                                 |
| `clients/web/apps/dashboard/src/components/config/`           | New files         | ~8 new Vue config section components                                 |
| `clients/web/apps/dashboard/src/types/admin-config.ts`        | Modified          | Extended types for new config sections, new `ConfigSection` variants |
| `clients/web/apps/dashboard/src/composables/useConfig.ts`     | Modified          | New composable logic for additional sections                         |
| `clients/web/apps/dashboard/src/composables/configPayload.ts` | Modified          | Payload builders for new sections                                    |
| `clients/web/apps/chat/src/composables/useChat.ts`            | Modified          | Streaming, tool approval, session persistence                        |
| `clients/web/apps/chat/src/components/`                       | New files         | Tool approval UI, health indicator                                   |
| `clients/web/packages/ui/`                                    | Possibly modified | Shared UI components if needed                                       |

## Risks

| Risk                                                   | Likelihood | Mitigation                                                                                                                               |
|--------------------------------------------------------|------------|------------------------------------------------------------------------------------------------------------------------------------------|
| Large surface area increases review burden             | High       | Phase independently; each phase is a separate PR with its own sub-issue                                                                  |
| New endpoints expose sensitive config data             | Medium     | All new endpoints go through existing pairing auth. Follow redaction patterns from `AdminConfigView` (e.g., `has_api_key` not `api_key`) |
| Dashboard bundle size growth from many new components  | Low        | Vue components are small; lazy-load sections not visible on initial render                                                               |
| Gateway endpoint proliferation complicates API surface | Medium     | Group related endpoints logically. Document in API section of design. Consider a `/web/admin/overview` aggregate endpoint                |
| SSE streaming adds complexity to gateway               | Medium     | Use axum's built-in SSE support. Keep streaming endpoint simple (text chunks only)                                                       |
| Breaking existing dashboard behavior                   | Low        | Existing 7 sections are unchanged. New sections are additive. Test existing components after changes                                     |

## Rollback Plan

Each phase is independently reversible:

1. **Phase 1** (wire already-built): Delete new Vue components. Revert `ConfigSection` type
   additions. No gateway changes needed — endpoints already exist.
2. **Phase 2** (channels): Revert the `GET /web/admin/channels` handler and route. Delete
   `ChannelsOverview.vue`.
3. **Phase 3** (operational): Revert new endpoint handlers and routes. Delete new components. Revert
   `SecuritySettings.vue` changes.
4. **Phase 4** (extended config): Same pattern — revert endpoints and delete components.
5. **Phase 5** (chat): Revert streaming endpoint. Delete tool approval components. Revert
   `useChat.ts` changes.

Git: all work is on `feature/dallay-181-expand-dashboard-and-web-operational-clients-to-match`.
Each phase merges as a separate PR. `git revert` of any merge commit cleanly undoes that phase.

## Dependencies

- **Existing gateway admin API** — Phases 1-4 depend on the current `AdminConfigView` and endpoint
  structure in `gateway/admin.rs`.
- **Existing dashboard architecture** — Config section component pattern, `useConfig` composable,
  `configPayload` builder.
- **No external dependencies** — All changes are within `clients/agent-runtime` and `clients/web`.

## Success Criteria

- [ ] Dashboard surfaces provider account pools with add/edit/remove capability
- [ ] Dashboard shows update status with version info and update controls
- [ ] Dashboard shows channel overview with health status for all configured channels
- [ ] Dashboard surfaces web search, browser, composio, memory, and identity config
- [ ] Dashboard shows autonomy policy details (auto_approve, always_ask) with editing
- [ ] All new gateway endpoints require pairing auth and redact secrets
- [ ] Chat app supports streaming responses via SSE
- [ ] Chat app shows tool approval UI when runtime requests human confirmation
- [ ] All existing dashboard tests pass after changes
- [ ] New components have unit tests matching existing coverage patterns
- [ ] Each phase is independently deployable and reversible
