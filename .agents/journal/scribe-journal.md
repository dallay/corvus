# Scribe Journal

## 2025-05-15 - AI Providers Documentation - Completed

**Verification:** I have thoroughly inspected `clients/agent-runtime/src/providers/mod.rs` and individual provider implementations (e.g., `gemini.rs`, `anthropic.rs`) to identify all supported providers and their environment variables.
**Changes:**
- Updated `clients/web/apps/docs/src/content/docs/clients/agent-runtime/providers/index.mdx` (EN) and `clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/providers/index.mdx` (ES).
- Added missing providers: `OpenAI Codex`, `Synthetic`, `OpenCode Zen`, `Amazon Bedrock`, `LM Studio`.
- Updated environment variables for `Anthropic` (`ANTHROPIC_OAUTH_TOKEN`), `GitHub Copilot` (`GH_TOKEN`), and `Google Gemini` (`GOOGLE_API_KEY`).
- Added "Advanced Authentication" section covering OAuth/CLI reuse for Gemini, Codex, and Copilot, and setup tokens for Anthropic.
**Validation:** Ran `make docs-web-build` and `make docs-web-check`.
**Notes:**
- Confirmed `gemini-cli` OAuth token support in `gemini.rs`.
- Confirmed `ANTHROPIC_OAUTH_TOKEN` support in `mod.rs`.
- Maintained strict bilingual parity between English and Spanish versions.

## 2026-03-25 - Cerebro Migration Guide Sync - Completed

**Verification:** Inspected `clients/agent-runtime/src/config/schema.rs` to confirm the correct environment variable for Cerebro auth token is `CORVUS_CEREBRO_AUTH_TOKEN`.
**Changes:**
- Removed `> **ES pending**` markers from `clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md` (EN).
- Fixed environment variable name in `clients/web/apps/docs/src/content/docs/es/guides/cerebro/migration.md` (ES) from `CEREBRO_AUTH_TOKEN` to `CORVUS_CEREBRO_AUTH_TOKEN`.
- Clarified that no SurrealDB fallback is attempted in the runtime if Cerebro is unreachable in the Spanish version.
- Ensured strict bilingual parity for the entire guide.
**Validation:** Ran `make docs-web-build` and `make docs-web-check` (via `make docs-build` and `make docs-check`).
**Notes:**
- The Spanish translation was already complete but had pending markers in the English file.
- Corrected a logic discrepancy in the Spanish version regarding storage fallback.
