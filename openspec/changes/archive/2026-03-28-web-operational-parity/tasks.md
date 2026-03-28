# Tasks: Web Operational Parity

## Phase 1: Wire Already-Built (dashboard components for existing gateway data)

### 1A: TypeScript Type Extensions

- [x] 1.1 Extend `AdminConfigView` in `types/admin-config.ts` with typed interfaces for
  `web_search`, `browser`, `composio`, `memory`, and `identity` fields
    - Add `AdminWebSearchView`, `AdminBrowserView`, `AdminComposioView`, `AdminMemoryView`,
      `AdminCerebroMemoryView`, `AdminIdentityView` interfaces matching Rust `Admin*View` structs in
      `gateway/admin.rs`
    - Add corresponding fields to `AdminConfigView` interface
    - **Acceptance**: TypeScript types compile and match the JSON output of `GET /web/admin/config`
    - **Spec ref**: REQ-9

- [x] 1.2 Extend `AdminConfigForm` and `AdminConfigSnapshot` with form fields for new sections
    - Add form fields: `web_search_enabled`, `web_search_provider`, `web_search_max_results`,
      `web_search_timeout_secs`, `composio_enabled`, `composio_entity_id`, `memory_backend` (already
      exists), `memory_cerebro_endpoint`, `identity_format`, `identity_aieos_path`
    - Add matching snapshot fields with correct types (string → number for numeric fields)
    - **Acceptance**: Form types compile; `configPayload` can produce update payloads for new
      sections
    - **Spec ref**: REQ-9

- [x] 1.3 Extend `AdminConfigUpdateRequest` with patch types for new sections
    - Add `web_search?: AdminWebSearchPatch`, `composio?: AdminComposioPatch`,
      `memory?: AdminMemoryPatch`, `identity?: AdminIdentityPatch`, `browser?: AdminBrowserPatch`
    - Each patch type should use `SecretUpdate` for secret fields (e.g., `composio.api_key`,
      `browser.computer_use_api_key`)
    - **Acceptance**: Update request types compile and match Rust `AdminConfigUpdateRequest` in
      `gateway/admin.rs`
    - **Spec ref**: REQ-9

- [x] 1.4 Extend `ConfigSection` type with new section variants
    - Add: `"provider-pools"`, `"updates"`, `"web-search"`, `"browser"`, `"composio"`, `"memory"`,
      `"identity"`
    - **Acceptance**: All new section names are valid `ConfigSection` values
    - **Spec ref**: REQ-9

### 1B: Dashboard Config Section Components

- [x] 1.5 Create `ProviderPoolsSettings.vue` component
    - Display account pools grouped by provider with strategy selector
    - Show per-account: id, api_url, weight, enabled, has_api_key indicator
    - Support add/edit/remove accounts via `PUT /web/admin/provider-pools`
    - Follow existing component pattern (props from `useConfig`)
    - **Acceptance**: Component renders pools; add/remove/edit works; form submits correct payload
    - **Spec ref**: REQ-1 (provider pools scenario)

- [x] 1.6 Create `ProviderPoolsSettings.spec.ts` unit tests
    - Test rendering with empty pools, populated pools, and multiple providers
    - Test add account, remove account, edit account flows
    - Test payload matches `AdminProviderPoolsUpdateRequest`
    - **Acceptance**: All tests pass; coverage matches existing component test patterns
    - **Spec ref**: REQ-10

- [x] 1.7 Create `UpdateSettings.vue` component
    - Display: current version, latest version, update available flag
    - Display: last check timestamp (formatted), last check outcome, effective install method
    - Read-only (no edit) — update actions (check, install) are Phase 3
    - **Acceptance**: Component renders update status from `AdminConfigView.updates`
    - **Spec ref**: REQ-1 (update status scenario)

- [x] 1.8 Create `UpdateSettings.spec.ts` unit tests
    - Test rendering with no update available, update available, and missing status fields
    - **Acceptance**: All tests pass
    - **Spec ref**: REQ-10

- [x] 1.9 Create `WebSearchSettings.vue` component
    - Display and edit: enabled toggle, provider, max_results, timeout_secs
    - Show Brave API key indicator (has_brave_api_key) with SecretUpdate for key changes
    - **Acceptance**: Component renders; form submission produces correct `web_search` patch payload
    - **Spec ref**: REQ-1 (web search scenario)

- [x] 1.10 Create `WebSearchSettings.spec.ts` unit tests
    - **Acceptance**: Tests cover render, edit, and secret field semantics
    - **Spec ref**: REQ-10

- [x] 1.11 Create `BrowserSettings.vue` component
    - Display computer-use API key indicator
    - Support SecretUpdate for key changes
    - **Acceptance**: Component renders; secret field uses correct SecretUpdate pattern
    - **Spec ref**: REQ-1

- [x] 1.12 Create `BrowserSettings.spec.ts` unit tests
    - **Acceptance**: Tests cover render and secret field semantics
    - **Spec ref**: REQ-10

- [x] 1.13 Create `ComposioSettings.vue` component
    - Display and edit: enabled toggle, entity_id
    - Show API key indicator with SecretUpdate
    - **Acceptance**: Component renders; form submission produces correct `composio` patch payload
    - **Spec ref**: REQ-1

- [x] 1.14 Create `ComposioSettings.spec.ts` unit tests
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

- [x] 1.15 Create `MemorySettings.vue` component
    - Display and edit: memory backend (reuse existing field from GeneralSettings or move here)
    - Display and edit Cerebro config: endpoint, auth token indicator, timeout, insecure loopback
    - **Acceptance**: Component renders; form submission produces correct `memory` patch payload
    - **Spec ref**: REQ-1

- [x] 1.16 Create `MemorySettings.spec.ts` unit tests
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

- [x] 1.17 Create `IdentitySettings.vue` component
    - Display and edit: format selection, AIEOS path, inline indicator
    - **Acceptance**: Component renders; form submission produces correct `identity` patch payload
    - **Spec ref**: REQ-1

- [x] 1.18 Create `IdentitySettings.spec.ts` unit tests
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

### 1C: Dashboard Integration

- [x] 1.19 Update `App.vue` to render new config sections
    - Add conditional rendering for all new `ConfigSection` values
    - Add navigation/menu items for new sections
    - **Acceptance**: All new sections are reachable from the dashboard navigation
    - **Spec ref**: REQ-1

- [x] 1.20 Update `configPayload.ts` to build payloads for new sections
    - Add payload builder functions for web_search, browser, composio, memory, identity
    - **Acceptance**: Each new section's form data correctly maps to `AdminConfigUpdateRequest`
      fields
    - **Spec ref**: REQ-9

- [x] 1.21 Update `useConfig.ts` composable for new sections
    - Extend form initialization and snapshot logic for new fields
    - **Acceptance**: `useConfig` populates form from config response and tracks dirty state for new
      fields
    - **Spec ref**: REQ-9

- [x] 1.22 Verify existing dashboard tests still pass
    - Run `pnpm --dir clients/web --filter @corvus/dashboard test`
    - **Acceptance**: All pre-existing tests pass without modification
    - **Spec ref**: REQ-10

## Phase 2: Channel Visibility (new endpoint + dashboard UI)

### 2A: Gateway Endpoint

- [x] 2.1 Add `AdminChannelStatusView` and `ChannelHealth` types to `gateway/admin.rs`
    - Define structs per design interface contracts
    - **Acceptance**: Types compile; `ChannelHealth` serializes to lowercase strings
    - **Spec ref**: REQ-2

- [x] 2.2 Implement `handle_admin_channels` handler in `gateway/admin.rs`
    - Enumerate all channel types (telegram, discord, whatsapp, slack, webhook, cli)
    - For each: determine enabled/disabled from config, serialize redacted config summary
    - For health: use channel's internal health state (best-effort, ADR-5)
    - **Acceptance**: `GET /web/admin/channels` returns list of channel statuses with no secrets
    - **Spec ref**: REQ-2, REQ-8

- [x] 2.3 Register `/web/admin/channels` route in `gateway/mod.rs`
    - Add `get` route with pairing auth middleware
    - **Acceptance**: Route responds to GET requests; unauthenticated requests return 401
    - **Spec ref**: REQ-2, REQ-8

- [x] 2.4 Add gateway integration tests for channels endpoint
    - Test: authenticated request returns channel list
    - Test: unauthenticated request returns 401
    - Test: response contains no secret values
    - **Acceptance**: Tests pass with `cargo test`
    - **Spec ref**: REQ-2, REQ-8, REQ-10

### 2B: Dashboard Component

- [x] 2.5 Add `AdminChannelStatusView` TypeScript type to `types/admin-config.ts`
    - Match Rust `AdminChannelStatusView` serialization
    - **Acceptance**: Type compiles and matches gateway response
    - **Spec ref**: REQ-9

- [x] 2.6 Create `ChannelsOverview.vue` component
    - Fetch from `GET /web/admin/channels`
    - Display each channel with type, enabled status, health indicator (color-coded)
    - Show redacted config summary
    - **Acceptance**: Component renders all channels; health indicators use visual differentiation
    - **Spec ref**: REQ-2

- [x] 2.7 Create `ChannelsOverview.spec.ts` unit tests
    - Test rendering with mixed channel states (connected, disconnected, not_configured)
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

## Phase 3: Operational Visibility (extended views + new endpoints)

### 3A: Autonomy Policy Details

- [x] 3.1 Extend `SecuritySettings.vue` to show autonomy policy details
    - Add display for `auto_approve` and `always_ask` command lists
    - Add `require_approval_for_medium_risk` and `block_high_risk_commands` toggles
    - Support editing lists (add/remove entries)
    - **Acceptance**: Security section shows full autonomy policy; list editing works
    - **Spec ref**: REQ-3

- [x] 3.2 Extend `SecuritySettings.spec.ts` for new autonomy fields
    - Test rendering with populated and empty lists
    - Test add/remove entry flows
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

- [x] 3.3 Extend `AdminConfigUpdateRequest` and payload for autonomy policy patches
    - Add `auto_approve`, `always_ask`, `require_approval_for_medium_risk`,
      `block_high_risk_commands` to autonomy patch
    - **Acceptance**: Payload builder produces correct autonomy patch with list fields
    - **Spec ref**: REQ-3, REQ-9

### 3B: Health Dashboard

- [x] 3.4 Add `AdminHealthView` and `ComponentHealth` types to `gateway/admin.rs`
    - Define structs per design interface contracts
    - **Acceptance**: Types compile
    - **Spec ref**: REQ-4

- [x] 3.5 Implement `handle_admin_health` handler
    - Aggregate health from: provider connectivity, channel states, memory backend, scheduler,
      gateway
    - Compute overall status (healthy if all healthy, degraded if any degraded, unhealthy if any
      unhealthy)
    - Include runtime uptime
    - **Acceptance**: `GET /web/admin/health` returns aggregate health view
    - **Spec ref**: REQ-4

- [x] 3.6 Register `/web/admin/health` route and add tests
    - **Acceptance**: Route registered; integration tests pass
    - **Spec ref**: REQ-4, REQ-8, REQ-10

- [x] 3.7 Create `HealthDashboard.vue` component
    - Fetch from `GET /web/admin/health`
    - Display aggregate status with color coding (green/yellow/red)
    - Display individual component statuses with details
    - **Acceptance**: Component renders; degraded components are visually highlighted
    - **Spec ref**: REQ-4

- [x] 3.8 Create `HealthDashboard.spec.ts` unit tests
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

### 3C: Scheduled Task List

- [x] 3.9 Add `AdminTaskListView` and `AdminTaskView` types to `gateway/admin.rs`
    - **Acceptance**: Types compile
    - **Spec ref**: REQ-5

- [x] 3.10 Implement `handle_admin_tasks` handler
    - Enumerate scheduled tasks from the scheduler
    - Return task name, schedule, last run, next run, enabled status
    - **Acceptance**: `GET /web/admin/tasks` returns task list
    - **Spec ref**: REQ-5

- [x] 3.11 Register `/web/admin/tasks` route and add tests
    - **Acceptance**: Route registered; integration tests pass
    - **Spec ref**: REQ-5, REQ-10

- [x] 3.12 Extend `SchedulerSettings.vue` to include task list
    - Display task list below existing scheduler config
    - Show each task with name, schedule, last run timestamp, next run, outcome
    - **Acceptance**: Scheduler section shows both settings and task list
    - **Spec ref**: REQ-5

- [x] 3.13 Extend `SchedulerSettings.spec.ts` for task list
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

## Phase 4: Extended Config (Tier 2 endpoints — future)

This phase covers Tier 2 capabilities that require new runtime-side implementation beyond
just gateway endpoints. These are tracked as follow-up issues under DALLAY-181.

- [x] 4.1 MCP config endpoint and dashboard section
- [x] 4.2 Tunnel config endpoint and dashboard section
- [x] 4.3 Cost tracking endpoint and dashboard section
- [x] 4.4 Model catalog endpoint and dashboard section
- [x] 4.5 Daemon status endpoint and dashboard section

**Note**: Phase 4 tasks are intentionally high-level. Each will be broken down into sub-tasks
when the corresponding follow-up issue is scoped.

## Phase 5: Chat Enhancements (parallel track)

### 5A: SSE Streaming

- [x] 5.1 Implement `POST /web/chat/stream` SSE endpoint in gateway
    - Accept message payload with pairing auth
    - Stream response chunks as SSE `chunk` events
    - Send `done` event on completion with message_id and token count
    - Send `error` event on provider errors
    - **Acceptance**: Endpoint streams text chunks; curl can receive SSE events
    - **Spec ref**: REQ-6

- [x] 5.2 Add gateway integration tests for streaming endpoint
    - Test: SSE stream produces chunk + done events
    - Test: provider error produces error event
    - Test: unauthenticated request returns 401
    - **Acceptance**: Tests pass with `cargo test`
    - **Spec ref**: REQ-6, REQ-10

- [x] 5.3 Update `useChat.ts` to support SSE streaming
    - Use `EventSource` or `fetch` with ReadableStream to consume SSE
    - Render chunks incrementally as they arrive
    - Handle `done` and `error` events
    - **Acceptance**: Chat messages stream in the UI; errors show error state
    - **Spec ref**: REQ-6

- [x] 5.4 Update `ChatMessage.vue` for streaming state
    - Show typing/streaming indicator while chunks arrive
    - Render partial text during streaming
    - Finalize message display on `done` event
    - **Acceptance**: Messages animate during streaming; final state is clean
    - **Spec ref**: REQ-6

- [x] 5.5 Add chat app tests for streaming
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

### 5B: Tool Approval

- [x] 5.6 Implement `POST /web/chat/tool-approval` endpoint in gateway
    - Accept `{ approval_id, approved }` payload
    - Route approval decision to the runtime's tool execution flow
    - Return `{ acknowledged: true }` on success
    - **Acceptance**: Endpoint accepts approval/rejection; runtime receives decision
    - **Spec ref**: REQ-7

- [x] 5.7 Implement tool approval delivery via SSE `tool_approval` event
    - When runtime needs approval during a streaming response, send `tool_approval` SSE event
    - Include tool name, parameters (redacted if sensitive), risk level, approval_id
    - **Acceptance**: SSE stream includes tool_approval event when tool needs approval
    - **Spec ref**: REQ-7

- [x] 5.8 Create `ToolApprovalCard.vue` component
    - Display tool name, parameters, risk level with visual risk indication
    - Show Approve and Reject buttons
    - Send decision via `POST /web/chat/tool-approval`
    - **Acceptance**: Card renders; approve/reject sends correct payload
    - **Spec ref**: REQ-7

- [x] 5.9 Create `ToolApprovalCard.spec.ts` unit tests
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

### 5C: Session Persistence and Health

- [x] 5.10 Add session persistence to chat app
    - Store conversation history in localStorage or sessionStorage
    - Restore conversation on page reload
    - **Acceptance**: Chat history survives page refresh
    - **Spec ref**: REQ-6 (implicit — chat enhancement)

- [x] 5.11 Create `HealthIndicator.vue` component
    - Simple ping to gateway root or health endpoint
    - Show connected/disconnected status in chat UI
    - **Acceptance**: Indicator shows green when gateway is reachable, red when not
    - **Spec ref**: REQ-4 (chat surface)

- [x] 5.12 Create `HealthIndicator.spec.ts` unit tests
    - **Acceptance**: Tests pass
    - **Spec ref**: REQ-10

## Summary

| Phase     | Tasks  | New files                    | Modified files                                   | New endpoints |
|-----------|--------|------------------------------|--------------------------------------------------|---------------|
| 1         | 22     | ~16 (8 components + 8 tests) | 4 (types, composables, App.vue, configPayload)   | 0             |
| 2         | 7      | ~4 (2 components + 2 tests)  | 2 (gateway/admin.rs, gateway/mod.rs)             | 1             |
| 3         | 13     | ~4 (2 components + 2 tests)  | 4 (SecuritySettings, SchedulerSettings, gateway) | 2             |
| 4         | 5      | TBD                          | TBD                                              | TBD           |
| 5         | 12     | ~6 (3 components + 3 tests)  | 3 (useChat, ChatMessage, gateway)                | 2             |
| **Total** | **59** | **~30**                      | **~13**                                          | **5**         |
