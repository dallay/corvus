---
name: c4-diagrams
description: >
  Generates and keeps C4 models in docs/architecture/diagrams up to date. Trigger: files in docs/architecture/diagrams, changes to .mmd/.puml/.puml.md diagrams
license: Apache-2.0
metadata:
  author: @dallay
  version: "1.0"
---

# C4 Diagrams Skill

## When to Use

- When you need to generate or update system/containers/components/context diagrams following the C4
  model.
- Trigger: changes under `docs/architecture/diagrams` or creating new architecture artifacts.

## Critical Patterns

- Keep diagrams as source files in `docs/architecture/diagrams` using either Mermaid (.mmd/.mmdx) or
  PlantUML (.puml).
- Prefer small, focused diagrams per file (one diagram per file). Don't bloat a single file with all
  C4 levels.
- Use consistent IDs and naming for elements across diagrams (SYSTEM_xxx, CONTAINER_xxx, COMP_xxx).
- Version diagrams in git. The skill MUST not modify files without creating clear diffs and a
  suggested commit message.
- Always provide both Mermaid and PlantUML variants when a diagram is shared with external
  stakeholders who may prefer one renderer.

## Commands

```bash
# Validate Mermaid syntax (requires mmdc installed)
mmdc -i docs/architecture/diagrams/example.mmd -o /tmp/example.png

# Render PlantUML (requires plantuml.jar or plantuml binary)
plantuml -tpng docs/architecture/diagrams/example.puml

# Quick lint: search for .mmd/.puml files changed by git
git diff --name-only --diff-filter=ACMRTUXB HEAD~1..HEAD | rg "docs/architecture/diagrams/.*\.(mmd|puml)"
```

## Templates (assets)

- assets/mermaid-example.mmd -> minimal C4-style Mermaid snippet
- assets/plantuml-example.puml -> PlantUML using C4-PlantUML macros
- assets/README.md -> instructions to render locally

## Code Examples

Mermaid (minimal C4-like context diagram):

```mmd
%% assets/mermaid-example.mmd
flowchart TB
  %% Context - System and external actors
  actor(User):::person
  subgraph SYSTEM [My System]
    APP["Web App"]
  end
  User --> APP

  classDef person fill:#f9f,stroke:#333,stroke-width:1px
```

PlantUML (using C4-PlantUML):

```puml
@startuml
!include https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Context.puml
Person(user, "User")
System(webapp, "Web App")
Rel(user, webapp, "uses")
@enduml
```

## Rules for the Agent

- When asked to update diagrams, propose both Mermaid and PlantUML outputs when possible.
- Validate diagram syntax and show render command output or error.
- When creating new diagram files, place them under
  `docs/architecture/diagrams/<level>/<name>.<ext>` where `<level>` is
  context|container|component|code.
- Add a short header comment in each generated file with: generation timestamp, generator name (
  `c4-diagrams skill`), and git SHA (if available).

## Resources

- Templates: `assets/`
- Docs location: `docs/architecture/diagrams/`
