# Web Operational Parity Specification

**Domain**: web / dashboard / chat
**Status**: draft
**Issue**: DALLAY-181 / GitHub #276
**Date**: 2026-03-28
**Parent**: DALLAY-176

## Overview

This specification defines the contract for expanding the Corvus dashboard and chat web clients to
surface runtime admin capabilities that are currently only accessible via CLI. It covers gateway
endpoint contracts, dashboard section requirements, chat enhancements, and security boundaries.

## Definitions

- **Dashboard**: The operator-facing web app at `clients/web/apps/dashboard/`. Requires pairing
  auth. Surfaces configuration and operational views.
- **Chat app**: The end-user-facing web app at `clients/web/apps/chat/`. Requires pairing auth.
  Surfaces conversation UI only.
- **Gateway admin API**: HTTP endpoints under `/web/admin/` in the agent-runtime gateway. All
  require valid pairing token.
- **Config section**: A Vue component in the dashboard that displays and optionally edits a group
  of related configuration fields.
- **Operational view**: A dashboard component that displays runtime state (health, tasks, channels)
  without direct edit capability.
- **Redacted view**: A serialized config view that replaces secret values with boolean indicators
  (e.g., `has_api_key: true` instead of exposing the key).

## Requirements

### REQ-1: Dashboard Config Section Expansion

The dashboard MUST add config section components for all `AdminConfigView` fields that currently
lack UI representation. Specifically:

- **Provider pools** — MUST surface account pool management (add, edit, remove accounts per pool,
  strategy selection) using the existing `GET/PUT /web/admin/provider-pools` endpoints.
- **Updates** — MUST surface update status (current version, latest version, update availability,
  last check outcome, install method) from the `updates` field of `AdminConfigView`.
- **Web search** — MUST surface enabled toggle, provider selection, max results, timeout, and
  Brave API key indicator from the `web_search` field.
- **Browser** — MUST surface computer-use API key indicator from the `browser` field.
- **Composio** — MUST surface enabled toggle, entity ID, and API key indicator from the `composio`
  field.
- **Memory** — MUST surface memory backend selection and Cerebro config (endpoint, auth token
  indicator, timeout, insecure loopback toggle) from the `memory` field.
- **Identity** — MUST surface format selection, AIEOS path, and inline indicator from the
  `identity` field.

Each new section MUST follow the existing dashboard component pattern: Vue SFC with props from
`useConfig`, emit updates through `configPayload`, and include unit tests.

#### Scenario: Operator views provider account pools

- GIVEN an operator is authenticated via pairing
- WHEN they navigate to the provider pools section
- THEN they see all configured account pools grouped by provider
- AND each pool shows its strategy (round_robin or weighted_round_robin)
- AND each account shows id, api_url, weight, enabled status, and API key indicator
- AND they can add, edit, or remove accounts within a pool

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

The gateway MUST expose a `GET /web/admin/channels` endpoint that returns the status of all
configured channels.

The response MUST include for each channel:

- Channel type (telegram, discord, whatsapp, slack, webhook, cli)
- Enabled/disabled status
- Configuration summary (redacted — no tokens or secrets)
- Health indicator (connected, disconnected, degraded, not_configured)

The dashboard MUST display a channels overview section showing all channels with their status.

#### Scenario: Operator views channel overview

- GIVEN an operator is authenticated via pairing
- AND the runtime has Telegram enabled and Discord disabled
- WHEN they navigate to the channels section
- THEN they see Telegram with status "connected" or "disconnected"
- AND they see Discord with status "not_configured" or "disabled"
- AND they see webhook channel with its current configuration
- AND no channel secrets (bot tokens, API keys) are visible

#### Scenario: Channel health reflects runtime state

- GIVEN a Telegram channel is configured and enabled
- WHEN the bot connection is healthy
- THEN the channels endpoint returns `health: "connected"` for Telegram
- WHEN the bot connection fails
- THEN the channels endpoint returns `health: "disconnected"` for Telegram

### REQ-3: Autonomy Policy Visibility

The dashboard MUST surface the full autonomy policy in the security section, including:

- Auto-approve command list (`auto_approve`)
- Always-ask command list (`always_ask`)
- `require_approval_for_medium_risk` toggle
- `block_high_risk_commands` toggle

These fields already exist in `AdminAutonomyView` but are not shown in `SecuritySettings.vue`.

The dashboard SHOULD allow editing these lists (add/remove entries) through the config update
endpoint.

#### Scenario: Operator views autonomy policy details

- GIVEN an operator is authenticated via pairing
- WHEN they navigate to the security section
- THEN they see the current autonomy level, workspace_only, and cost/action limits (existing)
- AND they see the auto_approve command list
- AND they see the always_ask command list
- AND they see the medium-risk approval and high-risk blocking toggles

### REQ-4: Health Dashboard

The gateway MUST expose a `GET /web/admin/health` endpoint that returns an aggregate health view:

- Runtime uptime
- Provider connectivity (can reach configured provider API)
- Channel health summary (from REQ-2 data)
- Memory backend status
- Scheduler status (running, task count)
- Gateway status (listening, paired tokens count)

The dashboard SHOULD display a health overview as the landing page or a prominent section.

#### Scenario: Operator views system health

- GIVEN an operator is authenticated via pairing
- WHEN they navigate to the health dashboard
- THEN they see an aggregate health status (healthy, degraded, unhealthy)
- AND they see individual component statuses
- AND degraded or unhealthy components are visually highlighted

### REQ-5: Scheduled Task Visibility

The gateway MUST expose a `GET /web/admin/tasks` endpoint that returns the list of scheduled
(cron) tasks, including:

- Task name/ID
- Schedule (cron expression or interval)
- Last run timestamp and outcome
- Next scheduled run
- Enabled/disabled status

The dashboard MUST display a task list in the scheduler section alongside the existing config.

#### Scenario: Operator views scheduled tasks

- GIVEN an operator is authenticated via pairing
- AND the scheduler has 3 configured tasks
- WHEN they navigate to the scheduler section
- THEN they see the scheduler settings (existing)
- AND they see a list of 3 tasks with name, schedule, last run, and next run

### REQ-6: Chat Streaming Support

The chat app MUST support streaming responses from the runtime via Server-Sent Events (SSE).

The gateway MUST expose a streaming chat endpoint (e.g., `POST /web/chat/stream`) that:

- Accepts a message payload
- Returns an SSE stream with text chunks as they are generated
- Sends a final event when generation completes
- Handles errors with an error event type

The chat app MUST render streamed text incrementally as chunks arrive.

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

The chat app MUST support a tool approval flow when the runtime requests human confirmation for
tool execution.

The approval UI MUST show:

- Tool name
- Tool parameters (redacted if sensitive)
- Risk level (low, medium, high)
- Approve and Reject buttons

The chat app MUST send the approval decision back to the runtime.

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

All new gateway endpoints MUST:

- Require valid pairing token authentication (consistent with existing endpoints)
- Redact all secret values (API keys, tokens, passwords) — expose only boolean indicators
- Return `401 Unauthorized` for missing or invalid pairing tokens
- Return `403 Forbidden` if the pairing token lacks admin scope (future-proofing)

Admin endpoints (`/web/admin/*`) MUST NOT be accessible from the chat app. The chat app MUST only
use `/web/chat/*` and `/web/pair/*` endpoints.

No endpoint SHALL log or include secret values in response bodies, error messages, or
observability events.

#### Scenario: Unauthenticated request to admin endpoint

- GIVEN a request to `GET /web/admin/channels` without a pairing token
- THEN the gateway returns `401 Unauthorized`
- AND no config data is included in the response

#### Scenario: Admin endpoint redacts secrets

- GIVEN a valid authenticated request to `GET /web/admin/config`
- WHEN the runtime has a Brave API key configured for web search
- THEN the response includes `web_search.has_brave_api_key: true`
- AND the response does NOT include the actual API key value

### REQ-9: Dashboard Type Safety

All new dashboard components MUST have corresponding TypeScript types in
`types/admin-config.ts` that match the gateway response contracts.

The `ConfigSection` type MUST be extended to include all new sections.

The `AdminConfigForm` and `AdminConfigSnapshot` types MUST be extended to include form fields
for all new editable sections.

Type definitions MUST match the Rust `serde::Serialize` output of the corresponding
`Admin*View` structs.

#### Scenario: TypeScript types match gateway contract

- GIVEN the gateway returns an `AdminConfigView` with `web_search.enabled: true`
- WHEN the dashboard deserializes the response
- THEN the `AdminConfigView.web_search.enabled` field is typed as `boolean`
- AND no type assertion or `any` cast is needed

### REQ-10: Test Coverage

All new dashboard components MUST have unit tests following the existing patterns in
`components/config/*.spec.ts`.

Each test file MUST cover:

- Component renders with default/empty config
- Component renders with populated config
- Form submission produces correct payload
- Secret fields use `SecretUpdate` semantics where applicable

All new gateway endpoints MUST have integration tests in the existing gateway test suite.

#### Scenario: New component has matching test file

- GIVEN a new `ProviderPoolsSettings.vue` component is created
- THEN a `ProviderPoolsSettings.spec.ts` test file exists in the same directory
- AND the tests cover rendering, form submission, and edge cases
