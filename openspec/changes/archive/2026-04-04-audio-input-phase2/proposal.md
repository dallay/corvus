# Proposal: Audio Input Phase 2 — HTTP Gateway + CLI

**Change**: audio-input-phase2
**Issue**: #412
**Branch**: `feature/dallay-412-audio-input-gateway-cli`
**Date**: 2026-04-04
**Depends on**: Phase 1 audio infrastructure (merged, commit 258d3c39)

## Intent

Phase 1 delivered the core audio input infrastructure (Transcriber trait, whisper.cpp CLI wrapper,
audio pipeline in `process_channel_message()`, Telegram channel support, config, error taxonomy,
observability, RAII cleanup, concurrency semaphore). It proved the pipeline through the Telegram
channel.

Phase 2 extends audio input to the two remaining entry points defined in the PRD (#246): the HTTP
Gateway (`POST /web/chat/audio` multipart endpoint) and the CLI (`/audio <path>` slash command).
The core pipeline is channel-agnostic after the "fetch bytes" step — only the byte-acquisition
method differs per channel. This makes Phase 2 an incremental extension, not a rearchitecture.

## Scope

### In Scope

1. **Shared staging utility** — Extract `stage_audio_from_bytes(bytes, metadata) -> Result<StagedAudio>` into `audio_media.rs` so gateway, CLI, and Telegram all share one validation + staging path (MIME sniffing, size check, SHA-256, temp file write).

2. **HTTP Gateway endpoint** — `POST /web/chat/audio` accepting `multipart/form-data`:
   - Fields: `audio` (file, required), `session_id` (text, optional), `language` (text, optional)
   - Per-route body limit: 25 MiB (overriding the global 64KB `RequestBodyLimitLayer`)
   - Per-route timeout: 180s (overriding the global 30s) to accommodate transcription
   - Auth: same bearer token pairing as existing endpoints
   - Response: SSE stream (same pattern as `/web/chat/stream`) with transcription + agent response
   - Enable `axum/multipart` feature in Cargo.toml

3. **CLI `/audio <path>` command**:
   - Read local file from provided path
   - Validate format (magic bytes), size, duration via shared staging utility
   - Route through `process_channel_message()` (not the direct `Agent::turn()` bypass)
   - Transcribe via `Transcriber`, inject text, continue through agent conversation

4. **Config updates**:
   - Recognize `"gateway"` and `"cli"` as valid `allowed_channels` values
   - Update `allowed_channels` validation (currently warns on non-`"telegram"` names)

5. **`stage_channel_audio()` routing** — Add `"gateway"` and `"cli"` match arms (currently `_ => Ok(Vec::new())`)

### Out of Scope

- whisper-rs embedded transcription (future enhancement)
- Model auto-download tooling (future enhancement)
- Audio output / text-to-speech
- Real-time / streaming transcription
- Changes to existing text or image flows
- New error taxonomy variants (Phase 1's 11 reasons are sufficient)

## Approach

### Strategy: Incremental Extension with Shared Staging

The core insight is that Phase 1's audio pipeline (gate -> stage -> transcribe -> inject) is
channel-agnostic after the "fetch bytes" step. The only channel-specific part is HOW bytes are
obtained:

- **Telegram**: fetch via Bot API `getFile` + HTTP download
- **Gateway**: extract from multipart upload
- **CLI**: read from local filesystem

By factoring out `stage_audio_from_bytes(bytes, metadata) -> Result<StagedAudio>`, all three
channels share the same validation, staging, and transcription path.

### Gateway Architecture

```
Client -> POST /web/chat/audio (multipart/form-data)
       -> axum::extract::Multipart -> extract audio bytes + fields
       -> stage_audio_from_bytes() -> StagedAudio
       -> transcribe_audio() -> TranscriptionResult
       -> inject into ChannelMessage as Text
       -> dispatch through existing chat pipeline
       -> SSE response stream
```

**Body limit**: Use axum nested router. The audio route gets its own `Router` with
`DefaultBodyLimit::max(25 * 1024 * 1024)`, merged into the main router. The global 64KB limit
only applies to non-audio routes.

**Timeout**: The audio handler uses its own `tokio::time::timeout()` wrapper with
`audio.transcription_timeout_secs + 60s` buffer, independent of the global 30s
`REQUEST_TIMEOUT_SECS`.

**AppState**: Add `transcriber: Option<Arc<dyn Transcriber>>` reference to `AppState` (L981-1003).

### CLI Architecture

```
User types: /audio ~/recording.mp3
       -> parse command in CliChannel::listen()
       -> read file bytes from disk
       -> stage_audio_from_bytes() -> StagedAudio
       -> build ChannelMessage with ContentPart::Audio
       -> route through process_channel_message() (NOT Agent::turn())
       -> transcribe -> inject -> agent response
```

**Pipeline bypass fix**: Currently `Agent::run_interactive()` (L1585-1609) bypasses
`process_channel_message()` and calls `self.turn()` directly. When `/audio` is detected, the CLI
MUST construct a `ChannelMessage` and route it through the channel runtime's
`process_channel_message()` to ensure the full audio pipeline (gate, stage, transcribe, inject)
runs.

### Shared Staging Extraction

Extract from Telegram's `fetch_and_stage_audio()` (L1740-1882) the bytes-to-StagedAudio logic
into a shared function in `audio_media.rs`:

```rust
pub async fn stage_audio_from_bytes(
    bytes: &[u8],
    channel_origin: &str,
    declared_mime: Option<&str>,
    declared_duration_secs: Option<u64>,
    audio_config: &AudioConfig,
) -> Result<StagedAudio, AudioRejectionReason>
```

This function handles: MIME sniffing, size validation, SHA-256 hashing, temp file write. Telegram's
existing fetch function calls this after downloading; gateway calls it after multipart extraction;
CLI calls it after file read.

## Affected Areas

| File | Impact | Description |
|------|--------|-------------|
| `Cargo.toml` | Modified | Add `"multipart"` to axum features |
| `src/channels/audio_media.rs` | Modified | Add `stage_audio_from_bytes()` shared utility |
| `src/channels/mod.rs` | Modified | Add `"gateway"` and `"cli"` arms in `stage_channel_audio()` |
| `src/gateway/mod.rs` | Modified | Add `POST /web/chat/audio` handler, nested router with body limit override, transcriber in AppState |
| `src/channels/cli.rs` | Modified | Parse `/audio <path>`, read file, build Audio ChannelMessage |
| `src/agent/agent.rs` | Modified | Route `/audio` CLI messages through `process_channel_message()` |
| `src/config/schema.rs` | Modified | Document `"gateway"` and `"cli"` as valid `allowed_channels` |
| `src/config/validation.rs` | Modified | Recognize `"gateway"` and `"cli"` in allowed_channels validation |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `multer` crate adds binary size | Medium | Check delta; multer is ~50KB compiled; acceptable for file upload support |
| Gateway timeout for long audio | High | Per-route timeout (180s) independent of global 30s; configurable via `transcription_timeout_secs` |
| CLI pipeline bypass complexity | Medium | Route `/audio` through channel runtime; keep interactive text via `Agent::turn()` unchanged |
| Concurrent multipart uploads exhaust memory | Low | Existing transcription semaphore limits concurrent processing; multipart streaming limits memory |
| Body limit layer ordering | Medium | Test that nested router body limit overrides global; axum handles this correctly with merge |

## Rollback Plan

1. **Config kill-switch**: Remove `"gateway"` and `"cli"` from `audio.allowed_channels` — audio
   disabled for these channels immediately, no code change needed.
2. **Feature revert**: All changes are additive — new handler, new match arms, new utility
   function. Reverting the commit removes Phase 2 without affecting Phase 1 Telegram audio.
3. **Cargo.toml**: Remove `"multipart"` from axum features to eliminate the dependency.

## Dependencies

- Phase 1 audio infrastructure (merged, commit 258d3c39)
- `axum` `multipart` feature (new Cargo dependency, pulls in `multer`)
- No new external binary dependencies (reuses whisper.cpp from Phase 1)

## Success Criteria

- [ ] Audio file uploaded via `POST /web/chat/audio` returns transcription + agent response via SSE
- [ ] `/audio ~/file.ogg` in CLI transcribes and produces agent response
- [ ] All 4 formats work through gateway (OGG/Opus, MP3, WAV, M4A)
- [ ] All 4 formats work through CLI
- [ ] Gateway rejects files > 25 MiB with appropriate error
- [ ] Gateway rejects unsupported formats with appropriate error
- [ ] CLI rejects non-existent file paths with clear error
- [ ] CLI rejects unsupported formats with clear error
- [ ] Error handling matches the 11 error types from Phase 1 (REQ-11)
- [ ] Existing text and image flows completely unaffected (REQ-17)
- [ ] Gateway audio auth uses same bearer token as other endpoints
- [ ] Transcription semaphore limits concurrent processing across all channels (REQ-12)
- [ ] All new code has unit tests
- [ ] Gateway endpoint has integration test with mock transcriber
