# Agent Instructions

Gradle-based multi-module project in Kotlin. Emphasizes centralized build configurations, custom
plugins, and version catalogs.

## Core Principles

> **⚠️ CRITICAL: Security First, Performance Second**
>
> Every decision, every line of code, every architecture choice MUST prioritize:
>
> 1. **Security First** - Always think about attacks, vulnerabilities, and safe defaults
>    - Never trust user input
>    - Use parameterized queries, never string concatenation for SQL
>    - Validate and sanitize all data
>    - Follow principle of least privilege
>    - Keep dependencies updated to patch security vulnerabilities
> 2. **Extreme Performance Second** - Optimize for efficiency after security
>    - Think about algorithmic complexity (O(n) vs O(n²))
>    - Avoid unnecessary allocations
>    - Use lazy initialization when appropriate
>    - Profile before optimizing - measure don't guess
>    - Consider memory footprint and startup time
>
> These principles override convenience, speed of development, and "getting it done quickly."

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

## Project Structure

```
├── apps/
│   ├── composeApp/         # Shared Kotlin Multiplatform Compose UI module
│   ├── androidApp/         # Native Android host app for Compose
│   ├── iosApp/             # Native iOS host app for Compose
│   └── docs/               # Documentation website
├── modules/
│   ├── agent-core-kmp/     # Shared Kotlin Multiplatform core
│   └── agent-core-rust/    # Embedded Rust AI core
├── gradle/
│   ├── build-logic/        # Custom plugins
│   └── libs.versions.toml  # Version catalog
└── settings.gradle.kts
```

## Available Skills

Located in `.agents/skills/`. Reference for detailed patterns:

| Skill                                                                | Description                               | Trigger                          |
| -------------------------------------------------------------------- | ----------------------------------------- | -------------------------------- |
| [gradle](.agents/skills/gradle/SKILL.md)                             | Gradle best practices, custom tasks       | `build.gradle.kts`, build config |
| [kotlin](.agents/skills/kotlin/SKILL.md)                             | Kotlin conventions, null safety           | `.kt` files                      |
| [c4-diagrams](.agents/skills/c4-diagrams/SKILL.md)                   | C4 architecture diagrams                  | `docs/architecture/diagrams`     |
| [pr-creator](.agents/skills/pr-creator/SKILL.md)                     | PR creation workflow                      | Creating PRs                     |
| [pinned-tag](.agents/skills/pinned-tag/SKILL.md)                     | Pin GitHub Actions                        | CI security                      |
| [release](.agents/skills/release/SKILL.md)                           | Release process, Maven Central publishing | Creating releases                |
| [android-expert](.agents/skills/android-expert/SKILL.md)             | Android-specific patterns, best practices | Android development              |
| [compose-expert](.agents/skills/compose-expert/SKILL.md)             | Jetpack Compose UI patterns               | Compose UI code                  |
| [desktop-expert](.agents/skills/desktop-expert/SKILL.md)             | Compose Desktop, desktop patterns         | Desktop app development          |
| [gradle-expert](.agents/skills/gradle-expert/SKILL.md)               | Advanced Gradle, custom plugins           | Complex Gradle configs           |
| [kotlin-coroutines](.agents/skills/kotlin-coroutines/SKILL.md)       | Coroutines, async patterns                | Coroutines, Flow                 |
| [kotlin-expert](.agents/skills/kotlin-expert/SKILL.md)               | Advanced Kotlin features                  | Advanced Kotlin                  |
| [kotlin-multiplatform](.agents/skills/kotlin-multiplatform/SKILL.md) | KMP patterns, expect/actual               | KMP modules                      |
| [conventional-commits](.opencode/skill/conventional-commits/SKILL.md) | Conventional Commits specification        | Creating commits, git messages   |

## Testing

```kotlin
class ServiceTest {
  @Test
  fun `should return result`() {
    val result = service.action()
    assertEquals(expected, result)
  }
}
```

Run specific test:

```bash
./gradlew :composeApp:jvmTest --tests "*ComposeAppCommonTest*"
```

## License

Apache 2.0 - See LICENSE file
