# AGENTS.md
Agent operating guide for the Corvus monorepo.
Scope: entire repository.

## Agent Metadata

```yaml
name: corvus-agent
description: >-
  Operating guide for AI agents working in the Corvus multi-interface
  agentic platform monorepo.
purpose: >-
  Provide deterministic, machine-parseable instructions so that any
  compliant AI coding agent can navigate, build, lint, test, and
  contribute to the Corvus repository safely and effectively.
capabilities:
  - kotlin-multiplatform-development
  - rust-runtime-development
  - astro-vue-web-development
  - gradle-build-logic
  - ci-cd-workflow-management
  - documentation-governance
  - multi-language-linting-and-formatting
  - test-driven-development
version: "1.0"
compatibility:
  - claude-code
  - opencode
  - github-copilot
  - cursor
```

## Mission
Corvus is a multi-interface agentic platform:
- Kotlin Multiplatform in `clients/composeApp` and `modules/agent-core-kmp`
- Rust runtime in `clients/agent-runtime`
- Astro/Vue web apps in `clients/web`
- shared Gradle build logic in `gradle/`
Optimize in this order: security, performance, maintainability, delivery speed.

## Repo rule sources
Checked for extra agent rules:
- `.cursor/rules/`: not present
- `.cursorrules`: not present
- `.github/copilot-instructions.md`: not present
Incorporated guidance from `.agents/AGENTS.md` and `clients/agent-runtime/AGENTS.md`.

## Non-negotiables
- Follow TDD by default: Red → Green → Refactor.
- For bug fixes, add a regression test first when practical.
- Keep patches small and local.
- Never invent APIs, modules, commands, or file paths.
- Do not weaken sandboxing, auth, secrets handling, or policy boundaries.
- Do not log secrets, tokens, pairing codes, or sensitive payloads.
- Kotlin: no `!!` in production code.
- Rust: no `unwrap()` / `expect()` in production paths unless failure is truly impossible.
- Prefer safe, reversible changes with a clear rollback path.

## Architecture essentials
- Tier 1: `clients/agent-runtime` — Rust runtime core
- Tier 2: gateway / bridge layer
- Tier 3: `clients/web/*`, `clients/composeApp`, mobile hosts
Transport rules: web clients use the HTTP gateway only; mobile clients use the Rust CLI bridge only; CLI/operator flows may access runtime capabilities directly.
Rust extension points in `clients/agent-runtime/src/` are trait-based: `Provider`, `Channel`, `Tool`, `Memory`, `Observer`, `SecurityPolicy` under `providers/`, `channels/`, `tools/`, `memory/`, `observability/`, and `security/`.
When adding an integration, implement the trait and register it in the module/factory entrypoint.

## Prerequisites
JDK 21+, Node.js 22+, pnpm 10+, Rust 1.75+, Docker optional.
Bootstrap: `make check-tools`, `make setup`, `make doctor`.

## Build, lint, and test commands
Prefer `make` targets first.

### Whole repo
- `make build`, `make build-fast`, `make clean`
- `make check`, `make check-all`, `make quick`

### Kotlin / Android / KMP
- `make run`, `make android-build`, `make android-lint`
- `make test`, `make test-app`, `make test-core`
Single Gradle test examples:
- `bash ./scripts/gradlew.sh :composeApp:jvmTest --tests "ClassName.methodName"`
- `bash ./scripts/gradlew.sh :composeApp:jvmTest --tests "*Pattern*"`
- `bash ./scripts/gradlew.sh :agent-core-kmp:jvmTest --tests "ClassName.methodName"`
- `bash ./scripts/gradlew.sh :agent-core-kmp:jvmTest --tests "*Pattern*"`
Module builds:
- `bash ./scripts/gradlew.sh :composeApp:build`
- `bash ./scripts/gradlew.sh :agent-core-kmp:build`
- `bash ./scripts/gradlew.sh :androidApp:assembleDebug`

### Rust runtime
If using Gradle directly for Rust tasks, include `-PenableRustTasks=true`.
- `make rust-check`, `make rust-test`, `make rust-clippy`, `make rust-fmt`, `make rust-build`
- `make rust-test-matrix`
Direct cargo examples:
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib test_name`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml module_name::tests::test_name`
- `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`

### Web workspace
- `make web-install`, `make web-build-all`, `make web-test-all`, `make web-check-all`
- `make docs-dev` / `make docs-build` / `make docs-check`
- `make chat-dev` / `make chat-build` / `make chat-check` / `make chat-test`
- `make dashboard-dev` / `make dashboard-build` / `make dashboard-check` / `make dashboard-test`
- `make marketing-dev` / `make marketing-build` / `make marketing-check`
Direct pnpm examples from `clients/web`:
- `pnpm test`, `pnpm test:chat`, `pnpm test:dashboard`, `pnpm test:dashboard:e2e`
Single web test examples:
- `pnpm --dir clients/web --filter @corvus/chat test -- src/path/to/file.test.ts`
- `pnpm --dir clients/web --filter @corvus/chat test -- src/path/to/file.test.ts -t "case name"`
- `pnpm --dir clients/web --filter @corvus/dashboard test -- src/path/to/file.test.ts`
- `pnpm --dir clients/web --filter @corvus/dashboard exec playwright test e2e/file.spec.ts --grep "case name"`

### Formatting / lint / coverage
- `make format`, `make check-format`, `make lint-kotlin`, `make lint-rust`, `make lint-android`
- `make docs-check`, `make chat-check`, `make dashboard-check`, `make marketing-check`
- `make test-coverage`, `make rust-coverage`, `make link-check`, `make link-check-local`

## Code style
Formatting and imports:
- Follow `.editorconfig`: 2-space indentation, UTF-8, trim trailing whitespace, final newline.
- Kotlin/KTS max line length is 100.
- Use trailing commas where formatter expects them.
- Do not use wildcard imports.
- Keep import lists explicit and stable.
- Web code follows `clients/web/biome.json`: spaces, width 100, double quotes, trailing commas `es5`.
Kotlin:
- Prefer strong types and small focused APIs.
- Use value classes where they improve safety.
- Prefer sealed interfaces/classes for result and error hierarchies.
- Prefer expression bodies for simple functions.
- Boolean names should read like predicates: `isX`, `hasX`, `canX`.
- Use null-safe operators and `requireNotNull`; avoid `!!`.
- Keep coroutines structured; do not leak scopes.
- Tests may use backtick names: ``fun `should do something`()``.
Rust:
- Prefer minimal dependencies; binary size matters.
- Avoid needless clones, allocations, and blocking operations.
- Keep tests near code with `#[cfg(test)]` where practical.
- Prefer trait-first extension points over hard-coded branching.
- Use `Result`, `?`, `thiserror`, or `anyhow` appropriately.
TypeScript / Vue / Astro:
- Use strict typing; avoid `any` unless justified and tightly contained.
- Prefer shared types over stringly typed objects.
- Keep components small and composables focused.
- Validate external data at the boundary.
- In web surfaces, do not bypass the gateway contract.
Naming:
- Types/classes/structs/components: `PascalCase`
- Kotlin functions/properties: `camelCase`
- Rust functions/modules: `snake_case`
- Constants: `UPPER_SNAKE_CASE`
- Boolean fields: predicate style (`isReady`, `hasAccess`)

## Error handling, dependencies, and build logic
- Fail closed on auth, permission, sandbox, and policy checks.
- Prefer explicit errors over silent fallbacks.
- Validate and sanitize tool inputs.
- Use allowlists over blocklists.
- Preserve existing CLI and API contracts unless intentionally changing them.
- Use `tasks.register`, not eager task creation; use `configureEach`, not `all`; never use `afterEvaluate`.
- Keep dependencies in `gradle/libs.versions.toml` and reusable Gradle conventions in `gradle/build-logic/`.
- Prefer the wrapper script: `bash ./scripts/gradlew.sh`.

## Validation expectations
Before declaring work done, run the smallest relevant checks:
- docs-only: relevant formatting/checks
- Kotlin/KMP: targeted tests + relevant lint/format
- Rust: targeted `cargo test` + clippy if behavior changed
- Web: target app `check` + relevant Vitest/Playwright tests
If validation cannot be run, state exactly what was skipped and why.

## Handy paths
- `clients/agent-runtime/`, `clients/web/`, `clients/composeApp/`, `clients/androidApp/`
- `modules/agent-core-kmp/`, `gradle/build-logic/`, `.githooks/`
