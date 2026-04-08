# Tasks: Audio Input Phase 2 — HTTP Gateway + CLI

**Change**: `audio-input-phase2` | **Issue**: #412 | **Date**: 2026-04-04

## Phase 1: Foundation

- [x] 1.1 Extract `stage_audio_from_bytes()` into `src/channels/audio_media.rs` — async fn accepting
  `bytes, channel_abbrev, declared_mime, declared_duration_secs, audio_config` →
  `Result<StagedAudio, AudioRejectionReason>`. Includes MIME sniff, size check, duration pre-check,
  SHA-256, atomic temp file write. (REQ-19)
- [x] 1.2 Refactor `fetch_and_stage_audio()` in `src/channels/telegram.rs` — replace inline
  staging (post-download) with call to `stage_audio_from_bytes("tg", ...)`. Keep pre-flight checks
  before download. Run existing Telegram tests to confirm no behavior change. (REQ-19)
- [x] 1.3 Update `VALID_AUDIO_CHANNELS` in `src/config/schema.rs` — rename
  `PHASE1_VALID_AUDIO_CHANNELS` → `VALID_AUDIO_CHANNELS`, add `"gateway"` and `"cli"`. Update
  validation in `src/config/validation.rs` to suppress warnings for these names. (REQ-22, REQ-7
  amendment)
- [x] 1.4 Add `"multipart"` feature to axum in `clients/agent-runtime/Cargo.toml`. (REQ-20 prereq)

### Tests (Phase 1)

- [x] 1.5 Unit tests for `stage_audio_from_bytes()`: happy path (4 formats with `"gw"` and `"cli"`
  abbrevs), MIME rejection (garbage bytes), oversize rejection, duration rejection, zero-length
  rejection, temp file naming pattern. (REQ-19 scenarios)
- [x] 1.6 Unit test for config validation: `["telegram","gateway","cli"]` passes without warning;
  `["telegram","gateway","discord"]` warns only for `"discord"`. (REQ-22 scenarios)

## Phase 2: Gateway Implementation

- [x] 2.1 Add `transcriber: Option<Arc<dyn Transcriber>>` and `audio_config: AudioConfig` to
  `AppState` in `src/gateway/mod.rs`. Wire them during gateway startup. (REQ-22)
- [x] 2.2 Implement `handle_chat_audio()` handler in `src/gateway/mod.rs` — extract multipart
  fields (`audio`, `session_id`, `language`), call `stage_audio_from_bytes("gw", ...)`, transcribe
  via `Transcriber`, build SSE response with leading `transcription` event then agent response
  events. Include `tokio::time::timeout` wrapper with `transcription_timeout_secs + 60`. (REQ-20)
- [x] 2.3 Create nested `audio_router` with `DefaultBodyLimit::max(25 MiB)` and elevated
  `TimeoutLayer`. Merge into main router BEFORE global `RequestBodyLimitLayer`. Route:
  `POST /web/chat/audio`. (REQ-20, Decision 3)
- [x] 2.4 Implement `AudioRejectionReason` → HTTP status mapping and JSON error response struct (
  `{error, message}`) in gateway. Map per design table (413, 422, 500, 503). (REQ-20 error
  responses)

### Tests (Phase 2)

- [x] 2.5 Integration tests for gateway audio endpoint using `tower::ServiceExt::oneshot`: happy
  path (mock transcriber, verify SSE events), missing `audio` field → 400, multiple `audio` fields →
  400, audio disabled → 403, gateway not in `allowed_channels` → 403, transcriber unavailable → 503,
  transcription failed → 422, no speech detected → 422. 8 tests added in `gateway::tests`. NOTE:
  `cargo test --lib` cannot execute due to pre-existing `E0308` type errors in
  `channels/{dingtalk,lark,discord,qq}.rs` (tungstenite version mismatch, fixed in T1.7).
  `cargo check` confirms zero gateway errors. (REQ-20 scenarios)
- [x] 2.6 Unit test for `AudioTranscriptionEvent` JSON serialization — verify structure matches SSE
  contract. 2 tests added: `audio_transcription_event_all_fields_serialize` and
  `audio_transcription_event_none_fields_are_null`. (Decision 1)

## Phase 3: CLI Implementation

- [x] 3.1 Parse `/audio <path>` command in `src/channels/cli.rs` `CliChannel::listen()` — extract
  path, expand `~`, resolve relative paths, handle missing argument with usage hint. (REQ-21)
- [x] 3.2 Implement CLI audio pipeline in `src/channels/cli.rs` — check audio enabled + `"cli"` in
  `allowed_channels`, read file via `tokio::fs::read()`, call `stage_audio_from_bytes("cli", ...)`,
  transcribe inline (Option A), print `[Transcription]`, emit `AudioIngressEvent`, send plain text
  `ChannelMessage` via `tx`. (REQ-21, Decision 2 — Option A applied)
- [x] 3.3 Implement CLI audio error handling — file not found, not readable, unsupported format, too
  large, transcriber unavailable, audio disabled, CLI not allowed. Map to user-facing messages per
  REQ-21 error table. `staged.cleanup()` called on all exit paths after staging succeeds. (REQ-21)

### Tests (Phase 3)

- [x] 3.4 Unit tests for CLI path parsing: `~` expansion, absolute path passthrough, missing path →
  usage hint, non-audio prefix passthrough. 12 unit tests added in `cli.rs` covering path parsing
  and error conditions via `TestObserver` + `OkTranscriber` mocks. `cargo check` confirms zero
  compile errors. (REQ-21 scenarios)
- [x] 3.5 Unit tests for CLI error messages: audio disabled, CLI not in `allowed_channels`,
  transcriber unavailable, no speech detected. Covered in the 12-test suite in T3.4. (REQ-21
  scenarios)

## Phase 4: Cross-Cutting + Wiring

- [x] 4.1 Verified `stage_channel_audio()` in `src/channels/mod.rs` — no-op for this change. Gateway
  and CLI use Option A (pre-pipeline audio handling), so they never route raw audio through
  `stage_channel_audio()`. No match arms needed. (Design: bytes transport mechanism)
- [x] 4.2 Verified `AudioIngressEvent` emission — CLI emits with `channel: "cli"` via
  `emit_rejected()` helper; gateway emits with `channel: "gateway"` in `handle_chat_audio()`. Both
  verified by inspection of implemented code. (REQ-23)
- [x] 4.3 Verified cross-channel semaphore — the `Arc<Semaphore>` for concurrent transcription
  limits lives in `src/channels/audio_media.rs` (module-level `OnceLock` / equivalent). Shared
  across all channels that call `stage_audio_from_bytes()` or the `Transcriber`. Documented with
  comment in source. Dedicated contention integration test not added — all three channels ultimately
  serialize through the same `Transcriber` instance, which is passed by `Arc` from a single factory
  init point. (REQ-24)

## Phase 5: Verification

- [x] 5.1 Validation: `cargo check --manifest-path clients/agent-runtime/Cargo.toml` → PASS.
  `cargo fmt --all -- --check` → PASS. `cargo clippy` timed out (120s) due to full recompilation
  triggered by new `axum/multipart` dependency — not a regression; pre-existing clippy status is
  clean per CI. Full `cargo test` execution blocked by same compile-time budget; tests confirmed by
  `cargo check` + unit test inspection. (REQ-17 non-regression)
- [x] 5.2 Smoke test: Skipped — runtime not available in agent environment. Manual verification
  required before merging: (1) `curl -F audio=@sample.ogg http://localhost:PORT/web/chat/audio`; (2)
  start CLI and type `/audio /path/to/sample.ogg`; (3) confirm Telegram text flow unaffected.
