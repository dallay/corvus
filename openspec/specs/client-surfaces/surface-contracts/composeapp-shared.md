# Surface Contract: composeApp (Shared Module)

## Metadata

- **Role**: Supporting (Shared Contracts Library)
- **Transport**: Contracts only (no execution)
- **Location**: `modules/agent-core-kmp/`
- **Status**: Contracts only
- **Spec**: [Canonical matrix](../spec.md)

## Role Definition

Shared Kotlin Multiplatform contracts library providing type definitions and bridge interfaces
for mobile and potential future desktop platforms. Contains **no execution logic**, **no UI**, and
**no state management**.

## Shared Types

### Core Contracts (`CoreContracts.kt`)

```kotlin
// Version identifier
val contractVersion: String = "0.1"

// Invocation contract (surface → runtime)
data class CoreInvocation(
  val prompt: String,
  val sessionId: String? = null,      // UUID, optional
  val metadata: Map<String, String> = emptyMap(),
  val timeoutMs: Long? = null,
)

// Output contract (runtime → surface)
data class CoreOutput(
  val text: String,
  val transport: String,              // "rust-cli" for bridge
  val rawOutput: String? = null,
)

// Result contract
sealed interface CoreResult {
  data class Success(val output: CoreOutput) : CoreResult
  data class Failure(
    val message: String,
    val details: String? = null,
    val recoverable: Boolean = false,
  ) : CoreResult
}

// Bridge interface
fun interface AgentCoreBridge {
  fun invoke(invocation: CoreInvocation): CoreResult
}
```

### Module Metadata (`AgentKernel.kt`)

```kotlin
object AgentKernel {
  const val MODULE_NAME = "agent-core-kmp"
  const val CONTRACT_VERSION = "0.1"
  const val KOTLIN_VERSION = "..."  // Current Kotlin version
}
```

### Session Interface (Future, for M3)

```kotlin
// Planned for session-aware bridge
interface CliBridgeSession {
  val sessionId: String  // UUID
  suspend fun send(prompt: String): Flow<String>  // Streaming
  suspend fun sendStructured(prompt: String): CoreResult
  suspend fun close()
}
```

## Mandatory Capabilities

### Type Definitions
- [ ] `CoreInvocation` data class
- [ ] `CoreOutput` data class
- [ ] `CoreResult` sealed interface
- [ ] `AgentCoreBridge` functional interface

### Module Metadata
- [ ] `AgentKernel` object with version constants
- [ ] Contract version tracking

### Platform Adapters
- [ ] `RustCliBridge` implementation (jvmMain only)
- [ ] Platform detection utilities

## Out-of-Scope

| Capability | Reason |
|-----------|--------|
| Runtime execution | Library only, no execution |
| UI components | Compose UI lives in composeApp |
| State management | Handled by platform targets |
| HTTP Gateway logic | Web-specific, not in KMP |
| Configuration management | CLI handles this |

## Contract Versioning Policy

1. **Patch versions** (0.1.x): Additive changes (new optional fields, new interfaces)
2. **Minor versions** (0.x.1): Breaking changes to existing contracts (new major version)
3. **Major versions** (x.0.0): Reserved for fundamental contract redesigns

Breaking changes require:
- Migration guide in spec
- Deprecation period (minimum 2 minor versions)
- Migration tooling if possible

## Source Set Structure

```
modules/agent-core-kmp/src/
├── commonMain/kotlin/com/profiletailors/agent/core/
│   ├── AgentKernel.kt          # Module metadata
│   └── CoreContracts.kt        # Shared types (platform-agnostic)
├── jvmMain/kotlin/com/profiletailors/agent/core/
│   └── RustCliBridge.kt       # Process bridge (JVM/Android/desktop)
└── jvmTest/kotlin/com/profiletailors/agent/core/
    └── RustCliBridgeTest.kt   # Bridge tests
```

Note: `iosMain` is not currently defined. iOS bridge requires separate implementation
(see [iOS Bridge Strategy](../spec.md#requirement-ios-bridge)).

## Relationship to composeApp

The composeApp `ChatWorkspace` UI types (e.g., `ChatMessage`, `ChatUiState`, `AgentGatewayConfig`)
are **UI-layer** types, not runtime contracts. They belong in `clients/composeApp/`, not in
`modules/agent-core-kmp/`.

The boundary:
- `agent-core-kmp`: Runtime communication contracts (CoreInvocation, CoreResult)
- `composeApp`: UI state and platform integration (ChatUiState, ChatWorkspace)

## Related Specifications

- [ComposeApp Mobile Contract](./composeapp-mobile.md) — Mobile surface that consumes these contracts
- [MCP Runtime](../../mcp-runtime/spec.md) — Tool registry contract
