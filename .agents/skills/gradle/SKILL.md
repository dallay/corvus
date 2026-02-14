---
name: gradle
description: >
    Gradle best practices for build configuration, custom plugins, and performance.
    Trigger: When working with build.gradle.kts, settings.gradle.kts, custom tasks, or Gradle plugins.
allowed-tools: Read, Edit, Write, Glob, Grep, Bash
metadata:
    author: @dallay
    version: "1.0"
    source: https://github.com/liutikas/gradle-best-practices
---

# Gradle Best Practices Skill

Conventions for writing efficient, maintainable, and cacheable Gradle builds.

## When to Use

- Creating or modifying `build.gradle.kts` or `settings.gradle.kts`
- Writing custom Gradle tasks or plugins
- Configuring dependencies and version catalogs
- Optimizing build performance and cacheability
- Setting up multi-module projects

## Critical Patterns

### 1. Use Latest Versions

**ALWAYS use the latest Gradle and plugin versions**. Benefits include performance improvements, bug
fixes, and new features.

```kotlin
// gradle/wrapper/gradle-wrapper.properties
distributionUrl = https\://services.gradle.org/distributions/gradle-9.2.1-bin.zip

// settings.gradle.kts - use shadow jobs to test upcoming versions
plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "0.10.0"
}
```

> **Tip**: Set up shadow CI jobs to test against upcoming Gradle versions and catch regressions
> early.

### 2. Never Use Internal APIs

**Internal APIs can break in ANY release, even minor ones**. If you need functionality from an
internal API:

```kotlin
// ❌ NEVER do this
import org.gradle.internal.something.InternalClass  // BREAKS!

// ✅ Copy relevant bits to your codebase or find public alternatives
// Internal APIs are "fair game" for breaking changes
```

### 3. Avoid Ordering Assumptions

**Use lazy configuration, callbacks, and provider chains**. Never assume plugin application order:

```kotlin
// ❌ WRONG: Assumes ordering
val javaExtension = project.extensions.getByType<JavaPluginExtension>()

// ✅ CORRECT: React to plugin being applied
pluginManager.withPlugin("java") {
    val javaExtension = extensions.getByType<JavaPluginExtension>()
    // Configure safely
}
```

### 4. Avoid `afterEvaluate`

**`afterEvaluate` creates subtle ordering issues that are extremely hard to debug**:

```kotlin
// ❌ NEVER do this
afterEvaluate {
    // This introduces ordering nightmares
    tasks.named("someTask").configure { /* ... */ }
}

// ✅ Use Provider/Property for lazy evaluation
tasks.register<MyTask>("myTask") {
    inputFile.set(layout.projectDirectory.file("input.txt"))
    outputFile.set(layout.buildDirectory.file("output.txt"))
}
```

## Custom Tasks - ALWAYS Create Task Classes

**NEVER use generic tasks with `doFirst`/`doLast`**. Even for simple tasks, create a custom class:

```kotlin
// ❌ WRONG: Generic task with doLast
tasks.register("processFiles") {
    doLast {
        // No inputs, outputs, or cacheability!
        file("input.txt").readText()
    }
}

// ✅ CORRECT: Custom task class with proper annotations
abstract class ProcessFilesTask : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val inputFile: RegularFileProperty

    @get:OutputFile
    abstract val outputFile: RegularFileProperty

    @TaskAction
    fun process() {
        val content = inputFile.get().asFile.readText()
        outputFile.get().asFile.writeText(content.uppercase())
    }
}

// Register the task
tasks.register<ProcessFilesTask>("processFiles") {
    inputFile.set(layout.projectDirectory.file("input.txt"))
    outputFile.set(layout.buildDirectory.file("output.txt"))
}
```

> **Note**: Making task and input/output properties `abstract` lets Gradle auto-initialize them
> without `project.objects` factory methods.

### Enable Stricter Plugin Validation

**For plugin projects, enable strict validation**:

```kotlin
// build.gradle.kts (in plugin project)
plugins {
    `java-gradle-plugin`
}

tasks.withType<ValidatePlugins>().configureEach {
    failOnWarning.set(true)
    enableStricterValidation.set(true)
}
```

## Dependencies

### Keep Dependencies Clustered

**Group dependencies by configuration for readability**:

```kotlin
dependencies {
    // Production dependencies
    implementation(libs.spring.boot.starter.webflux)
    implementation(libs.kotlin.coroutines.reactor)

    // Test dependencies
    testImplementation(libs.kotest.runner.junit5)
    testImplementation(libs.mockk)

    // Integration test dependencies
    integrationTestImplementation(libs.testcontainers.postgresql)
}
```

### Use Appropriate Configurations

**Prefer `implementation` over `api`. Add dependencies where you use them**:

```kotlin
// ❌ WRONG: api when implementation suffices
api(libs.jackson.core)  // Unnecessarily exposes to consumers

// ✅ CORRECT: implementation hides transitive dependencies
implementation(libs.jackson.core)

// Use dependency-analysis plugin to maintain clean lists
// https://github.com/autonomousapps/dependency-analysis-android-gradle-plugin
```

### Use Version Catalogs

**Centralize version management in `gradle/libs.versions.toml`**:

> **Note**: The example below is illustrative. Actual versions and key names may differ from
> `gradle/libs.versions.toml` in this project (e.g., `springBoot` vs `spring-boot`).

```toml
# gradle/libs.versions.toml
[versions]
kotlin = "2.2.21"
springBoot = "4.0.1"
kotest = "5.9.1"

[libraries]
kotlin-stdlib = { module = "org.jetbrains.kotlin:kotlin-stdlib", version.ref = "kotlin" }
spring-boot-starter-webflux = { module = "org.springframework.boot:spring-boot-starter-webflux", version.ref = "springBoot" }
kotest-runner-junit5 = { module = "io.kotest:kotest-runner-junit5", version.ref = "kotest" }

[plugins]
kotlin-jvm = { id = "org.jetbrains.kotlin.jvm", version.ref = "kotlin" }
spring-boot = { id = "org.springframework.boot", version.ref = "springBoot" }
```

```kotlin
// build.gradle.kts - use the catalog
dependencies {
    implementation(libs.spring.boot.starter.webflux)
    testImplementation(libs.kotest.runner.junit5)
}
```

## Laziness - Configuration Phase Performance

### No Expensive Computations in Configuration

**Configuration phase runs for EVERY build. Keep it fast**:

```kotlin
// ❌ WRONG: Expensive operation in configuration phase
val gitSha = "git rev-parse HEAD".execute()  // Runs on every build!

// ✅ CORRECT: Defer to task action
abstract class GitInfoTask : DefaultTask() {
    @get:OutputFile
    abstract val outputFile: RegularFileProperty

    @TaskAction
    fun generate() {
        val sha = "git rev-parse HEAD".execute()  // Runs only when task executes
        outputFile.get().asFile.writeText(sha)
    }
}
```

### Use `register` Instead of `create`

**`create` eagerly initializes tasks; `register` is lazy**:

```kotlin
// ❌ WRONG: Eager initialization
tasks.create<MyTask>("myTask") {
    // This runs immediately during configuration
}

// ✅ CORRECT: Lazy registration
tasks.register<MyTask>("myTask") {
    // This runs only if the task is needed
}
```

### Use `configureEach` Instead of `all`

**`all` eagerly initializes all elements; `configureEach` is lazy**:

```kotlin
// ❌ WRONG: Forces all tasks to initialize
tasks.all {
    if (this is Test) {
        useJUnitPlatform()
    }
}

// ✅ CORRECT: Only configures when task is needed
tasks.withType<Test>().configureEach {
    useJUnitPlatform()
}
```

### Never Call `get()` Outside Task Actions

**Calling `get()` defeats the purpose of lazy evaluation**:

```kotlin
// ❌ WRONG: Eager evaluation breaks ordering
val inputPath = inputFile.get().asFile.absolutePath  // Too early!

// ✅ CORRECT: Use map/flatMap for transformations
val inputPath = inputFile.map { it.asFile.absolutePath }

// ✅ CORRECT: Only call get() inside task actions
@TaskAction
fun execute() {
    val path = inputFile.get().asFile.absolutePath  // OK here
}
```

## Cacheability

### Make Tasks Cacheable by Default

**Gradle defaults to NOT caching. Explicitly enable caching**:

```kotlin
@CacheableTask  // Enable caching
abstract class ProcessFilesTask : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)  // Content-only sensitivity
    abstract val inputFile: RegularFileProperty

    @get:OutputFile
    abstract val outputFile: RegularFileProperty

    @TaskAction
    fun process() { /* ... */
    }
}
```

**Exceptions - Don't cache these**:

| Task Type         | Reason                                            |
|-------------------|---------------------------------------------------|
| Copy/Package/Zip  | Faster to re-run locally than download from cache |
| Unpack/Extract    | Same - re-running is cheaper than cache overhead  |
| Non-stable inputs | Tasks using git SHA, timestamps get no cache hits |

### Annotate All Inputs and Outputs

**Without proper annotations, Gradle can't track changes**:

```kotlin
abstract class MyTask : DefaultTask() {
    // File inputs
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val configFile: RegularFileProperty

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sourceFiles: ConfigurableFileCollection

    // Simple inputs
    @get:Input
    abstract val version: Property<String>

    // Outputs
    @get:OutputFile
    abstract val outputFile: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    // Non-input properties
    @get:Internal
    abstract val logger: Property<Logger>
}
```

### Path Sensitivity - Prefer NONE

**Default absolute path sensitivity causes unnecessary cache misses**:

```kotlin
// ❌ DEFAULT: Absolute path sensitive (bad for caching)
@get:InputFile
abstract val inputFile: RegularFileProperty

// ✅ CORRECT: Content-only (best for caching)
@get:InputFile
@get:PathSensitive(PathSensitivity.NONE)
abstract val inputFile: RegularFileProperty

// Other options:
// PathSensitivity.NAME_ONLY - file name matters, not path
// PathSensitivity.RELATIVE - relative path from project root matters
// @Classpath - for JVM classpath entries
```

### No Overlapping Outputs

**Two tasks sharing output locations causes constant cache invalidation**:

```kotlin
// ❌ WRONG: Shared output directory
tasks.register<ProcessTask>("processA") {
    outputDir.set(layout.buildDirectory.dir("processed"))  // Collision!
}
tasks.register<ProcessTask>("processB") {
    outputDir.set(layout.buildDirectory.dir("processed"))  // Collision!
}

// ✅ CORRECT: Unique outputs per task
tasks.register<ProcessTask>("processA") {
    outputDir.set(layout.buildDirectory.dir("processed/a"))
}
tasks.register<ProcessTask>("processB") {
    outputDir.set(layout.buildDirectory.dir("processed/b"))
}
```

### Make Outputs Deterministic

**Non-deterministic outputs break caching**:

```kotlin
@TaskAction
fun process() {
    // ❌ WRONG: Non-deterministic ordering
    val files = inputDir.get().asFile.listFiles()  // Order may vary!

    // ✅ CORRECT: Sort for deterministic output
    val files = inputDir.get().asFile.listFiles()?.sortedBy { it.name }
}
```

### Don't Use `upToDateWhen`

**This API predates proper input/output handling**:

```kotlin
// ❌ AVOID: Legacy API
tasks.named("myTask") {
    outputs.upToDateWhen { false }  // Only acceptable use: force re-run
}

// ✅ CORRECT: Use proper input/output annotations
@CacheableTask
abstract class MyTask : DefaultTask() {
    @get:Input
    abstract val version: Property<String>  // Proper tracking
}
```

## Configuration Cache

### Never Access Project in Task Actions

**Breaks configuration cache and will be deprecated**:

```kotlin
abstract class MyTask : DefaultTask() {
    // ❌ WRONG: Accessing project in action
    @TaskAction
    fun execute() {
        val name = project.name  // BREAKS CONFIGURATION CACHE!
    }

    // ✅ CORRECT: Declare explicit inputs
    @get:Input
    abstract val projectName: Property<String>

    @TaskAction
    fun execute() {
        val name = projectName.get()  // OK!
    }
}

// When registering:
tasks.register<MyTask>("myTask") {
    projectName.set(project.name)  // Set during configuration
}
```

### Never Access Other Project's Instance

**Cross-project configuration is fragile and breaks isolation**:

```kotlin
// ❌ WRONG: Cross-project configuration
project(":other-module").tasks.named("build")  // FRAGILE!

// ✅ CORRECT: Declare dependencies between projects
dependencies {
    implementation(project(":other-module"))
}
```

## Plugin Public APIs (DSL)

### Use Extensions for Public API

**Don't use Gradle/system properties for plugin configuration**:

```kotlin
// ❌ WRONG: Using properties
val apiKey = project.findProperty("myPlugin.apiKey") as String?

// ✅ CORRECT: Create an extension
abstract class MyPluginExtension {
    abstract val apiKey: Property<String>
    abstract val features: NamedDomainObjectContainer<Feature>
}

// In plugin:
val extension = project.extensions.create<MyPluginExtension>("myPlugin")

// Usage in build.gradle.kts:
myPlugin {
    apiKey.set("secret")
    features {
        register("featureA") {
            enabled.set(true)
        }
    }
}
```

### Use `Action<T>`, Not Kotlin Lambdas

**Gradle enhances bytecode for `Action<T>` to provide better DSL experience**:

```kotlin
// ❌ WRONG: Kotlin lambda
fun configure(block: (Config) -> Unit)

// ✅ CORRECT: Gradle Action
fun configure(action: Action<Config>)
```

### Use Domain Object Containers, Not Lists

**Containers enable enhanced DSL support**:

```kotlin
// ❌ WRONG: Plain list
abstract class MyExtension {
    val features: MutableList<Feature> = mutableListOf()
}

// ✅ CORRECT: Domain object container
abstract class MyExtension {
    abstract val features: NamedDomainObjectContainer<Feature>
}

// Enables DSL:
myPlugin {
    features {
        register("featureA") { /* configure */ }
        register("featureB") { /* configure */ }
    }
}
```

## Testing

### Run Integration Tests with `--warning-mode=fail`

**Catch deprecated API usage early**:

```kotlin
// In plugin test setup
tasks.withType<Test>().configureEach {
    // Make warnings fail the build
    systemProperty("gradle.warning.mode", "fail")
}

// Or in GradleRunner for integration tests
GradleRunner.create()
    .withProjectDir(testProjectDir)
    .withArguments("build", "--warning-mode=fail")
    .build()
```

## Anti-Patterns

| Anti-Pattern                       | Why It's Bad                                    | Alternative                          |
|------------------------------------|-------------------------------------------------|--------------------------------------|
| Using internal APIs                | Can break in any release                        | Copy code or find public APIs        |
| `afterEvaluate`                    | Ordering nightmares                             | Use `Provider`/`Property`            |
| `doFirst`/`doLast` on ad-hoc tasks | No caching, no input/output tracking            | Create custom task classes           |
| `tasks.create`                     | Eager initialization                            | `tasks.register`                     |
| `tasks.all`                        | Eagerly initializes all tasks                   | `tasks.configureEach`                |
| `provider.get()` in configuration  | Breaks lazy evaluation                          | Use `map`/`flatMap`                  |
| Accessing `project` in task action | Breaks configuration cache                      | Declare explicit `@Input` properties |
| Cross-project configuration        | Fragile, breaks isolation                       | Use dependencies                     |
| Overlapping task outputs           | Cache invalidation                              | Unique output paths per task         |
| Kotlin lambdas in DSL              | Loses Gradle bytecode enhancement               | Use `Action<T>`                      |
| Lists in extensions                | No DSL enhancement                              | Use `NamedDomainObjectContainer`     |
| `outputs.upToDateWhen`             | Legacy API, bypasses proper input/output system | Use proper annotations               |

## Commands

```bash
# Run with latest Gradle wrapper
./gradlew wrapper --gradle-version=9.2.1

# Check for deprecated API usage
./gradlew build --warning-mode=fail

# Analyze dependencies
./gradlew buildHealth  # requires dependency-analysis plugin

# Show task dependencies
./gradlew myTask --dry-run

# Debug configuration cache
./gradlew build --configuration-cache

# Profile build
./gradlew build --scan
```

## Resources

- [Gradle Lazy Configuration](https://docs.gradle.org/current/userguide/lazy_configuration.html)
- [Gradle Custom Tasks](https://docs.gradle.org/current/userguide/custom_tasks.html)
- [Gradle Configuration Cache](https://docs.gradle.org/current/userguide/configuration_cache.html)
- [Gradle Build Cache](https://docs.gradle.org/current/userguide/build_cache.html)
- [Version Catalogs](https://docs.gradle.org/current/userguide/platforms.html)
- [Tony's Rules for Gradle Plugin Authors](https://dev.to/autonomousapps/tonys-rules-for-gradle-plugin-authors-28k3)
- [Original Best Practices](https://github.com/liutikas/gradle-best-practices)
