---
title: Personalización de la Plantilla
description: Guía de referencia para adaptar la identidad del proyecto Corvus, metadatos de publicación y configuración del sitio de documentación.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: guide
---

Este repositorio está construido sobre una base Gradle multiplataforma y personalizado para **Corvus**.

## Identidad del Proyecto

Nombre del proyecto raíz en `settings.gradle.kts`:

```kotlin
rootProject.name = "corvus"
```

Metadatos de publicación en `gradle.properties`:

```properties
GROUP=com.profiletailors
VERSION=3.0.0
POM_DEVELOPER_NAME=corvus-team
POM_URL=https://github.com/dallay/corvus
POM_SCM_CONNECTION=scm:git:https://github.com/dallay/corvus.git
POM_LICENSE_URL=https://www.apache.org/licenses/LICENSE-2.0
```

## Namespace de Paquetes

Corvus mantiene actualmente el namespace `com.profiletailors` para los plugin IDs de Gradle y
módulos Kotlin, manteniendo compatibilidad mientras evoluciona la arquitectura. Los nuevos
módulos se incluyen mediante el helper `includeProjects` en `settings.gradle.kts`.

## Arquitectura Actual

Corvus es una plataforma agénticas con estos componentes principales:

- **Rust Agent Runtime** (`clients/agent-runtime`) — Ejecución autónoma de agentes con 22+
  proveedores de IA, 14 canales de comunicación, 32+ herramientas, periféricos de hardware,
  programación cron, enrutamiento de modelos y arquitectura basada en traits.
- **Clientes Kotlin Multiplatform** (`clients/composeApp`, `modules/agent-core-kmp`) — UI
  Compose compartida para desktop, Android e iOS con un core bootstrap común.
- **Aplicaciones Web** (`clients/web`) — Apps Astro/Vue incluyendo el dashboard de operador,
  sitio de documentación y páginas de marketing.
- **Servicio de Memoria Cerebro** (`modules/cerebro`) — Servicio de memoria MCP independiente
  con SurrealDB embebido, 13 herramientas de memoria y dashboard TUI opcional.

## Sitio de Documentación

La configuración se encuentra en `clients/web/apps/docs/astro.config.mjs`:

- `site`: URL del sitio de documentación
- `base`: `/corvus`
- `starlight.title`: `Corvus`
- Los enlaces del repositorio apuntan a `https://github.com/dallay/corvus`

Para personalizar la documentación, edita archivos bajo
`clients/web/apps/docs/src/content/docs/`. Cada cambio en el root en inglés debe reflejarse
en `es/` para mantener paridad bilingüe.

## CI/CD y Repositorio

Revisa y personaliza:

1. `.github/workflows/` — Pipelines CI para Kotlin, Rust y web.
2. `.github/CODEOWNERS` — Reglas de propiedad de código.
3. `README.md` — Resumen del proyecto.
4. Enlaces de release en páginas de documentación.

## Agregar Nuevos Módulos

Los nuevos módulos Gradle se registran en `settings.gradle.kts` mediante el helper `includeProjects`:

```kotlin
includeProjects(
  mapOf(
    ":androidApp" to "clients/androidApp",
    ":web" to "clients/web",
    ":composeApp" to "clients/composeApp",
    ":agent-runtime" to "clients/agent-runtime",
    ":agent-core-kmp" to "modules/agent-core-kmp",
    // Agrega nuevos módulos aquí:
    // ":mi-modulo" to "modules/mi-modulo",
  )
)
```

Coloca los módulos Kotlin bajo `modules/` y las aplicaciones cliente bajo `clients/`.
