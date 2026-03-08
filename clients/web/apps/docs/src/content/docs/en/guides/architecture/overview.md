---
title: Architecture Overview
description: Collection of C4 diagrams for the Corvus project
---

# Architecture Diagrams

This section contains the architecture diagrams for the Corvus project, following the C4 model (
Context, Container, Component, Code).

## Level 1: System Context

Shows the Corvus system as a black box and its interactions with actors and external systems.

- **File**: [`context/system-context.mmd`](/guides/architecture/diagrams/context/system-context.mmd)
- **Format**: Mermaid
- **Description**: High-level view of the complete system, including users (Developer, End User) and
  external systems (LLM Providers, Neo4j, Web Sources).

## Level 2: Containers

Decomposes the system into main containers/applications and their interactions.

- **Files**:
  - [`container/system-containers.mmd`](/guides/architecture/diagrams/container/system-containers.mmd) (Mermaid)
  - [`container/system-containers.puml`](/guides/architecture/diagrams/container/system-containers.puml) (PlantUML)
- **Description**: Shows the main containers: Web Dashboard, Android App, iOS App, Agent Runtime,
  Rust Sidecar, and Graph DB.

## Level 3: Components

Decomposes individual containers into their internal components.

### Agent Core KMP

- **File**: [`component/agent-core-kmp.mmd`](/guides/architecture/diagrams/component/agent-core-kmp.mmd)
- **Description**: Internal components of the core module: Domain, Use Cases, Interfaces, and
  Infrastructure.

## Module Dependencies

Additional diagram showing Gradle dependencies between modules.

- **File**: [`module-dependencies.mmd`](/guides/architecture/diagrams/module-dependencies.mmd)
- **Description**: Shows how clients depend on the shared core and external frameworks.

## How to Visualize

### Option 1: GitHub/GitLab

`.mmd` files are automatically rendered on GitHub and GitLab.

### Option 2: VS Code

Install the "Markdown Preview Mermaid Support" extension to view diagrams in VS Code.

### Option 3: Mermaid CLI

```bash
# Install mermaid-cli
npm install -g @mermaid-js/mermaid-cli

# Render to PNG
mmdc -i diagrams/context/system-context.mmd -o context.png
```

### Option 4: PlantUML

For `.puml` files:

```bash
# Use PlantUML online or locally
plantuml -tpng diagrams/container/system-containers.puml
```

## Conventions

- **Context Level (C1)**: One diagram showing the complete system
- **Container Level (C2)**: One diagram per system, showing main applications
- **Component Level (C3)**: Diagrams for significant containers
- **Code Level (C4)**: UML class diagrams for critical components (optional)

## Maintenance

When adding new modules or changing architecture:

1. Update the corresponding diagram
2. Keep IDs consistent across levels
3. Update this index page if adding new diagrams
4. Commit changes with format: `docs(architecture): update C4 diagrams`
