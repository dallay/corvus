---
title: Architecture Overview
description: High-level architecture of the Corvus project
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

### 1. **Reactive Orchestrator**

- **Technology**: Kotlin + Spring Boot + Coroutines/WebFlux
- **Purpose**: Handle non-blocking, always-on workflows
- **Location**: Agent runtime in `clients/agent-runtime/`

### 2. **Graph Memory**

- **Technology**: Neo4j (planned)
- **Purpose**: Knowledge model with connected context and durable memory
- **Integration**: Through `agent-core-kmp`

### 3. **High-Performance Sidecars**

- **Technology**: Rust (planned)
- **Purpose**: Scraping and sandboxed execution at high performance
- **Communication**: FFI or gRPC with Kotlin runtime

### 4. **Control Dashboard**

- **Technology**: Astro + Vue (planned)
- **Purpose**: Real-time observability and transparent operation
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

For a more detailed view of the architecture, see the
[Architecture Diagrams](./architecture/index.md) section.
