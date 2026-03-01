---
title: Estructura del Proyecto
---

Una mirada detallada a la organización del repositorio **Corvus**.

## Directorio Raíz

- `clients/`: Aplicaciones cliente (Android, iOS, Web, runtime del agente).
- `modules/`: Módulos compartidos y reutilizables (core del agente).
- `gradle/`: Configuraciones específicas de Gradle y lógica de construcción.
- `docs/`: Documentación del proyecto (sitio Astro + Starlight).
- `Makefile`: Comandos estandarizados para tareas comunes.
- `settings.gradle.kts`: Define la jerarquía del proyecto e incluye los módulos.
- `README.md`: Descripción general del proyecto a alto nivel.
- `AGENTS.md`: Instrucciones especializadas para agentes de IA.

## El Directorio `clients`

Contiene todas las aplicaciones cliente que consumen los módulos compartidos:

- `clients/composeApp`: Módulo UI compartido en Compose Kotlin Multiplatform.
- `clients/androidApp`: App wrapper nativa de Android conectada al módulo Compose compartido.
- `clients/iosApp`: App wrapper nativa de iOS conectada al framework Compose compartido.
- `clients/web`: Aplicación web y panel de operadores.
- `clients/agent-runtime`: Runtime del agente para ejecución de tareas.

## El Directorio `modules`

- `modules/agent-core-kmp`: Base compartida en Kotlin Multiplatform para el núcleo del agente.
  Lógica de negocio, modelos de dominio, y contratos reutilizables entre todas las plataformas.

## El Directorio `gradle`

- **`build-logic/`**: Contiene plugins de convención personalizados escritos en Kotlin. Es el "
  cerebro" del sistema de construcción.
- **`libs.versions.toml`**: El catálogo de versiones central para gestionar las dependencias.
- **`aggregation/`**: Módulo utilizado para agregar reportes de pruebas y cobertura de todos los
  submódulos.
- **`versions/`**: Módulo dedicado a la gestión de versiones y comprobación de consistencia del
  catálogo.
- **`wrapper/`**: Contiene los archivos del wrapper de Gradle, asegurando entornos de construcción
  consistentes.
- **`configs/`**: Configuraciones adicionales de herramientas (Detekt, Spotless, etc.).

## El Directorio `docs`

- **`docs/es/`**: Documentación en español.
  - `index.mdx`: Página de inicio.
  - `guides/`: Guías detalladas del proyecto.
- **`docs/en/`**: Documentación en inglés (si aplica).
