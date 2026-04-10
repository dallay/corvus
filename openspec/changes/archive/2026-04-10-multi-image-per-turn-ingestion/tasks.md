# Tasks: Multi-Image Per-Turn Ingestion

## Phase 1: Foundation / Config

- [x] 1.1 RED: Extend `clients/agent-runtime/src/config/schema.rs` tests for `multimodal.max_images_per_turn` default=4, valid override, `0` rejection, and `>8` rejection.
- [x] 1.2 GREEN: Add `MultimodalConfig.max_images_per_turn`, `effective_max_images_per_turn()`, startup validation, and startup log fields in `clients/agent-runtime/src/config/schema.rs`.
- [x] 1.3 RED/GREEN: Update `clients/agent-runtime/src/channels/media.rs` tests and helpers to use `DEFAULT_MAX_IMAGES_PER_TURN=4`, `MAX_IMAGES_PER_TURN_CEILING=8`, and limit-aware `validate_image_count(count, limit)`.

## Phase 2: Channel gating / staging

- [x] 2.1 RED: Add `clients/agent-runtime/src/channels/mod.rs` regression tests for admitted 3-4 image turns, configured-limit rejection, and deterministic over-limit error text.
- [x] 2.2 GREEN: Update `clients/agent-runtime/src/channels/mod.rs` gate logic to use `effective_max_images_per_turn()`, reject whole turns before staging, and preserve all admitted `Vec<StagedImage>` entries in order.
- [x] 2.3 RED: Add `clients/agent-runtime/src/channels/mod.rs` regression coverage for partial staging failure where image 1 stages, image 2 fails, and prior temp files are cleaned up.
- [x] 2.4 GREEN: Implement best-effort cleanup inside `stage_channel_images()` in `clients/agent-runtime/src/channels/mod.rs` for mid-loop failure without changing success-path RAII cleanup.

## Phase 3: Observability / logging

- [x] 3.1 RED: Extend observer/log tests in `clients/agent-runtime/src/observability/traits.rs` and channel observer tests to assert `image_count`, ordered `images[]`, `total_byte_len`, rejected attempted count, and legacy single-image shim fields.
- [x] 3.2 GREEN: Evolve `ImageIngressEvent` in `clients/agent-runtime/src/observability/traits.rs` to turn-level metadata, keeping `mime_type`/`byte_len` only for single-image turns.
- [x] 3.3 GREEN: Update `clients/agent-runtime/src/observability/log.rs` and channel emission sites to log the new schema without raw bytes/base64 payloads.

## Phase 4: Provider/runtime regressions

- [x] 4.1 RED: Add provider-local regressions in `clients/agent-runtime/src/providers/compatible.rs`, `anthropic.rs`, and `gemini.rs` proving all staged images are serialized in order on the last user turn.
- [x] 4.2 GREEN: Fix any provider request builders still collapsing to the first image while preserving existing fail-closed behavior for invalid last-message shapes.
  - Verified existing compatible, Anthropic, and Gemini builders already preserved full ordered slices; regression coverage now locks that behavior in.
- [x] 4.3 RED/GREEN: Add end-to-end runtime regressions in `clients/agent-runtime/src/channels/mod.rs` or adjacent runtime tests for multi-image success, over-limit whole-turn rejection, provider-sent telemetry, and provider-error telemetry.

## Phase 5: Documentation / verify handoff

- [x] 5.1 Add brief implementation notes in `openspec/changes/multi-image-per-turn-ingestion/tasks.md` completion comments or verify handoff notes identifying config defaults/ceiling, cleanup behavior, and telemetry compatibility shims to confirm during `sdd-verify`.
  - Verify `multimodal.max_images_per_turn` defaults to 4, rejects `0`, and rejects values above ceiling 8 at startup.
  - Verify `stage_channel_images()` removes already-staged temp files when a later image in the same turn fails.
  - Verify observability keeps `mime_type`/`byte_len` only for single-image turns while multi-image turns use ordered `images[]` plus `total_byte_len`.
- [x] 5.2 During `sdd-archive`, sync `openspec/specs/channel-image-ingestion/spec.md` and `openspec/specs/runtime-image-pipeline/spec.md` from the approved deltas after implementation is verified.
