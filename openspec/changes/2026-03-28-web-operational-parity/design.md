# Design: Web Operational Parity

## Technical Approach

Expand the Corvus web clients to match the runtime's admin capabilities by:

1. Adding new Vue config section components in the dashboard for data already served by the gateway
2. Adding new gateway endpoints for runtime state not yet exposed via HTTP
3. Adding SSE streaming and tool approval flows in the chat app

The approach is deliberately incremental — each phase ships independently and is reversible. We
follow existing patterns (component structure, composable architecture, gateway handler patterns)
and avoid new abstractions.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Agent Runtime (Rust)                           │
│                                                                         │
│  Config     Channels    Scheduler    Provider    Memory    Security     │
│  Schema     (Tg/Dc/WA)  (cron)      (pools)    (cerebro)  (autonomy)  │
│    │            │           │            │          │          │         │
│    └────────────┴───────────┴────────────┴──────────┴──────────┘         │
│                              │                                           │
│                    ┌─────────┴─────────┐                                 │
│                    │  Gateway (axum)   │                                 │
│                    │                   │                                 │
│                    │  /web/admin/*     │◄── pairing auth required        │
│                    │  /web/chat/*      │◄── pairing auth required        │
│                    │  /web/pair/*      │◄── public (rate-limited)        │
│                    └─────────┬─────────┘                                 │
└──────────────────────────────┼──────────────────────────────────────────┘
                               │ HTTP
                 ┌─────────────┼─────────────┐
                 │                           │
        ┌────────┴────────┐         ┌────────┴────────┐
        │    Dashboard    │         │    Chat App     │
        │  (operator)     │         │  (end-user)     │
        │                 │         │                 │
        │  Config sections│         │  Conversation   │
        │  Operational    │         │  Streaming (SSE)│
        │  views          │         │  Tool approval  │
        │  Health dash    │         │  Health ping    │
        └─────────────────┘         └─────────────────┘
```

## Architecture Decisions

### ADR-1: Extend Existing Config Pattern, Don't Restructure

**Decision**: Add new config section components following the identical pattern of existing
components (props from `useConfig`, emit via `configPayload`, matching `.spec.ts` file). Do not
refactor the existing component architecture.

**Rationale**: The existing pattern works and is well-tested. The dashboard has 7 sections using
this exact pattern. Adding 7-8 more is straightforward. Restructuring (e.g., dynamic section
registry, tab-based navigation) adds risk without proportional benefit for this change.

**Consequence**: The `ConfigSection` type union grows from 7 to ~15 variants. The `App.vue` will
need conditional rendering for new sections. If section count exceeds ~20 in the future, a
navigation refactor may become warranted.

### ADR-2: New Endpoints for Operational Views, Config Reuse for Settings

**Decision**: Use the existing `GET/PUT /web/admin/config` endpoints for configuration data that
already exists in `AdminConfigView`. Add new dedicated endpoints only for operational state that
is not configuration (channel health, task list, aggregate health).

**Rationale**: The gateway already serializes all config fields in `AdminConfigView`. Adding
separate endpoints for web_search, browser, etc. would duplicate this data. New endpoints are only
needed for runtime state (health checks, task enumeration) that requires active computation or
aggregation.

**New endpoints**:

| Endpoint                  | Method | Purpose                      | Response type              |
|---------------------------|--------|------------------------------|----------------------------|
| `/web/admin/channels`     | GET    | Channel statuses with health | `AdminChannelStatusView[]` |
| `/web/admin/health`       | GET    | Aggregate system health      | `AdminHealthView`          |
| `/web/admin/tasks`        | GET    | Scheduled task list          | `AdminTaskListView`        |
| `/web/chat/stream`        | POST   | SSE streaming chat           | SSE event stream           |
| `/web/chat/tool-approval` | POST   | Tool approval response       | `ToolApprovalResponse`     |

**Existing endpoints reused** (no changes needed):

| Endpoint                        | Additional data surfaced                                 |
|---------------------------------|----------------------------------------------------------|
| `GET /web/admin/config`         | web_search, browser, composio, memory, identity, updates |
| `PUT /web/admin/config`         | web_search, browser, composio, memory, identity patches  |
| `GET /web/admin/provider-pools` | (already exists, needs UI only)                          |
| `PUT /web/admin/provider-pools` | (already exists, needs UI only)                          |

### ADR-3: SSE for Streaming, Not WebSocket

**Decision**: Use Server-Sent Events (SSE) for chat streaming, not WebSocket.

**Rationale**: SSE is simpler (unidirectional), works through HTTP proxies and CDNs, and axum has
built-in support via `axum::response::sse`. The chat flow is request-response (user sends message,
server streams reply) — bidirectional communication is not needed. Tool approval can use a separate
POST endpoint rather than multiplexing on a WebSocket.

**Consequence**: Each message sends a new HTTP request. This is acceptable for chat where messages
are infrequent relative to connection overhead. If real-time bidirectional features (collaborative
editing, live cursors) are needed later, WebSocket can be added independently.

### ADR-4: Tool Approval via Polling, Not Push

**Decision**: Implement tool approval using a polling model initially. The chat app polls
`GET /web/chat/pending-approval` periodically while a message is being processed. When an approval
is pending, the UI shows the approval card and sends the decision via
`POST /web/chat/tool-approval`.

**Rationale**: Push-based approval (via SSE or WebSocket) is more elegant but requires maintaining
a persistent connection and correlating approval requests with specific chat sessions. Polling is
simpler to implement, easier to debug, and sufficient for the expected interaction frequency
(operators are not approving tools every second).

**Consequence**: There is a small latency between the runtime requesting approval and the UI
showing the prompt (polling interval, e.g., 1-2 seconds). This is acceptable for a human-in-the-
loop flow. The polling endpoint can be replaced with SSE push in a follow-up if latency is
unacceptable.

### ADR-5: Channel Health is Best-Effort, Not Real-Time

**Decision**: The `GET /web/admin/channels` endpoint returns the last-known health status of each
channel, not a real-time probe.

**Rationale**: Real-time health probing (connecting to Telegram API, Discord gateway, etc.) on
every dashboard request would add latency and could trigger rate limits. Instead, channels maintain
their own health state internally (they already do for reconnection logic), and the endpoint
serializes that cached state.

**Consequence**: Health status may be slightly stale (up to the channel's internal health check
interval). This is acceptable for a dashboard view. A "refresh" button can trigger a one-time
health check if needed.

## Interface Contracts

### New Gateway Response Types (Rust)

```rust
// Channel status endpoint
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminChannelStatusView {
    pub channel_type: String,        // "telegram", "discord", "whatsapp", "slack", "webhook", "cli"
    pub enabled: bool,
    pub health: ChannelHealth,       // "connected", "disconnected", "degraded", "not_configured"
    pub config_summary: serde_json::Value, // redacted config (no secrets)
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum ChannelHealth {
    Connected,
    Disconnected,
    Degraded,
    NotConfigured,
}

// Aggregate health endpoint
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminHealthView {
    pub status: HealthStatus,        // "healthy", "degraded", "unhealthy"
    pub uptime_secs: u64,
    pub provider: ComponentHealth,
    pub channels: ComponentHealth,
    pub memory: ComponentHealth,
    pub scheduler: ComponentHealth,
    pub gateway: ComponentHealth,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

// Task list endpoint
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminTaskListView {
    pub tasks: Vec<AdminTaskView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminTaskView {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub enabled: bool,
    pub last_run_at: Option<u64>,
    pub last_run_outcome: Option<String>,
    pub next_run_at: Option<u64>,
}
```

### New Dashboard TypeScript Types

```typescript
// Channel status
export interface AdminChannelStatusView {
  channel_type: string;
  enabled: boolean;
  health: "Connected" | "Disconnected" | "Degraded" | "NotConfigured";
  config_summary: Record<string, unknown>;
}

// Aggregate health
export interface AdminHealthView {
  status: "Healthy" | "Degraded" | "Unhealthy";
  uptime_secs: number;
  provider: ComponentHealth;
  channels: ComponentHealth;
  memory: ComponentHealth;
  scheduler: ComponentHealth;
  gateway: ComponentHealth;
}

export interface ComponentHealth {
  status: "Healthy" | "Degraded" | "Unhealthy";
  detail: string;
}

// Task list
export interface AdminTaskListView {
  tasks: AdminTaskView[];
}

export interface AdminTaskView {
  id: string;
  name: string;
  schedule: string;
  enabled: boolean;
  last_run_at?: number | null;
  last_run_outcome?: string | null;
  next_run_at?: number | null;
}

// Extended ConfigSection
export type ConfigSection =
    | "general"
    | "security"
    | "observability"
    | "runtime"
    | "scheduler"
    | "gateway"
    | "webhook"
    | "provider-pools"
    | "updates"
    | "web-search"
    | "browser"
    | "composio"
    | "memory"
    | "identity"
    | "channels"
    | "health";
```

### SSE Streaming Contract

```
POST /web/chat/stream
Content-Type: application/json
Authorization: Bearer <pairing-token>

{ "message": "Hello, how are you?" }

Response: text/event-stream

event: chunk
data: {"text": "I'm "}

event: chunk
data: {"text": "doing well"}

event: tool_approval
data: {"tool": "shell", "params": {"command": "ls -la"}, "risk": "low", "approval_id": "uuid"}

event: done
data: {"message_id": "uuid", "total_tokens": 42}

event: error
data: {"code": "provider_error", "message": "Rate limit exceeded"}
```

### Tool Approval Contract

```
POST /web/chat/tool-approval
Content-Type: application/json
Authorization: Bearer <pairing-token>

{ "approval_id": "uuid", "approved": true }

Response: 200 OK
{ "acknowledged": true }
```

## Component Structure

### New Dashboard Components

```
clients/web/apps/dashboard/src/components/config/
├── GeneralSettings.vue          (existing)
├── SecuritySettings.vue         (existing — extended for autonomy policy)
├── ObservabilitySettings.vue    (existing)
├── RuntimeSettings.vue          (existing)
├── SchedulerSettings.vue        (existing — extended for task list)
├── GatewaySettings.vue          (existing)
├── WebhookSettings.vue          (existing)
├── ProviderPoolsSettings.vue    (new — Phase 1)
├── UpdateSettings.vue           (new — Phase 1)
├── WebSearchSettings.vue        (new — Phase 1)
├── BrowserSettings.vue          (new — Phase 1)
├── ComposioSettings.vue         (new — Phase 1)
├── MemorySettings.vue           (new — Phase 1)
├── IdentitySettings.vue         (new — Phase 1)
├── ChannelsOverview.vue         (new — Phase 2)
└── HealthDashboard.vue          (new — Phase 3)
```

### New Chat Components

```
clients/web/apps/chat/src/components/
├── chat/ChatMessage.vue         (existing — extended for streaming)
├── ToolApprovalCard.vue         (new — Phase 5)
└── HealthIndicator.vue          (new — Phase 5)
```

## Sequence Diagrams

### Dashboard Config Section Load

```
Operator          Dashboard App       Gateway           Runtime
   │                   │                  │                 │
   │  navigate to      │                  │                 │
   │  config section   │                  │                 │
   │──────────────────►│                  │                 │
   │                   │  GET /web/admin/ │                 │
   │                   │  config          │                 │
   │                   │─────────────────►│                 │
   │                   │                  │  read config    │
   │                   │                  │────────────────►│
   │                   │                  │◄────────────────│
   │                   │  AdminConfigView │                 │
   │                   │◄─────────────────│                 │
   │                   │                  │                 │
   │  render section   │                  │                 │
   │◄──────────────────│                  │                 │
   │                   │                  │                 │
   │  edit + submit    │                  │                 │
   │──────────────────►│                  │                 │
   │                   │  PUT /web/admin/ │                 │
   │                   │  config          │                 │
   │                   │─────────────────►│                 │
   │                   │                  │  apply patch    │
   │                   │                  │────────────────►│
   │                   │                  │◄────────────────│
   │                   │  updated view    │                 │
   │                   │◄─────────────────│                 │
   │  confirm          │                  │                 │
   │◄──────────────────│                  │                 │
```

### Chat Streaming with Tool Approval

```
User              Chat App          Gateway           Runtime        Provider
  │                  │                 │                  │               │
  │  send message    │                 │                  │               │
  │─────────────────►│                 │                  │               │
  │                  │  POST /web/     │                  │               │
  │                  │  chat/stream    │                  │               │
  │                  │────────────────►│                  │               │
  │                  │                 │  process msg     │               │
  │                  │                 │─────────────────►│               │
  │                  │                 │                  │  chat()       │
  │                  │                 │                  │──────────────►│
  │                  │  SSE: chunk     │                  │               │
  │  render chunk    │◄────────────────│◄─────────────────│◄──────────────│
  │◄─────────────────│                 │                  │               │
  │                  │  SSE: chunk     │                  │               │
  │  render chunk    │◄────────────────│◄─────────────────│◄──────────────│
  │◄─────────────────│                 │                  │               │
  │                  │                 │                  │               │
  │                  │  SSE: tool_     │  tool approval   │               │
  │  show approval   │  approval       │  needed          │               │
  │  card            │◄────────────────│◄─────────────────│               │
  │◄─────────────────│                 │                  │               │
  │                  │                 │                  │               │
  │  click approve   │                 │                  │               │
  │─────────────────►│                 │                  │               │
  │                  │  POST /web/chat │                  │               │
  │                  │  /tool-approval │                  │               │
  │                  │────────────────►│                  │               │
  │                  │                 │  resume with     │               │
  │                  │                 │  approval        │               │
  │                  │                 │─────────────────►│               │
  │                  │  SSE: chunk     │                  │               │
  │  render result   │◄────────────────│◄─────────────────│               │
  │◄─────────────────│                 │                  │               │
  │                  │  SSE: done      │                  │               │
  │  mark complete   │◄────────────────│                  │               │
  │◄─────────────────│                 │                  │               │
```

## Migration Notes

- No breaking changes to existing endpoints or components
- All new sections are additive to the dashboard
- The `AdminConfigView` type in TypeScript must be extended to include fields for web_search,
  browser, composio, memory, and identity that already exist in the Rust serialization but are
  not typed in the current TypeScript interface
- Existing `AdminConfigUpdateRequest` must be extended similarly for new editable sections
- The `AdminConfigForm` and `AdminConfigSnapshot` types grow but do not change existing fields
