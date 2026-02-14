---
title: Features Checklist
---

This page provides a comprehensive checklist of all functionalities, modules, and options available in this repository.

## Modules

- [x] **app**: Main application module (Kotlin).
- [x] **docs**: Documentation website (Starlight/Astro).
- [x] **examples/example-java**: Java application example.
- [x] **examples/example-kotlin**: Kotlin application example.
- [x] **examples/example-spring**: Spring Boot application example.
- [x] **gradle/build-logic**: Centralized convention plugins.
- [x] **gradle/aggregation**: Aggregated reporting for tests and coverage.
- [x] **gradle/versions**: Dependency version management and consistency checks.

## Build Functionalities

- [x] **Convention Plugins**: Modular and reusable build logic.
- [x] **Version Catalog**: Centralized dependency management in `libs.versions.toml`.
- [x] **Dependency Analysis**: Tools to detect unused or misconfigured dependencies.
- [x] **Reproducible Builds**: Dependency locking with Gradle lockfiles.
- [x] **Multi-language Support**: Seamless integration for Java and Kotlin.

## Quality & Maintenance

- [x] **Code Formatting**: Automated formatting with Spotless.
- [x] **Static Analysis**:
    - [x] Detekt (Kotlin)
    - [x] SpotBugs (Java)
    - [x] PMD (Java)
    - [x] Checkstyle (Java)
    - [x] NullAway (Java)
- [x] **Testing**:
    - [x] JUnit 5 support.
    - [x] Code coverage with Kover.
- [x] **SBOM**: Software Bill of Materials generation.
- [x] **Git Hooks**: Automated pre-commit checks.

## Documentation

- [x] **Static Website**: Built with Astro and Starlight.
- [x] **API Docs**: Generated with Dokka (Kotlin/Java).
- [x] **README/AGENTS**: In-repo documentation for developers and agents.

## Deployment & Distribution

- [x] **Shadow JAR**: Executable fat-jars with bundled dependencies.
- [x] **Maven Publishing**: Pre-configured publishing to Maven repositories.
- [x] **BOM Support**: Bill of Materials for dependency alignment.
