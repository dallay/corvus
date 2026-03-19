---
title: Architecture
---

The project follows a modular architecture with a strong emphasis on centralized build logic and
clear separation between clients and shared modules.

## Project Structure

```text
.
├── clients/                    # Client applications
│   ├── agent-runtime/          # Agent runtime
│   ├── androidApp/             # Android host app
│   ├── composeApp/             # Shared Compose Multiplatform module
│   ├── iosApp/                 # iOS host app (Xcode project)
│   └── web/                    # Web app and operator dashboard
├── modules/                    # Shared modules
│   └── agent-core-kmp/         # Agent core in Kotlin Multiplatform
├── gradle/                     # Build configuration
│   ├── build-logic/            # Custom convention plugins
│   ├── aggregation/            # Report aggregation
│   ├── configs/                # Tool configurations
│   ├── versions/               # Version management
│   ├── libs.versions.toml      # Version catalog
│   └── wrapper/                # Gradle wrapper
├── docs/                       # Documentation (Astro + Starlight)
├── Makefile                    # Standardized command interface
└── settings.gradle.kts         # Global project configuration
```

## High-Level Architecture

Corvus is designed as a reactive agent platform with the following pillars:

### 1. **Reactive Runtime**

- **Technology**: Rust (Tokio/Async)
- **Purpose**: High-performance, concurrent agent execution & lifecycle
- **Location**: Agent runtime in `clients/agent-runtime/`

### 2. **Cerebro (Long-Term Memory)**

- **Technology**: Rust + SurrealDB (Embedded)
- **Purpose**: Centralized, agent-agnostic memory service with graph support
- **Integration**: Via MCP (JSON-RPC) from the runtime

### 3. **Sandboxed Tools**

- **Technology**: Rust + Docker (optional)
- **Purpose**: Secure execution of untrusted code and system operations
- **Communication**: Managed by the reactive runtime

### 4. **Operator Dashboard**

- **Technology**: Astro + Vue 3
- **Purpose**: Real-time observability and manual intervention
- **Location**: `clients/web/`

## Build Logic (Convention Plugins)

Instead of repeating build logic in every `build.gradle.kts`, we use **Convention Plugins** located
in `gradle/build-logic`.

### Plugin Categories

1. **Base Plugins**: Fundamental configuration like identity, lifecycle, and JVM conflict
   resolution.
2. **Module Plugins**: Language-specific configurations (`kotlin`, `java`, `spring-boot`,
   `compose`).
3. **Feature Plugins**: Opt-in features like `publish-library`, `shadow`, `test-fixtures` and
   `git-hook`.
4. **Check Plugins**: Code quality and formatting tools (`spotless`, `detekt`, `spotbugs`).
5. **Report Plugins**: Aggregated reports for testing, coverage, and SBOM.

## Dependency Management

We use **Gradle Version Catalogs** (`libs.versions.toml`) to define all dependencies and versions in
a single location. This ensures consistency across all modules.

### Example usage:

```kotlin
dependencies {
    implementation(libs.kotlin.stdlib)
    testImplementation(libs.junit.jupiter)
}
```

## Module Dependency Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      clients/composeApp                      │
│              (Shared Compose Multiplatform UI)               │
└────────────────────┬────────────────────────────────────────┘
                     │ uses
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    modules/agent-core-kmp                    │
│         (Business logic, domain, contracts)                  │
└────────────────────┬────────────────────────────────────────┘
                     │ provides to
                     ▼
    ┌────────────────┴────────────────┐
    │                                  │
    ▼                                  ▼
┌──────────────┐            ┌────────────────────┐
│clients/      │            │clients/            │
│androidApp    │            │iosApp              │
└──────────────┘            └────────────────────┘
```

## C4 Architecture Diagrams

For a more detailed view of the architecture, see the following C4 diagrams:

| Level           | Diagram                                                                    | Description                                       | Diagram ID                        |
|-----------------|----------------------------------------------------------------------------|---------------------------------------------------|-----------------------------------|
| C1 - Context    | [System Context](/guides/architecture/diagrams/context/system-context.mmd)       | High-level view of the runtime and external actors | `context/system-context.mmd`     |
| C2 - Containers | [Runtime Containers](/guides/architecture/diagrams/container/runtime-containers.mmd) | Runtime services and operator surfaces         | `container/runtime-containers.mmd` |
| C3 - Components | [Runtime Core](/guides/architecture/diagrams/component/runtime-core.mmd)         | Internal components of the runtime core           | `component/runtime-core.mmd`      |
| -               | [Cargo Dependencies](/guides/architecture/diagrams/cargo-dependencies.mmd)       | Cargo workspace and runtime dependency flow       | `cargo-dependencies.mmd`          |

See [Architecture Index](./overview) for more details on how to visualize them.
