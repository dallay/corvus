---
title: Project Structure
---

A detailed look at the organization of the **Corvus** repository.

## Root Directory

- `apps/`: Standalone applications (backend services, web apps, mobile apps, docs web).
- `modules/`: Reusable shared modules and cores.
- `gradle/`: Gradle-specific configurations and build logic.
- `Makefile`: Standardized commands for common tasks.
- `settings.gradle.kts`: Defines the project hierarchy and includes modules.
- `README.md`: High-level project overview.
- `AGENTS.md`: Specialized instructions for AI agents.

## The `apps` Directory

- `apps/composeApp`: Shared Kotlin Multiplatform Compose UI module.
- `apps/androidApp`: Native Android wrapper app wired to the shared Compose module.
- `apps/iosApp`: Native iOS wrapper app wired to the shared Compose framework.
- `apps/docs`: Documentation website module (Astro + Starlight).

## The `modules` Directory

- `modules/agent-core-kmp`: Shared Kotlin Multiplatform bootstrap for the future agent core.
- `apps/agent-runtime-rust`: Rust agent runtime app imported from corvus.

## The `gradle` Directory

- **`build-logic`**: Contains custom convention plugins written in Kotlin. This is the "brain" of the build system.
- **`libs.versions.toml`**: The central version catalog for managing dependencies.
- **`aggregation`**: Module used to aggregate test and coverage reports from all submodules.
- **`versions`**: Module dedicated to version management and catalog consistency checks.
- **`wrapper`**: Contains the Gradle wrapper files, ensuring consistent build environments.

## The `apps/docs` Directory

- **`website`**: Source code for this documentation site, built with Astro and Starlight.
