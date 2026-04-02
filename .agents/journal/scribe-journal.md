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

## 2025-05-18 - Tools Reference Documentation - Completed

**Verification:** Audited `clients/agent-runtime/src/tools/` to identify all built-in tools, their parameters, and security tiers. Verified the integration of `agent-browser` and MCP.
**Changes:**
- Created a comprehensive Tools Reference section in both English and Spanish (14 new files).
- Categorized tools into: Core (shell, file_read/write), Web (browser, http_request, search), Memory (store/recall/forget), Automation (git, cron, schedule), Media (screenshot, image_info), and MCP.
- Documented Security Operation Tiers (Safe/Read-Only vs. Risk/Action-Bearing).
- Updated index pages in `docs/clients/agent-runtime/` and `docs/es/clients/agent-runtime/` to link to the new Tools Reference.
**Validation:**
- Ran `make docs-check` and `make docs-build`. 58 pages built successfully.
- Visual verification performed via Playwright for both English and Spanish layouts.
**Notes:**
- Confirmed strict 1:1 parity between `en/` and `es/` directories.
- Technical details like `mcp.<server>.<tool>` naming convention and `agent-browser` requirements are now documented.
