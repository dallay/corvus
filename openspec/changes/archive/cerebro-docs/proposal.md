# Cerebro Docs Section — Planning & Proposal

**Change:** cerebro-docs
**Parent Issue:** #248
**Status:** APPROVED
**Date:** 2026-04-02

## Intent

Define the dedicated documentation presence for Cerebro in the Corvus docs website,
closing all product and content structure decisions before implementation starts.

## Context

Cerebro is a standalone Rust MCP memory service (`clients/cerebro/`) with its own binaries
(`cerebro`, `cerebro-serve`), configuration surface, protocol (MCP over HTTP), and deployment
lifecycle. Today it has only one migration guide under `guides/cerebro/` and 13 MCP schema
JSON files — no dedicated section.

## Decision 1: Top-Level Section

**YES.** Cerebro gets its own top-level sidebar section, positioned after "Agent Runtime".
Rationale: standalone service with its own binaries, config, protocol, and lifecycle.

## Decision 2: Information Architecture

```text
Cerebro (top-level)
├── Overview                    # What it is, when to use it, key concepts
├── Configuration               # CerebroConfig, env vars, storage backends
├── Running                     # cerebro serve, cerebro-serve, ports, health
├── CLI Reference               # cerebro serve, cerebro migrate subcommands
├── MCP Tools Reference         # 13 tools — schema, params, examples
├── Integration with Corvus     # [memory.cerebro] config in agent-runtime
├── Migration                   # MOVED from guides/cerebro/migration.md
└── Operations                  # Storage modes, backup, monitoring, TUI
```

Order follows user journey: What → Install/Config → Run → Use → Integrate → Migrate → Operate.

## Decision 3: Minimum Launch Content

| Page                    | Priority     | Source                                      |
|-------------------------|--------------|---------------------------------------------|
| Overview (index.mdx)    | MUST         | New — from README.md + openspec             |
| Configuration           | MUST         | New — from config.rs, env vars              |
| Running                 | MUST         | New — cerebro serve flags, health endpoint  |
| CLI Reference           | MUST         | New — from bin/cerebro.rs clap definitions  |
| MCP Tools Reference     | MUST         | Restructure existing 13 JSON schemas        |
| Migration               | MUST         | MOVE existing guides/cerebro/migration.md   |
| Installation            | SHOULD       | Can be short section in Overview initially  |
| Integration with Corvus | SHOULD       | New — MemoryCerebroConfig, CORVUS_CEREBRO_* |
| Operations              | NICE-TO-HAVE | New — TUI, storage modes, backup            |

Minimum launch = 6 pages.

## Decision 4: Existing Content Disposition

| Content                          | Action                                     |
|----------------------------------|--------------------------------------------|
| guides/cerebro/migration.md (EN) | MOVE to cerebro/migration.md + redirect    |
| guides/cerebro/migration.md (ES) | MOVE to es/cerebro/migration.md + redirect |
| guides/cerebro/mcp-schema/*.json | KEEP in place, reference from MCP Tools    |
| guides/architecture.md           | KEEP — cross-link to Cerebro Overview      |
| guides/surrealdb.md              | KEEP — not Cerebro-specific                |
| guides/configuration.md          | KEEP — add cross-link to Cerebro Config    |
| clients/cerebro/README.md        | KEEP — dev-facing                          |
| openspec/specs/cerebro/spec.md   | KEEP — internal spec                       |

## Decision 5: Bilingual Parity EN/ES

**Both languages are REQUIRED from day one.** No launch without EN/ES parity.

Rules:

- Each page is written EN first (Starlight default locale).
- ES translation ships in the SAME PR.
- If ES content is incomplete, mark with `:::caution[Traduccion en progreso]` banner.
- Minimum launch = 12 files (6 pages x 2 languages).

## Sidebar Configuration

Position: after "Agent Runtime" section in astro.config.mjs.

```js
{
  label: "Cerebro",
  translations: { es: "Cerebro" },
  items: [
    { label: "Overview", translations: { es: "Descripcion General" }, slug: "cerebro" },
    { label: "Configuration", translations: { es: "Configuracion" }, slug: "cerebro/configuration" },
    { label: "Running", translations: { es: "Ejecucion" }, slug: "cerebro/running" },
    { label: "CLI Reference", translations: { es: "Referencia CLI" }, slug: "cerebro/cli-reference" },
    { label: "MCP Tools Reference", translations: { es: "Referencia de Herramientas MCP" }, slug: "cerebro/mcp-tools" },
    { label: "Integration", translations: { es: "Integracion" }, slug: "cerebro/integration" },
    { label: "Migration", translations: { es: "Migracion" }, slug: "cerebro/migration" },
    { label: "Operations", translations: { es: "Operaciones" }, slug: "cerebro/operations" },
  ],
}
```

## File Structure

```text
src/content/docs/cerebro/
├── index.mdx
├── configuration.md
├── running.md
├── cli-reference.md
├── mcp-tools.md
├── integration.md
├── migration.md          (moved from guides/cerebro/)
└── operations.md

src/content/docs/es/cerebro/
├── index.mdx
├── configuration.md
├── running.md
├── cli-reference.md
├── mcp-tools.md
├── integration.md
├── migration.md          (moved from es/guides/cerebro/)
└── operations.md
```

## Risks

| Risk                                       | Mitigation                                      |
|--------------------------------------------|-------------------------------------------------|
| Migration guide move breaks links          | Add Starlight redirects in astro.config.mjs     |
| MCP schemas reference NotImplemented tools | Mark clearly as "Planned" in reference page     |
| Content from source code drifts            | lastReviewed frontmatter + review cadence       |
| ES content quality                         | Same-PR delivery + caution banners where needed |

## Implementation Issues

8 follow-up issues created in Linear as sub-issues of DALLAY-152:

| # | Linear ID  | Title                                       | Priority           |
|---|------------|---------------------------------------------|--------------------|
| 1 | DALLAY-223 | Scaffold section structure + sidebar config | High (MUST)        |
| 2 | DALLAY-224 | Configuration page EN/ES                    | High (MUST)        |
| 3 | DALLAY-225 | Running page EN/ES                          | High (MUST)        |
| 4 | DALLAY-226 | CLI Reference page EN/ES                    | High (MUST)        |
| 5 | DALLAY-227 | MCP Tools Reference page EN/ES              | High (MUST)        |
| 6 | DALLAY-228 | Move migration guide to Cerebro section     | High (MUST)        |
| 7 | DALLAY-229 | Integration with Corvus page EN/ES          | Medium (SHOULD)    |
| 8 | DALLAY-230 | Operations page EN/ES                       | Low (NICE-TO-HAVE) |

Execution order: 223 → 228 → (224, 225, 226, 227 parallel) → 229 → 230
