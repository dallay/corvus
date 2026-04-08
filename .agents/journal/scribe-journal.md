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

## 2025-05-22 - Automation and Hardware Tools Reference - Completed

**Verification:** Audited `clients/agent-runtime/src/tools/` and `clients/agent-runtime/src/peripherals/` to verify parameters and security tiers for all mission-critical tools. Confirmed `delegate` execution modes (OneShot/Session) and verified the full set of `cron_*` tools.
**Changes:**
- Updated `automation.md` (EN/ES) to include `delegate`, `composio`, and complete `cron_*`/`schedule` reference.
- Created `hardware.md` (EN/ES) documenting board discovery (`hardware_board_info`), memory operations (`hardware_memory_map`, `hardware_memory_read`), and peripheral control (`gpio_read`, `gpio_write`, `arduino_upload`).
- Updated `index.mdx` (EN/ES) to include Hardware category and update tool counts/examples.
**Validation:**
- Ran `pnpm --dir clients/web --filter @corvus/locales test` (Passed).
- Ran `./gradlew :web:docsCheck` (Passed: Astro check, Biome lint, Metadata validation).
**Notes:**
- Maintained strict 1:1 parity between English and Spanish.
- Confirmed that `gpio_read`/`gpio_write` are available on both RPi (native) and Uno Q (bridge).
- Documented `arduino_upload` requirements (`arduino-cli`).
