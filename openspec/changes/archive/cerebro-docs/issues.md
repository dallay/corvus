# Cerebro Docs — Implementation Issues

Parent: #248
All issues require `documentation` and `cerebro` labels.

---

## Issue 1: Scaffold section structure and sidebar config

**Title:** `docs(cerebro): scaffold section structure and sidebar config`

### Goal

Create the Cerebro docs section scaffold: directory structure, sidebar configuration, and the
Overview page (`index.mdx`) in both EN and ES.

### Scope

- Create `src/content/docs/cerebro/` and `src/content/docs/es/cerebro/` directories
- Add Cerebro top-level sidebar section in `astro.config.mjs` (after "Agent Runtime")
- Write `cerebro/index.mdx` (EN) — Overview page covering what Cerebro is, key concepts, when to use
  it
- Write `es/cerebro/index.mdx` (ES) — Spanish translation
- Source content from `modules/cerebro/README.md` and `openspec/specs/cerebro/spec.md`
- Installation info included as a section within Overview (SHOULD priority, can be brief)

### Acceptance Criteria

- [ ] `src/content/docs/cerebro/index.mdx` exists with complete Overview content
- [ ] `src/content/docs/es/cerebro/index.mdx` exists with Spanish translation
- [ ] Sidebar shows "Cerebro" section after "Agent Runtime" in both EN and ES
- [ ] `make docs-check` passes

---

## Issue 2: Write Configuration page EN/ES

**Title:** `docs(cerebro): write Configuration page EN/ES`

### Goal

Document Cerebro's configuration surface: `CerebroConfig` struct, environment variables, storage
backend options, and defaults.

### Scope

- Create `src/content/docs/cerebro/configuration.md` (EN)
- Create `src/content/docs/es/cerebro/configuration.md` (ES)
- Extract configuration details from `modules/cerebro/src/config.rs`
- Document all `CORVUS_CEREBRO_*` env var overrides from
  `clients/agent-runtime/src/config/schema.rs`
- Cover storage backends: kv-rocksdb (default), disk, in-memory, remote SurrealDB
- Include configuration examples (TOML snippets)

### Acceptance Criteria

- [ ] EN page exists with complete configuration documentation
- [ ] ES page exists with Spanish translation
- [ ] All env vars documented with types and defaults
- [ ] All storage backends documented with tradeoffs
- [ ] `make docs-check` passes

---

## Issue 3: Write Running page EN/ES

**Title:** `docs(cerebro): write Running page EN/ES`

### Goal

Document how to run Cerebro: the two binary entry points, CLI flags, ports, health endpoint, and
startup verification.

### Scope

- Create `src/content/docs/cerebro/running.md` (EN)
- Create `src/content/docs/es/cerebro/running.md` (ES)
- Document `cerebro serve` (full CLI with migrate subcommands)
- Document `cerebro-serve` (lightweight server-only entry point)
- Cover bind address, port configuration, health check endpoint
- Include quick-start examples

### Source Files

- `modules/cerebro/src/bin/cerebro.rs` — full CLI entry point
- `modules/cerebro/src/main.rs` — cerebro-serve binary
- `modules/cerebro/src/server.rs` — Axum HTTP/MCP router, health endpoint

### Acceptance Criteria

- [ ] EN page exists with complete running documentation
- [ ] ES page exists with Spanish translation
- [ ] Both binaries documented with usage examples
- [ ] Health check endpoint documented
- [ ] `make docs-check` passes

---

## Issue 4: Write CLI Reference page EN/ES

**Title:** `docs(cerebro): write CLI Reference page EN/ES`

### Goal

Document Cerebro's CLI commands and flags extracted from clap definitions.

### Scope

- Create `src/content/docs/cerebro/cli-reference.md` (EN)
- Create `src/content/docs/es/cerebro/cli-reference.md` (ES)
- Document all subcommands: `serve`, `migrate` (and sub-subcommands like `migrate export`,
  `migrate import`, `migrate validate`)
- Document all flags with types, defaults, and descriptions
- Include usage examples for common workflows
- Frontmatter: `docType: reference`

### Source Files

- `modules/cerebro/src/bin/cerebro.rs` — clap definitions for CLI
- `modules/cerebro/src/migration/` — migration subcommand logic

### Acceptance Criteria

- [ ] EN page exists with complete CLI reference
- [ ] ES page exists with Spanish translation
- [ ] All subcommands and flags documented
- [ ] Usage examples included for common workflows
- [ ] `make docs-check` passes

---

## Issue 5: Create MCP Tools Reference page EN/ES

**Title:** `docs(cerebro): create MCP Tools Reference page EN/ES`

### Goal

Restructure the 13 existing MCP JSON schema files into a readable reference page documenting all
Cerebro memory tools.

### Scope

- Create `src/content/docs/cerebro/mcp-tools.md` (EN)
- Create `src/content/docs/es/cerebro/mcp-tools.md` (ES)
- Document all 13 MCP tools with: description, parameters, return type, example request/response
- Clearly distinguish **implemented** vs **planned** tools:
    - Implemented: `mem_save`, `mem_search`, `mem_delete`, `mem_get_observation`, `mem_update`,
      `mem_suggest_topic_key`, `mem_stats`, `mem_timeline`
    - Planned (NotImplemented): `mem_save_prompt`, `mem_session_start`, `mem_session_end`,
      `mem_session_summary`, `mem_context`
- Reference existing JSON schemas at `guides/cerebro/mcp-schema/*.json`
- Frontmatter: `docType: reference`

### Source Files

- `clients/web/apps/docs/src/content/docs/guides/cerebro/mcp-schema/*.json` — 13 schema files
- `modules/cerebro/src/tools.rs` — tool implementations

### Acceptance Criteria

- [ ] EN page exists with all 13 tools documented
- [ ] ES page exists with Spanish translation
- [ ] Implemented vs Planned tools clearly marked
- [ ] Each tool has parameters, return type, and example
- [ ] JSON schemas referenced (not duplicated)
- [ ] `make docs-check` passes

---

## Issue 6: Move migration guide to Cerebro section

**Title:** `docs(cerebro): move migration guide to Cerebro section`

### Goal

Move the existing Cerebro migration guide from `guides/cerebro/` to the new `cerebro/` section,
update sidebar, and add redirects.

### Scope

- Move `src/content/docs/guides/cerebro/migration.md` → `src/content/docs/cerebro/migration.md`
- Move `src/content/docs/es/guides/cerebro/migration.md` →
  `src/content/docs/es/cerebro/migration.md`
- Update frontmatter slug from `guides/cerebro/migration` to `cerebro/migration`
- Add Starlight redirect in `astro.config.mjs` from old path to new path
- Update sidebar: remove old Cerebro Migration entry from Guides section
- Update any cross-references in other docs that link to the old path
- Keep `guides/cerebro/mcp-schema/*.json` files in place (referenced by MCP Tools page)

### Notes

- The EN migration guide is complete
- The ES migration guide exists but has some "ES pending" sections — preserve as-is
- The `mcp-schema/` directory stays under `guides/cerebro/` since it's data, not a page

### Acceptance Criteria

- [ ] Migration guide accessible at new Cerebro section URL
- [ ] Old URL redirects to new URL (no broken bookmarks)
- [ ] Both EN and ES files moved
- [ ] Sidebar updated (old entry removed, new entry in Cerebro section)
- [ ] No broken cross-references
- [ ] `make docs-check` passes

---

## Issue 7: Write Integration with Corvus page EN/ES (SHOULD)

**Title:** `docs(cerebro): write Integration with Corvus page EN/ES`

### Priority

SHOULD — not blocking launch but strongly recommended.

### Goal

Document how Cerebro integrates with the Corvus agent runtime as its long-term memory backend.

### Scope

- Create `src/content/docs/cerebro/integration.md` (EN)
- Create `src/content/docs/es/cerebro/integration.md` (ES)
- Document `[memory.cerebro]` TOML configuration in agent-runtime
- Document `MemoryCerebroConfig` struct and its fields
- Document `CORVUS_CEREBRO_*` environment variable overrides
- Document `cerebro_configured()` wiring in memory module
- Include example runtime config snippets
- Cross-link to Cerebro Configuration page and Agent Runtime docs

### Source Files

- `clients/agent-runtime/src/config/schema.rs` — `MemoryCerebroConfig`
- `clients/agent-runtime/src/memory/mod.rs` — `cerebro_configured()` function
- `clients/agent-runtime/src/memory/traits.rs` — `cerebro_configured` in `MemoryStats`
- `clients/agent-runtime/src/gateway/admin.rs` — admin API for Cerebro settings

### Acceptance Criteria

- [ ] EN page exists with integration documentation
- [ ] ES page exists with Spanish translation
- [ ] Runtime config examples included
- [ ] Cross-links to related pages
- [ ] `make docs-check` passes

---

## Issue 8: Write Operations page EN/ES (NICE-TO-HAVE)

**Title:** `docs(cerebro): write Operations page EN/ES`

### Priority

NICE-TO-HAVE — can ship after launch.

### Goal

Document day-2 operations for Cerebro: storage modes, backup strategies, monitoring, and the TUI
dashboard.

### Scope

- Create `src/content/docs/cerebro/operations.md` (EN)
- Create `src/content/docs/es/cerebro/operations.md` (ES)
- Document storage mode selection and tradeoffs (kv-rocksdb, disk, in-memory, remote)
- Document backup and restore procedures
- Document the optional TUI dashboard (`--features tui` / ratatui)
- Document monitoring and observability hooks
- Document log levels and troubleshooting

### Source Files

- `modules/cerebro/src/storage/` — storage backend implementations
- `modules/cerebro/src/tui/` — TUI dashboard module

### Acceptance Criteria

- [ ] EN page exists with operations documentation
- [ ] ES page exists with Spanish translation
- [ ] Storage modes documented with selection guidance
- [ ] TUI dashboard documented with screenshot or description
- [ ] `make docs-check` passes

---

## Suggested Execution Order

1. **Issue 1** (scaffold) — unblocks everything
2. **Issue 6** (move migration) — cleans up existing content
3. **Issues 2-5** (new MUST pages) — can be parallelized
4. **Issue 7** (integration, SHOULD)
5. **Issue 8** (operations, NICE-TO-HAVE)
