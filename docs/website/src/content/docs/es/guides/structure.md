---
title: Estructura del Proyecto
---

Una mirada detallada a la organización del repositorio **Starter Gradle**.

## Directorio Raíz

- `app/`: El módulo principal de la aplicación.
- `docs/`: Código fuente de la documentación y sitio web estático.
- `examples/`: Plantillas para diferentes lenguajes y frameworks.
- `gradle/`: Configuraciones específicas de Gradle y lógica de construcción.
- `Makefile`: Comandos estandarizados para tareas comunes.
- `settings.gradle.kts`: Define la jerarquía del proyecto e incluye los módulos.
- `README.md`: Descripción general del proyecto a alto nivel.
- `AGENTS.md`: Instrucciones especializadas para agentes de IA.

## El Módulo `app`

Aquí es donde reside la lógica principal de la aplicación. Está configurado para usar Kotlin por defecto, pero se puede adaptar fácilmente.

## El Directorio `gradle`

- **`build-logic`**: Contiene plugins de convención personalizados escritos en Kotlin. Es el "cerebro" del sistema de construcción.
- **`libs.versions.toml`**: El catálogo de versiones central para gestionar las dependencias.
- **`aggregation`**: Módulo utilizado para agregar reportes de pruebas y cobertura de todos los submódulos.
- **`versions`**: Módulo dedicado a la gestión de versiones y comprobación de consistencia del catálogo.
- **`wrapper`**: Contiene los archivos del wrapper de Gradle, asegurando entornos de construcción consistentes.

## El Directorio `examples`

Proporciona ejemplos listos para usar para:
- **Java**: Aplicación JVM estándar usando Java.
- **Kotlin**: Aplicación JVM moderna usando Kotlin.
- **Spring**: Una configuración robusta de aplicación Spring Boot.

## El Directorio `docs`

- **`website`**: Código fuente de este sitio de documentación, construido con Astro y Starlight.
- **`GPG_SETUP.md`**: Instrucciones para configurar la firma de artefactos.
