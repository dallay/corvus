# Surface Contract: composeApp (Mobile)

## Metadata

- **Role**: End-user (Mobile)
- **Transport**: RustCliBridge (process bridge)
- **Location**: `clients/composeApp/` + `clients/androidApp/` + `clients/iosApp/`
- **Status**: Scaffold (no runtime bridge wired)
- **Spec**: [Canonical matrix](../spec.md)
- **Migration**: M1 (GatewayConfig → RustCliBridge), M3 (Session-Aware Bridge)

## Role Definition

Primary mobile end-user chat interface for Android and iOS via shared Kotlin Multiplatform Compose.
Each platform hosts the shared `composeApp` module with native wrappers.

## Mandatory Capabilities

### Chat Composition
- [ ] Platform-native text input
- [ ] Send and cancel controls
- [ ] Message bubble rendering (user/assistant)
- [ ] Streaming response display
- [ ] Sync response display fallback
- [ ] Model name display

### Session Management
- [ ] Session creation via CLI bridge
- [ ] Session resumption (UUID-based)
- [ ] Session termination
- [ ] Filesystem persistence for background resumption
- [ ] Timeout handling with user feedback

### Tool Approval
- [ ] Inline tool call display
- [ ] Approve button/action
- [ ] Deny button/action
- [ ] Approval status indicators

### RustCliBridge Integration
- [ ] Process spawning (`corvus agent`)
- [ ] Prompt passing via stdin/stdout
- [ ] Structured JSON output parsing (`--output json`)
- [ ] Session lifecycle subcommands
- [ ] Timeout management (configurable)

### Platform-Specific Features
- [ ] Push notifications (OS-native)
- [ ] Background session handling
- [ ] Platform file picker
- [ ] Biometric authentication
- [ ] Offline mode with graceful degradation

## Optional Capabilities

### Memory Display
- [ ] Short-term memory context (session-scoped)
- [ ] Long-term memory query results

### MCP Tool Visibility
- [ ] Tool call metadata
- [ ] Execution progress

## Out-of-Scope

| Capability | Reason |
|-----------|--------|
| HTTP Gateway integration | Mobile uses CLI bridge by design |
| Runtime configuration editing | Dashboard handles this |
| Admin/operator controls | CLI or dashboard handles this |
| Web-only features | Browser sandbox limitations |

## iOS-Specific Notes

iOS cannot spawn processes like Android/desktop. Bridge strategy:

1. **Near-term**: Companion daemon on macOS with IPC over local network
2. **Long-term**: Embedded Rust via FFI or Swift-Rust bindings

See: [iOS Bridge Strategy in spec](../spec.md#requirement-ios-bridge)

## Current Status

**Gap**: `ChatWorkspace.kt` has `AgentGatewayConfig` pointing to HTTP Gateway and `buildLocalAssistantReply` stub. The `RustCliBridge` exists in `modules/agent-core-kmp` but is not wired up.

**Required for completion**:
1. Wire `RustCliBridge` into `ChatWorkspace`
2. Remove `AgentGatewayConfig` (mobile doesn't use HTTP)
3. Add session state management
4. Implement background session persistence
5. Create onboarding for corvus CLI installation

See: [Migration M1 & M3](../migrations.md#m1-composeapp-gatewayconfig--rustclibridge)

## Transport Rule

Mobile composeApp **MUST** use RustCliBridge only. HTTP Gateway is out-of-scope as primary transport.

## Security Notes

- Corvus binary integrity verification (future)
- Session token storage (platform secure storage)
- Pairing code never persisted
- Rate limiting via bridge timeouts

## UI Framework

- Kotlin Multiplatform Compose (commonMain)
- Shared UI logic across Android/iOS
- Platform-specific implementations via expect/actual
- Glass morphism styling
- Dark/light theme support

## Session ID Format

UUID-based (e.g., `550e8400-e29b-41d4-a716-446655440000`) for consistency with gateway and cross-surface compatibility.

## Background Session Strategy

1. Persist session ID to filesystem on app background
2. On resume, check session state via bridge
3. If active, resume conversation
4. If expired, notify user and offer new session
