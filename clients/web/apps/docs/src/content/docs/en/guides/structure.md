---
title: Project Structure
---

A detailed look at the organization of the **Corvus** repository.

## Root Directory

- `clients/`: Client applications (Android, iOS, Web, agent runtime).
- `modules/`: Reusable shared modules and cores (agent core).
- `gradle/`: Gradle-specific configurations and build logic.
- `docs/`: Project documentation (Astro + Starlight website).
- `Makefile`: Standardized commands for common tasks.
- `settings.gradle.kts`: Defines the project hierarchy and includes modules.
- `README.md`: High-level project overview.
- `AGENTS.md`: Specialized instructions for AI agents.

## The `clients` Directory

Contains all client applications that consume the shared modules:

- `clients/composeApp`: Shared Kotlin Multiplatform Compose UI module.
- `clients/androidApp`: Native Android wrapper app wired to the shared Compose module.
- `clients/iosApp`: Native iOS wrapper app wired to the shared Compose framework.
- `clients/web`: Web application and operator dashboard.
- `clients/agent-runtime`: Agent runtime for task execution.

## The `modules` Directory

- `modules/agent-core-kmp`: Shared Kotlin Multiplatform core for the agent.
  Business logic, domain models, and reusable contracts across all platforms.

## The `gradle` Directory

- **`build-logic/`**: Contains custom convention plugins written in Kotlin. This is the "brain" of
  the build system.
- **`libs.versions.toml`**: The central version catalog for managing dependencies.
- **`aggregation/`**: Module used to aggregate test and coverage reports from all submodules.
- **`versions/`**: Module dedicated to version management and catalog consistency checks.
- **`wrapper/`**: Contains the Gradle wrapper files, ensuring consistent build environments.
- **`configs/`**: Additional tool configurations (Detekt, Spotless, etc.).

## The `docs` Directory

- **`docs/es/`**: Documentation in Spanish.
  - `index.mdx`: Home page.
  - `guides/`: Detailed project guides.
- **`docs/en/`**: Documentation in English.
