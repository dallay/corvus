# Tasks: Tooling Parity for Search, Fetch, and Task Tools

## Phase 1: Shared Foundations

- [x] 1.1 Add metadata-only discovery coverage in `clients/agent-runtime/src/search/discovery.rs` for workspace-relative results, escape rejection, and stable sort inputs; keep content-search discovery behavior unchanged. Validate with targeted discovery/search tests.
- [x] 1.2 Create `clients/agent-runtime/src/search/content.rs`, export it from `clients/agent-runtime/src/search/mod.rs`, and refactor `clients/agent-runtime/src/tools/code_search.rs` to use the shared backend with parity-preserving ordering/count regression tests. Validate with targeted `search` and `code_search` tests.
- [x] 1.3 Create `clients/agent-runtime/src/tools/http_common.rs` and refactor `clients/agent-runtime/src/tools/http_request.rs` to reuse shared URL-policy and bounded GET helpers without changing `http_request`’s public contract. Validate with targeted HTTP policy tests.

## Phase 2: Parity Tool Implementations

- [x] 2.1 Write failing tests, then implement `clients/agent-runtime/src/tools/glob.rs` for `pattern`/`path` validation, deterministic `filenames`, `numFiles`, `durationMs`, and `truncated`; register it in `clients/agent-runtime/src/tools/mod.rs`. Validate with targeted `glob` tests.
- [x] 2.2 Write failing tests, then implement `clients/agent-runtime/src/tools/grep.rs` on the shared search backend for `glob`, `output_mode`, context validation, offset/limit handling, zero-match success, and deterministic ordering; register it in `clients/agent-runtime/src/tools/mod.rs`. Validate with targeted `grep` tests.
- [x] 2.3 Write failing tests, then implement `clients/agent-runtime/src/tools/web_fetch.rs` for read-only `url`/`prompt` validation, textual extraction, private-host and unsupported-scheme denial, and binary-response rejection; register it in `clients/agent-runtime/src/tools/mod.rs`. Validate with targeted `web_fetch` tests.

## Phase 3: Runtime Wiring and Inventory

- [x] 3.1 Update `clients/agent-runtime/src/bootstrap/mod.rs` and `clients/agent-runtime/src/security/policy.rs` so code/full profiles and plan-safe read-only sets expose `Glob`, `Grep`, and `WebFetch` without removing legacy names; extend gating tests in the touched modules. Validate with targeted bootstrap/profile tests.
- [x] 3.2 Update `clients/agent-runtime/src/session_commands/service.rs` so `/tools` lists enabled parity tools with descriptions that explain the native backing relationship and does not advertise disabled parity tools; refresh related rendering tests if present. Validate with targeted session-command tests.

## Phase 4: Parity Mapping Documentation

- [x] 4.1 Update `clients/web/apps/docs/src/content/docs/clients/agent-runtime/tools/index.mdx`, `core.md`, and `web.md` to publish the parity mapping table, additive/canonical status, and explicit `Task*` deferral for this slice.
- [x] 4.2 Mirror the same parity mapping and scope-boundary updates in `clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/tools/index.mdx`, `core.md`, and `web.md`; keep names and status labels aligned with the English docs and runtime inventory wording.

## Phase 5: Targeted Validation

- [x] 5.1 Run Rust validation for the slice: `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`, `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`, and focused `cargo test` filters for `glob`, `grep`, `web_fetch`, shared search, bootstrap, and session inventory coverage.
- [x] 5.2 Run docs checks only if the touched docs workspace requires them; otherwise record the skipped docs validation and reason in implementation notes so verification can account for it.
