---
title: Opciones de Configuración
---

El proyecto es altamente configurable a través de propiedades de Gradle y catálogos de versiones.

## Catálogo de versiones (`libs.versions.toml`)

Este archivo contiene las versiones de todas las herramientas y dependencias utilizadas en el proyecto.

### Versiones clave

- **JDK**: Versión de Java de destino (por defecto 21).
- **Gradle**: Versión del sistema de construcción.
- **Kotlin**: Versión del compilador y la biblioteca estándar de Kotlin.
- **Node**: Requerido para la construcción de la documentación y otras herramientas JS.

### Gestión de dependencias

Las dependencias se agrupan en:
- `versions`: Fuente única de verdad para los números de versión.
- `libraries`: Definiciones de dependencias individuales.
- `bundles`: Grupos de dependencias que a menudo se usan juntas.
- `plugins`: Plugins de Gradle utilizados en el proyecto.

## Propiedades de Gradle

La configuración global de la construcción se encuentra en `gradle.properties`. Esto incluye la configuración del demonio de Gradle, la ejecución en paralelo y el almacenamiento en caché.

## Variables de entorno

Algunas funcionalidades pueden requerir variables de entorno, especialmente para CI/CD o tareas especializadas (por ejemplo, llaves GPG para la firma, credenciales de repositorio para la publicación).
