---
title: Template Customization
---

This repository was created from a Gradle template and customized for **Corvus**.

## Project Identity

Update root identity in `settings.gradle.kts`:

```kotlin
rootProject.name = "corvus"
```

Update publishing metadata in `gradle.properties`:

```properties
GROUP=com.corvus
VERSION=0.1.0-SNAPSHOT
POM_DEVELOPER_NAME=corvus-team
POM_URL=https://github.com/dallay/corvus
POM_SCM_CONNECTION=scm:git:https://github.com/dallay/corvus.git
POM_LICENSE_URL=https://www.apache.org/licenses/LICENSE-2.0
```

## Package Namespace

Corvus currently **keeps the existing package namespace** `com.profiletailors` for runtime code and
build-logic plugin IDs to avoid breaking compatibility while architecture evolves.

## Platform Direction

Corvus architecture targets:

- Kotlin + Spring Boot (WebFlux + Coroutines) for orchestration.
- Neo4j for graph memory.
- Rust sidecars for performance-critical and sandboxed tasks.
- Astro + Vue for control-plane visibility.

## Documentation Site

Customize `apps/docs/website/astro.config.mjs`:

- `base`: `/corvus`
- `starlight.title`: `Corvus`
- Repository links: `https://github.com/dallay/corvus`

## CI/CD and Repository

Review and customize:

1. `.github/workflows/`
2. `.github/CODEOWNERS`
3. `README.md`
4. Release links in docs

## Suggested Incremental Path

1. Stabilize identity and publishing metadata.
2. Keep package namespace unchanged while adding new Corvus modules.
3. Introduce graph-memory and reasoning modules behind clear boundaries.
4. Add sidecar interfaces and observability endpoints.
