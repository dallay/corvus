---
title: Architecture
---

The project follows a modular architecture with a strong emphasis on centralized build logic.

## Project Structure

```text
.
├── app/                # Main application
├── docs/               # Documentation (Starlight website)
├── examples/           # Multi-language examples
│   ├── example-java
│   ├── example-kotlin
│   └── example-spring
├── gradle/
│   ├── build-logic/    # Custom Gradle convention plugins
│   ├── libs.versions.toml # Version catalog
│   └── wrapper/        # Gradle wrapper
├── Makefile            # Standardized command interface
└── settings.gradle.kts # Global project configuration
```

## Build Logic (Convention Plugins)

Instead of repeating build logic in every `build.gradle.kts`, we use **Convention Plugins** located in `gradle/build-logic`.

### Plugin Categories

1. **Base Plugins**: Fundamental configuration like identity, lifecycle, and JVM conflict resolution.
2. **Module Plugins**: Language-specific configurations (`kotlin`, `java`, `spring-boot`).
3. **Feature Plugins**: Opt-in features like `publish-library`, `shadow`, `test-fixtures`, and `git-hook`.
4. **Check Plugins**: Code quality and formatting tools (`spotless`, `detekt`, `spotbugs`).
5. **Report Plugins**: Aggregated reports for testing, coverage, and SBOM.

## Dependency Management

We use **Gradle Version Catalogs** (`libs.versions.toml`) to define all dependencies and versions in a single location. This ensures consistency across all modules and examples.

### Example usage:

```kotlin
dependencies {
    implementation(libs.kotlin.stdlib)
    testImplementation(libs.junit.jupiter)
}
```
