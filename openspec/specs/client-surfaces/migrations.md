# Client Surfaces Migrations

Migration status legend: `not-started` | `in-progress` | `blocked` | `complete`

---

## M1: composeApp GatewayConfig → RustCliBridge

**Issue**: `ChatWorkspace.kt` uses HTTP Gateway config and `buildLocalAssistantReply` stub.
Mobile must use CLI bridge per transport rules.

**Status**: not-started

**Blocks**: M3

**Dependencies**: None

**Tasks**:
- [ ] Create `RustCliBridgeSession` in `modules/agent-core-kmp/jvmMain`
- [ ] Add session creation/resumption/close methods
- [ ] Update composeApp `ChatWorkspace.kt` to use bridge
- [ ] Remove `AgentGatewayConfig` from composeApp
- [ ] Remove `buildLocalAssistantReply` stub
- [ ] Add onboarding for corvus CLI installation

**Related Files**:
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/App.kt`
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatComponents.kt`
- `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt`

**Related Specs**:
- [composeApp Mobile Contract](./surface-contracts/composeapp-mobile.md)
- [composeApp Shared Contract](./surface-contracts/composeapp-shared.md)

---

## M2: Web Chat Stub → Full Implementation

**Issue**: `clients/web/apps/chat` has `buildLocalAssistantReply` stub and empty `useGateway.ts`.

**Status**: not-started

**Blocks**: None

**Dependencies**: M4 (gateway endpoints must exist)

**Tasks**:
- [ ] Implement `useGateway.ts` composable:
  - [ ] `connect()`: Establish gateway connection with bearer token
  - [ ] `sendMessage(prompt: string)`: Send chat message
  - [ ] `subscribeToolApproval(callback)`: Real-time tool approval events
  - [ ] `approveTool(toolId: string)`: Approve pending tool
  - [ ] `denyTool(toolId: string)`: Deny pending tool
- [ ] Implement `useChat.ts` composable:
  - [ ] Message state management
  - [ ] Session lifecycle hooks
- [ ] Replace `buildLocalAssistantReply` with `useGateway` hook integration
- [ ] Add WebSocket/SSE support for streaming responses
- [ ] Wire session management (start, resume, end)
- [ ] Implement session ID persistence (sessionStorage)

**Related Files**:
- `clients/web/apps/chat/src/composables/useGateway.ts` (empty → implement)
- `clients/web/apps/chat/src/composables/useChat.ts` (empty → implement)
- `clients/web/apps/chat/src/views/ChatView.vue`
- `clients/web/apps/chat/src/components/ChatPanel.vue`

**Related Specs**:
- [Web Chat Contract](./surface-contracts/web-chat.md)
- [Gateway API](./gateway-api.md) (TBD)

---

## M3: RustCliBridge → Session-Aware Bridge

**Issue**: Current bridge only passes `prompt` argument. No sessions, no structured output.

**Status**: not-started

**Blocked by**: M1 (needs session interface contract first)

**Dependencies**: None

**Tasks**:
- [ ] Define `CliBridgeSession` interface in `modules/agent-core-kmp/commonMain`:
  ```kotlin
  interface CliBridgeSession {
    val sessionId: String  // UUID
    suspend fun send(prompt: String): Flow<String>  // Streaming
    suspend fun sendStructured(prompt: String): CoreResult
    suspend fun close()
  }
  ```
- [ ] Implement `RustCliBridgeSession` in `modules/agent-core-kmp/jvmMain`
- [ ] Add `--output json|text` flag to corvus CLI
- [ ] Add `--stream` streaming mode to corvus CLI
- [ ] Add session lifecycle subcommands to corvus CLI:
  - [ ] `SESSION CREATE` → returns UUID
  - [ ] `SESSION RESUME <id>` → continues session
  - [ ] `SESSION END <id>` → terminates session
- [ ] Update composeApp `ChatWorkspace` to use session-aware bridge

**Related Files**:
- `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CliBridgeSession.kt` (new)
- `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt`
- `clients/agent-runtime/src/main.rs` (CLI changes)

**Related Specs**:
- [composeApp Shared Contract](./surface-contracts/composeapp-shared.md)
- [Agent Loop](../agent-loop/spec.md)

---

## M4: Gateway API Session Endpoints

**Issue**: Verify gateway implements full session/memory/tools/admin columns per capability matrix.

**Status**: not-started

**Blocks**: M2 (web chat depends on gateway)

**Dependencies**: None

**Tasks**:
- [ ] Audit current gateway endpoints against capability matrix:
  - [ ] `/session/create`, `/session/resume`, `/session/end` → Sessions column
  - [ ] `/memory/short-term`, `/memory/long-term` → Memory column
  - [ ] `/tool/invoke`, `/tool/approve`, `/tool/deny` → Tools column
  - [ ] `/admin/*` → Admin column (dashboard only)
- [ ] Implement missing endpoints if any
- [ ] Add session persistence
- [ ] Document API in `openspec/specs/gateway-api/spec.md` (TBD)

**Related Files**:
- `clients/agent-runtime/src/gateway/mod.rs`
- `clients/agent-runtime/src/gateway/admin.rs`
- `openspec/specs/gateway-api/spec.md` (TBD)

**Related Specs**:
- [Canonical Matrix](./spec.md)
- [Dashboard Contract](./surface-contracts/web-dashboard.md)

---

## Cross-Migration Dependencies

```
M1 (composeApp bridge) ──┬──→ M3 (session-aware bridge)
                        │
M4 (gateway audit) ─────┴──→ M2 (web chat)
```

## Migration Priority

| Priority | Migration | Rationale |
|----------|-----------|-----------|
| 1 | M4 | Unblocks M2; gateway must be ready first |
| 2 | M2 | Web chat stub → full; most visible to users |
| 3 | M1 + M3 | Mobile bridge; enables composeApp to work |

## Tracking Issues

Create GitHub issues for each migration:
- [ ] `#276` — M1: composeApp RustCliBridge integration
- [ ] `#277` — M2: Web chat full implementation
- [ ] `#278` — M3: Session-aware CLI bridge
- [ ] `#279` — M4: Gateway session endpoints audit

## Status

Last updated: 2026-03-21
