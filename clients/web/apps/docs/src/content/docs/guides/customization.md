---
title: Template Customization
description: Reference guide for tailoring the Corvus project identity, publishing metadata, and documentation site configuration.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: guide
---

This repository is built on a Gradle multiplatform foundation and customized for **Corvus**.

## Project Identity

Root project name in `settings.gradle.kts`:

```kotlin
rootProject.name = "corvus"
```

Publishing metadata in `gradle.properties`:

```properties
GROUP=com.profiletailors
VERSION=3.0.0
POM_DEVELOPER_NAME=corvus-team
POM_URL=https://github.com/dallay/corvus
POM_SCM_CONNECTION=scm:git:https://github.com/dallay/corvus.git
POM_LICENSE_URL=https://www.apache.org/licenses/LICENSE-2.0
```

## Package Namespace

Corvus currently keeps the `com.profiletailors` namespace for Gradle plugin IDs and Kotlin
modules to maintain compatibility while the architecture evolves. New modules are included via
the `includeProjects` helper in `settings.gradle.kts`.

## Current Architecture

Corvus is an agentic platform with these core components:

- **Rust Agent Runtime** (`clients/agent-runtime`) — Autonomous agent execution with 22+ AI
  providers, 14 communication channels, 32+ tools, hardware peripherals, cron scheduling, model
  routing, and a trait-driven extension architecture.
- **Kotlin Multiplatform Clients** (`clients/composeApp`, `modules/agent-core-kmp`) — Shared
  Compose UI for desktop, Android, and iOS with a common core bootstrap.
- **Web Applications** (`clients/web`) — Astro/Vue web apps including the operator dashboard,
  documentation site, and marketing pages.
- **Cerebro Memory Service** (`clients/cerebro`) — Standalone MCP memory service client with embedded
  SurrealDB, 13 memory tools, and optional TUI dashboard.

## Documentation Site

Configuration lives in `clients/web/apps/docs/astro.config.mjs`:

- `site`: Documentation site URL
- `base`: `/corvus`
- `starlight.title`: `Corvus`
- Repository links point to `https://github.com/dallay/corvus`

To customize the docs site, edit files under
`clients/web/apps/docs/src/content/docs/`. Every change in the English root must be mirrored in
`es/` for bilingual parity.

## CI/CD and Repository

Review and customize:

1. `.github/workflows/` — CI pipelines for Kotlin, Rust, and web.
2. `.github/CODEOWNERS` — Code ownership rules.
3. `README.md` — Top-level project overview.
4. Release links in documentation pages.

## Adding New Modules

New Gradle modules are registered in `settings.gradle.kts` via the `includeProjects` helper:

```kotlin
includeProjects(
  mapOf(
    ":androidApp" to "clients/androidApp",
    ":web" to "clients/web",
    ":composeApp" to "clients/composeApp",
    ":agent-runtime" to "clients/agent-runtime",
    ":agent-core-kmp" to "modules/agent-core-kmp",
    // Add new modules here:
    // ":my-module" to "modules/my-module",
  )
)
```

Place Kotlin modules under `modules/` and client applications under `clients/`.
