---
title: Lista de Funcionalidades
---

Esta página proporciona una lista completa de todas las funcionalidades, módulos y opciones disponibles en este repositorio.

## Módulos

- [x] **app**: Módulo principal de la aplicación (Kotlin).
- [x] **docs**: Sitio web de documentación (Starlight/Astro).
- [x] **examples/example-java**: Ejemplo de aplicación en Java.
- [x] **examples/example-kotlin**: Ejemplo de aplicación en Kotlin.
- [x] **examples/example-spring**: Ejemplo de aplicación con Spring Boot.
- [x] **gradle/build-logic**: Plugins de convención centralizados.
- [x] **gradle/aggregation**: Reportes agregados para pruebas y cobertura.
- [x] **gradle/versions**: Gestión de versiones de dependencias y comprobaciones de consistencia.

## Funcionalidades de Construcción

- [x] **Plugins de Convención**: Lógica de construcción modular y reutilizable.
- [x] **Catálogo de Versiones**: Gestión centralizada de dependencias en `libs.versions.toml`.
- [x] **Análisis de Dependencias**: Herramientas para detectar dependencias no utilizadas o mal configuradas.
- [x] **Construcciones Reproducibles**: Bloqueo de dependencias con lockfiles de Gradle.
- [x] **Soporte Multilenguaje**: Integración fluida para Java y Kotlin.

## Calidad y Mantenimiento

- [x] **Formateo de Código**: Formateo automático con Spotless.
- [x] **Análisis Estático**:
    - [x] Detekt (Kotlin)
    - [x] SpotBugs (Java)
    - [x] PMD (Java)
    - [x] Checkstyle (Java)
    - [x] NullAway (Java)
- [x] **Pruebas**:
    - [x] Soporte para JUnit 5.
    - [x] Cobertura de código con Kover.
- [x] **SBOM**: Generación de Software Bill of Materials.
- [x] **Git Hooks**: Comprobaciones automáticas de pre-commit.

## Documentación

- [x] **Sitio Web Estático**: Construido con Astro y Starlight.
- [x] **Documentación de API**: Generada con Dokka (Kotlin/Java).
- [x] **README/AGENTS**: Documentación interna para desarrolladores y agentes.

## Despliegue y Distribución

- [x] **Shadow JAR**: Jars ejecutables "fat" con dependencias incluidas.
- [x] **Publicación en Maven**: Publicación preconfigurada en repositorios Maven.
- [x] **Soporte para BOM**: Bill of Materials para la alineación de dependencias.
