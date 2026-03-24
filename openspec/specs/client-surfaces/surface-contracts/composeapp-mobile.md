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

Mobile composeApp **MUST** use a `MobileBridgeContract` implementation. HTTP Gateway is out-of-scope as primary transport.

The `MobileBridgeContract` defines the interface for mobile-to-runtime communication. Platform-specific implementations:

| Platform | Implementation | Notes |
|----------|---------------|-------|
| Android | `RustCliBridge` | Process bridge via JVM subprocess |
| Desktop (JVM) | `RustCliBridge` | Process bridge via JVM subprocess |
| macOS/iOS | Companion daemon or Embedded Rust | IPC over local network (near-term), FFI/Swift-Rust bindings (long-term) |

**Reference**: `AgentCoreBridge` interface in `modules/agent-core-kmp/commonMain`

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

## i18n Requirements

**Locale Tier**: Tier 1 — Full
**Supported Locales**: en, es
**Parity Requirement**: Mandatory — CI-enforced
**Glossary Compliance**: Mandatory — CI-enforced

### String Externalization

- The surface MUST use Compose Resources (`values/strings.xml`, `values-es/strings.xml`)
- All UI strings MUST use `stringResource(Res.string.*)` — no hardcoded user-facing strings
- Translation keys MUST follow the `{domain}.{feature}.{element}` naming convention, adapted to
  XML `name` attributes using underscores (e.g., `onboarding.pairing.title` →
  `onboarding_pairing_title`)
- `AGENT_NAME` MUST be moved from a hardcoded constant to string resources
- Recovery messages MUST use canonical recovery patterns from the i18n governance spec
- All product terms MUST match the canonical glossary
- The inconsistency of "link" MUST be resolved to "pair" per the canonical glossary

### Parity Testing

- The surface MUST implement a Kotlin parity test validating key parity across locale files
- All `<string name="...">` entries MUST exist in both `strings.xml` and `strings-es.xml`
- The Gradle parity test MUST pass on every PR that touches locale files

### Design Tokens

- The surface MUST use `CorvusTheme.*` extensions for all visual tokens
- The surface MUST support light and dark themes via `MaterialTheme` token switching
- Glass morphism styling MUST use canonical glass tokens via `CorvusTheme`
- No hardcoded color values MUST exist in composable functions

### Scenarios

#### Scenario: Mobile surface passes i18n audit

- GIVEN the `composeApp` surface
- WHEN the i18n compliance audit runs
- THEN all `<string name="...">` entries MUST exist in both `strings.xml` and `strings-es.xml`
- AND no hardcoded user-facing strings MUST exist in Kotlin/Compose source files
- AND `AGENT_NAME` MUST be sourced from string resources, not a constant
- AND the Gradle parity test MUST pass

#### Scenario: Mobile onboarding uses canonical "pair" term

- GIVEN the mobile surface renders the onboarding trust step
- WHEN the step text is displayed
- THEN the text MUST use "pair" (en) or "emparejar" (es)
- AND the text MUST NOT use "link" (the current inconsistency MUST be resolved)

#### Scenario: Mobile XML key naming convention

- GIVEN the Compose resource format uses XML `name` attributes
- WHEN a canonical key `onboarding.pairing.title` is mapped
- THEN the XML name MUST be `onboarding_pairing_title` (dots replaced with underscores)
- AND the mapping MUST be deterministic and reversible

#### Scenario: Mobile surface uses canonical tokens

- GIVEN the composeApp's Compose theme
- WHEN the token audit runs
- THEN all color, spacing, and typography values MUST reference `CorvusTheme.*` properties
- AND no hardcoded color values MUST exist in composable functions

### References

- [i18n Governance Specification](../../i18n-governance/spec.md)
- [Design Token Governance](../../design-tokens/spec.md)
- [Canonical Glossary](../../../glossary/README.md)

## Change History

| Version | Date       | Changes                                                  |
|---------|------------|----------------------------------------------------------|
| 1.1.0   | 2026-03-24 | Added i18n Requirements section (Tier 1 — Full, #278)   |
