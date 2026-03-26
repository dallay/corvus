# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Corvus is a multi-interface agentic platform — a Kotlin Multiplatform (KMP) monorepo with a
high-performance Rust agent runtime, web surfaces (Astro/Vue 3), and Compose Multiplatform
desktop/mobile apps.

## Core Principles

**Security First, Performance Second.** Every decision must prioritize security, then extreme
performance. These override convenience and speed of development.

- Never trust user input; parameterized queries only; principle of least privilege
- Optimize for algorithmic complexity, avoid unnecessary allocations, profile before optimizing
- **TDD by default**: Red → Green → Refactor for new behavior, bug fixes, risky refactors
- **No `!!` operator** in Kotlin; use `?.`, `?:`, `requireNotNull`
- **No `unwrap()` in production Rust**; use `?`, `anyhow`, or `thiserror`

## Build & Run Commands

```bash
# Build
make build              # Full build with tests
make build-fast         # Build without tests
make run                # Run Compose desktop app

# Testing — Kotlin
make test               # All Gradle tests
make test-app           # Desktop app tests only
make test-core          # Core KMP module tests
./gradlew :composeApp:jvmTest --tests "ClassName.methodName"  # Single test
./gradlew :composeApp:jvmTest --tests "*Pattern*"             # Pattern match

# Testing — Rust
make rust-test          # cargo test via Gradle
cargo test --manifest-path clients/agent-runtime/Cargo.toml   # Direct cargo
cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib test_name  # Single test

# Testing — Web
make chat-test          # Chat app tests
make dashboard-test     # Dashboard tests
make web-test-all       # All web tests

# Code Quality
make format             # Spotless apply (Kotlin/Gradle)
make check-format       # Spotless check
make lint-kotlin        # Detekt
make rust-clippy        # Clippy
make rust-fmt           # Rust format check
make check              # All checks (includes Rust with -PenableRustTasks=true)
make check-all          # Full quality gate (format + lint + all tests)

# Web Dev Servers
make docs-dev           # Docs at localhost
make chat-dev           # Chat app
make dashboard-dev      # Dashboard
make marketing-dev      # Marketing site

# Docker Dev Environment
make dev-up             # Start at corvus.localhost
make dev-down           # Stop
make dev-shell          # Enter sandbox container
```

**Important**: Rust tasks require `-PenableRustTasks=true` when using Gradle directly. The Makefile
handles this automatically.

## Architecture

### 3-Tier Client Surfaces

```
Tier 1: Runtime Core   → clients/agent-runtime (Rust CLI/daemon)
Tier 2: Gateway Layer   → HTTP Gateway (web) + RustCliBridge (mobile)
Tier 3: Client Surfaces → web/{chat,dashboard,docs,marketing}, composeApp
```

**Transport rules (mandatory)**:
- Web clients → HTTP Gateway only
- Mobile (composeApp) → RustCliBridge only
- CLI operators → Direct runtime access

### Rust Agent Runtime — Trait-Based Pluggability

Every subsystem in `clients/agent-runtime/src/` is swappable via traits:

| Directory        | Trait              | Purpose                         |
|------------------|--------------------|---------------------------------|
| `providers/`     | `Provider`         | LLM backends (OpenAI, Gemini…) |
| `channels/`      | `Channel`          | Messaging (Slack, Discord, Telegram…) |
| `observability/` | `Observer`         | Metrics/logging (Prometheus, OTLP) |
| `tools/`         | `Tool`             | Agent tools (file, web, shell…) |
| `memory/`        | `Memory`           | Persistence backends            |
| `security/`      | `SecurityPolicy`   | Sandboxing                      |

To add a new integration: implement the trait, register in the subsystem's `mod.rs`.

### Rust Feature Flags

Default features: `hardware`, `mcp-runtime`. Optional: `browser-native`, `sandbox-landlock`,
`sandbox-bubblewrap`, `probe`, `rag-pdf`, `peripheral-rpi` (Linux only).

### Cerebro Memory Module

MCP-based memory service using SurrealDB (embedded). Agents interact via MCP JSON-RPC protocol
(13 memory/session tools). Sync API for fast responses, async worker for background tasks
(embeddings, entity extraction, graph edges).

### Web Monorepo

`clients/web/` is a pnpm workspace with four apps:
- `apps/docs` — Astro Starlight documentation
- `apps/chat` — Vue 3 chat UI
- `apps/dashboard` — Vue 3 admin panel
- `apps/marketing` — Astro landing pages
- `packages/shared` — Shared utilities

Formatting: Biome. Package manager: pnpm 10+.

### KMP Modules

- `modules/agent-core-kmp` — Core contracts library, Tier 2 bridge between mobile and runtime
- `composeApp` — Shared Compose Multiplatform UI (Desktop + Android + iOS targets)

## Code Style

### Formatting (.editorconfig)

- 2 spaces indent, 100 char max line, trailing commas required
- No wildcard imports, UTF-8, trim whitespace, final newline

### Kotlin Conventions

- Data classes with value classes for type safety (`@JvmInline value class UserId(val value: UUID)`)
- Sealed interfaces for result/error hierarchies
- Expression bodies for simple functions
- Booleans: `is`/`has` prefix
- Tests: backtick names `` `should do something` ``

### Rust Conventions

- Minimal dependencies (binary size matters — release profile uses `opt-level = "z"`, `strip = true`)
- Inline tests at bottom of each file: `#[cfg(test)] mod tests {}`
- Trait-first design: define trait, then implement
- Security by default: sandbox everything, allowlist over blocklist

### Naming

- Classes/structs: `PascalCase`
- Functions/methods: `camelCase` (Kotlin) / `snake_case` (Rust)
- Constants: `UPPER_SNAKE_CASE`
- Commit scopes: `provider`, `channel`, `memory`, `security`, `runtime`, `ci`, `docs`, `tests`

## Gradle Guidelines

- Use `tasks.register` not `create` (lazy registration)
- Use `configureEach` not `all`
- Never use `afterEvaluate`
- All dependencies via version catalog (`gradle/libs.versions.toml`)
- Custom plugins in `gradle/build-logic/`
- Gradle wrapper invoked via `bash ./scripts/gradlew.sh`

## Git Hooks & CI

Git hooks in `.githooks/` — enable with `git config core.hooksPath .githooks`:
- **pre-commit**: runs `spotlessApply` on staged Kotlin/Gradle files, checks forbidden files,
  validates local links with lychee
- **pre-push**: Rust validations, `checkLocksAll`, Kover XML reports

CI runs the same checks. Conventional Commits required. Squash merge preferred.

## PR Guidelines

- One concern per PR, prefer small PRs (XS/S/M)
- Template is mandatory (`.github/pull_request_template.md`)
- Every PR must include a fast rollback path
- Changes in `security/`, runtime, and CI need stricter validation

## Prerequisites

- JDK 21+, Rust 1.75+, Node.js 22+, pnpm 10+
- Docker (optional, for sandbox environment)
- Run `make setup` for initial project setup
- Run `make doctor` to diagnose environment health

## Project Structure

```text
├── clients/
│   ├── agent-runtime/       # Rust CLI/daemon (Tier 1)
│   ├── web/                 # pnpm monorepo: docs, chat, dashboard, marketing
│   ├── composeApp/          # KMP Compose UI (Desktop/Mobile)
│   ├── androidApp/          # Android host
│   └── iosApp/              # iOS host
├── modules/
│   ├── agent-core-kmp/      # KMP contracts (Tier 2 bridge)
│   ├── agent-core-rust/     # Embedded Rust AI core
│   └── cerebro/             # MCP memory service
├── gradle/
│   ├── build-logic/         # Custom Gradle plugins
│   └── libs.versions.toml   # Version catalog
├── dev/                     # Docker dev environment
├── openspec/                # External specs (volatile)
└── .agents/skills/          # AI agent skill definitions
```
