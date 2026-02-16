# agent-core-kmp

Shared Kotlin Multiplatform foundation for the Corvus agent core.

Current scope:

- Common contracts for invoking the core (`CoreInvocation`, `CoreResult`, `AgentCoreBridge`).
- Stable module identity metadata (`AgentKernel`).
- Initial JVM bridge that can invoke the Rust CLI core (`RustCliBridge`).

## Contract Baseline

- Package: `com.profiletailors.agent.core`
- Contract version: `0.1`
- Transport currently implemented: `rust-cli` (JVM)

## JVM Rust Bridge

The JVM bridge shells out to the Rust binary and maps process outcomes to typed results.

Default command:

```bash
corvus agent -m "<prompt>"
```

Example:

```kotlin
val bridge = RustCliBridge()
val result = bridge.invoke(CoreInvocation(prompt = "Hello from KMP"))
```

This is intentionally a first bridge layer so we can evolve toward richer IPC/FFI later without
breaking shared contracts.
