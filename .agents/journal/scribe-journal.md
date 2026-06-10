# Scribe Documentation Journal

## 2026-04-12 - Full Documentation Accuracy Audit - Complete

**Verification:** Systematic comparison of all documentation files in `clients/web/apps/docs/src/content/docs/` (both en/ and es/) against the actual codebase implementation in `clients/agent-runtime/`, `clients/cerebro/`, and root-level build/tooling files.

### Structure Assessment
- **Bilingual Parity:** ✅ Perfect. 53 files in English (root-level default), 53 files in Spanish (`es/`). 1:1 file mapping with identical directory structures.
- **Note:** The documentation does NOT use an `en/` directory. English docs live at the root of `docs/`, with `es/` as the translation overlay. This is consistent with Starlight's default locale behavior.

### CLI Reference (`guides/cli-reference.md`)
- **Accuracy:** ✅ High. All documented commands, subcommands, and flags verified against `clients/agent-runtime/src/main.rs` and `composer.rs`.
- **Verified commands:** onboard, agent, code, daemon, gateway, service, doctor, status, cron, models, providers, auth, skills, integrations, channel, hardware, peripheral, migrate, update, cost.
- **Dashboard activation codes:** DASH-001 through DASH-004, DASH-999 — all documented and verified in the onboard/dashboard code.
- **`make dev-up`**: Verified exists at Makefile:308.

### Cerebro CLI (`cerebro/cli-reference.md`)
- **Accuracy:** ✅ Fully implemented. Verified at `clients/cerebro/src/bin/cerebro.rs`. Two binaries: `cerebro` (full CLI) and `cerebro-serve` (lightweight server). Commands `serve` and `migrate import/validate` match docs exactly.

### Cerebro Configuration (`cerebro/configuration.md`)
- **Accuracy:** ✅ All config fields, env vars, storage modes, and TUI settings verified against `clients/cerebro/` source. `remote_surreal` correctly marked as "not yet implemented" in both docs and code.

### Architecture (`clients/agent-runtime/architecture.md`)
- **Accuracy:** ✅ High. All components verified as implemented:
  - 22+ providers ✅ (including all regional variants and custom endpoints)
  - 14 channels ✅ (CLI, Telegram, Discord, Slack, WhatsApp, Signal, iMessage, Matrix, DingTalk, QQ, Lark, Email, IRC, Mattermost)
  - 32+ tools ✅
  - 4 memory backends ✅ (sqlite, lucid, markdown, none)
  - 5 observability backends ✅ (noop, log, prometheus, otel, multi)
  - Security: Landlock ✅, Bubblewrap ✅, Firejail ✅ (verified in `security/firejail.rs`), Noop ✅
  - **Minor note:** Docs mention "Firejail" in sandboxing section — this IS implemented (77 matches in code). The mermaid diagram says "landlock/firejail/bubblewrap" which is accurate.
  - **Capability profiles** (full, code, lite) — verified in bootstrap code.

### Tools Reference (`clients/agent-runtime/tools/`)
- **Accuracy:** ✅ All tool category pages verified against actual tool implementations in `src/tools/`.
- **Security tiers** (Read-Only vs Action-Bearing) — verified against `security/policy.rs` risk classification.
- **MCP tool runtime** — verified in `tools/mcp/`, correctly documented as gated by `mcp.enabled`.

### Providers (`clients/agent-runtime/providers/index.mdx`)
- **Accuracy:** ✅ All 22+ providers verified. LM Studio confirmed at `providers/mod.rs:527`. Gemini OAuth and env vars (`GEMINI_API_KEY`, `GOOGLE_API_KEY`) verified in `providers/gemini.rs`. Regional providers (Moonshot, GLM, MiniMax, Qwen, Qianfan, Z.AI) all confirmed.

### Model Routing (`guides/model-routing.md`)
- **Accuracy:** ✅ `[[model_routes]]` and `[query_classification]` config structures verified against `config/schema.rs`. `corvus doctor` validation warnings verified in doctor module code. `allow_image_input` gate verified.

### Sandbox Isolation (`guides/runtime-sandbox-isolation.md`)
- **Accuracy:** ✅ All sandbox backends verified: landlock, firejail, bubblewrap, docker, none. `require = true` fail-closed behavior verified. Computer-use sidecar health check (`GET /v1/health`) verified. Audit log fields match implementation.

### Getting Started (`guides/getting-started.md`)
- **Accuracy:** ✅ Prerequisites match AGENTS.md. `make setup`, `make build`, `make run`, `make test` all verified in Makefile. Dashboard activation flow verified.

### SurrealDB Guide (`guides/surrealdb.md`)
- **Assessment:** This is a general operational guide for running SurrealDB with Docker Compose. Not specific to Corvus implementation but technically accurate for SurrealDB v3. No discrepancies found.

### Configuration Options (`guides/configuration.md`)
- **Assessment:** ⚠️ **PARTIAL COVERAGE**. This document focuses heavily on Gradle properties, version catalogs, and MCP configuration. It does NOT cover the full `config.toml` schema for the agent runtime (which is extensive — 30+ sections). The MCP section is accurate. **Recommendation:** This doc should be expanded or split to cover the agent runtime's full configuration surface.

### Customization (`guides/customization.md`)
- **Assessment:** ⚠️ **OUTDATED**. References `com.profiletailors` as the package namespace and mentions "This repository was created from a Gradle template." This is legacy template language. The "Platform Direction" section mentions "Kotlin + Spring Boot (WebFlux + Coroutines)" and "Neo4j for graph memory" which do NOT appear to be implemented in the current codebase. The actual architecture is Rust-based agent-runtime with Kotlin Multiplatform clients. **This section needs review and likely significant updates.**

### Features Checklist (`guides/features.md`)
- **Assessment:** ⚠️ **PARTIALLY OUTDATED**. Lists modules as `apps/composeApp`, `apps/androidApp`, etc. but the actual directory structure uses `clients/` prefix (e.g., `clients/composeApp`). Mentions "apps/agent-runtime-rust" but actual path is `clients/agent-runtime`. Missing several implemented features: cost tracking, update system, skills system, cron/scheduler, hardware peripherals, model routing, computer-use sidecar, tunnel providers. **Needs comprehensive update.**

### Development Procedures (`guides/development.md`)
- **Accuracy:** ✅ Makefile commands verified. `make setup`, `make run`, `make build`, `make test`, `make check`, `make format`, `make clean` all present. Documentation commands (`make docs-web-build`, etc.) verified against Makefile.

### Structure (`guides/structure.md`)
- **Accuracy:** ✅ Directory structure matches reality. `clients/`, `modules/`, `gradle/` all correctly described.

### Issues Summary

| Severity | File | Issue |
|----------|------|-------|
| 🔴 HIGH | `guides/customization.md` | References Spring Boot/Neo4j architecture not in codebase; legacy template language |
| 🔴 HIGH | `guides/features.md` | Wrong module paths (`apps/` vs `clients/`); missing 10+ major features |
| 🟡 MEDIUM | `guides/configuration.md` | Only covers Gradle/MCP; missing full agent runtime config reference |
| 🟢 LOW | `cerebro/` docs | All accurate, but lastReviewed date (2026-04-02) is recent — no action needed |

### Remediation (2026-04-12)

All three identified issues have been fixed:

1. **`guides/customization.md`** — ✅ Fixed. Removed Spring Boot/Neo4j references. Updated with current architecture (Rust agent runtime, KMP clients, Cerebro, web apps). Corrected VERSION to 3.0.0. Added `includeProjects` module registration guide. Both en/es synced.
2. **`guides/features.md`** — ✅ Fixed. Corrected all paths from `apps/` to `clients/`. Expanded from basic checklist to comprehensive coverage: 22+ providers, 14 channels, 32+ tools, memory backends, infrastructure, security, hardware, tunnels. Both en/es synced.
3. **`guides/configuration.md`** — ✅ Improved. Added full agent runtime config reference covering autonomy, security/sandbox, runtime, gateway, memory, agent profiles, model routing, multimodal/audio, scheduler, MCP, observability, cost, updates, skills, and env var overrides. Both en/es synced.

### Validation Results
- `make docs-check` — ✅ PASSED (astro check + biome + metadata validation: 0 errors, 0 warnings, 8 files validated)
- `make docs-build` — ✅ PASSED (80 pages built, search index generated, no broken links)

### Notes
- Glossary: "Firejail" = Linux user-space sandbox (confirmed implemented). "Landlock" = Linux kernel-level sandbox (confirmed). "Cerebro" = standalone MCP memory service client in `clients/cerebro/` (confirmed).
- Remaining gap: `cost` command subcommands (`summary`, `history`, `reset`) exist in code but are not yet in the CLI reference doc. Low priority.

## 2026-04-15 - CLI Reference Update - Complete

**Verification:** Verified `clients/agent-runtime/src/main.rs` for `code` and `cost` command implementations.
**Changes:**
- Updated `guides/cli-reference.md` and `es/guides/cli-reference.md`.
- Added `code` command documentation.
- Added `cost` command documentation with `summary`, `history`, and `reset` subcommands.
- Updated `agent` command with new flags (`--override-budget`, `--plan`) and subcommands (`build`, `run`, `new`).
- Removed deprecated `surreal-graphs` and `surreal` memory backends from `onboard` command.
**Validation:** Results of `make docs-check` and `make docs-build` passed (80 pages built).
**Notes:** Bilingual parity maintained for all changes.

## 2026-04-29 - Orchestration & Tooling Parity Update - Complete

**Verification:** Verified actual tool implementations in `clients/agent-runtime/src/tools/` for new delegation and task/cron tools.
**Changes:**
- Updated `clients/web/apps/docs/src/content/docs/es/guides/cli-reference.md` to sync with English (dashboard activation, status example cleanup).
- Updated `clients/web/apps/docs/src/content/docs/clients/agent-runtime/tools/index.mdx` (en/es) to include `memory_forget` in Action-Bearing tier.
- Added `delegate_launch`, `delegate_inspect`, and `delegate_cancel` documentation to `automation.md` (en/es).
- Improved `cron_runs` and `cron_update` parameter documentation in `automation.md` (en/es).
**Validation:** Results of `make docs-check` and `make docs-build` passed (84 pages built).
**Notes:** Maintaining bilingual parity between root (English) and `es/` directory.

## 2026-06-10 - PDF Inspection & Plan Mode Documentation - Complete

**Verification:** Verified `pdf_inspect.rs` for tool contract and `policy.rs` for Plan Mode allowlist and security logic.
**Changes:**
- Updated `media.md` (en/es) with `pdf_inspect` tool documentation.
- Updated `cli-reference.md` (en/es) with Plan Mode (`--plan`) details and allowed tools list.
- Updated `runtime-sandbox-isolation.md` (en/es) to include Plan Mode as the third security layer.
- Updated `index.mdx` (en/es) to categorize `pdf_inspect` and reference Plan Mode.
- Updated all modified files' `lastReviewed` frontmatter to 2026-06-10.
**Validation:** Results of `make docs-check` and `make docs-build` passed (84 pages built).
**Notes:** `pdf_inspect` is not yet in the `PLAN_MODE_SAFE_TOOLS` allowlist in code, so it was excluded from the Plan Mode documentation list to maintain technical accuracy.
