# Scribe Journal

## 2025-05-15 - AI Providers Documentation - In Progress

**Verification:** I have inspected `clients/agent-runtime/src/providers/mod.rs` to identify supported providers and their credential requirements (environment variables).
**Changes:** Planned to create `providers/` directory under `clients/agent-runtime/` in both English and Spanish documentation.
**Validation:** Will run `make docs-web-build` and `make docs-web-check`.
**Notes:**
- Supported providers: OpenRouter, Anthropic, OpenAI, Ollama, Gemini, etc.
- Credentials:
    - `OPENROUTER_API_KEY`
    - `ANTHROPIC_API_KEY` / `ANTHROPIC_OAUTH_TOKEN`
    - `OPENAI_API_KEY`
    - `GEMINI_API_KEY` / `GOOGLE_API_KEY`
    - `OLLAMA_API_KEY` (for cloud routing)
    - `GITHUB_TOKEN` / `GH_TOKEN` (for Copilot)
