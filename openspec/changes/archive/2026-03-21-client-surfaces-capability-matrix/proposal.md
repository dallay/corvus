# Proposal: Client Surfaces Capability Matrix

## Intent

Establish an explicit canonical capability matrix for all Corvus client surfaces, defining which
surfaces serve end-users versus operators/administrators versus supporting roles. This proposal
removes ambiguity about what capabilities each surface must, may, or must not expose, resolves the
boundary between chat surfaces, and defines the mandatory parity contract across mobile and web
platforms.

## Scope

### In Scope

- Classify all 7 identified surfaces by role: end-user client, operator/admin, or supporting.
- Define capability tiers (mandatory, optional, out-of-scope) per surface.
- Define mobile-web parity requirements for composeApp and web apps.
- Clarify which capabilities are runtime-only by design and must never surface in clients.
- Establish the canonical capability matrix as the authoritative reference.
- Resolve the boundary between `clients/web/apps/chat` and `clients/composeApp` chat surfaces.

### Out of Scope

- Implementing missing capabilities (this proposal is a definitions document).
- Backend runtime changes to enforce surface contracts.
- Defining the operator CLI surface (separate change).
- Marketing site content and feature completeness.
- Build tooling and packaging differences between platforms.

## Surface Role Classification

### End-User Clients

Surfaces that expose the primary agent interaction model to non-technical end users.

| Surface | Role | Rationale |
|---------|------|-----------|
| `clients/web/apps/chat` | End-user: web | Primary web-based chat interface for users accessing Corvus via browser |
| `clients/composeApp` (androidApp + iosApp) | End-user: mobile | Primary mobile chat interfaces via KMP Compose, bridging to runtime |

### Operator/Admin Surfaces

Surfaces that expose runtime configuration, agent management, and operational controls to
administrators and operators.

| Surface | Role | Rationale |
|---------|------|-----------|
| `clients/web/apps/dashboard` | Operator/admin: web | Admin panel for runtime configuration, agent management, and operational oversight |
| `clients/agent-runtime` (CLI/daemon) | Operator/admin: runtime | CLI and daemon interface for operators deploying and managing the runtime directly |

### Supporting Surfaces

Surfaces that provide documentation, marketing, or shared infrastructure but do not expose
interactive agent capabilities.

| Surface | Role | Rationale |
|---------|------|-----------|
| `clients/web/apps/docs` | Supporting: documentation | Astro Starlight documentation site; zero agent interaction |
| `clients/web/apps/marketing` | Supporting: marketing | Astro marketing site; informational only, no agent interaction |
| `clients/composeApp` (shared module) | Supporting: contracts | Shared KMP contracts library; provides type definitions, no runtime interaction |

## Capability Tiers by Surface

### 1. `clients/agent-runtime` (CLI/Daemon)

The canonical runtime with full capabilities. All other surfaces are clients that access runtime
capabilities through defined interfaces.

| Capability | Tier | Notes |
|-----------|------|-------|
| Full tool registry execution | Mandatory | Complete MCP tool loop |
| Session and memory management | Mandatory | All memory scopes |
| Policy and approval evaluation | Mandatory | Full approval gates |
| Streaming and sync responses | Mandatory | Both response modes |
| Gateway webhook endpoints | Mandatory | HTTP transport for external callers |
| CLI command interface | Mandatory | Local operator control |
| Configuration management | Mandatory | Runtime config, not surface config |
| **Surface-specific UI rendering** | Out-of-scope | UI is handled by client surfaces |

### 2. `clients/web/apps/chat` (Vue 3)

Primary web end-user chat surface. **Status: scaffold with stub implementation.**

| Capability | Tier | Notes |
|-----------|------|-------|
| Chat message composition | Mandatory | Input handling, send/cancel |
| Message thread display | Mandatory | Streaming and sync response rendering |
| Session management | Mandatory | Session start, resume, end |
| Short-term memory display | Optional | Session-scoped memory context |
| Long-term memory queries | Optional | Cerebro memory tools |
| MCP tool invocation display | Optional | Show tool calls in conversation |
| Tool approval interaction | Mandatory | Approve/deny inline tools |
| Gateway API integration | Mandatory | All runtime calls via gateway |
| **Direct runtime process bridge** | Out-of-scope | Web clients use HTTP only |
| **Local file system access** | Out-of-scope | No native OS access |
| **Native notification dispatch** | Out-of-scope | Browser-only notifications |

### 3. `clients/web/apps/dashboard` (Vue 3)

Admin panel for operators. **Status: complete.**

| Capability | Tier | Notes |
|-----------|------|-------|
| Runtime configuration | Mandatory | Edit runtime settings via gateway |
| Agent management | Mandatory | Create, configure, delete agents |
| Session monitoring | Mandatory | Active session list and inspection |
| Memory administration | Mandatory | View/manage Cerebro memory |
| MCP server configuration | Mandatory | Add, remove, configure MCP servers |
| Approval policy management | Mandatory | Define approval rules |
| Audit log viewing | Mandatory | Session and action audit |
| **Direct runtime process access** | Out-of-scope | All via gateway API |
| **Runtime binary modification** | Out-of-scope | Configuration only, not runtime code |

### 4. `clients/composeApp` (androidApp + iosApp)

Primary mobile end-user chat surface via KMP. **Status: scaffold, no runtime bridge.**

| Capability | Tier | Notes |
|-----------|------|-------|
| Chat message composition | Mandatory | Platform-native input |
| Message thread display | Mandatory | Streaming and sync rendering |
| Session management | Mandatory | Session lifecycle |
| Short-term memory display | Optional | Session context |
| Long-term memory queries | Optional | Via RustCliBridge |
| MCP tool invocation display | Optional | Tool call visibility |
| Tool approval interaction | Mandatory | Inline approval UI |
| **RustCliBridge to runtime** | Mandatory | Process bridge for all runtime calls |
| **Gateway API integration** | Out-of-scope | Mobile uses CLI bridge, not gateway |
| **Native notification dispatch** | Mandatory | OS push/local notifications |
| **Background session handling** | Mandatory | Background refresh, notifications |

### 5. `clients/web/apps/docs` (Astro Starlight)

Documentation site. **Status: complete.**

| Capability | Tier | Notes |
|-----------|------|-------|
| Static documentation pages | Mandatory | Core documentation |
| API reference docs | Mandatory | Gateway API, MCP protocol |
| Search and navigation | Mandatory | Standard Starlight features |
| **Any agent interaction** | Out-of-scope | Zero runtime calls |

### 6. `clients/web/apps/marketing` (Astro)

Marketing site. **Status: partial.**

| Capability | Tier | Notes |
|-----------|------|-------|
| Marketing content | Mandatory | Landing, features, pricing |
| Static asset serving | Mandatory | Images, fonts, etc. |
| Contact/CTA forms | Optional | Lead capture |
| **Any agent interaction** | Out-of-scope | No runtime calls |
| **User authentication** | Out-of-scope | Marketing-only |

### 7. `clients/composeApp` (shared KMP module)

Shared contracts library for mobile platforms. **Status: contracts only.**

| Capability | Tier | Notes |
|-----------|------|-------|
| Shared data models | Mandatory | Session, Message, ToolResult types |
| Gateway API contracts | Mandatory | Web parity |
| CLI bridge contracts | Mandatory | Mobile parity |
| **Runtime execution** | Out-of-scope | Library only, no execution |
| **State management** | Out-of-scope | UI state handled by platform targets |

## Mobile-Web Parity Requirements

### Required Parity

| Capability | Web (`chat`) | Mobile (`composeApp`) | Parity Level |
|-----------|--------------|-----------------------|--------------|
| Chat composition | Yes | Yes | **Mandatory** |
| Streaming response display | Yes | Yes | **Mandatory** |
| Sync response display | Yes | Yes | **Mandatory** |
| Session lifecycle | Yes | Yes | **Mandatory** |
| Tool approval UI | Yes | Yes | **Mandatory** |
| Short-term memory display | Yes | Yes | **Optional** |
| Long-term memory queries | Yes | Yes | **Optional** |
| MCP tool visibility | Yes | Yes | **Optional** |

### Platform-Specific Capabilities

| Capability | Web (`chat`) | Mobile (`composeApp`) | Rationale |
|-----------|--------------|-----------------------|-----------|
| Push notifications | Browser API | OS-native | Platform constraint |
| Background sessions | Not applicable | Yes | Mobile lifecycle |
| File picker | Browser file API | Platform file picker | Platform constraint |
| Biometric auth | WebAuthn | Platform biometrics | Platform constraint |
| Offline support | Service worker | OS offline mode | Platform constraint |

### Transport Differences

- **Web clients** MUST use HTTP Gateway APIs for all runtime communication.
- **Mobile clients** MUST use the RustCliBridge (process bridge) for all runtime communication.
- Both transports expose the same capability surface; differences are implementation-only.

## Runtime-Only Capabilities

The following capabilities are explicitly runtime-only and must never be exposed through any client
surface:

| Capability | Rationale |
|-----------|-----------|
| Raw tool registry access | Security: tool execution is gated by policy |
| Direct session database queries | Security: memory access is gated by policy |
| Configuration hot-reload | Operations: only via explicit operator commands |
| Runtime code injection | Security: runtime integrity |
| Raw MCP server management without policy | Security: MCP access is gated |
| Audit log modification | Integrity: audit logs are append-only |
| Credential vault access | Security: credentials are runtime-internal |

Client surfaces MAY expose **results** of runtime-only capabilities (e.g., tool execution
results, memory query results) but MUST NOT expose raw access to the underlying mechanisms.

## Canonical Capability Matrix

| Surface | Role | Chat | Config | Memory | Tools | Sessions | Admin | Transport |
|---------|------|------|--------|--------|-------|----------|-------|-----------|
| `agent-runtime` (CLI) | Operator | Yes | Yes | Yes | Yes | Yes | Yes | Direct |
| `web/apps/chat` | End-user | **Yes** | No | Opt | Opt | Yes | No | Gateway |
| `web/apps/dashboard` | Operator | No | Yes | Yes | Yes | Yes | **Yes** | Gateway |
| `composeApp` (mobile) | End-user | **Yes** | No | Opt | Opt | Yes | No | CLI Bridge |
| `web/apps/docs` | Supporting | No | No | No | No | No | No | None |
| `web/apps/marketing` | Supporting | No | No | No | No | No | No | None |
| `composeApp` (shared) | Supporting | Contracts | Contracts | Contracts | Contracts | Contracts | No | Contracts |

**Legend:**
- **Yes** = Mandatory capability
- **Opt** = Optional capability
- **No** = Out-of-scope for this surface
- **Contracts** = Provides shared type/interface definitions

## Surface Boundary Resolution

### Chat Surface Boundary

The chat capability exists in two surfaces with distinct roles:

1. **`clients/web/apps/chat`** - Primary web end-user chat surface
   - Scope: Web browser users
   - Transport: HTTP Gateway API
   - State: Managed via gateway session APIs

2. **`clients/composeApp`** (androidApp + iosApp) - Primary mobile end-user chat surface
   - Scope: Native mobile users
   - Transport: RustCliBridge (process bridge)
   - State: Managed via CLI bridge session APIs

**Boundary rule**: Chat is the shared capability across these surfaces. The shared KMP module
provides contracts for chat data models. Each platform implements chat UI using its native toolkit
(Vue 3 for web, Compose for mobile). Runtime session semantics are platform-agnostic.

### Operator Surface Boundary

Operator surfaces are:

1. **`clients/web/apps/dashboard`** - Web admin panel for gateway-accessible operations
2. **`clients/agent-runtime`** (CLI) - Direct runtime operator interface

**Boundary rule**: Dashboard operates on runtime state via gateway APIs. CLI operates directly on
runtime internals. Dashboard never exposes runtime-only capabilities; CLI exposes them with
explicit operator intent.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/2026-03-21-client-surfaces-capability-matrix/proposal.md` | New | This proposal artifact |
| `clients/web/apps/chat` | Clarified | Defines mandatory vs optional vs out-of-scope for web chat |
| `clients/composeApp` | Clarified | Defines mandatory vs optional vs out-of-scope for mobile chat |
| `clients/web/apps/dashboard` | Clarified | Confirms admin scope boundaries |
| `clients/agent-runtime` | Clarified | Documents runtime-only boundary |
| `clients/composeApp/src/commonMain/kotlin` | Clarified | Documents contract scope, not execution scope |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Surfaces implement capabilities marked out-of-scope | Medium | Enforce via code review guidelines and spec references |
| Mobile parity lags web implementation | High | Track parity in implementation tasks; gate mobile releases on mandatory parity |
| RustCliBridge scope creep | Medium | Bridge is for transport only; runtime capabilities are gateway-defined |
| Ambiguity in "optional" vs "out-of-scope" | Low | Provide concrete examples in each surface's implementation guidance |

## Dependencies

- Completed exploration artifact (this proposal's source context)
- Existing gateway API contracts (`openspec/specs/gateway-api/`)
- Existing MCP tool contracts (`openspec/specs/mcp-runtime/`)

## Success Criteria

- [ ] All 7 surfaces have explicit role classifications (end-user, operator/admin, supporting).
- [ ] Each surface has defined mandatory, optional, and out-of-scope capabilities.
- [ ] Mobile-web parity requirements are defined for all shared capabilities.
- [ ] Runtime-only capabilities are documented and excluded from all client surfaces.
- [ ] Canonical capability matrix provides authoritative reference for implementation.
- [ ] Chat surface boundary between web and mobile is resolved with transport rules.
- [ ] Follow-up implementation work can proceed without reopening role/capability definitions.
