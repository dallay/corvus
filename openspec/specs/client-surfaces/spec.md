---
doc_id: client-surfaces-capability-matrix
version: 1.3.0
created: 2026-03-21
status: active
owner: architecture
---

# Client Surfaces Capability Matrix

## Purpose

This specification establishes the canonical capability matrix for all Corvus client surfaces,
defining which surfaces serve end-users versus operators/administrators versus supporting roles.
It removes ambiguity about what capabilities each surface must, may, or must not expose, resolves
the boundary between chat surfaces, and defines the mandatory parity contract across mobile and web
platforms.

## Surface Registry

| Surface                              | Role                   | Transport  | Status               |
|--------------------------------------|------------------------|------------|----------------------|
| `clients/agent-runtime` (CLI)        | Operator/Admin         | Direct     | Complete             |
| `clients/web/apps/chat`              | End-user (Web)         | Gateway    | Scaffold (stub)      |
| `clients/web/apps/dashboard`         | Operator/Admin (Web)   | Gateway    | Complete             |
| `clients/composeApp` (mobile)        | End-user (Mobile)      | CLI Bridge | Scaffold (no bridge) |
| `clients/web/apps/docs`              | Supporting (Docs)      | None       | Complete             |
| `clients/web/apps/marketing`         | Supporting (Marketing) | None       | Partial              |
| `clients/composeApp` (shared module) | Supporting (Contracts) | Contracts  | Contracts only       |

## Capability Matrix

| Surface               | Chat      | Config    | Memory    | Tools     | Sessions  | Admin   | Transport  | i18n Tier  |
|-----------------------|-----------|-----------|-----------|-----------|-----------|---------|------------|------------|
| `agent-runtime` (CLI) | Yes       | Yes       | Yes       | Yes       | Yes       | Yes     | Direct     | Tier 3     |
| `web/apps/chat`       | **Yes**   | No        | Opt       | Opt       | Yes       | No      | Gateway    | **Tier 1** |
| `web/apps/dashboard`  | No        | Yes       | Yes       | Yes       | Yes       | **Yes** | Gateway    | **Tier 1** |
| `composeApp` (mobile) | **Yes**   | No        | Opt       | Opt       | Yes       | No      | CLI Bridge | **Tier 1** |
| `web/apps/docs`       | No        | No        | No        | No        | No        | No      | None       | Tier 2     |
| `web/apps/marketing`  | No        | No        | No        | No        | No        | No      | None       | Tier 3     |
| `composeApp` (shared) | Contracts | Contracts | Contracts | Contracts | Contracts | No      | Contracts  | Exempt     |

**Legend**: `Yes` = Mandatory, `Opt` = Optional, `No` = Out-of-scope, `Contracts` = Type definitions
only

## Transport Architecture

### Transport Per Surface

Each surface uses exactly one transport mechanism for all runtime communication:

| Surface                           | Transport      | Protocol                      |
|-----------------------------------|----------------|-------------------------------|
| Web clients (`chat`, `dashboard`) | HTTP Gateway   | REST/WebSocket over HTTPS     |
| Mobile (`composeApp`)             | RustCliBridge  | Process bridge (stdin/stdout) |
| CLI operators                     | Direct runtime | CLI subprocess                |
| Supporting surfaces               | None           | No runtime communication      |

**Rationale**: Transport choice is constrained by platform capability, not preference.

- Web clients use HTTP Gateway because browser sandboxing prevents process bridges.
- Mobile clients use CLI bridge because the gateway may not be network-accessible on embedded
  devices.
- CLI operators use direct runtime access for full capability access during development and server
  management.

Each surface MUST use exactly one transport for all runtime communication.

#### Scenario: Web surfaces use HTTP Gateway

- GIVEN a web client surface (`chat` or `dashboard`) is evaluated for transport
- WHEN it implements runtime communication
- THEN the surface MUST use HTTP Gateway endpoints
- AND it MUST NOT use process bridges or CLI invocation.

#### Scenario: Mobile surfaces use CLI bridge

- GIVEN a mobile client surface (`composeApp` for desktop/Android/iOS) is evaluated for transport
- WHEN it implements runtime communication
- THEN the surface MUST use the RustCliBridge (process bridge)
- AND it MUST NOT use HTTP Gateway as the primary transport.

### HTTP Gateway Endpoints

The gateway exposes a **client-safe subset** of runtime capabilities. Route groups below use the
implemented HTTP prefixes rather than shorthand aliases:

| Endpoint Group        | Capabilities                                                    | Access                    |
|-----------------------|-----------------------------------------------------------------|---------------------------|
| `/web/chat/*`         | Message sending, streaming                                      | All authenticated clients |
| `/session/*`          | Session lifecycle, history                                      | All authenticated clients |
| `/web/admin/*`        | Configuration, session monitoring, memory administration, audit | Dashboard only            |
| `/health`, `/metrics` | Health, observability                                           | Public                    |

### Runtime-Only Capabilities (Never Exposed)

The following capabilities are runtime-only and must never be exposed through any client surface:

| Capability                      | Reason                                      |
|---------------------------------|---------------------------------------------|
| Raw tool registry access        | Security: tool execution gated by policy    |
| Direct session database queries | Security: memory access gated by policy     |
| Configuration hot-reload        | Operations: explicit operator commands only |
| Runtime code injection          | Security: runtime integrity                 |
| Audit log modification          | Integrity: append-only                      |
| Credential vault access         | Security: runtime-internal                  |

Client surfaces MAY expose **results** of runtime-only capabilities but MUST NOT expose raw access.

## Requirements

### Requirement: Surface Role Classification

Every surface MUST be classified into exactly one role category.

#### Scenario: Surface has explicit role

- GIVEN a Corvus surface exists in the repository
- WHEN the surface is evaluated for capability decisions
- THEN the surface MUST be classified as end-user, operator/admin, or supporting
- AND the classification MUST match the canonical matrix in this spec.

#### Scenario: New surface addition

- GIVEN a new surface is introduced to the repository
- WHEN the surface is first committed
- THEN the surface MUST be classified in the canonical matrix
- AND the classification MUST be documented in a change proposal before implementation.

### Requirement: Transport Invariant

Each surface MUST use exactly one approved transport for all runtime communication, and its
onboarding
flow MUST validate readiness only through that approved transport.

For the composeApp client surfaces in this milestone:

- Desktop MUST be treated as a client-first surface.
- Android MUST be treated as a client-first surface.
- iOS MUST be treated as a client-first surface.
- Desktop and Android MUST NOT assume a locally installed `corvus` binary, packaged executable, or
  immediate local process execution as the default path.
- iOS MUST NOT imply that local `corvus` execution is the expected default path.
- Desktop MUST support connecting to an existing runtime through runtime URL or endpoint
  configuration.
- Android MUST support connecting to an existing runtime through runtime URL or endpoint
  configuration.
- iOS MUST expose only the approved client connection path or paths supported on iOS for this
  milestone, which MAY include runtime URL or endpoint configuration and MAY include pairing or a
  trusted companion flow.

#### Scenario: Desktop starts as a client instead of a local host

- GIVEN a desktop user opens composeApp with no saved ready connection
- WHEN startup is evaluated
- THEN the surface MUST enter onboarding, readiness, or configuration UX
- AND it MUST NOT immediately spawn, probe, or require a local `corvus` process as the default
  action.

#### Scenario: Android starts as a client instead of a packaged runtime host

- GIVEN an Android user opens composeApp with no saved ready connection
- WHEN startup is evaluated
- THEN the surface MUST enter onboarding, readiness, or configuration UX
- AND it MUST NOT assume a packaged executable, local binary, or immediate process launch is
  available.

#### Scenario: iOS shows only supported client connection paths

- GIVEN an iOS user opens composeApp for first-run setup
- WHEN the app presents connection options
- THEN the app MUST present only the iOS connection path or paths approved for this milestone
- AND it MUST NOT present local runtime execution as a default or required iOS path.

### Requirement: Onboarding Contract Alignment

The `client-surfaces` capability matrix MUST remain the transport and capability source of truth for
all surfaces, while onboarding behavior MUST align to the shared product onboarding specification.
Each onboarding-capable surface SHALL map its first-run flow to the canonical onboarding steps
without changing its approved transport.

#### Scenario: Web dashboard aligns onboarding without changing transport

- GIVEN `clients/web/apps/dashboard` participates in first-run onboarding
- WHEN its flow is evaluated against the canonical onboarding model
- THEN it MUST implement the shared onboarding outcomes using HTTP Gateway transport only
- AND it MUST NOT introduce process bridges or direct runtime access.

#### Scenario: Mobile aligns onboarding without adopting HTTP pairing language

- GIVEN `clients/composeApp` participates in first-run onboarding
- WHEN its flow is evaluated against the canonical onboarding model
- THEN it MUST implement the shared onboarding outcomes using the approved CLI bridge path
- AND it MUST NOT redefine mobile linking as HTTP gateway pairing.

### Requirement: Cross-Surface Recovery State Coverage

All onboarding-capable surfaces MUST expose the shared recovery taxonomy defined by the onboarding
specification and MUST map transport-specific failures into those normalized states.

#### Scenario: Web and mobile expose comparable recovery states

- GIVEN `clients/web/apps/chat` and `clients/composeApp` encounter different transport failures
- WHEN each surface renders recovery guidance
- THEN each surface MUST use the normalized product-level recovery state that matches the failure
- AND a user comparing surfaces MUST be able to recognize equivalent failure categories.

#### Scenario: Operator surfaces expose operator-relevant recovery states

- GIVEN `clients/agent-runtime` or `clients/web/apps/dashboard` encounters an onboarding blockage
- WHEN recovery guidance is rendered
- THEN the surface MUST use the normalized recovery taxonomy for applicable states
- AND it MAY omit chat-only states that cannot occur on that operator surface.

### Requirement: Capability Tier Enforcement

Each surface MUST implement only the capabilities assigned to it in the canonical matrix.

For this milestone, the required composeApp client capability set for desktop, Android, and iOS MUST
be limited to:

- startup routing into onboarding, readiness, and configuration UX,
- supported connection-path selection or configuration,
- display of the currently targeted runtime or endpoint,
- display of trust, auth, link, or pairing state as applicable to that platform,
- user-safe reachability and readiness checks,
- retry, edit, reset, disconnect, or re-pair actions appropriate to the active connection path,
- gating of chat or session entry until ready state is confirmed.

For this milestone, composeApp client surfaces MUST NOT be required to provide:

- runtime-backed chat-turn parity,
- session creation, resumption, or termination parity,
- tool approval handling,
- operator or admin capabilities,
- runtime configuration editing beyond client connection settings,
- memory browsing,
- multimodal input,
- notifications,
- offline mode,
- local runtime hosting as a milestone acceptance condition.

**Exception**: Runtime-level approval submission IS implemented via
`AndroidRuntimeBridge.submitApproval` and `MobileRuntimeCoordinator.submitApproval`, but
the milestone acceptance does NOT require the full UI/UX for approve/deny controls. The
approval flow is available at the runtime bridge layer; the UI is deferred.

#### Scenario: Client settings expose only readiness-critical controls in this milestone

- GIVEN a desktop, Android, or iOS user opens client settings during this milestone
- WHEN the available controls are inspected
- THEN the surface MUST provide only connection setup, readiness diagnostics, and recovery controls
- AND it MUST NOT fail milestone acceptance for omitting full chat, session, approval, admin, or
  memory features.

#### Scenario: Chat entry remains gated until readiness succeeds

- GIVEN a desktop, Android, or iOS user has not yet completed the required connection and readiness
  checks
- WHEN the user attempts to enter normal chat or session flow
- THEN the surface MUST keep that entry blocked
- AND it MUST direct the user to the unresolved onboarding, readiness, or configuration state.

### Requirement: Client-First Startup Routing

Desktop, Android, and iOS composeApp clients MUST route startup into onboarding, readiness, or
configuration UX before normal chat startup whenever a ready client connection is not already
available.

If a previously configured connection exists, startup MUST still land in a readiness-confirmed
client
state rather than silently starting a local runtime.

#### Scenario: First launch goes to onboarding instead of chat workspace

- GIVEN a user launches desktop, Android, or iOS composeApp for the first time
- WHEN no ready client connection has been established yet
- THEN startup MUST open onboarding, readiness, or configuration UX first
- AND the surface MUST NOT drop the user directly into a normal chat workspace backed by assumed
  local execution.

#### Scenario: Relaunch with saved configuration still stays client-first

- GIVEN a user relaunches desktop, Android, or iOS composeApp with a previously saved target runtime
  configuration
- WHEN startup validates the saved state
- THEN the surface MUST show readiness-confirmed client state or an actionable recovery state
- AND it MUST NOT silently spawn a local runtime as part of normal relaunch behavior.

### Requirement: Platform-Specific Connection Path Disclosure

Each composeApp client surface MUST disclose only the connection paths that are actually supported
on
that platform in this milestone.

- Desktop MUST disclose runtime URL or endpoint configuration as a supported path.
- Android MUST disclose runtime URL or endpoint configuration as a supported path.
- If desktop or Android also support pairing or a trusted companion flow in this milestone, the
  surface MUST guide that flow explicitly.
- iOS MUST disclose at least one approved client connection path for this milestone.
- If pairing or a trusted companion flow is the approved iOS path, iOS onboarding MUST guide that
  flow explicitly.
- A surface MUST NOT present unsupported connection paths as available, required, or coming from
  default local execution.

#### Scenario: Unsupported connection path is not shown as available

- GIVEN a platform does not support a pairing, trusted companion, or endpoint path in this milestone
- WHEN the connection setup UI is rendered on that platform
- THEN the unsupported path MUST be absent or clearly marked unavailable
- AND the user MUST NOT be told to complete setup through that unsupported path.

#### Scenario: Supported connection path includes platform-appropriate guidance

- GIVEN a platform supports runtime endpoint configuration or a pairing or trusted companion flow in
  this milestone
- WHEN the user starts connection setup
- THEN the surface MUST guide the user through that supported path
- AND the guidance MUST describe the path as connecting to an existing runtime rather than starting
  a
  local host by default.

### Requirement: Milestone Scope Exclusions

This milestone MUST stay limited to client-first onboarding, readiness, and connection configuration
for desktop, Android, and iOS.

The system MUST NOT treat the following as required for milestone completion:

- default local runtime execution on any composeApp client surface,
- mandatory local `corvus` installation guidance as the normal path,
- runtime-backed chat, session, or approval parity,
- dashboard or admin capabilities,
- raw memory visibility,
- multimodal features,
- notifications,
- offline mode,
- background automation beyond preserving client configuration needed to re-enter readiness UX.

#### Scenario: Milestone acceptance does not depend on full chat parity

- GIVEN desktop, Android, and iOS satisfy the client-first startup, connection setup, readiness, and
  recovery requirements
- WHEN the milestone is evaluated for acceptance
- THEN missing runtime-backed chat, session, approval, notification, offline, or admin features MUST
  NOT fail the milestone
- AND those capabilities MUST remain follow-on work until another change adds them.

### Requirement: Mobile-Web Parity

Mobile and web end-user chat surfaces MUST maintain parity on mandatory capabilities.

#### Scenario: Mandatory parity for chat composition

- GIVEN `clients/web/apps/chat` and `clients/composeApp`
- WHEN the surface implements chat composition
- THEN both surfaces MUST implement: text input, send/cancel, message submission
- AND neither surface MAY omit chat composition from its mandatory set.

#### Scenario: Mandatory parity for session lifecycle

- GIVEN `clients/web/apps/chat` and `clients/composeApp`
- WHEN the surface implements session management
- THEN both surfaces MUST implement: session creation, resumption, termination
- AND the session ID format MUST be UUID-based for cross-surface consistency.

#### Scenario: Mandatory parity for tool approval

- GIVEN `clients/web/apps/chat` and `clients/composeApp`
- WHEN the surface displays a tool call requiring approval
- THEN both surfaces MUST provide: approve and deny UI controls
- AND both MUST use the same approval semantics as the gateway.

**Milestone exception**: For composeApp surfaces (desktop/Android/iOS), the full tool approval
UI/UX is deferred for this milestone. Runtime-level approval submission is available via
`AndroidRuntimeBridge.submitApproval` and `MobileRuntimeCoordinator.submitApproval`, but the
UI controls are not required for milestone acceptance per the earlier exception in this
spec.

#### Scenario: Platform-specific capabilities differ

- GIVEN `clients/web/apps/chat` and `clients/composeApp`
- WHEN platform-specific capabilities are evaluated
- THEN push notifications MAY be implemented differently (browser API vs OS-native)
- AND background session handling is applicable to mobile only, not web.

### Requirement: Session ID Format

All surfaces MUST use UUID-based session identifiers.

#### Scenario: Session ID consistency across surfaces

- GIVEN any surface that manages sessions
- WHEN a session is created
- THEN the session ID MUST be a UUID v4 (e.g., `550e8400-e29b-41d4-a716-446655440000`)
- AND session IDs MUST NOT use integer counters or platform-specific formats.

**Rationale**: UUIDs provide collision resistance, work across distributed systems, and match the
existing gateway implementation.

### Requirement: CLI Bridge Output Modes

The RustCliBridge MUST support both text and structured JSON output.

#### Scenario: Text output mode (default)

- GIVEN a CLI bridge invocation without output specification
- WHEN the bridge communicates with the runtime
- THEN the output MUST be plain text (backward compatible with existing behavior)
- AND no structured parsing is required.

#### Scenario: JSON output mode

- GIVEN a CLI bridge invocation with `--output json`
- WHEN the bridge communicates with the runtime
- THEN the output MUST be structured JSON including: `session_id`, `message_type`, `content`,
  `tool_results`, `metadata`
- AND mobile clients MAY parse structured output for rich UI rendering.

### Requirement: Background Session Handling

Mobile surfaces MUST support background session handling via filesystem persistence.

#### Scenario: Background session resumption

- GIVEN a mobile user backgrounds the app during an active session
- WHEN the app resumes
- THEN the surface MUST persist the session ID to filesystem
- AND MUST query session state on resume without losing conversation context.

#### Scenario: Session timeout handling

- GIVEN a mobile session that has timed out
- WHEN the user attempts to resume
- THEN the surface MUST display an appropriate timeout message
- AND MUST offer to create a new session.

**Note**: Push notifications for background sessions require a companion service for delivery and
are out-of-scope for this spec.

### Requirement: Contract Layer Scope

The `modules/agent-core-kmp` module has a two-tier structure:

- `commonMain`: MUST contain only type definitions and bridge interfaces
- `jvmMain`/`iosMain`/`androidMain`: MAY contain platform-specific bridge implementations

#### Scenario: Common main contains no execution logic

- GIVEN `modules/agent-core-kmp/src/commonMain/`
- WHEN the module is examined
- THEN it MUST contain only: data models (`CoreInvocation`, `CoreOutput`, `CoreResult`), bridge
  interfaces (`AgentCoreBridge`, `CliBridgeSession`), module metadata (`AgentKernel`)
- AND it MUST NOT contain: UI components, state management, runtime execution logic.

#### Scenario: Platform targets contain bridge implementations

- GIVEN `modules/agent-core-kmp/src/jvmMain/`
- WHEN platform-specific implementations are needed (e.g., `RustCliBridge`)
- THEN the implementation MAY spawn processes and perform I/O as required by the bridge contract
- AND the implementation MUST NOT leak platform-specific types to `commonMain`

#### Scenario: ComposeApp UI types are separate

- GIVEN `clients/composeApp/src/commonMain/`
- WHEN UI types are defined (e.g., `ChatMessage`, `ChatUiState`)
- THEN those types MUST be in the composeApp UI layer, not in agent-core-kmp
- AND the agent-core-kmp contract layer remains platform-agnostic.

**Migration**: Current `RustCliBridge` implementation in `jvmMain` is compliant with this spec. No
changes required.

### Requirement: Dashboard Session List View (CS-1)

The dashboard MUST include a session monitoring page that displays a paginated table of sessions.

- The page MUST be accessible from the dashboard navigation.
- The table MUST display columns: Session ID, Started, Last Activity, Messages, Status (
  Active/Ended).
- The table MUST support filtering by status (`active`, `ended`, `all`).
- The table MUST support sorting by `started_at` and `last_activity`.
- The table MUST support pagination (page size selector, next/previous navigation).
- Active sessions SHOULD be visually distinguished from ended sessions (e.g., badge or row
  highlight).
- The view MUST consume `GET /web/admin/sessions` from the gateway.

#### Scenario: Dashboard displays session list

- GIVEN the admin user is authenticated in the dashboard
- AND 5 active and 3 ended sessions exist
- WHEN the admin navigates to the session monitoring page
- THEN a table MUST display 8 rows
- AND each row MUST show: session ID, started timestamp, last activity, message count, status
- AND active sessions MUST be visually distinct from ended sessions.

#### Scenario: Filter sessions by active status

- GIVEN the admin is on the session monitoring page
- AND 5 active and 3 ended sessions exist
- WHEN the admin selects the "Active" status filter
- THEN the table MUST display only the 5 active sessions
- AND ended sessions MUST NOT appear.

### Requirement: Dashboard Session Detail View (CS-2)

The dashboard MUST provide a session detail panel accessible by clicking a session row.

- The detail view MUST display: session ID, started_at, ended_at (or "Active"), message_count,
  last_activity, metadata (if present).
- The detail view MUST display a memory summary: count of memory entries by category for that
  session.
- The detail view SHOULD provide a link/button to view the session's memory entries in the memory
  browser (pre-filtered by session_id).
- The detail view MUST consume `GET /web/admin/sessions/:id` from the gateway.

#### Scenario: Admin views session detail

- GIVEN session "abc-123" is active with 15 messages and 6 memory entries (4 Conversation, 2 Core)
- WHEN the admin clicks session "abc-123" in the session list
- THEN the session detail panel MUST open
- AND it MUST show: id "abc-123", message count 15, status "Active"
- AND it MUST show memory summary: Conversation 4, Core 2.

### Requirement: Dashboard Memory Browser (CS-3)

The dashboard MUST include a memory administration page with a searchable, filterable list of memory
entries.

- The page MUST be accessible from the dashboard navigation.
- The page MUST display a table/list of memory entries with columns: Key, Category, Timestamp,
  Session ID, Content (truncated preview).
- The page MUST support filtering by category (Core, Daily, Conversation, Custom).
- The page MUST support filtering by session ID (dropdown or text input).
- The page MUST support full-text search via a search input field.
- The page MUST support pagination.
- Each entry MUST have a "Delete" action (with confirmation dialog).
- The page MUST consume `GET /web/admin/memory` and `DELETE /web/admin/memory/:key`.

#### Scenario: Admin deletes a memory entry

- GIVEN a memory entry with key "outdated-fact" exists in the browser
- WHEN the admin clicks "Delete" on that entry
- THEN a confirmation dialog MUST appear
- WHEN the admin confirms deletion
- THEN the entry MUST be removed from the list
- AND a DELETE request MUST be sent to /web/admin/memory/outdated-fact.

### Requirement: Dashboard Memory Stats Summary (CS-4)

The dashboard memory browser page MUST display a stats summary panel.

- The panel MUST show: total entry count, entries by category, total sessions, active sessions,
  backend name, Cerebro status.
- The panel MUST consume `GET /web/admin/memory/stats`.
- The panel SHOULD be displayed above or alongside the memory entry list.

#### Scenario: Memory stats panel displays correctly

- GIVEN 50 memory entries (20 Core, 15 Conversation, 10 Daily, 5 Custom)
- AND 8 total sessions, 3 active
- AND backend is "sqlite", Cerebro is not configured
- WHEN the admin views the memory browser page
- THEN the stats panel MUST show: 50 total entries
- AND the panel MUST show category breakdown
- AND the panel MUST show 8 total sessions, 3 active
- AND the panel MUST show backend "sqlite"
- AND the panel MUST show Cerebro as "Not configured".

### Requirement: Chat Session History Sidebar (CS-5)

The chat app MUST include a collapsible session history sidebar.

- The sidebar MUST list past sessions from `GET /session/list`.
- Each session entry MUST display: session start time (relative or absolute) and message count.
- The current active session MUST be visually highlighted.
- Clicking a past session MUST switch the chat to that session's context.
- The sidebar MUST include a "New Chat" action that creates a new session.
- The sidebar MUST be collapsible to preserve chat viewport space.
- The sidebar MUST NOT display memory contents, keys, or categories.

#### Scenario: Chat sidebar lists past sessions

- GIVEN the user is authenticated in the chat app
- AND the user has 4 past sessions and 1 current active session
- WHEN the chat app loads
- THEN the sidebar MUST display 5 session entries
- AND the current session MUST be visually highlighted
- AND each entry MUST show start time and message count.

#### Scenario: User switches to a past session

- GIVEN the sidebar shows sessions including "sess-old" with 12 messages
- WHEN the user clicks "sess-old" in the sidebar
- THEN the chat MUST load the context for session "sess-old"
- AND the X-Session-Id header MUST be set to "sess-old" for subsequent requests
- AND "sess-old" MUST become the highlighted session.

### Requirement: Chat Session Data Persistence (CS-6)

Chat session context MUST be persisted across page reloads.

- The current session ID MUST be stored in `sessionStorage` (existing behavior).
- The session list MUST be fetched from the server via `GET /session/list` on load.
- Messages for the current session MUST continue to use `sessionStorage` persistence (existing
  behavior).
- When switching sessions, the current session's messages MUST be saved to `sessionStorage` before
  loading the new session.

#### Scenario: Session list loads from server on mount

- GIVEN the user is authenticated
- AND the user has 3 past sessions on the server
- WHEN the chat app mounts
- THEN useChat MUST call GET /session/list
- AND the sidebar MUST populate with the 3 sessions plus the current session.

### Requirement: Chat No Memory Visibility (CS-7)

The chat app MUST NOT expose raw memory contents to end users.

- The chat MUST NOT display memory keys, categories, or raw content.
- The chat MAY display subtle "context used" indicators as a future enhancement — this is NOT
  required for Phase 1.
- The chat MUST NOT call any `/web/admin/memory*` endpoint.

#### Scenario: Chat does not expose memory data

- GIVEN session "abc-123" has 10 associated memory entries
- WHEN the user is chatting in session "abc-123"
- THEN no memory entries, keys, or categories MUST be visible in the chat UI
- AND no requests to /web/admin/memory endpoints MUST be made.

### Requirement: Dashboard Admin TypeScript Types (CS-8)

The dashboard MUST define TypeScript types for all new API responses.

- `AdminSessionView`: session list item (id, started_at, ended_at, message_count, last_activity).
- `AdminSessionDetail`: extends `AdminSessionView` with metadata and memory_summary.
- `AdminMemoryEntry`: memory entry (id, key, content, category, timestamp, session_id).
- `AdminMemoryStats`: stats response (total_entries, by_category, total_sessions, active_sessions,
  backend, cerebro_configured).
- Types MUST be defined in the existing `admin-config.ts` or a co-located types file.

#### Scenario: TypeScript types match API response shape

- GIVEN the dashboard makes a request to GET /web/admin/sessions
- WHEN the response is received
- THEN the response MUST be parseable as PaginatedResponse<AdminSessionView>
- AND all fields defined in AdminSessionView MUST be present.

### Requirement: KMP/Mobile Session Visibility Deferred (CS-9)

KMP and mobile clients (composeApp, androidApp) are OUT OF SCOPE for Phase 1.

- Session history and memory visibility for KMP clients MUST NOT be implemented in this change.
- The KMP `CoreContracts.kt` MAY be updated with session history type stubs if convenient, but this
  is NOT required.
- The mobile bridge is not wired — session history depends on bridge completion (tracked
  separately).

#### Scenario: KMP contracts remain unchanged

- GIVEN the KMP module CoreContracts.kt
- WHEN this change is implemented
- THEN CoreContracts.kt MUST NOT be modified unless adding optional type stubs
- AND existing KMP functionality MUST NOT be affected.

### Requirement: Visibility Rules (CS-10)

| Capability         | Dashboard (Admin) | Chat (End-User) | KMP/Mobile |
|--------------------|-------------------|-----------------|------------|
| Session list (all) | MUST              | -               | Deferred   |
| Session list (own) | -                 | MUST            | Deferred   |
| Session detail     | MUST              | -               | Deferred   |
| Memory browser     | MUST              | MUST NOT        | Deferred   |
| Memory stats       | MUST              | MUST NOT        | Deferred   |
| Memory delete      | MUST              | MUST NOT        | Deferred   |
| Memory search      | MUST              | MUST NOT        | Deferred   |
| Session switching  | -                 | MUST            | Deferred   |
| New chat / session | -                 | MUST            | Deferred   |

#### Scenario: Admin has full visibility

- GIVEN an admin user authenticated in the dashboard
- THEN the user MUST have access to: session list, session detail, memory browser, memory stats,
  memory delete.

#### Scenario: End-user has scoped visibility

- GIVEN an end user authenticated in the chat app
- THEN the user MUST have access to: own session list, session switching, new chat
- AND the user MUST NOT have access to: memory browser, memory stats, memory delete, all-sessions
  list.

## Matrix Immutability Rules

1. **Adding a new surface**: Requires a new change (proposal → spec → design → tasks)
2. **Changing a capability tier**: Requires a change proposal with justification
3. **Adding new capability columns**: Requires architectural review
4. **Exception process**: Security-critical changes can fast-track via signed approval from two
   maintainers

## Cross-Reference

- [Gateway API Specification](./gateway-api.md) (TBD) — HTTP Gateway endpoint definitions (see
  `clients/agent-runtime/src/gateway/mod.rs` for current implementation)
- [MCP Runtime Specification](../mcp-runtime/spec.md) — Tool registry and MCP contract
- [Agent Loop Specification](../agent-loop/spec.md) — Canonical loop behavior
- [Dashboard Specification](../dashboard/spec.md) — Admin surface contract
- [Cerebro Specification](../cerebro/spec.md) — Memory system
- [i18n Governance Specification](../i18n-governance/spec.md) — Locale tiers, parity rules, and
  terminology governance
- [Design Token Governance](../design-tokens/spec.md) — Cross-platform visual token naming
  conventions
- [Canonical Glossary](../../glossary/README.md) — Product terminology reference

## Change History

| Version | Date       | Changes                                                                                                                                                                                                                  |
|---------|------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1.3.0   | 2026-03-28 | Added session monitoring (CS-1, CS-2), memory browser (CS-3, CS-4), chat session sidebar (CS-5, CS-6, CS-7), admin types (CS-8), KMP deferred (CS-9), and visibility rules (CS-10) from session-memory-visibility change |
| 1.2.0   | 2026-03-24 | Added i18n Tier column to capability matrix; cross-references to i18n governance and design token specs                                                                                                                  |
| 1.1.0   | 2026-03-24 | Added onboarding alignment and recovery coverage requirements; clarified transport validation during onboarding                                                                                                          |
| 1.0.0   | 2026-03-21 | Initial specification — canonical matrix, transport rules, parity requirements                                                                                                                                           |
