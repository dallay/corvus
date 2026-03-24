---
doc_id: client-surfaces-capability-matrix
version: 1.1.0
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

| Surface | Role | Transport | Status |
|---------|------|-----------|--------|
| `clients/agent-runtime` (CLI) | Operator/Admin | Direct | Complete |
| `clients/web/apps/chat` | End-user (Web) | Gateway | Scaffold (stub) |
| `clients/web/apps/dashboard` | Operator/Admin (Web) | Gateway | Complete |
| `clients/composeApp` (mobile) | End-user (Mobile) | CLI Bridge | Scaffold (no bridge) |
| `clients/web/apps/docs` | Supporting (Docs) | None | Complete |
| `clients/web/apps/marketing` | Supporting (Marketing) | None | Partial |
| `clients/composeApp` (shared module) | Supporting (Contracts) | Contracts | Contracts only |

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

**Legend**: `Yes` = Mandatory, `Opt` = Optional, `No` = Out-of-scope, `Contracts` = Type definitions only

## Transport Architecture

### Transport Per Surface

Each surface uses exactly one transport mechanism for all runtime communication:

| Surface | Transport | Protocol |
|---------|-----------|----------|
| Web clients (`chat`, `dashboard`) | HTTP Gateway | REST/WebSocket over HTTPS |
| Mobile (`composeApp`) | RustCliBridge | Process bridge (stdin/stdout) |
| CLI operators | Direct runtime | CLI subprocess |
| Supporting surfaces | None | No runtime communication |

**Rationale**: Transport choice is constrained by platform capability, not preference.
- Web clients use HTTP Gateway because browser sandboxing prevents process bridges.
- Mobile clients use CLI bridge because the gateway may not be network-accessible on embedded devices.
- CLI operators use direct runtime access for full capability access during development and server management.

### HTTP Gateway Endpoints

The gateway exposes a **client-safe subset** of runtime capabilities:

| Endpoint Group | Capabilities | Access |
|----------------|--------------|--------|
| `/chat/*` | Message sending, streaming | All authenticated clients |
| `/session/*` | Session lifecycle, history | All authenticated clients |
| `/memory/*` | Memory queries (policy-gated) | All authenticated clients |
| `/tool/*` | Tool invocation, approval | Chat surface |
| `/admin/*` | Configuration, audit | Dashboard only |
| `/health`, `/metrics` | Health, observability | Public |

### Runtime-Only Capabilities (Never Exposed)

The following capabilities are runtime-only and must never be exposed through any client surface:

| Capability | Reason |
|-----------|--------|
| Raw tool registry access | Security: tool execution gated by policy |
| Direct session database queries | Security: memory access gated by policy |
| Configuration hot-reload | Operations: explicit operator commands only |
| Runtime code injection | Security: runtime integrity |
| Audit log modification | Integrity: append-only |
| Credential vault access | Security: runtime-internal |

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

Each surface MUST use exactly one transport for all runtime communication, and its onboarding flow
MUST validate readiness only through that approved transport.

#### Scenario: Web surface uses HTTP Gateway
- GIVEN a web client surface (`chat`, `dashboard`)
- WHEN the surface communicates with the runtime
- THEN the surface MUST use HTTP Gateway endpoints
- AND the surface MUST NOT use process bridges, CLI invocation, or direct runtime access.

#### Scenario: Mobile surface uses CLI Bridge
- GIVEN a mobile client surface (`composeApp` on Android/iOS)
- WHEN the surface communicates with the runtime
- THEN the surface MUST use the RustCliBridge (process bridge)
- AND the surface MUST NOT use HTTP Gateway as the primary transport.

#### Scenario: iOS bridge exception
- GIVEN iOS cannot spawn processes like Android/desktop
- WHEN the bridge mechanism is evaluated
- THEN the surface MUST use a companion daemon with IPC (near-term)
- OR MUST use embedded Rust via FFI (long-term)
- AND MUST NOT require HTTP Gateway as the only path.

#### Scenario: Onboarding validates readiness through the approved transport
- GIVEN any onboarding-capable surface is preparing to enter ready state
- WHEN it validates runtime connectivity
- THEN it MUST perform that validation through the transport assigned in the canonical matrix
- AND it MUST NOT instruct the user to complete onboarding through another surface's transport as a
  substitute.

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

#### Scenario: Chat surface implements mandatory chat capabilities
- GIVEN `clients/web/apps/chat` or `clients/composeApp`
- WHEN the surface is evaluated for chat functionality
- THEN the surface MUST implement: message composition, session lifecycle, tool approval UI
- AND the surface MUST NOT implement: runtime configuration, admin controls, direct tool registry access.

#### Scenario: Dashboard surface implements admin capabilities
- GIVEN `clients/web/apps/dashboard`
- WHEN the surface is evaluated for admin functionality
- THEN the surface MUST implement: runtime config, session monitoring, audit viewing
- AND the surface MUST NOT implement: chat message composition, direct runtime process access.

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

**Rationale**: UUIDs provide collision resistance, work across distributed systems, and match the existing gateway implementation.

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
- THEN the output MUST be structured JSON including: `session_id`, `message_type`, `content`, `tool_results`, `metadata`
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

**Note**: Push notifications for background sessions require a companion service for delivery and are out-of-scope for this spec.

### Requirement: Contract Layer Scope

The `modules/agent-core-kmp` module has a two-tier structure:

- `commonMain`: MUST contain only type definitions and bridge interfaces
- `jvmMain`/`iosMain`/`androidMain`: MAY contain platform-specific bridge implementations

#### Scenario: Common main contains no execution logic
- GIVEN `modules/agent-core-kmp/src/commonMain/`
- WHEN the module is examined
- THEN it MUST contain only: data models (`CoreInvocation`, `CoreOutput`, `CoreResult`), bridge interfaces (`AgentCoreBridge`, `CliBridgeSession`), module metadata (`AgentKernel`)
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

**Migration**: Current `RustCliBridge` implementation in `jvmMain` is compliant with this spec. No changes required.

## Matrix Immutability Rules

1. **Adding a new surface**: Requires a new change (proposal → spec → design → tasks)
2. **Changing a capability tier**: Requires a change proposal with justification
3. **Adding new capability columns**: Requires architectural review
4. **Exception process**: Security-critical changes can fast-track via signed approval from two maintainers

## Cross-Reference

- [Gateway API Specification](./gateway-api.md) (TBD) — HTTP Gateway endpoint definitions (see `clients/agent-runtime/src/gateway/mod.rs` for current implementation)
- [MCP Runtime Specification](../mcp-runtime/spec.md) — Tool registry and MCP contract
- [Agent Loop Specification](../agent-loop/spec.md) — Canonical loop behavior
- [Dashboard Specification](../dashboard/spec.md) — Admin surface contract
- [Cerebro Specification](../cerebro/spec.md) — Memory system

## Change History

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2026-03-24 | Added onboarding alignment and recovery coverage requirements; clarified transport validation during onboarding |
| 1.0.0 | 2026-03-21 | Initial specification — canonical matrix, transport rules, parity requirements |
