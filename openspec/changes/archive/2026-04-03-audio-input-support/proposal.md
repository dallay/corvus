# Proposal: Audio Input Support for Agents

**Change**: `audio-input-support`
**Issue**: #246 / DALLAY-150
**Branch**: `feature/dallay-150-add-audio-input-support-for-agents-telegram-http-gateway-cli`
**Date**: 2026-04-03

## Intent

Corvus agents currently accept only text and images from users. Users — especially on mobile via
Telegram — frequently communicate through voice notes and audio messages. This change adds audio
input support: the runtime receives audio files (voice notes or uploaded audio), transcribes them
locally to text using whisper.cpp, and feeds the transcription into the normal agent conversation
flow.

**Why now**: Audio is the most-requested missing input modality. The existing image multimodal
pipeline provides a proven architectural precedent — audio mirrors the same parse → gate → stage →
process → inject pattern, making this a natural incremental extension rather than a greenfield
effort.

**Privacy constraint (NFR1)**: All transcription MUST be local. No audio data leaves the operator's
infrastructure. This eliminates cloud STT services and mandates a local engine (whisper.cpp).

## Scope

### In Scope

**Phase 1 — Core + Telegram (this change)**:
- `ContentPart::Audio` variant in the channel message enum
- Audio media module: MIME sniffing (OGG/Opus, MP3, WAV, M4A), size/duration validation, staging
- `Transcriber` trait as a new runtime extension point
- whisper.cpp CLI wrapper implementation (proven pattern from `crates/robot-kit/src/listen.rs`)
- `[audio]` TOML config section with enabled, allowed_channels, limits, model, language
- Telegram channel: parse `message.voice` and `message.audio` into `ContentPart::Audio`
- Audio pipeline stages in `process_channel_message()`: gate → stage → transcribe → inject text
- Audio observability events (`AudioIngressEvent`, `on_audio_ingress()`)
- Audio history metadata (`AudioHistoryMeta` with transcription text)
- `StagedAudioGuard` RAII cleanup
- 6 error types: unsupported format, too large, too long, corrupted, transcription failed, no speech
- Concurrency guard (semaphore) for transcription to prevent CPU overload
- `corvus doctor` check for whisper.cpp binary and model availability

**Phase 2 — HTTP Gateway + CLI (follow-up change)**:
- `POST /web/chat/audio` multipart endpoint on the HTTP Gateway
- CLI `/audio <path>` command for local file transcription
- (Optional) whisper-rs embedded transcription behind `--features audio-transcription`
- (Optional) Model auto-download tooling

### Out of Scope

- Audio **output** (text-to-speech) — separate feature
- Real-time / streaming transcription — batch file only
- Speaker diarization or speaker identification
- Audio sent as part of multi-media messages (audio + image in same turn)
- Non-whisper transcription engines (Vosk, candle-whisper) — Transcriber trait allows future addition
- Transcription of video files
- Cloud-based STT services (explicitly prohibited by NFR1)
- WhatsApp, Discord, Slack audio — channels not yet scoped for audio

## Approach

### Strategy: Incremental Extension of the Image Multimodal Pipeline

The image pipeline (`channel-image-ingestion` spec, `runtime-image-pipeline` spec) established
validated patterns for media ingestion. Audio follows the same architecture with one critical
difference: **audio is transcribed to text before the agent loop; the provider never sees audio
bytes**.

```
Image flow:  Channel → ContentPart::Image → stage → provider.chat(images: &[StagedImage])
Audio flow:  Channel → ContentPart::Audio → stage → transcribe → inject Text → provider.chat(text)
```

### Pipeline Integration Point

Audio processing inserts into `process_channel_message()` (in `src/channels/mod.rs`) between
`extract_user_text()` and `enrich_with_memory()`:

```
extract_user_text()
→ gate_audio_config()           // check [audio] enabled + allowed_channels
→ gate_and_stage_audio()        // fetch from channel, validate MIME/size/duration, stage to disk
→ StagedAudioGuard              // RAII cleanup on all exit paths
→ transcribe_audio()            // Transcriber::transcribe() via whisper.cpp CLI
→ inject_transcription()        // replace Audio parts with Text, store AudioHistoryMeta
→ enrich_with_memory()          // existing flow continues with text-only message
```

### Transcription Engine: whisper.cpp CLI Wrapper

- The `crates/robot-kit/src/listen.rs` module already wraps whisper.cpp as an external binary —
  this is a proven pattern in the codebase
- Zero Rust dependency impact (no new crates, no binary size increase)
- Operator installs whisper.cpp + model separately; runtime validates availability at startup
- Best Spanish transcription quality via multilingual models
- Default model: `base` (~150 MB, good speed/quality balance)
- Models stored in `~/.corvus/models/whisper/`

### Config: Separate `[audio]` Section

Audio config is intentionally separate from `[multimodal]` because concerns differ (transcription
model/language vs. vision route/provider routing):

```toml
[audio]
enabled = false
allowed_channels = ["telegram"]
max_audio_bytes = 26214400       # 25 MiB
max_audio_duration_secs = 600    # 10 minutes
transcription_model = "base"     # whisper model name
transcription_language = "es"    # primary language hint
```

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/channels/traits.rs` | Modified | Add `ContentPart::Audio` variant; add `has_audio_parts()`, `audio_parts()` helpers |
| `src/channels/audio_media.rs` | **New** | `AllowedAudioMime`, `AudioRejectionReason`, `StagedAudio`, `AudioHistoryMeta`, MIME sniffing, validation |
| `src/channels/mod.rs` | Modified | Add `gate_audio_config()`, `gate_and_stage_audio()`, `transcribe_audio()`, `inject_transcription()`, `StagedAudioGuard` |
| `src/channels/telegram.rs` | Modified | Parse `message.voice` and `message.audio` in `build_telegram_content_parts()`; add `fetch_and_stage_audio()` |
| `src/transcription/mod.rs` | **New** | Module exports |
| `src/transcription/traits.rs` | **New** | `Transcriber` trait, `TranscriptionResult` struct |
| `src/transcription/whisper_cli.rs` | **New** | whisper.cpp CLI wrapper (spawn process, parse output, error handling) |
| `src/config/schema.rs` | Modified | Add `AudioConfig` struct, wire into main config |
| `src/config/validation.rs` | Modified | Startup validation for `[audio]` section |
| `src/observability/traits.rs` | Modified | Add `AudioIngressOutcome`, `AudioIngressReason`, `AudioIngressEvent`, `on_audio_ingress()` |
| `src/observability/` impls | Modified | Implement `on_audio_ingress()` for existing observer implementations |
| `src/lib.rs` or `src/main.rs` | Modified | Register `transcription` module, wire `Transcriber` into runtime |
| `src/doctor.rs` (or equivalent) | Modified | Add whisper.cpp binary + model health checks |
| Config TOML example/docs | Modified | Document `[audio]` section |
| `openspec/specs/channel-image-ingestion/` | Reference only | Audio mirrors these patterns but does NOT modify image specs |
| `openspec/specs/runtime-image-pipeline/` | Reference only | Audio mirrors pipeline architecture; no image spec changes |

### Phase 2 Additional Areas (follow-up change)

| Area | Impact | Description |
|------|--------|-------------|
| `src/gateway/mod.rs` | Modified | New `POST /web/chat/audio` multipart endpoint |
| `src/channels/cli.rs` | Modified | `/audio <path>` command handler |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Model distribution complexity** — Whisper models are 150 MB–3 GB; operator must install separately | High | Medium | `corvus doctor` check; clear setup docs; default to `base` model (~150 MB); graceful error message when model missing |
| **Memory footprint during inference** — Whisper base model uses ~500 MB RAM during transcription | Medium | Medium | Document minimum RAM requirements; recommend `base` model for constrained environments; transcription semaphore limits concurrent usage |
| **Concurrent transcription CPU load** — Multiple simultaneous audio messages could overwhelm the system | Medium | High | Implement a `tokio::sync::Semaphore` with configurable concurrency limit (default: 1); queue excess requests; timeout after configurable duration |
| **OGG/Opus duration detection** — Getting duration from OGG headers without a dependency is non-trivial | Medium | Low | Phase 1: trust Telegram's `duration` field for gating; whisper.cpp reports actual duration post-transcription; add `ogg` crate parsing in Phase 2 if needed |
| **Format conversion dependency** — whisper.cpp natively handles WAV 16kHz mono; OGG/Opus may need conversion | Low | Medium | whisper.cpp recent versions handle OGG/Opus and MP3 natively; if conversion needed, require `ffmpeg` as optional external dependency with doctor check |
| **whisper.cpp binary availability** — Operator must install whisper.cpp separately | Medium | Medium | Fail gracefully at runtime with user-friendly message; `corvus doctor` warns; document installation in setup guide |
| **Transcription quality variance** — Noisy audio, accents, or mixed languages may produce poor transcriptions | Low | Low | whisper.cpp has good noise tolerance; `base` model handles Spanish well; future: expose confidence score and let operator configure rejection threshold |

## Rollback Plan

This change is **safely reversible** at multiple levels:

1. **Config kill-switch**: Set `audio.enabled = false` — immediately disables all audio processing.
   Audio messages receive a friendly "Audio input is currently disabled" reply. Zero code changes
   required.

2. **Feature branch revert**: The change is isolated to:
   - New files (`audio_media.rs`, `src/transcription/`) — delete entirely
   - New enum variant (`ContentPart::Audio`) — remove variant; any remaining match arms cause
     compile errors (safe detection)
   - New config section (`[audio]`) — ignored when struct is removed
   - Modified files (`mod.rs`, `telegram.rs`, `traits.rs`) — revert additions only; no existing
     behavior is modified

3. **No schema migrations**: Audio metadata in conversation history uses the same extensible
   metadata pattern as images. Removing audio support doesn't corrupt existing conversations —
   audio history entries simply stop being written.

4. **No provider contract changes**: The provider `ChatRequest` struct is NOT modified. Audio
   transcription produces plain text that flows through the existing text path. Reverting audio
   support has zero impact on provider integrations.

5. **External dependency isolation**: whisper.cpp is an external binary, not a Rust dependency.
   Removing audio support requires no `Cargo.toml` changes.

## Dependencies

### Required (Phase 1)

- **whisper.cpp binary** — External CLI tool. Operator installs via package manager or builds from
  source. Not bundled with Corvus. Runtime validates availability via `corvus doctor`.
- **Whisper model file** — Downloaded separately to `~/.corvus/models/whisper/`. Default: `base`
  (~150 MB). Not distributed with Corvus.

### Optional

- **ffmpeg** — Only needed if whisper.cpp cannot directly decode a submitted audio format. Recent
  whisper.cpp versions handle OGG/Opus, MP3, and WAV natively. Listed as optional dependency with
  `corvus doctor` check.

### Rust Crate Dependencies

- **None** for Phase 1. The CLI wrapper approach avoids new Rust dependencies entirely. This is a
  deliberate choice to keep binary size unchanged and avoid C/C++ build complexity.

## Success Criteria

- [ ] A Telegram user can send a voice note and receive an agent response based on the transcribed content
- [ ] A Telegram user can send an audio file (MP3, WAV, M4A, OGG) and receive an agent response based on the transcribed content
- [ ] Audio transcription happens locally — no network calls to external services
- [ ] Spanish voice notes are transcribed with usable accuracy (whisper `base` model)
- [ ] `[audio]` config section controls enabled state, allowed channels, size/duration limits, model, and language
- [ ] `audio.enabled = false` completely disables audio processing with a friendly user message
- [ ] Audio messages on channels not in `audio.allowed_channels` are rejected with a clear message
- [ ] Audio files exceeding 25 MB are rejected before full download when Content-Length is available
- [ ] Audio files exceeding 10 minutes are rejected (via channel-declared duration or post-transcription check)
- [ ] Unsupported audio formats are rejected via magic-byte MIME sniffing
- [ ] Corrupted audio files produce a clear error, not a crash
- [ ] When whisper.cpp is not installed, audio messages get a friendly "not available" reply (not a panic)
- [ ] `corvus doctor` reports whisper.cpp binary and model availability
- [ ] `AudioIngressEvent` observability events are emitted for all audio ingestion attempts (admitted and rejected)
- [ ] Audio history metadata (including transcription text) is stored in conversation history
- [ ] Concurrent transcription is bounded by a semaphore (default: 1 concurrent transcription)
- [ ] All staged audio temp files are cleaned up via RAII on all exit paths
- [ ] Existing image pipeline behavior is completely unaffected
- [ ] All new code has unit tests; Telegram audio parsing has integration tests
