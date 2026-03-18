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
