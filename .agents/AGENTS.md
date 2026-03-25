# Agent Instructions

<!-- Agent metadata block -->
<!--
name: Corvus Agent Instructions
description: Gradle-based multi-module project in Kotlin with Rust agent runtime
purpose: Provide comprehensive coding guidance for Corvus project contributors
capabilities: Code analysis, implementation, testing, documentation, security review
version: 1.0.0
compatibility: Claude Code, GitHub Copilot, OpenAI Codex, Gemini, OpenCode
-->

Gradle-based multi-module project in Kotlin. Emphasizes centralized build configurations, custom
plugins, and version catalogs.

## Core Principles

**⚠️ CRITICAL: Security First, Performance Second**

Every decision, every line of code, every architecture choice MUST prioritize:

1. **Security First** - Always think about attacks, vulnerabilities, and safe defaults
   - Never trust user input
   - Use parameterized queries, never string concatenation for SQL
   - Validate and sanitize all data
   - Follow principle of least privilege
   - Keep dependencies updated to patch security vulnerabilities

2. **Extreme Performance Second** - Optimize for efficiency after security
   - Think about algorithmic complexity (O(n) vs O(n²))
   - Avoid unnecessary allocations
   - Use lazy initialization when appropriate
   - Profile before optimizing - measure don't guess
   - Consider memory footprint and startup time

These principles override convenience, speed of development, and "getting it done quickly."

We develop using **TDD by default**: Red -> Green -> Refactor for new behavior, bug fixes, and
risky refactors.

## Quick Commands

```bash
# Build & Run
make build              # Full build with tests
make build-fast         # Build without tests
make run                # Run Compose desktop app
./gradlew composeApp:run # Direct Gradle

# Testing
make test                    # All tests
make test-app                # Single module
./gradlew :composeApp:jvmTest --tests "ClassName.methodName"  # Single test
./gradlew :composeApp:jvmTest --tests "*Pattern*"             # Pattern match
make test-coverage           # With Kover report
make test-verbose            # --info output

# Code Quality
make format             # Spotless apply
make check-format       # Spotless check
make lint-kotlin        # Detekt analysis
make lint-java          # SpotBugs analysis
make check              # All checks

# Maintenance
make clean              # Clean build artifacts
make clean-all          # Clean + Gradle cache
make deps               # Show dependencies
make tasks              # List all tasks
make info               # Project info
```

## Code Style

### Formatting (.editorconfig)

- **Indent**: 2 spaces (no tabs)
- **Max line**: 100 characters
- **Trailing commas**: Required
- **No wildcard imports**: Explicit only
- **Charset**: UTF-8, trim whitespace, final newline

### Kotlin Patterns

```kotlin
// Data classes with value classes
data class User(
    val id: UserId,
    val name: String,
    val createdAt: Instant = Instant.now(),
)

@JvmInline
value class UserId(val value: UUID)

// Sealed types for results
sealed interface Result<out T> {
    data class Success<T>(val data: T) : Result<T>
    data class Failure(val error: Throwable) : Result<Nothing>
}

// Null safety - NO !! operator
val name = user?.name ?: "Unknown"
requireNotNull(value) { "Required" }

// Expression bodies
fun double(x: Int): Int = x * 2
```

### Naming

- **Classes**: PascalCase (`UserService`)
- **Functions**: camelCase (`findById`)
- **Constants**: UPPER_SNAKE_CASE (`MAX_RETRY`)
- **Tests**: Backticks `` `should work` ``
- **Booleans**: `is`/`has` prefix (`isActive`)

### Error Handling

```kotlin
// Result for recoverable
fun find(id: UUID): Result<User> = runCatching {
    repo.find(id) ?: throw NotFoundException(id)
}

// Sealed exceptions
sealed class DomainError(msg: String) : RuntimeException(msg)
class NotFoundException(id: UUID) : DomainError("Not found: $id")
```

## Gradle Guidelines

### Best Practices

- Use `tasks.register` not `create` (lazy)
- Use `configureEach` not `all`
- Never use `afterEvaluate`
- Avoid `project` in task actions
- Annotate task inputs/outputs for caching

### Dependencies

```kotlin
// Version catalog
implementation(libs.slf4j.api)
testImplementation(libs.junit.jupiter)
```

## Client Surfaces Architecture

Corvus uses a 3-tier architecture:

```
Tier 1: Runtime Core      → clients/agent-runtime (Rust, full capabilities)
Tier 2: Gateway Layer      → HTTP Gateway (web) + RustCliBridge (mobile)
Tier 3: Client Surfaces    → web/chat, web/dashboard, composeApp (mobile), docs, marketing
```

**Transport rules** (mandatory):

- Web clients (`chat`, `dashboard`) → **HTTP Gateway** only
- Mobile clients (`composeApp`) → **RustCliBridge** (process bridge) only
- CLI operators → **Direct runtime** access
- Supporting surfaces → **No runtime** communication

Surface contracts and detailed specs live in the openspec workspace (external, volatile).
Refer to those docs when available, but do not hardlink from this file.

## Project Structure
```text
├── clients/
│   ├── agent-runtime/       # Rust CLI/daemon (Tier 1 core)
│   ├── web/
│   │   ├── apps/
│   │   │   ├── chat/        # Web chat UI (Tier 3, HTTP Gateway)
│   │   │   ├── dashboard/   # Admin panel (Tier 3, HTTP Gateway)
│   │   │   ├── docs/        # Documentation (Tier 3, static)
│   │   │   └── marketing/   # Landing pages (Tier 3, static)
│   │   └── packages/shared/ # Web shared utilities
│   ├── composeApp/          # KMP Compose UI (Tier 3, CLI Bridge)
│   ├── androidApp/          # Android host for composeApp
│   └── iosApp/              # iOS host for composeApp
├── modules/
│   ├── agent-core-kmp/      # KMP contracts library (Tier 2 bridge)
│   └── agent-core-rust/     # Embedded Rust AI core
├── gradle/
│   ├── build-logic/         # Custom plugins
│   └── libs.versions.toml   # Version catalog
└── settings.gradle.kts
```

## Cerebro Memory Module

Cerebro is an agent-agnostic, high-performance memory system designed for use with any AI agent or
LLM that supports the Model Context Protocol (MCP). It is implemented as a single Rust binary and
uses SurrealDB (embedded) for multi-model storage (document, graph, vector search).

- **Integration:** Agents interact with Cerebro via the MCP JSON-RPC protocol, using a set of 13
  memory/session tools (see the cerebro spec in the openspec workspace for full API and business logic).
- **Architecture:** Cerebro uses a sync API for fast agent responses and an async worker for
  background tasks (e.g., vector embeddings, entity extraction, graph edges) if an LLM is
  configured.
- **Data Model:** Structured around `session`, `memory` (engram), and `prompt` nodes, with graph
  edges for relations and chronology.
- **Memory Hygiene:** Implements deduplication, topic upserts, and global filters for deleted
  records.
- **TUI:** Provides a terminal UI (ratatui + crossterm) for real-time observability, memory
  browsing, and session timelines.

**Note:** Cerebro is a separate module. Agents should use the documented MCP tools API for all
memory/session operations. See the spec for details on the drill-in retrieval strategy, memory
hygiene, and supported operations.

## Available Skills

Located in `.agents/skills/`. Reference for detailed patterns:

| Skill                                                                | Description                                                                                         | Trigger                                                                   |
|----------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------|
| [gradle](./skills/gradle/SKILL.md)                             | Gradle best practices, custom tasks                                                                 | `build.gradle.kts`, build config                                          |
| [kotlin](./skills/kotlin/SKILL.md)                             | Kotlin conventions, null safety                                                                     | `.kt` files                                                               |
| [c4-diagrams](./skills/c4-diagrams/SKILL.md)                   | C4 architecture diagrams                                                                            | `docs/architecture/diagrams`                                              |
| [pr-creator](./skills/pr-creator/SKILL.md)                     | PR creation workflow                                                                                | Creating PRs                                                              |
| [pinned-tag](./skills/pinned-tag/SKILL.md)                     | Pin GitHub Actions                                                                                  | CI security                                                               |
| [release](./skills/release/SKILL.md)                           | Release process, Maven Central publishing                                                           | Creating releases                                                         |
| [android-expert](./skills/android-expert/SKILL.md)             | Android-specific patterns, best practices                                                           | Android development                                                       |
| [compose-expert](./skills/compose-expert/SKILL.md)             | Jetpack Compose UI patterns                                                                         | Compose UI code                                                           |
| [desktop-expert](./skills/desktop-expert/SKILL.md)             | Compose Desktop, desktop patterns                                                                   | Desktop app development                                                   |
| [docker-expert](./skills/docker-expert/SKILL.md)               | Docker optimization, Compose, multi-stage                                                           | Dockerfile, docker-compose.yml                                            |
| [gradle-expert](./skills/gradle-expert/SKILL.md)               | Advanced Gradle, custom plugins                                                                     | Complex Gradle configs                                                    |
| [kotlin-coroutines](./skills/kotlin-coroutines/SKILL.md)       | Coroutines, async patterns                                                                          | Coroutines, Flow                                                          |
| [kotlin-expert](./skills/kotlin-expert/SKILL.md)               | Advanced Kotlin features                                                                            | Advanced Kotlin                                                           |
| [kotlin-multiplatform](./skills/kotlin-multiplatform/SKILL.md) | KMP patterns, expect/actual                                                                         | KMP modules                                                               |
| [frontend-design](./skills/frontend-design/SKILL.md)           | Create production-grade frontend UI with strong visual direction while avoiding generic AI patterns | Building or refining web components, pages, dashboards, or application UI |
| [conventional-commits](./skills/conventional-commits/SKILL.md) | Conventional Commits specification                                                                  | Creating commits, git messages                                            |
| [github-actions](./skills/github-actions/SKILL.md)            | GitHub Actions CI/CD best practices, security, optimization                                         | `.github/workflows/*.yml`, CI/CD pipelines                                |
