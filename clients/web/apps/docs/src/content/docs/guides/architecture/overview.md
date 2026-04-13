---
title: Architecture Overview
description: Collection of C4 diagrams for the Corvus project
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: architecture
translations:
  es: pending
---

# Architecture Diagrams

> **Migration Notice**: Architecture diagrams are being migrated to a separate repository. 
> Track progress at https://github.com/dallay/corvus/issues?q=label:architecture-diagrams

This section will be updated once migration completes. The C4 model diagrams (Context, Container, Component, Code) are temporarily unavailable.

## Level 1: System Context

Shows the Corvus system as a black box and its interactions with actors and external systems.

## Level 2: Containers

Decomposes the system into main containers/applications and their interactions.

## Level 3: Components

Decomposes individual containers into their internal components.

### Runtime Core

## Module Dependencies

Additional diagram showing Cargo dependencies between workspace modules.

## How to Visualize

> **Note**: Diagram rendering instructions will be available after migration completes.
> See the migration notice at the top of this page for timeline and updates.

### Option 1: GitHub/GitLab

`.mmd` files are automatically rendered on GitHub and GitLab.

### Option 2: VS Code

Install the "Markdown Preview Mermaid Support" extension to view diagrams in VS Code.

### Option 3: Mermaid CLI

```bash
# Install mermaid-cli
npm install -g @mermaid-js/mermaid-cli

# Render to PNG (example path - adjust to actual diagram location)
mmdc -i <path-to-diagram>/<diagram-file>.mmd -o output.png
```

### Option 4: PlantUML

For `.puml` files:

```bash
# Use PlantUML online or locally (example path - adjust to actual diagram location)
plantuml -tpng <path-to-diagram>/<diagram-file>.puml
```

## Conventions

> **Note**: These conventions apply after migration completes. The C4 diagrams are temporarily unavailable.

- **Context Level (C1)**: One diagram showing the complete system
- **Container Level (C2)**: One diagram per system, showing main applications
- **Component Level (C3)**: Diagrams for significant containers
- **Code Level (C4)**: UML class diagrams for critical components (optional)

## Maintenance

> **Note**: These maintenance guidelines apply after migration completes. The C4 diagrams are temporarily unavailable.

When adding new modules or changing architecture:

1. Update the corresponding diagram
2. Keep IDs consistent across levels
3. Update this index page if adding new diagrams
4. Commit changes with format: `docs(architecture): update C4 diagrams`
