# Archive Report: Audio Input Support

**Change**: `audio-input-support`
**Issue**: [#246](https://github.com/dallay/corvus/issues/246) / DALLAY-150
**Branch**: `feature/dallay-150-add-audio-input-support-for-agents-telegram-http-gateway-cli`
**Archived**: 2026-04-03
**Verify Verdict**: PASS WITH WARNINGS (0 CRITICAL, 7 WARNING, 5 SUGGESTION)

---

## What Was Delivered

Phase 1 of audio input support for Corvus agents: core infrastructure + Telegram channel integration. Users can now send voice notes and audio files (OGG/Opus, MP3, WAV, M4A) via Telegram. The runtime validates format/size/duration, transcribes audio locally using whisper.cpp CLI, and injects the transcription into the normal agent conversation flow as text. The provider never sees audio bytes — privacy is preserved via local-only processing.

### Key Capabilities

- `ContentPart::Audio` variant for multimodal message parsing
- 7-step audio pipeline: parse → gate → fetch → validate → stage → transcribe → inject
- `Transcriber` trait as a new runtime extension point for STT engines
- `WhisperCliTranscriber` — whisper.cpp CLI wrapper with concurrency semaphore
- `[audio]` TOML config section (disabled by default, deny-by-default posture)
- Magic-byte MIME sniffing for OGG, MP3, WAV, M4A
- Size limits (25 MiB default) and duration limits (10 min default)
- 11-variant `AudioRejectionReason` error taxonomy with user-friendly messages
- `StagedAudioGuard` RAII cleanup on all exit paths
- `AudioIngressEvent` observability events (admitted + rejected)
- `AudioHistoryMeta` for conversation history
- `corvus doctor` health checks for whisper binary and model availability
- Zero new Rust crate dependencies

---

## Files Created/Modified

### New Files (under `clients/agent-runtime/`)

| File | Description |
|------|-------------|
| `src/channels/audio_media.rs` | Audio validation, MIME sniffing, staging, history metadata |
| `src/transcription/mod.rs` | Transcription module exports |
| `src/transcription/traits.rs` | `Transcriber` trait, `TranscriptionResult` struct |
| `src/transcription/whisper_cli.rs` | whisper.cpp CLI wrapper implementation |

### Modified Files (under `clients/agent-runtime/`)

| File | Description |
|------|-------------|
| `src/channels/traits.rs` | `ContentPart::Audio` variant; `has_audio_parts()`, `audio_parts()` helpers |
| `src/channels/mod.rs` | `StagedAudioGuard`; 4 pipeline stages; wired into `process_channel_message()` |
| `src/channels/telegram.rs` | Voice/audio parsing in `build_telegram_content_parts()`; `fetch_and_stage_audio()` |
| `src/config/schema.rs` | `AudioConfig` struct with defaults; wired into `Config` |
| `src/config/mod.rs` | Re-exports `AudioConfig` |
| `src/observability/traits.rs` | `AudioIngressEvent`, `AudioIngressOutcome`, `AudioIngressReason`, `on_audio_ingress()` |
| `src/observability/log.rs` | Handles `AudioIngress` event |
| `src/doctor/mod.rs` | Audio health checks (whisper binary + model) |
| `src/lib.rs` | `pub mod transcription` |
| `src/main.rs` | `mod transcription` |

---

## Build & Test Results

- **Build**: ✅ Passed (zero warnings)
- **Clippy**: ✅ Passed (zero warnings)
- **Tests**: ✅ 6,487 passed / 0 failed / 0 ignored
- **Compliance**: 42/68 scenarios fully COMPLIANT, 24 PARTIAL (structural evidence), 2 UNTESTED (require real whisper-cli)

---

## Spec Deviations Fixed at Archive

The following 4 minor deviations (identified in verify) were synced into the spec before archiving:

1. `TranscriptionResult.duration_secs`: `f64` → `Option<f64>` (whisper-cli may not always report duration)
2. `AudioRejectionReason`: 10 → 11 variants (added `SystemError` for unexpected internal errors)
3. `Transcriber::health_check`: `bool` → `Result<(), String>` (more informative for doctor diagnostics)
4. `Transcriber::transcribe`: `Result<TranscriptionResult>` → `Result<TranscriptionResult, AudioRejectionReason>` (typed error for pipeline mapping)

---

## Known Follow-Up Items

### Behavioral Integration Tests (Priority: Medium)

- End-to-end pipeline test with mock transcriber (shell script returning known text)
- Telegram voice/audio JSON parsing unit tests with mock payloads
- Explicit `StagedAudioGuard` drop-cleanup integration test
- Audio config validation boundary tests
- Concurrent transcription semaphore behavioral test

### Phase 2: HTTP Gateway + CLI (Separate Change)

- `POST /web/chat/audio` multipart endpoint on the HTTP Gateway
- CLI `/audio <path>` command for local file transcription
- (Optional) whisper-rs embedded transcription behind `--features audio-transcription`
- (Optional) Model auto-download tooling

---

## SDD Cycle

| Phase | Status | Date |
|-------|--------|------|
| Explore | ✅ Complete | 2026-04-03 |
| Propose | ✅ Complete | 2026-04-03 |
| Spec | ✅ Complete | 2026-04-03 |
| Design | ✅ Complete | 2026-04-03 |
| Tasks | ✅ Complete (17/17) | 2026-04-03 |
| Apply | ✅ Complete | 2026-04-03 |
| Verify | ✅ PASS WITH WARNINGS | 2026-04-03 |
| Archive | ✅ Complete | 2026-04-03 |

---

## Source of Truth

The canonical spec is now at: `openspec/specs/audio-input/spec.md`
