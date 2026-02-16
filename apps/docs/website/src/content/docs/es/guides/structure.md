---
title: Estructura del Proyecto
---

Una mirada detallada a la organización del repositorio **Corvus**.

## Directorio Raíz

- `apps/`: Aplicaciones standalone (backend, web, móvil, docs web).
- `modules/`: Módulos compartidos y reutilizables.
- `gradle/`: Configuraciones específicas de Gradle y lógica de construcción.
- `Makefile`: Comandos estandarizados para tareas comunes.
- `settings.gradle.kts`: Define la jerarquía del proyecto e incluye los módulos.
- `README.md`: Descripción general del proyecto a alto nivel.
- `AGENTS.md`: Instrucciones especializadas para agentes de IA.

## El Directorio `apps`

- `apps/app`: Módulo principal de la aplicación backend.
- `apps/docs`: Módulo del sitio de documentación (Astro + Starlight).

## El Directorio `modules`

- `modules/agent-core-kmp`: Base compartida en Kotlin Multiplatform para el núcleo del agente.
- `modules/agent-core-rust`: Núcleo de IA en Rust importado desde ZeroClaw.

## El Directorio `gradle`

- **`build-logic`**: Contiene plugins de convención personalizados escritos en Kotlin. Es el "cerebro" del sistema de construcción.
- **`libs.versions.toml`**: El catálogo de versiones central para gestionar las dependencias.
- **`aggregation`**: Módulo utilizado para agregar reportes de pruebas y cobertura de todos los submódulos.
- **`versions`**: Módulo dedicado a la gestión de versiones y comprobación de consistencia del catálogo.
- **`wrapper`**: Contiene los archivos del wrapper de Gradle, asegurando entornos de construcción consistentes.

## El Directorio `apps/docs`

- **`website`**: Código fuente de este sitio de documentación, construido con Astro y Starlight.
