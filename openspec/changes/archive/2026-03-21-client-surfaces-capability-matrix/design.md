# Design: Client Surfaces Capability Matrix

## Technical Approach

This design establishes the architectural framework for all Corvus client surfaces by defining a
3-tier model: **runtime core** (Rust agent-runtime), **gateway layer** (HTTP Gateway + CLI bridge),
and **client surfaces** (web, mobile, CLI, docs, marketing). The capability matrix in the proposal
becomes the authoritative contract that governs which capabilities each surface exposes and how
each surface communicates with the runtime.

## Architecture Decisions

### Decision: 3-Tier Architecture with Runtime-Only Boundary

**Choice**: Enforce a strict 3-tier architecture where all runtime capabilities are accessed through
defined gateway interfaces, never exposed directly to client surfaces.

**Alternatives considered**:
- Tiered RPC: Expose raw RPC endpoints to all surfaces, let each surface decide what to use
- Shared library: Bundle runtime core as a shared library linked into each surface
- Monolithic: Single binary with all surfaces embedded

**Rationale**: A strict boundary prevents capability leakage (security), enables independent surface
development (velocity), and provides clear ownership (maintainability). The runtime-only boundary
documented in the proposal is codified here as an architectural constraint enforced by the gateway
layer, not by convention alone.

### Decision: Transport Per Surface, Not Per Capability

**Choice**: Each surface uses exactly one transport mechanism for all runtime communication:
- Web clients → HTTP Gateway
- Mobile clients → RustCliBridge (process)
- CLI operators → Direct runtime CLI
- Dashboard → HTTP Gateway

**Alternatives considered**:
- Unified transport abstraction that could route to either HTTP or CLI
- Surface-optional transports (e.g., mobile could use gateway if available)

**Rationale**: Mobile cannot reliably use HTTP Gateway because it runs embedded on-device where the
gateway is not always available or network-accessible. The CLI bridge provides a local,
always-available path. Web clients cannot use process bridges due to browser sandboxing. Transport
choice is constrained by platform capability, not preference.

### Decision: Contract Layer Lives in agent-core-kmp Only

**Choice**: The `modules/agent-core-kmp` module provides shared data models and bridge interfaces
only. It contains no execution logic, no UI, and no state management.

**Alternatives considered**:
- Distribute contracts across surfaces (copy types into each surface)
- Create a dedicated "contracts" module separate from agent-core-kmp
- Include agent-core-kmp as a dependency in all surfaces for execution capability

**Rationale**: Keeping contracts in a single KMP module ensures type consistency between web and
mobile without requiring runtime execution. The `CoreInvocation` / `CoreResult` contract in
`CoreContracts.kt` is intentionally minimal. The `RustCliBridge` in `jvmMain` is a platform-specific
transport adapter, not a capability implementation.

## System Architecture

### C4-Style System Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              CORVUS ECOSYSTEM                                   │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                        TIER 1: RUNTIME CORE                              │   │
│  │  ┌───────────────────────────────────────────────────────────────────┐  │   │
│  │  │                     clients/agent-runtime                          │  │   │
│  │  │                                                                     │  │   │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │  │   │
│  │  │  │  Agent   │  │  Tool    │  │ Memory   │  │    Policy        │ │  │   │
│  │  │  │  Loop    │  │ Registry │  │ Backend  │  │    Engine        │ │  │   │
│  │  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘ │  │   │
│  │  │       └─────────────┬┴─────────────┴───────────────┘            │  │   │
│  │  │                     │                                           │  │   │
│  │  │              ┌──────┴───────┐                                    │  │   │
│  │  │              │  Session     │   ← RUNTIME-ONLY CAPABILITIES:    │  │   │
│  │  │              │  Manager     │     - Raw tool registry access    │  │   │
│  │  │              └──────────────┘     - Direct memory DB queries    │  │   │
│  │  │                                  - Config hot-reload             │  │   │
│  │  │                                  - Audit log modification        │  │   │
│  │  └───────────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                       │                                          │
│                                       │ Runtime-internal IPC                     │
│                                       │                                          │
│  ┌────────────────────────────────────┴────────────────────────────────────┐   │
│  │                         TIER 2: GATEWAY LAYER                             │   │
│  │                                                                           │   │
│  │  ┌─────────────────────────┐     ┌─────────────────────────────────────┐   │   │
│  │  │     HTTP Gateway        │     │         RustCliBridge               │   │   │
│  │  │  (clients/agent-runtime │     │  (modules/agent-core-kmp/jvmMain)   │   │   │
│  │  │   src/gateway/mod.rs)   │     │                                     │   │   │
│  │  │                         │     │   spawns `corvus agent -m` process  │   │   │
│  │  │  - Axum-based HTTP/1.1  │     │   passes prompt via stdin           │   │   │
│  │  │  - 64KB body limit      │     │   reads output via stdout           │   │   │
│  │  │  - 30s request timeout  │     │   30s default timeout               │   │   │
│  │  │  - Session APIs         │     │   Session support: TBD (see below)  │   │   │
│  │  │  - Chat APIs            │     │                                     │   │   │
│  │  │  - Memory APIs          │     │                                     │   │   │
│  │  │  - Admin APIs           │     │                                     │   │   │
│  │  └───────────┬─────────────┘     └─────────────────┬───────────────────┘   │   │
│  │              │ HTTP (REST/WS)                       │ Process (stdin/stdout) │   │
│  └──────────────┼──────────────────────────────────────┼───────────────────────┘   │
│                 │                                      │                         │
│                 │                                      │                         │
│  ┌──────────────┴──────────────┐   ┌─────────────────┴───────────────────────┐   │
│  │      TIER 3: CLIENT         │   │        TIER 3: CLIENT                 │   │
│  │         SURFACES            │   │           SURFACES                    │   │
│  │                              │   │                                       │   │
│  │  END-USER (Web)              │   │  END-USER (Mobile)                    │   │
│  │  ┌────────────────────────┐  │   │  ┌────────────────────────────────┐  │   │
│  │  │ clients/web/apps/chat  │  │   │  │   clients/composeApp           │  │   │
│  │  │                        │  │   │  │   (androidApp + iosApp)         │  │   │
│  │  │  - Vue 3 + TypeScript   │  │   │  │                                │  │   │
│  │  │  - useGateway composable│  │   │  │  - KMP Compose (commonMain)    │  │   │
│  │  │  - ChatWorkspace UI     │  │   │  │  - ChatWorkspace (Kotlin)      │  │   │
│  │  │  - Session management   │  │   │  │  - Session management (TBD)    │  │   │
│  │  │  - Tool approval UI     │  │   │  │  - Tool approval UI            │  │   │
│  │  │                        │  │   │  │  - Platform notifications      │  │   │
│  │  │  Transport: HTTP only  │  │   │  │  - Background session handling │  │   │
│  │  │  (gateway API)         │  │   │  │                                │  │   │
│  │  └────────────────────────┘  │   │  │  Transport: RustCliBridge only │  │   │
│  │                              │   │  │  (process, not HTTP)           │  │   │
│  │  OPERATOR (Web)              │   │  └────────────────────────────────┘  │   │
│  │  ┌────────────────────────┐  │   │                                       │   │
│  │  │ clients/web/apps/      │  │   │  OPERATOR (CLI)                       │   │
│  │  │        dashboard       │  │   │  ┌────────────────────────────────┐  │   │
│  │  │                        │  │   │  │  clients/agent-runtime (CLI)  │  │   │
│  │  │  - Vue 3 + TypeScript  │  │   │  │                               │  │   │
│  │  │  - Config forms        │  │   │  │  - Direct runtime commands    │  │   │
│  │  │  - Session monitoring  │  │   │  │  - Full capability access     │  │   │
│  │  │  - MCP server config   │  │   │  │  - Gateway CLI mode           │  │   │
│  │  │                        │  │   │  │                               │  │   │
│  │  │  Transport: HTTP only  │  │   │  │  Transport: Direct (CLI)     │  │   │
│  │  │  (gateway API)         │  │   │  │                               │  │   │
│  │  └────────────────────────┘  │   │  └────────────────────────────────┘  │   │
│  │                              │   │                                       │   │
│  │  SUPPORTING                   │   │  SUPPORTING                           │   │
│  │  ┌────────────────────────┐  │   │  ┌────────────────────────────────┐  │   │
│  │  │ clients/web/apps/docs  │  │   │  │  modules/agent-core-kmp        │  │   │
│  │  │ (Astro Starlight)      │  │   │  │  (shared KMP contracts)         │  │   │
│  │  │                        │  │   │  │                                │  │   │
│  │  │  - Static docs         │  │   │  │  - CoreContracts.kt: types     │  │   │
│  │  │  - API reference       │  │   │  │  - CoreInvocation / CoreResult │  │   │
│  │  │  - No runtime access   │  │   │  │  - AgentCoreBridge interface   │  │   │
│  │  │                        │  │   │  │  - AgentKernel metadata         │  │   │
│  │  │  Transport: None       │  │   │  │                                │  │   │
│  │  └────────────────────────┘  │   │  │  Transport: Contracts only     │  │   │
│  │  ┌────────────────────────┐  │   │  │  (no execution)                │  │   │
│  │  │ clients/web/apps/     │  │   │  └────────────────────────────────┘  │   │
│  │  │        marketing       │  │   │                                       │   │
│  │  │ (Astro)                │  │   │                                       │   │
│  │  │                        │  │   │                                       │   │
│  │  │  - Marketing content   │  │   │                                       │   │
│  │  │  - No runtime access   │  │   │                                       │   │
│  │  │                        │  │   │                                       │   │
│  │  │  Transport: None       │  │   │                                       │   │
│  │  └────────────────────────┘  │   │                                       │   │
│  └──────────────────────────────┘   └───────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Transport Architecture

### HTTP Gateway (Web Clients)

**Location**: `clients/agent-runtime/src/gateway/mod.rs`

The HTTP Gateway is an Axum-based server providing REST and WebSocket endpoints for web clients.
Current implementation features:
- HTTP/1.1 compliance with proper content-length validation
- 64KB request body limit (prevents slow-loris attacks)
- 30s request timeout
- Header sanitization

**Endpoints provided** (see canonical matrix):
| Endpoint Group | Capabilities Exposed |
|----------------|---------------------|
| `/chat/*` | Message sending, streaming, session management |
| `/session/*` | Session lifecycle, history, resumption |
| `/memory/*` | Short-term and long-term memory queries |
| `/tool/*` | Tool invocation, approval status |
| `/admin/*` | Configuration, agent management, audit logs |

**Security**: Gateway enforces pairing guards, bearer token authentication, and channel-specific
routing. Web clients authenticate via bearer token or pairing flow.

### RustCliBridge (Mobile Clients)

**Location**: `modules/agent-core-kmp/src/jvmMain/kotlin/.../RustCliBridge.kt`

The RustCliBridge spawns the `corvus agent -m` process and communicates via stdin/stdout:

```
Mobile App ──→ RustCliBridge ──→ ProcessBuilder ──→ corvus agent -m
                    ↑                                       │
                    │                                       ↓
              CoreResult ◄────────────────────────── stdout output
```

**Current contract** (`CoreContracts.kt`):
```kotlin
data class CoreInvocation(
  val prompt: String,
  val sessionId: String? = null,      // TBD: not currently used by bridge
  val metadata: Map<String, String> = emptyMap(),
  val timeoutMs: Long? = null,
)

data class CoreOutput(
  val text: String,
  val transport: String,              // Always "rust-cli" for this bridge
  val rawOutput: String? = null,
)

sealed interface CoreResult {
  data class Success(val output: CoreOutput) : CoreResult
  data class Failure(
    val message: String,
    val details: String? = null,
    val recoverable: Boolean = false,
  ) : CoreResult
}

fun interface AgentCoreBridge {
  fun invoke(invocation: CoreInvocation): CoreResult
}
```

**Current limitations**: The existing bridge only passes a single `prompt` string. Session
management, streaming responses, and structured tool results are not implemented. This is the
primary gap to close for mobile parity.

**Target: CLI session mode** (`corvus agent -m --session-id <id>`) should enable:
1. Session creation and resumption
2. Streaming output parsing
3. Structured tool result deserialization
4. Background session handling

### CLI Direct (Operators)

**Location**: `clients/agent-runtime/src/main.rs`

Operators interact directly with the runtime via CLI commands. This bypasses the gateway entirely,
providing full runtime capability access. CLI mode is appropriate for:
- Local development and debugging
- Server operators who prefer shell-based management
- Automated scripts and CI/CD pipelines

## Contract Layer Design

### modules/agent-core-kmp Structure

```
modules/agent-core-kmp/src/
├── commonMain/kotlin/com/profiletailors/agent/core/
│   ├── AgentKernel.kt          # Module metadata (name, version)
│   └── CoreContracts.kt        # Shared types: CoreInvocation, CoreOutput, CoreResult, AgentCoreBridge
├── jvmMain/kotlin/com/profiletailors/agent/core/
│   └── RustCliBridge.kt        # Process bridge implementation
└── jvmTest/kotlin/com/profiletailors/agent/core/
    └── RustCliBridgeTest.kt    # Bridge tests
```

### Contract Principles

1. **Contracts are inputs, not outputs**: `CoreContracts.kt` defines what surfaces send to the
   runtime, not what the runtime does internally.

2. **Versioned contracts**: `AgentKernel.contractVersion` (`"0.1"`) provides a version identifier
   for future compatibility tracking.

3. **Bridge is transport, not capability**: `RustCliBridge` translates Kotlin invocations to
   process calls. The runtime capability lives in `agent-runtime`, not in the bridge.

4. **Platform isolation**: `jvmMain` is the correct location for bridge implementations because
   only JVM-based targets (Android, desktop) can spawn processes. iOS would need a separate
   `iosMain` implementation using a different mechanism (e.g., native binary invocation via
   mobile notification or background task).

### Shared Data Models in composeApp

**Location**: `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/`

The composeApp chat models are **UI contracts**, not runtime contracts:

```kotlin
// ChatComponents.kt
data class ChatMessage(val id: Int, val role: ChatRole, val content: String)
enum class ChatRole { User, Assistant }

// ChatWorkspace.kt
data class ChatWorkspaceState(val modelName: String, val inputPlaceholder: String, val welcomeMessage: String)
data class ChatUiState(val workspaceState: ChatWorkspaceState, val messages: List<ChatMessage>, val query: String, ...)
data class AgentGatewayConfig(val baseUrl: String, val pairingCode: String, val bearerToken: String, ...)

@Composable fun ChatWorkspace(state: ChatWorkspaceState = ChatWorkspaceDefaults.state())
```

**Boundary rule**: These types are UI-layer data classes. They are **not** part of the
`agent-core-kmp` runtime contract layer. The `AgentGatewayConfig` references a gateway URL, but
the proposal specifies that mobile uses `RustCliBridge`, not the gateway. This is a migration
item (see below).

## Capability Classification Criteria

### Mandatory Capabilities

A capability is **Mandatory** for a surface when:
1. It is the primary function of the surface (e.g., chat for the chat surface)
2. Users of that surface cannot accomplish their core task without it
3. Its absence would make the surface non-functional for its intended role

**Examples**:
- Chat message composition: Mandatory for chat surfaces (web + mobile) because without it, there
  is no chat
- Gateway API integration: Mandatory for web chat because browser sandbox prevents process bridges
- RustCliBridge: Mandatory for mobile because gateway may not be network-accessible
- Session management: Mandatory for chat surfaces because agent context requires continuity

### Optional Capabilities

A capability is **Optional** for a surface when:
1. It enhances the surface but is not required for core functionality
2. Implementation may vary by platform or be deferred
3. Users can accomplish core tasks without it

**Examples**:
- Short-term memory display: Useful context but chat works without it
- Long-term memory queries: Powerful feature, not required for basic chat
- MCP tool visibility: Nice-to-have debugging aid

**Decision rationale**: Optional capabilities are tracked for implementation but do not block
surface releases. They are strong candidates for future mandatory elevation if user feedback
indicates essential functionality.

### Out-of-Scope Capabilities

A capability is **Out-of-Scope** for a surface when:
1. It violates the surface's role (e.g., admin in end-user surface)
2. It is a runtime-only capability that must not leak to clients
3. It is physically impossible on the platform (e.g., filesystem access from browser)
4. It is delegated to another surface (e.g., web chat does not need CLI bridge)

**Runtime-only capabilities explicitly excluded from all client surfaces**:
| Capability | Reason |
|------------|--------|
| Raw tool registry access | Security: tool execution gated by policy |
| Direct session database queries | Security: memory access gated by policy |
| Configuration hot-reload | Operations: explicit operator commands only |
| Runtime code injection | Security: runtime integrity |
| Audit log modification | Integrity: append-only |
| Credential vault access | Security: runtime-internal |

## Parity Enforcement Strategy

### Parity Matrix (from proposal)

| Capability | Web (`chat`) | Mobile (`composeApp`) | Parity Level |
|------------|--------------|----------------------|---------------|
| Chat composition | Yes | Yes | **Mandatory** |
| Streaming response display | Yes | Yes | **Mandatory** |
| Sync response display | Yes | Yes | **Mandatory** |
| Session lifecycle | Yes | Yes | **Mandatory** |
| Tool approval UI | Yes | Yes | **Mandatory** |
| Short-term memory display | Yes | Yes | **Optional** |
| Long-term memory queries | Yes | Yes | **Optional** |
| MCP tool visibility | Yes | Yes | **Optional** |

### Enforcement Mechanisms

**1. Contract parity tests**: `modules/agent-core-kmp/src/commonTest/` should include tests that
verify `CoreContracts` types are compatible across surfaces. A shared test module that both web
and mobile can reference ensures type-level parity.

**2. Feature flag gates**: Mobile releases gate on mandatory parity. A `mobileMandatoryParity`
feature flag in composeApp build configuration ensures that:
- Web chat features marked Mandatory must have mobile equivalents before mobile release
- Optional features can be independently released on either surface

**3. Surface interface contracts**: Each surface should define its interface contract as a
checklist that reviewers reference:
- `CLAUDE.md` in each surface directory references the canonical matrix
- PRs touching a surface must state which matrix rows are affected
- Code review verifies classification matches matrix

**4. Capability audit CI**: A periodic CI job (weekly or per-release) validates:
- Each surface implements exactly the capabilities in its matrix row
- No surface imports or calls capabilities outside its row
- Runtime-only capabilities are not imported into any client surface

**5. Transport invariant checks**:
```kotlin
// In composeApp: Verify RustCliBridge is used, not HTTP
assert(transport == "rust-cli", "Mobile must use RustCliBridge, not HTTP Gateway")

// In web chat: Verify HTTP is used, not process bridge
assert(transport == "http", "Web must use HTTP Gateway")
```

## Specification Document Structure

### Permanent Location

The canonical capability matrix lives in `openspec/specs/` as a delta spec that becomes part of
the permanent specification:

```
openspec/specs/
├── client-surfaces/
│   └── spec.md          # ← Canonical matrix and surface definitions (THIS CHANGE)
├── agent-loop/
│   └── spec.md
├── mcp-runtime/
│   └── spec.md
├── dashboard/
│   └── spec.md
└── cerebro/
    └── spec.md
```

### Canonical Matrix

From the proposal, the permanent spec at `openspec/specs/client-surfaces/spec.md` should contain:

```markdown
# Client Surfaces Capability Matrix

## Surfaces

| Surface | Role | Transport |
|---------|------|-----------|
| `clients/agent-runtime` (CLI) | Operator | Direct |
| `clients/web/apps/chat` | End-user | Gateway |
| `clients/web/apps/dashboard` | Operator | Gateway |
| `clients/composeApp` (mobile) | End-user | CLI Bridge |
| `clients/web/apps/docs` | Supporting | None |
| `clients/web/apps/marketing` | Supporting | None |
| `clients/composeApp` (shared) | Supporting | Contracts |

## Capability Matrix

| Surface | Chat | Config | Memory | Tools | Sessions | Admin | Transport |
|---------|------|--------|--------|-------|----------|-------|-----------|
| `agent-runtime` (CLI) | Yes | Yes | Yes | Yes | Yes | Yes | Direct |
| `web/apps/chat` | **Yes** | No | Opt | Opt | Yes | No | Gateway |
| `web/apps/dashboard` | No | Yes | Yes | Yes | Yes | **Yes** | Gateway |
| `composeApp` (mobile) | **Yes** | No | Opt | Opt | Yes | No | CLI Bridge |
| `web/apps/docs` | No | No | No | No | No | No | None |
| `web/apps/marketing` | No | No | No | No | No | No | None |
| `composeApp` (shared) | Contracts | Contracts | Contracts | Contracts | Contracts | No | Contracts |

Legend: Yes=Mandatory, Opt=Optional, No=Out-of-scope
```

### Matrix Immutability Rules

1. **Adding a new surface**: Requires a new change (proposal → spec → design → tasks)
2. **Changing a capability tier**: Requires a change proposal with justification
3. **Adding new capability columns**: Requires architectural review
4. **Exception process**: Security-critical changes can fast-track via signed approval from
   two maintainers

## Migration Notes

### Migration 1: composeApp GatewayConfig → RustCliBridge

**Current state**: `ChatWorkspace.kt` in composeApp has `AgentGatewayConfig` with `baseUrl`,
`pairingCode`, `bearerToken`, and `webhookSecret`. The `buildLocalAssistantReply` function
targets the HTTP Gateway webhook endpoint.

**Target state**: Mobile uses `RustCliBridge`, not HTTP Gateway. The `AgentGatewayConfig` type
is removed from composeApp (it belongs to the web chat surface). Session management moves to
CLI bridge session APIs.

**Migration steps**:
1. Create `RustCliBridgeSession` wrapper in `modules/agent-core-kmp/jvmMain` that supports:
   - Session creation (`--session-id` argument or `SESSION CREATE` subcommand)
   - Session resumption
   - Structured output parsing (JSON responses)
   - Streaming output handling
2. Update `composeApp/ChatWorkspace.kt` to:
   - Remove `AgentGatewayConfig`
   - Replace `buildLocalAssistantReply` with `RustCliBridge` invocations
   - Add session state management
3. Remove `endpointUrl` and HTTP-specific helpers from `ChatComponents.kt`
4. Update composeApp onboarding to guide users to install the `corvus` CLI binary

**Transport rule**: Mobile composeApp MUST use RustCliBridge only. HTTP Gateway is out-of-scope
as primary transport. This enforces the one-transport-per-surface invariant.

### Migration 2: Web Chat Stub → Full Implementation

**Current state**: `clients/web/apps/chat` has `ChatWorkspace` UI with `buildLocalAssistantReply`
that generates a stub response ("[$modelName] Recibido..."). The `useGateway.ts` composable is
empty.

**Target state**: Full chat functionality via HTTP Gateway:
1. Session creation and management via `/session/*` endpoints
2. Message sending via `/chat/send` (streaming)
3. Tool approval UI wired to `/tool/approve` and `/tool/deny`
4. Memory display via `/memory/*` endpoints

**Migration steps**:
1. Implement `useGateway.ts` composable:
   - `connect()`: Establish gateway connection with bearer token
   - `sendMessage(prompt: string)`: Send chat message
   - `subscribeToolApproval(callback)`: Real-time tool approval events
   - `approveTool(toolId: string)`: Approve pending tool
   - `denyTool(toolId: string)`: Deny pending tool
2. Replace `buildLocalAssistantReply` with `useGateway` hook integration
3. Add WebSocket support for streaming responses
4. Wire session management (start, resume, end)

### Migration 3: RustCliBridge Current → Session-Aware

**Current state**: `RustCliBridge` passes `prompt` via command-line argument only. No session
support, no structured output, no streaming.

**Target state**: Session-aware CLI bridge:
1. Session management commands: `SESSION CREATE`, `SESSION RESUME <id>`, `SESSION END <id>`
2. Structured JSON output mode: `--output json`
3. Streaming mode: `--stream` with SSE-like output
4. Tool result serialization: structured `ToolResult` in JSON

**Migration steps**:
1. Define `CliBridgeSession` interface in `modules/agent-core-kmp/commonMain`:
   ```kotlin
   interface CliBridgeSession {
     val sessionId: String
     suspend fun send(prompt: String): Flow<String>  // Streaming text
     suspend fun sendStructured(prompt: String): CoreResult
     suspend fun close()
   }
   ```
2. Implement `RustCliBridgeSession` in `modules/agent-core-kmp/jvmMain`
3. Update `RustCliBridge` to expose `createSession(): CliBridgeSession`
4. Add session lifecycle to composeApp `ChatWorkspace`

### Migration 4: Gateway API Session Endpoints

**Current state**: Gateway at `src/gateway/mod.rs` has session-related endpoints. Exact
capabilities need verification against the capability matrix.

**Target state**: Gateway fully implements the matrix columns:
- `/session/create`, `/session/resume`, `/session/end` → Sessions column
- `/memory/short-term`, `/memory/long-term` → Memory column (optional)
- `/tool/invoke`, `/tool/approve`, `/tool/deny` → Tools column (optional)
- `/admin/*` → Admin column (dashboard only, out-of-scope for chat)

**Migration steps**:
1. Audit current gateway endpoints against matrix
2. Implement missing endpoints with proper authentication and authorization
3. Add session persistence (if not already present)
4. Document API in `openspec/specs/gateway-api/spec.md`

## Open Questions

- [x] **Session format**: CLI bridge sessions MUST use UUID v4 to match the canonical matrix
  spec. This ensures cross-surface compatibility and matches the gateway's session ID format.
- [ ] **Structured output**: Should `RustCliBridge` output JSON or maintain text compatibility
  with the current prompt-response model? JSON enables richer mobile UI; text is simpler.
- [ ] **iOS bridge**: The proposal mentions RustCliBridge but iOS cannot spawn processes in the
  same way. How should iOS communicate with the runtime? Options: (a) embedded Rust via
  FFI, (b) macOS daemon with IPC, (c) require network gateway for iOS.
- [ ] **Background sessions**: Mobile requires background session handling. Does the CLI bridge
  support background mode, or does mobile need a separate background service?
- [ ] **Gateway parity with CLI**: Should all CLI capabilities be available via gateway? The
  proposal says CLI has "Direct" access, implying gateway is a subset. Confirm scope.
