---
title: Features Checklist
description: Repository-wide checklist of modules, capabilities, build features, and supported integration surfaces in Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: reference
---

This page provides a comprehensive checklist of all functionalities, modules, and options available
in this repository.

## Modules

- [x] **clients/composeApp**: Shared Kotlin Multiplatform Compose UI module (desktop + iOS + Android
  library target).
- [x] **clients/androidApp**: Native Android application host for the shared Compose UI.
- [x] **clients/web**: Web application workspace (Astro/Vue apps: docs, dashboard, marketing).
- [x] **clients/agent-runtime**: Rust agent runtime (gateway + daemon + CLI + 22+ providers + 14 channels + 32+ tools).
- [x] **modules/agent-core-kmp**: Shared Kotlin Multiplatform core bootstrap.
- [x] **clients/cerebro**: Standalone MCP memory service client (SurrealDB, 13 memory tools, optional TUI).
- [x] **gradle/build-logic**: Centralized convention plugins.
- [x] **gradle/aggregation**: Aggregated reporting for tests and coverage.
- [x] **gradle/versions**: Dependency version management and consistency checks.

## Agent Runtime — AI Providers (22+)

- [x] OpenRouter (recommended aggregator)
- [x] Anthropic (Claude models, setup-tokens, OAuth)
- [x] OpenAI (GPT models)
- [x] OpenAI Codex (OAuth-based)
- [x] Google Gemini (API key + CLI OAuth token reuse)
- [x] Ollama (local models)
- [x] LM Studio (local, `http://localhost:1234`)
- [x] Venice AI, Groq, Mistral, DeepSeek, xAI/Grok
- [x] Together AI, Fireworks AI, Perplexity, Cohere
- [x] GitHub Copilot (GitHub subscription)
- [x] Amazon Bedrock, Synthetic, OpenCode Zen, NVIDIA NIM
- [x] Vercel AI, Cloudflare AI, Astrai
- [x] Regional: Moonshot/Kimi, GLM/Zhipu, MiniMax, Qwen/DashScope, Qianfan/Baidu, Z.AI
- [x] Custom endpoints: `custom:<URL>`, `anthropic-custom:<URL>`
- [x] Provider pools (multi-account rotation)
- [x] Model routing with `[[model_routes]]` and query classification
- [x] Reliable provider wrapper (retries, backoff, fallback chains)

## Agent Runtime — Communication Channels (14)

- [x] CLI (interactive terminal)
- [x] Telegram (polling)
- [x] Discord (WebSocket gateway)
- [x] Slack (Web API)
- [x] WhatsApp (Meta Cloud API webhooks)
- [x] Signal (signal-cli)
- [x] iMessage (macOS AppleScript)
- [x] Matrix
- [x] DingTalk (Stream mode)
- [x] QQ (Tencent Bot SDK)
- [x] Lark/Feishu (WebSocket)
- [x] Email (IMAP/SMTP)
- [x] IRC
- [x] Mattermost

## Agent Runtime — Tools (32+)

- [x] `shell` — Command execution with security policy enforcement
- [x] `code_search` — Workspace file search (literal + regex)
- [x] `file_read` / `file_write` — Workspace filesystem access
- [x] `memory_store` / `memory_recall` / `memory_forget` — Long-term memory
- [x] `web_search` — Web search (Brave provider)
- [x] `http_request` — Structured API calls
- [x] `browser` / `browser_open` — Web browsing / computer use
- [x] `screenshot` / `image_info` — Vision capabilities
- [x] `git_operations` — Git repository management
- [x] `composio` — Managed app integrations
- [x] `delegate` — Multi-agent delegation
- [x] `pushover` — Notifications
- [x] `cron_add` / `cron_list` / `cron_remove` / `cron_update` / `cron_run` / `cron_runs` — Scheduled tasks
- [x] `schedule` — Task scheduling
- [x] `hardware_board_info` / `hardware_memory_map` / `hardware_memory_read` — Hardware introspection
- [x] MCP tools (namespaced as `mcp.<server>.<tool>`, gated by `mcp.enabled`)

## Agent Runtime — Memory

- [x] SQLite backend (hybrid vector + FTS5 search)
- [x] Lucid backend (SQLite with enhanced retrieval)
- [x] Markdown backend (file-based)
- [x] None backend (no persistence)
- [x] Embedding generation (OpenAI, custom URL, noop)
- [x] Response cache (LRU SQLite cache)
- [x] Memory snapshots (export/import)
- [x] Memory hygiene (retention with throttling)
- [x] Cerebro MCP integration (long-term memory via external service)

## Agent Runtime — Infrastructure

- [x] Gateway API (Axum HTTP server with health, pairing, webhooks, SSE streaming)
- [x] Cron scheduler (expressions, one-shot, fixed-interval, delayed tasks)
- [x] Heartbeat engine (periodic liveness signals)
- [x] Doctor diagnostics (daemon, scheduler, channel freshness, config validation)
- [x] OS service management (systemd on Linux, launchd on macOS)
- [x] Observability (noop, log, prometheus, OpenTelemetry/OTLP, multi-backend)
- [x] Auth profiles (OAuth for Codex, setup tokens for Anthropic, profile management)
- [x] Cost tracking (per-model pricing, session/daily/monthly limits, budget overrides)
- [x] Update system (self-update checking, install transactions, audit history)
- [x] Skills system (catalog, install, lockfile, trust/validation, sandbox)
- [x] Integrations browser (50+ entries across 9 categories)
- [x] Migration (OpenClaw memory import)
- [x] Agent composition (`agent build`, `agent run`, `agent new`)
- [x] Capability profiles (`full`, `code`, `lite`)
- [x] Model routing and query classification
- [x] Multimodal support (images, configurable limits)
- [x] Audio support (Whisper.cpp transcription)
- [x] Mission system (runtime limits, step/cost budgets)
- [x] Pre-execution evaluation pipeline

## Agent Runtime — Security

- [x] Security policy (autonomy levels, command allowlists, risk classification)
- [x] Command risk levels: low, medium, high with forbidden paths
- [x] Path traversal protection (iterative URL decoding, null byte blocking)
- [x] Rate limiting (configurable, default 20/hr)
- [x] Secret store (AEAD encryption with chacha20poly1305)
- [x] Pairing guard (6-digit one-time code, bearer token exchange)
- [x] Sandbox backends: Landlock (Linux kernel), Firejail (Linux user-space), Bubblewrap, Docker
- [x] Sandbox auto-detection with platform-specific ordering
- [x] Computer-use sidecar isolation with health verification
- [x] Audit logging for all sensitive operations

## Agent Runtime — Hardware & Peripherals

- [x] USB device enumeration (nusb)
- [x] STM32/Nucleo support (probe-rs, firmware flashing)
- [x] Raspberry Pi GPIO
- [x] ESP32 bridge
- [x] Arduino Uno Q bridge
- [x] Serial device support
- [x] Peripheral management CLI (`list`, `add`, `flash`, `setup`)

## Agent Runtime — Tunnel Providers

- [x] Cloudflare
- [x] Tailscale
- [x] Ngrok
- [x] Custom tunnels

## Build & Quality

- [x] **Convention Plugins**: Modular and reusable build logic in `gradle/build-logic/`.
- [x] **Version Catalog**: Centralized dependency management in `gradle/libs.versions.toml`.
- [x] **Dependency Analysis**: Tools to detect unused or misconfigured dependencies.
- [x] **Reproducible Builds**: Dependency locking with Gradle lockfiles.
- [x] **Multi-language Support**: Kotlin, Rust, TypeScript/JavaScript.
- [x] **Code Formatting**: Spotless (Kotlin/Java), Biome (web), rustfmt (Rust).
- [x] **Static Analysis**: Detekt (Kotlin), Clippy (Rust), Biome (web).
- [x] **Testing**: Kotlin test (JUnit 5 + Kover), Rust tests (cargo test), web tests (Vitest + Playwright).
- [x] **SBOM**: Software Bill of Materials generation.
- [x] **Git Hooks**: Automated pre-commit checks via `.githooks/`.

## Documentation

- [x] **Static Website**: Built with Astro and Starlight (bilingual: en/es).
- [x] **API Docs**: Generated with Dokka (Kotlin/Java).
- [x] **In-repo docs**: `AGENTS.md`, `CONTRIBUTING.md`, `README.md`.

## Deployment & Distribution

- [x] **Shadow JAR**: Executable fat-jars with bundled dependencies.
- [x] **Maven Publishing**: Pre-configured publishing to Maven repositories.
- [x] **BOM Support**: Bill of Materials for dependency alignment.
- [x] **Docker runtime**: Configurable container execution for agent sandboxing.
- [x] **Self-update**: Runtime update checking with install transactions and audit trail.
