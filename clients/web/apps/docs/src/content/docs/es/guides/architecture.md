---
title: Arquitectura
---

El proyecto sigue una arquitectura modular con un fuerte énfasis en la lógica de construcción
centralizada y separación clara entre clientes y módulos compartidos.

## Estructura del Proyecto

```text
.
├── clients/                    # Aplicaciones cliente
│   ├── agent-runtime/          # Runtime del agente
│   ├── androidApp/             # App host de Android
│   ├── composeApp/             # Módulo compartido Compose Multiplatform
│   ├── iosApp/                 # App host de iOS (proyecto Xcode)
│   └── web/                    # Aplicación web y panel de operadores
├── modules/                    # Módulos compartidos
│   └── agent-core-kmp/         # Core del agente en Kotlin Multiplatform
├── gradle/                     # Configuración de construcción
│   ├── build-logic/            # Plugins de convención personalizados
│   ├── aggregation/            # Agregación de reportes
│   ├── configs/                # Configuraciones de herramientas
│   ├── versions/               # Gestión de versiones
│   ├── libs.versions.toml      # Catálogo de versiones
│   └── wrapper/                # Wrapper de Gradle
├── docs/                       # Documentación (Astro + Starlight)
├── Makefile                    # Interfaz de comandos estandarizada
└── settings.gradle.kts         # Configuración global del proyecto
```

## Arquitectura de Alto Nivel

Corvus está diseñado como una plataforma de agentes reactivos con los siguientes pilares:

### 1. **Orquestador Reactivo**

- **Tecnología**: Kotlin + Spring Boot + Coroutines/WebFlux
- **Propósito**: Manejar flujos de trabajo no bloqueantes y always-on
- **Ubicación**: Runtime del agente en `clients/agent-runtime/`

### 2. **Memoria de Grafo**

- **Tecnología**: Neo4j (planificado)
- **Propósito**: Modelo de conocimiento con contexto conectado y memoria durable
- **Integración**: A través de `agent-core-kmp`

### 3. **Sidecars de Alto Rendimiento**

- **Tecnología**: Rust (planificado)
- **Propósito**: Operaciones de scraping y ejecución sandboxed de alta performance
- **Comunicación**: FFI o gRPC con el runtime Kotlin

### 4. **Panel de Control**

- **Tecnología**: Astro + Vue (planificado)
- **Propósito**: Observabilidad en tiempo real y operación transparente
- **Ubicación**: `clients/web/`

## Lógica de Construcción (Plugins de Convención)

En lugar de repetir la lógica de construcción en cada `build.gradle.kts`, utilizamos **Plugins de
Convención** ubicados en `gradle/build-logic`.

### Categorías de Plugins

1. **Plugins Base**: Configuración fundamental como identidad, ciclo de vida y resolución de
   conflictos de JVM.
2. **Plugins de Módulo**: Configuraciones específicas del lenguaje (`kotlin`, `java`, `spring-boot`,
   `compose`).
3. **Plugins de Funcionalidad**: Funcionalidades opcionales como `publish-library`, `shadow`,
   `test-fixtures` y `git-hook`.
4. **Plugins de Verificación**: Herramientas de calidad de código y formateo (`spotless`, `detekt`,
   `spotbugs`).
5. **Plugins de Reporte**: Reportes agregados para pruebas, cobertura y SBOM.

## Gestión de Dependencias

Utilizamos los **Catálogos de Versiones de Gradle** (`libs.versions.toml`) para definir todas las
dependencias y versiones en un solo lugar. Esto asegura la consistencia en todos los módulos.

### Ejemplo de uso:

```kotlin
dependencies {
    implementation(libs.kotlin.stdlib)
    testImplementation(libs.junit.jupiter)
}
```

## Flujo de Dependencias entre Módulos

```
┌─────────────────────────────────────────────────────────────┐
│                      clients/composeApp                      │
│              (UI compartida Compose Multiplatform)           │
└────────────────────┬────────────────────────────────────────┘
                     │ usa
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    modules/agent-core-kmp                    │
│         (Lógica de negocio, dominio, contratos)              │
└────────────────────┬────────────────────────────────────────┘
                     │ provee a
                     ▼
    ┌────────────────┴────────────────┐
    │                                  │
    ▼                                  ▼
┌──────────────┐            ┌────────────────────┐
│clients/      │            │clients/            │
│androidApp    │            │iosApp              │
└──────────────┘            └────────────────────┘
```

## Diagramas de Arquitectura C4

Para una vista más detallada de la arquitectura, consulta los siguientes diagramas C4:

| Nivel             | Diagrama                                                                            | Descripción                                        | ID del Diagrama                   |
|-------------------|-------------------------------------------------------------------------------------|----------------------------------------------------|-----------------------------------|
| C1 - Contexto     | [Sistema Completo](./architecture/diagrams/context/system-context.mmd)                 | Vista de alto nivel del runtime y actores externos | `context/system-context.mmd`      |
| C2 - Contenedores | [Contenedores del Runtime](./diagrams/container/runtime-containers.mmd) | Servicios del runtime y superficies operativas     | `container/runtime-containers.mmd` |
| C3 - Componentes  | [Núcleo del Runtime](./diagrams/component/runtime-core.mmd)             | Componentes internos del núcleo del runtime        | `component/runtime-core.mmd`      |
| -                 | [Dependencias de Cargo](./diagrams/cargo-dependencies.mmd)              | Flujo de dependencias del workspace Rust/Cargo     | `cargo-dependencies.mmd`          |

Ver [Visión General de la Arquitectura](./overview) para más detalles sobre cómo
visualizarlos.
