# Tasks: Slash Session Commands

## Phase 1: Parser and contracts

- [x] 1.1 Add RED parser/pre-execution tests in `clients/agent-runtime/src/session_commands/parser.rs` and `clients/agent-runtime/src/pre_execution/mod.rs` for exact matches, `/resume {id}`, trailing args, and `/resume-later` fallthrough.
- [x] 1.2 Create `clients/agent-runtime/src/session_commands/{mod.rs,types.rs,parser.rs,registry.rs}` and export the module from `clients/agent-runtime/src/lib.rs` and `clients/agent-runtime/src/main.rs` with static command specs for `/resume`, `/suspend`, `/tldr`, and `/compact`.
- [x] 1.3 Add RED contract tests in `clients/agent-runtime/crates/corvus-traits/src/memory.rs` for slash-session types/default methods, then extend `Memory` and `clients/agent-runtime/src/memory/traits.rs` with snapshot/state records, lifecycle enums, resumable listings, and explicit unsupported defaults.

## Phase 2: SQLite state and hydration

- [x] 2.1 Add RED SQLite persistence tests in `clients/agent-runtime/src/memory/sqlite.rs` for additive/idempotent `session_snapshots` + `session_state` migrations, snapshot creation, resume listing, and pending-hydration take-once behavior.
- [x] 2.2 Implement the new SQLite schema and slash-session CRUD/query helpers in `clients/agent-runtime/src/memory/sqlite.rs`, keeping `sessions` as identity/listing only.
- [x] 2.3 Add RED unsupported-backend tests, then implement explicit slash-session unsupported errors in `clients/agent-runtime/src/memory/{markdown.rs,lucid.rs,none.rs}`.
- [x] 2.4 Add RED hydration tests in `clients/agent-runtime/src/agent/memory_loader.rs` for normal recall performed first, then any persisted resume context prepended into the final assembled context, and clearing `pending_hydration_snapshot_id` atomically when hydration completes.
- [x] 2.5 Implement resume hydration in `clients/agent-runtime/src/agent/memory_loader.rs` using the new `Memory::take_pending_resume_hydration` seam.

## Phase 3: Deterministic command service and ingress wiring

- [x] 3.1 Add RED service tests in `clients/agent-runtime/src/session_commands/service.rs` for deterministic `/tldr`, `/compact`, `/suspend`, `/resume`, ended-session rejection, missing-snapshot errors, and no model/tool execution.
- [x] 3.2 Implement `clients/agent-runtime/src/session_commands/service.rs` and registry dispatch to build deterministic results from SQLite-backed session state only.
- [x] 3.3 Add RED ingress regression tests in `clients/agent-runtime/src/{gateway/webhook_dispatch.rs,gateway/mod.rs,channels/mod.rs,main.rs}` proving recognized commands intercept before autosave, memory enrichment, normal pre-execution, provider execution, and stream startup.
- [x] 3.4 Implement a shared ingress decision helper in `clients/agent-runtime/src/pre_execution/mod.rs` and wire it through `clients/agent-runtime/src/{gateway/mod.rs,gateway/webhook_dispatch.rs,channels/mod.rs,main.rs,bootstrap/mod.rs}`.

## Phase 4: End-to-end verification

- [x] 4.1 Add focused integration/regression coverage for CLI, channel, webhook, and `/web/chat/stream` response mapping so slash commands return deterministic user-visible results and unknown slash-like prompts still follow the normal loop.
- [x] 4.2 Run `cargo fmt --all -- --check`, `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`, and targeted `cargo test` filters for `session_commands`, `memory_loader`, and `/web/chat/stream` slash-session SSE handling; no full build was run per repo instruction.
