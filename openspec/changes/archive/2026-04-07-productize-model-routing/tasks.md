# Tasks: Productize Model Routing (Phase 1)

## Phase 1: Foundation / Spec Sync

- [x] 1.1 Create `openspec/specs/model-routing/spec.md` from `openspec/changes/productize-model-routing/specs/model-routing/spec.md`, preserving all Phase 1 requirements and scenarios.
- [x] 1.2 Verify the main spec matches the delta for docs coverage, doctor warnings, fallback logging, and image-routing contract; fix any wording drift before code changes.

## Phase 2: Documentation

- [x] 2.1 Create `clients/web/apps/docs/src/content/docs/guides/model-routing.md` with standard frontmatter, config field reference, TOML examples, hint-flow explanation, and troubleshooting tied to doctor warnings.
- [x] 2.2 Create `clients/web/apps/docs/src/content/docs/es/guides/model-routing.md` as a Spanish mirror of the English guide with equivalent examples, flow explanation, and troubleshooting.
- [x] 2.3 Confirm both guides align with the spec: fast/reasoning, code, vision, and multi-provider examples are present and avoid leaking Rust-internal details.

## Phase 3: Runtime Validation / Warning Logs

- [x] 3.1 RED: In `clients/agent-runtime/src/doctor/mod.rs`, add failing unit tests for orphaned hint, enabled-with-no-rules, enabled-with-no-routes, never-matching rule, and valid-config no-warning.
- [x] 3.2 GREEN: Implement `check_classification_integrity` in `clients/agent-runtime/src/doctor/mod.rs` and emit `Severity::Warn` diagnostics for the four warning cases only when applicable.
- [x] 3.3 Wire `check_classification_integrity` into the existing doctor semantic/config flow in `clients/agent-runtime/src/doctor/mod.rs`.
- [x] 3.4 REFACTOR: Keep doctor diagnostics messages specific and stable enough for docs troubleshooting and test assertions.
- [x] 3.5 Update `clients/agent-runtime/src/providers/router.rs` to improve the unknown-hint `tracing::warn!` message with the hint name and raw fallback model string, without changing routing behavior.
- [x] 3.6 Update `clients/agent-runtime/src/providers/mod.rs` to improve the skipped-provider `tracing::warn!` message with the failed provider and affected route hints, without changing init behavior.
- [x] 3.7 Add or update targeted Rust tests for router/provider warning changes only if the module already has a practical pattern; otherwise keep existing behavior tests unchanged.

## Phase 4: Validation

- [x] 4.1 Run targeted Rust tests for doctor changes, e.g. `cargo test --manifest-path clients/agent-runtime/Cargo.toml doctor` or the smallest equivalent module filter.
- [x] 4.2 Run targeted Rust tests for routing/provider modules affected by warning-log changes, using the smallest `cargo test` filters for `providers::router` and routed-provider factory coverage.
- [x] 4.3 Run the smallest docs check available for `clients/web/apps/docs/` that does not require a full build; if none exists, record that limitation in the implementation notes.
- [x] 4.4 Manually verify the final artifact set exists: main spec, EN/ES docs guides, doctor checks/tests, and warning-log updates.
