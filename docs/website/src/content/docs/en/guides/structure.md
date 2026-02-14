---
title: Project Structure
---

A detailed look at the organization of the **Starter Gradle** repository.

## Root Directory

- `app/`: The primary application module.
- `docs/`: Documentation source code and static website.
- `examples/`: Boilerplate for different languages and frameworks.
- `gradle/`: Gradle-specific configurations and build logic.
- `Makefile`: Standardized commands for common tasks.
- `settings.gradle.kts`: Defines the project hierarchy and includes modules.
- `README.md`: High-level project overview.
- `AGENTS.md`: Specialized instructions for AI agents.

## The `app` Module

This is where the main application logic resides. It is configured to use Kotlin by default but can be easily adapted.

## The `gradle` Directory

- **`build-logic`**: Contains custom convention plugins written in Kotlin. This is the "brain" of the build system.
- **`libs.versions.toml`**: The central version catalog for managing dependencies.
- **`aggregation`**: Module used to aggregate test and coverage reports from all submodules.
- **`versions`**: Module dedicated to version management and catalog consistency checks.
- **`wrapper`**: Contains the Gradle wrapper files, ensuring consistent build environments.

## The `examples` Directory

Provides ready-to-use examples for:
- **Java**: Standard JVM application using Java.
- **Kotlin**: Modern JVM application using Kotlin.
- **Spring**: A robust Spring Boot application setup.

## The `docs` Directory

- **`website`**: Source code for this documentation site, built with Astro and Starlight.
- **`GPG_SETUP.md`**: Instructions for setting up artifact signing.
