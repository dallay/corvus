# Web Operational Parity Specification

**Domain**: web / dashboard / chat
**Status**: implemented
**Issue**: DALLAY-181 / GitHub #276
**Date**: 2026-03-28
**Parent**: DALLAY-176
**Archived from**: `openspec/changes/archive/2026-03-28-web-operational-parity/`

## Overview

This specification defines the contract for the Corvus dashboard and chat web clients to surface
runtime admin capabilities that were previously only accessible via CLI. It covers gateway endpoint
contracts, dashboard section requirements, chat enhancements, and security boundaries.

## Definitions

- **Dashboard**: The operator-facing web app at `clients/web/apps/dashboard/`. Requires pairing
  auth. Surfaces configuration and operational views.
- **Chat app**: The end-user-facing web app at `clients/web/apps/chat/`. Requires pairing auth.
  Surfaces conversation UI only.
- **Gateway admin API**: HTTP endpoints under `/web/admin/` in the agent-runtime gateway. All
  require valid pairing token.
- **Config section**: A Vue component in the dashboard that displays and optionally edits a group
  of related configuration fields.
- **Operational view**: A dashboard component that displays runtime state (health, tasks, channels,
  costs, MCP, tunnels, reliability, heartbeat) without direct edit capability. Wired directly in
  `App.vue` rather than through `ConfigSection` routing.
- **Redacted view**: A serialized config view that replaces secret values with boolean indicators
  (e.g., `has_api_key: true` instead of exposing the key).

## Requirements

### REQ-1: Dashboard Config Section Expansion

The dashboard MUST add config section components for `AdminConfigView` fields that lack UI
representation. The following sections are implemented:

- **Updates** — Surfaces update status (current version, latest version, update availability,
  last check outcome, install method) from the `updates` field of `AdminConfigView`.
  Component: `UpdateSettings.vue`.
- **Web search** — Surfaces enabled toggle, provider selection, max results, timeout, and
  Brave API key indicator from the `web_search` field. Component: `WebSearchSettings.vue`.
- **Browser** — Surfaces computer-use API key indicator from the `browser` field.
  Component: `BrowserSettings.vue`.
- **Composio** — Surfaces enabled toggle, entity ID, and API key indicator from the `composio`
  field. Component: `ComposioSettings.vue`.
- **Memory** — Surfaces memory backend selection and Cerebro config (endpoint, auth token
  indicator, timeout, insecure loopback toggle) from the `memory` field.
  Component: `MemorySettings.vue`.
- **Identity** — Identity fields (format selection, AIEOS path, inline indicator) are surfaced
  within `SecuritySettings.vue` rather than as a separate component.

Each section follows the existing dashboard component pattern: Vue SFC with props from
`useConfig`, emit updates through `configPayload`, and includes unit tests.

#### Known gap: Provider Pools UI

Provider pool management (`ProviderPoolsSettings.vue`) is NOT implemented. The `ConfigSection`
type includes `"provider-pools"` and TypeScript types exist (`AdminProviderPoolsView`), but no
UI component renders for it. The `GET/PUT /web/admin/provider-pools` endpoints exist on the
gateway side. This is tracked for follow-up work.

#### Scenario: Operator views update status

- GIVEN an operator is authenticated via pairing
- WHEN they navigate to the updates section
- THEN they see the current version, latest available version, and whether an update is available
- AND they see the last check timestamp and outcome
- AND they see the effective install method and its source

#### Scenario: Operator edits web search config

- GIVEN an operator is authenticated via pairing
- WHEN they toggle web search enabled and change max results
- AND they submit the form
- THEN the dashboard sends a PUT to `/web/admin/config` with `web_search` patch
- AND the runtime applies the changes
- AND the dashboard reflects the updated values on reload

### REQ-2: Channel Visibility

The gateway exposes a `GET /web/admin/channels` endpoint that returns the status of all
configured channels.

The response includes for each channel:

- Channel type (telegram, discord, whatsapp, slack, webhook, cli)
- Configured status (boolean)
- Configuration summary (redacted — no tokens or secrets)

Channel health uses a best-effort model (ADR-5): `configured: boolean` plus `config_summary`
rather than real-time connectivity probing.

The dashboard displays a channels overview section (`ChannelsOverview.vue`) showing all channels
with their status. This component is wired directly in `App.vue` as an operational view rather
than through `ConfigSection` routing.

#### Scenario: Operator views channel overview

- GIVEN an operator is authenticated via pairing
- AND the runtime has Telegram enabled and Discord disabled
- WHEN they navigate to the channels section
- THEN they see Telegram with its configuration summary
- AND they see Discord with its status
- AND no channel secrets (bot tokens, API keys) are visible

### REQ-3: Autonomy Policy Visibility

The dashboard surfaces the full autonomy policy in `SecuritySettings.vue`, including:

- Auto-approve command list (`auto_approve`)
- Always-ask command list (`always_ask`)
- `require_approval_for_medium_risk` toggle
- `block_high_risk_commands` toggle

These fields are part of `AdminAutonomyView` and are displayed alongside identity fields in the
security section.

#### Scenario: Operator views autonomy policy details

- GIVEN an operator is authenticated via pairing
- WHEN they navigate to the security section
- THEN they see the current autonomy level, workspace_only, and cost/action limits (existing)
- AND they see the auto_approve command list
- AND they see the always_ask command list
- AND they see the medium-risk approval and high-risk blocking toggles

### REQ-4: Health Dashboard

The gateway exposes a `GET /web/admin/health` endpoint that returns an aggregate health view:

- Runtime uptime
- Provider connectivity
- Channel health summary
- Memory backend status
- Scheduler status (running, task count)
- Gateway status (listening, paired tokens count)

The dashboard displays a health overview (`HealthDashboard.vue`) wired directly in `App.vue`.
Components are color-coded by status (healthy/degraded/unhealthy).

#### Scenario: Operator views system health

- GIVEN an operator is authenticated via pairing
- WHEN they navigate to the health dashboard
- THEN they see an aggregate health status (healthy, degraded, unhealthy)
- AND they see individual component statuses
- AND degraded or unhealthy components are visually highlighted

### REQ-5: Scheduled Task Visibility

The gateway exposes a `GET /web/admin/tasks` endpoint that returns the list of scheduled
(cron) tasks, including:

- Task name/ID
- Schedule (cron expression or interval)
- Last run timestamp and outcome
- Next scheduled run
- Enabled/disabled status

The dashboard displays a task list in the scheduler section (`SchedulerStatus.vue`) alongside
the existing scheduler config (`SchedulerSettings.vue`).

#### Scenario: Operator views scheduled tasks

- GIVEN an operator is authenticated via pairing
- AND the scheduler has 3 configured tasks
- WHEN they navigate to the scheduler section
- THEN they see the scheduler settings (existing)
- AND they see a list of 3 tasks with name, schedule, last run, and next run

### REQ-6: Chat Streaming Support

The chat app supports streaming responses from the runtime via Server-Sent Events (SSE).

The gateway exposes a streaming chat endpoint (`POST /web/chat/stream`) that:

- Accepts a message payload
- Returns an SSE stream with text chunks as they are generated
- Sends a final event when generation completes
- Handles errors with an error event type

The chat app (`useChat.ts`) renders streamed text incrementally as chunks arrive. Session
persistence stores conversation history in localStorage for reload survival.

#### Scenario: User sends message and receives streaming response

- GIVEN a user is paired with the runtime
- WHEN they send a chat message
- THEN the chat app opens an SSE connection to the streaming endpoint
- AND text chunks appear incrementally in the chat UI
- AND the message is marked as complete when the final event arrives

#### Scenario: Streaming error is handled gracefully

- GIVEN a user is paired with the runtime
- WHEN they send a chat message and the provider returns an error mid-stream
- THEN the chat app receives an error SSE event
- AND the partial response is preserved with an error indicator
- AND the user can retry the message

### REQ-7: Tool Approval UI

The chat app supports a tool approval flow when the runtime requests human confirmation for
tool execution.

Tool approval is delivered via SSE `tool_approval` events during streaming (push model, not
polling). When the runtime needs approval, it sends a `tool_approval` SSE event containing
tool name, parameters (redacted if sensitive), risk level, and approval_id.

The approval UI (`ToolApprovalCard.vue`) shows:

- Tool name
- Tool parameters (redacted if sensitive)
- Risk level (low, medium, high)
- Approve and Reject buttons

The user's decision is sent back via `POST /web/chat/tool-approval`.

#### Scenario: User approves a tool execution

- GIVEN a user is in an active chat session
- WHEN the runtime requests approval for a shell command with medium risk
- THEN the chat app shows a tool approval card with the command and risk level
- AND the user clicks "Approve"
- THEN the approval is sent to the runtime
- AND the tool executes and the result appears in the chat

#### Scenario: User rejects a tool execution

- GIVEN a user is in an active chat session
- WHEN the runtime requests approval for a file deletion with high risk
- THEN the chat app shows a tool approval card
- AND the user clicks "Reject"
- THEN the rejection is sent to the runtime
- AND the assistant acknowledges the rejection and suggests alternatives

### REQ-8: Security Boundaries

All gateway endpoints:

- Require valid pairing token authentication (consistent with existing endpoints)
- Redact all secret values (API keys, tokens, passwords) — expose only `has_*` boolean indicators
- Return `401 Unauthorized` for missing or invalid pairing tokens
- Return `403 Forbidden` if the pairing token lacks admin scope (future-proofing)

Admin endpoints (`/web/admin/*`) are not accessible from the chat app. The chat app only
uses `/web/chat/*` and `/web/pair/*` endpoints.

No endpoint logs or includes secret values in response bodies, error messages, or
observability events.

### REQ-9: Dashboard Type Safety

All dashboard components have corresponding TypeScript types in `types/admin-config.ts` that
match the gateway response contracts.

The `ConfigSection` type is extended to include new sections (18 variants total).
`AdminConfigForm` and `AdminConfigSnapshot` types are extended for new editable sections.

Type definitions match the Rust `serde::Serialize` output of the corresponding `Admin*View`
structs. `tsc --noEmit` passes with zero errors.

### REQ-10: Test Coverage

All dashboard components have unit tests following the existing patterns in
`components/config/*.spec.ts`.

**Implemented coverage**:
- Dashboard: 78 tests across 23 test files (all pass)
- Chat: 63 tests across 6 test files (all pass)
- Total: 141 tests, 0 failures

**Known gaps**:
- `ProviderPoolsSettings.spec.ts` — does not exist (component not built)
- Some spec files have only 1 test (e.g., `SecuritySettings.spec.ts`, `BrowserSettings.spec.ts`)
- Coverage threshold (60%) configured but not measured in verification run
- Rust gateway integration tests verified structurally (`cargo check`) but not behaviorally

### REQ-11: Operational Overview Views (Phase 4)

The dashboard includes read-only operational overview components for runtime state beyond
config sections:

- **CostOverview.vue** — Cost tracking and budget status
- **McpOverview.vue** — MCP server configuration and status
- **TunnelOverview.vue** — Tunnel configuration and connectivity
- **ReliabilityOverview.vue** — Reliability metrics and circuit breaker status
- **HeartbeatOverview.vue** — Heartbeat monitoring and uptime

Each has a matching `.spec.ts` test file. These are wired directly in `App.vue` as operational
views.

### REQ-12: Chat Session Persistence and Health

- **Session persistence**: Conversation history stored in localStorage, survives page reload.
- **Health indicator** (`HealthIndicator.vue`): Simple ping to gateway, shows
  connected/disconnected status in chat UI.

## Gateway Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/web/admin/channels` | Pairing | Channel status list |
| GET | `/web/admin/health` | Pairing | Aggregate health view |
| GET | `/web/admin/tasks` | Pairing | Scheduled task list |
| POST | `/web/chat/stream` | Pairing | SSE streaming chat |
| POST | `/web/chat/tool-approval` | Pairing | Tool approval decision |
