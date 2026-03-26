---
title: Personalización de la Plantilla
description: Guía de referencia para adaptar la plantilla original de Gradle a la identidad, publicación y ajustes específicos de Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: guide
---

Este repositorio fue creado desde una plantilla de Gradle y personalizado para **Corvus**.

## Identidad del Proyecto

Actualiza la identidad raíz en `settings.gradle.kts`:

```kotlin
rootProject.name = "corvus"
```

Actualiza metadatos de publicación en `gradle.properties`:

```properties
GROUP=com.profiletailors
VERSION=0.1.0-SNAPSHOT
POM_DEVELOPER_NAME=corvus-team
POM_URL=https://github.com/dallay/corvus
POM_SCM_CONNECTION=scm:git:https://github.com/dallay/corvus.git
POM_LICENSE_URL=https://www.apache.org/licenses/LICENSE-2.0
```

## Namespace de Paquetes

Corvus actualmente **mantiene el namespace existente** `com.profiletailors` para el código de
runtime y los plugin IDs de build-logic, evitando romper compatibilidad mientras evoluciona la
arquitectura.

## Dirección de Plataforma

La arquitectura objetivo de Corvus:

- Kotlin + Spring Boot (WebFlux + Coroutines) para orquestación.
- Neo4j para memoria de grafo.
- Sidecars en Rust para tareas críticas y sandbox.
- Astro + Vue para visibilidad del control plane.

## Sitio de Documentación

Personaliza `clients/web/apps/docs/astro.config.mjs`:

- `base`: `/corvus`
- `starlight.title`: `Corvus`
- Links de repositorio: `https://github.com/dallay/corvus`

## CI/CD y Repositorio

Revisa y personaliza:

1. `.github/workflows/`
2. `.github/CODEOWNERS`
3. `README.md`
4. Links de release en docs

## Ruta Incremental Sugerida

1. Estabilizar identidad y metadatos de publicación.
2. Mantener el namespace de paquetes mientras agregas módulos Corvus.
3. Introducir módulos de memoria de grafo y razonamiento con fronteras claras.
4. Añadir interfaces de sidecar y endpoints de observabilidad.
