---
title: Configuration Options
---

The project is highly configurable through Gradle properties and version catalogs.

## Version Catalog (`libs.versions.toml`)

This file contains the versions of all tools and dependencies used in the project.

### Key Versions

- **JDK**: Target Java version (default 21).
- **Gradle**: Build system version.
- **Kotlin**: Kotlin compiler and standard library version.
- **Node**: Required for documentation builds and other JS tools.

### Managing Dependencies

Dependencies are grouped into:
- `versions`: Single source of truth for version numbers.
- `libraries`: Definitions of individual dependencies.
- `bundles`: Groups of dependencies that are often used together.
- `plugins`: Gradle plugins used in the project.

## Gradle Properties

Global build settings can be found in `gradle.properties`. This includes configuration for the Gradle daemon, parallel execution, and caching.

## Environment Variables

Some features might require environment variables, especially for CI/CD or specialized tasks (e.g., GPG keys for signing, repository credentials for publishing).
