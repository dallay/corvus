---
title: Descripción de Arquitectura
description: Colección de diagramas C4 para el proyecto Corvus
---

# Diagramas de Arquitectura

Esta sección contiene los diagramas de arquitectura del proyecto Corvus, siguiendo el modelo C4 (
Context, Container, Component, Code).

## Nivel 1: Contexto del Sistema

Muestra el sistema Corvus como una caja negra y sus interacciones con actores y sistemas externos.

- **Archivo**: [`context/system-context.mmd`](./diagrams/context/system-context.mmd)
- **Formato**: Mermaid
- **Descripción**: Vista de alto nivel del sistema completo, incluyendo usuarios (Developer, End
  User) y sistemas externos (LLM Providers, Neo4j, Fuentes Web).

## Nivel 2: Contenedores

Descompone el sistema en contenedores/aplicaciones principales y sus interacciones.

- **Archivos**:
  - [`container/runtime-containers.mmd`](./diagrams/container/runtime-containers.mmd) (Mermaid)
  - [`container/runtime-containers.puml`](./diagrams/container/runtime-containers.puml) (PlantUML)
- **Descripción**: Muestra los contenedores del runtime: CLI, gateway, servicios daemon,
  ejecución de tools, backends de memoria y superficies operativas alrededor de Corvus.

## Nivel 3: Componentes

Descompone contenedores individuales en sus componentes internos.

### Núcleo del Runtime

- **Archivo**: [`component/runtime-core.mmd`](./diagrams/component/runtime-core.mmd)
- **Descripción**: Componentes internos del núcleo del runtime: configuración, agent loop,
  providers, memoria, tools, canales, seguridad y observabilidad.

## Dependencias entre Módulos

Diagrama adicional mostrando las dependencias de Gradle entre módulos.

- **Archivo**: [`cargo-dependencies.mmd`](./diagrams/cargo-dependencies.mmd)
- **Descripción**: Muestra la estructura del workspace Cargo y el flujo principal de dependencias Rust del runtime.

## Cómo Visualizar

### Opción 1: GitHub/GitLab

Los archivos `.mmd` se renderizan automáticamente en GitHub y GitLab.

### Opción 2: VS Code

Instala la extensión "Markdown Preview Mermaid Support" para ver los diagramas en VS Code.

### Opción 3: Mermaid CLI

```bash
# Instalar mermaid-cli
npm install -g @mermaid-js/mermaid-cli

# Renderizar a PNG
mmdc -i diagrams/context/system-context.mmd -o context.png
```

### Opción 4: PlantUML

Para los archivos `.puml`:

```bash
# Usar PlantUML online o local
plantuml -tpng diagrams/container/runtime-containers.puml
```

## Convenciones

- **Nivel Contexto (C1)**: Un diagrama mostrando el sistema completo
- **Nivel Contenedor (C2)**: Un diagrama por sistema, mostrando aplicaciones principales
- **Nivel Componente (C3)**: Diagramas para contenedores significativos
- **Nivel Código (C4)**: Diagramas UML de clases para componentes críticos (opcional)

## Mantenimiento

Cuando agregues nuevos módulos o cambies la arquitectura:

1. Actualiza el diagrama correspondiente
2. Mantén los IDs consistentes entre niveles
3. Actualiza esta página de índice si agregas nuevos diagramas
4. Genera un commit con los cambios siguiendo el formato: `docs(architecture): update C4 diagrams`
