# Exploration: Web Operational Parity

**Change**: 2026-03-28-web-operational-parity
**Issue**: DALLAY-181 / GitHub #276
**Date**: 2026-03-28
**Parent**: DALLAY-176
**Branch**: feature/dallay-181-expand-dashboard-and-web-operational-clients-to-match

## Current State

### Runtime Capabilities (~60 admin/operator functions)

The Corvus agent-runtime (`clients/agent-runtime/`) exposes extensive operator capabilities through
CLI commands, config schema, and the gateway admin API. The gateway currently serves 5 admin
endpoints:

- `GET /web/admin/config` — redacted config view (`AdminConfigView` with ~15 top-level sections)
- `PUT /web/admin/config` — update config (`AdminConfigUpdateRequest` with matching patch structs)
- `GET /web/admin/options` — constrained enums/defaults for dashboard forms
- `GET /web/admin/provider-pools` — provider account pool management
- `PUT /web/admin/provider-pools` — update provider account pools

The `AdminConfigView` struct in `gateway/admin.rs` already serializes: default_provider,
default_model, api_url, default_temperature, memory_backend, provider (has_api_key), observability,
runtime, autonomy (with full policy details), identity, scheduler, gateway (with rate limiting,
idempotency, paired tokens count), channels (CLI + webhook), composio, web_search, memory (cerebro),
browser, and updates (with full status including version info).

### Dashboard Coverage (~10 config sections)

The dashboard (`clients/web/apps/dashboard/`) surfaces 7 config section components:

1. `GeneralSettings.vue` — provider, model, temperature, memory backend
2. `SecuritySettings.vue` — autonomy level, workspace_only, action/cost limits
3. `ObservabilitySettings.vue` — backend, OTEL endpoint/service name
4. `RuntimeSettings.vue` — runtime kind
5. `SchedulerSettings.vue` — enabled, max_tasks, max_concurrent
6. `GatewaySettings.vue` — port, host, pairing, public bind, rate limits
7. `WebhookSettings.vue` — enabled, port, secret

The `AdminConfigForm` type in `types/admin-config.ts` maps these to form fields. The
`AdminConfigUpdateRequest` type handles PATCH semantics with `SecretUpdate` for sensitive fields.

**Not surfaced in dashboard**: provider account pools (type exists but no UI component), channels
beyond webhook, updates status, web search config, browser config, composio config, memory/cerebro
config, identity details, autonomy policy details (auto_approve, always_ask lists).

### Chat App Coverage (end-user only)

The chat app (`clients/web/apps/chat/`) surfaces:

- Pairing flow (`useGateway.ts`)
- Message send/receive (`useChat.ts`)
- Config panel (`ConfigPanel.vue`) — minimal settings

**Boundary**: The chat app is strictly end-user facing. No admin surfaces should be added to it.

### Gap Analysis

The runtime exposes ~60 admin/operator capabilities. The dashboard surfaces ~10 config sections.
The chat app surfaces only pairing + message send/receive.

## Tier Classification

### Tier 1: Must Surface in Dashboard (P0–P2)

| Capability                        | Status                 | Gap                                                    |
|-----------------------------------|------------------------|--------------------------------------------------------|
| Channel management (all channels) | Partial (webhook only) | Need visibility for Telegram, Discord, WhatsApp, Slack |
| Provider account pools            | Types exist, no UI     | Wire existing `AdminProviderPoolsView` to a component  |
| Health dashboard                  | Not surfaced           | Need aggregate health endpoint + dashboard page        |
| Auth profiles / autonomy details  | Partial                | auto_approve, always_ask lists not editable            |
| Update management                 | Types exist, not shown | Wire `AdminUpdatesView` to status component            |
| Cron/scheduler tasks              | Partial (config only)  | Need task list view, not just settings                 |
| Skills inventory                  | Not surfaced           | Need new endpoint + UI                                 |
| Observability metrics             | Partial (config only)  | Need live metrics/logs view                            |
| MCP config                        | Not surfaced           | Need new endpoint + UI                                 |
| Tunnel config                     | Not surfaced           | Need new endpoint + UI                                 |

### Tier 2: Should Surface (P2)

| Capability                | Gap                                     |
|---------------------------|-----------------------------------------|
| Cost tracking             | New endpoint + UI                       |
| Reliability config        | New endpoint + UI                       |
| Agent profiles            | New endpoint + UI                       |
| Mission settings          | New endpoint + UI                       |
| Browser/web search config | Wire existing views to components       |
| Multimodal config         | Wire existing views to components       |
| Secrets indicator         | Aggregate view of which secrets are set |
| Daemon status             | New endpoint + UI                       |
| Model catalog             | New endpoint + UI                       |

### Tier 3: CLI-Only (No web surface needed)

Onboarding, interactive sessions, OS service management, hardware/peripheral, migration, OAuth
flows, sandbox config, Telegram binding, binary updates.

## Endpoint Gap

14 new gateway endpoints needed beyond the existing 5:

1. `GET /web/admin/channels` — all channel statuses
2. `GET /web/admin/health` — aggregate health
3. `GET /web/admin/skills` — skills inventory
4. `GET /web/admin/mcp` — MCP server configs
5. `PUT /web/admin/mcp` — update MCP config
6. `GET /web/admin/tunnels` — tunnel status
7. `PUT /web/admin/tunnels` — update tunnel config
8. `GET /web/admin/metrics` — observability metrics snapshot
9. `GET /web/admin/tasks` — scheduled task list
10. `GET /web/admin/models` — model catalog
11. `GET /web/admin/cost` — cost tracking summary
12. `GET /web/admin/daemon` — daemon process status
13. `GET /web/admin/updates/check` — trigger update check
14. `POST /web/admin/updates/install` — trigger update install

## Chat Enhancements (Parallel Track)

End-user enhancements for the chat app (no admin surfaces):

- Streaming responses (SSE)
- Tool approval UI
- Session persistence
- Health indicator (simple runtime reachability)
- File upload support

## Recommended Implementation Phases

1. **Wire already-built** — Provider pools UI, health dashboard, update status (use existing types)
2. **Channel visibility** — New channel status endpoints + dashboard UI
3. **Operational visibility** — Auth profiles, cron task list, skills inventory
4. **Extended config** — MCP, tunnels, cost, reliability sections
5. **Chat enhancements** — Streaming, tool approval (parallel with phases 2-4)

## Follow-up Issues Identified

15 implementation issues to be created under DALLAY-181, one per dashboard section or chat
enhancement. These will be tracked as sub-issues of #276.
