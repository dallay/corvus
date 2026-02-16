---
title: Arquitectura
---

El proyecto sigue una arquitectura modular con un fuerte énfasis en la lógica de construcción centralizada.

## Estructura del Proyecto

```text
.
├── apps/
│   ├── app/            # Aplicación backend principal
│   └── docs/           # Documentación (sitio Starlight)
├── modules/
│   ├── agent-core-kmp/ # Base compartida KMP
│   └── agent-core-rust/# Núcleo de IA en Rust
├── gradle/
│   ├── build-logic/    # Plugins de convención personalizados de Gradle
│   ├── libs.versions.toml # Catálogo de versiones
│   └── wrapper/        # Wrapper de Gradle
├── Makefile            # Interfaz de comandos estandarizada
└── settings.gradle.kts # Configuración global del proyecto
```

## Lógica de Construcción (Plugins de Convención)

En lugar de repetir la lógica de construcción en cada `build.gradle.kts`, utilizamos **Plugins de Convención** ubicados en `gradle/build-logic`.

### Categorías de Plugins

1. **Plugins Base**: Configuración fundamental como identidad, ciclo de vida y resolución de conflictos de JVM.
2. **Plugins de Módulo**: Configuraciones específicas del lenguaje (`kotlin`, `java`, `spring-boot`).
3. **Plugins de Funcionalidad**: Funcionalidades opcionales como `publish-library`, `shadow`, `test-fixtures` y `git-hook`.
4. **Plugins de Verificación**: Herramientas de calidad de código y formateo (`spotless`, `detekt`, `spotbugs`).
5. **Plugins de Reporte**: Reportes agregados para pruebas, cobertura y SBOM.

## Gestión de Dependencias

Utilizamos los **Catálogos de Versiones de Gradle** (`libs.versions.toml`) para definir todas las dependencias y versiones en un solo lugar. Esto asegura la consistencia en todos los módulos.

### Ejemplo de uso:

```kotlin
dependencies {
    implementation(libs.kotlin.stdlib)
    testImplementation(libs.junit.jupiter)
}
```
