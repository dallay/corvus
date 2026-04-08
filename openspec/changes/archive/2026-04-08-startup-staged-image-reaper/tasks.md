# Tasks: Startup Staged Image Reaper

## Phase 1: Foundation / Config

- [x] 1.1 RED: Extend config tests in `clients/agent-runtime/src/config/schema.rs` for missing threshold -> default 30, explicit override, and `staged_image_reaper_threshold_minutes = 0` failing startup validation.
- [x] 1.2 GREEN: Add `staged_image_reaper_threshold_minutes: Option<u64>` and validation in `clients/agent-runtime/src/config/schema.rs` without changing existing multimodal defaults or MVP channel warnings.
- [x] 1.3 REFACTOR: Keep threshold resolution explicit and reusable from `clients/agent-runtime/src/main.rs`, using the shared default contract from the design.

## Phase 2: Core Reaper Logic

- [x] 2.1 RED: Add filename-matching tests in `clients/agent-runtime/src/channels/media.rs` for valid `corvus-{channel}-img-{sha16}-{nonce8}.{ext}` and legacy `corvus-tg-img-{sha16}.{ext}` names, plus near-miss rejections.
- [x] 2.2 RED: Add temp-dir sweep tests in `clients/agent-runtime/src/channels/media.rs` covering stale deletion, fresh-file preservation, unrelated-file preservation, future/undeterminable timestamp skip, and duplicate execution safety.
- [x] 2.3 GREEN: Implement `DEFAULT_STAGED_IMAGE_REAPER_THRESHOLD_MINUTES`, strict filename parsing, `StagedImageReaperReport`, and best-effort sweep helpers in `clients/agent-runtime/src/channels/media.rs` scoped to `std::env::temp_dir()` root entries only.
- [x] 2.4 REFACTOR: Preserve existing `StagedImageGuard` request-path cleanup semantics in `clients/agent-runtime/src/channels/media.rs` and keep delete/metadata races non-fatal.

## Phase 3: Startup Wiring

- [x] 3.1 RED: Add focused startup-hook tests in `clients/agent-runtime/src/main.rs` proving gateway, daemon, channel start, and onboarding autostart all route through one reaper entry point with default and overridden thresholds.
- [x] 3.2 GREEN: Add `run_startup_staged_image_reaper` in `clients/agent-runtime/src/main.rs`, call it from the required startup boundaries, and log aggregate cleanup counts at info level without file names.
- [x] 3.3 REFACTOR: Keep `clients/agent-runtime/src/main.rs` wiring thin so long-running command behavior is unchanged beyond the pre-start cleanup hook.

## Phase 4: Verification

- [x] 4.1 Run targeted Rust tests for `clients/agent-runtime/src/config/schema.rs`, `clients/agent-runtime/src/channels/media.rs`, and `clients/agent-runtime/src/main.rs`, confirming all spec scenarios for stale cleanup, preservation, legacy support, and duplicate execution.
- [x] 4.2 Run a focused lint pass for `clients/agent-runtime` and fix any warnings introduced by the new reaper helpers or startup-hook extraction.
