# Tasks: Audio Input Support (Phase 1: Core + Telegram)

## Phase 1: Foundation — Types, Config, Observability

- [x] 1.1 Add `ContentPart::Audio` variant to enum in `src/channels/traits.rs`; add `has_audio_parts()`, `audio_parts()` helpers on `ChannelMessage`; update `text_projection()` for audio captions. Write unit tests for helpers first (TDD red→green).
- [x] 1.2 Create `src/config/schema.rs` `AudioConfig` struct with all defaults; wire `pub audio: AudioConfig` into `Config`. Write serde deserialization test for empty `[audio]` section first.
- [x] 1.3 Add audio startup validation in `src/config/validation.rs`: enabled+empty channels error, size/duration ceiling checks, non-Phase-1 channel warning. Write tests for each validation rule first.
- [x] 1.4 Add `AudioIngressOutcome`, `AudioIngressReason`, `AudioIngressEvent`, `ObserverEvent::AudioIngress` variant, `on_audio_ingress()` to `src/observability/traits.rs`. Implement for existing observers. Test Display impls.

## Phase 2: Core — Audio Media Module + Transcriber

- [x] 2.1 Create `src/channels/audio_media.rs`: `AllowedAudioMime` enum with `from_mime_str()`, `as_str()`, `file_extension()`. Write round-trip tests first.
- [x] 2.2 Implement `validate_audio_mime()` magic-byte sniffing (OGG, MP3, WAV, M4A) in `audio_media.rs`. Write tests with real magic bytes and garbage input first (REQ-3 scenarios).
- [x] 2.3 Implement `AudioRejectionReason` enum (11 variants) with `thiserror::Error` Display in `audio_media.rs`. Test all Display strings.
- [x] 2.4 Implement `StagedAudio` struct and `cleanup()` method, `AudioHistoryMeta` struct with `from_staged()` and `to_context_string()` in `audio_media.rs`. Test formatting.
- [x] 2.5 Create `src/transcription/mod.rs`, `src/transcription/traits.rs` with `Transcriber` trait and `TranscriptionResult`. Add `pub mod transcription` to `src/lib.rs`.
- [x] 2.6 Create `src/transcription/whisper_cli.rs`: `WhisperCliTranscriber` with process spawning, output parsing, timeout handling, semaphore concurrency. Write unit tests for output parsing and error mapping first.

## Phase 3: Integration — Pipeline + Telegram

- [x] 3.1 Add `pub mod audio_media` to `src/channels/mod.rs`. Implement `StagedAudioGuard` RAII wrapper. Test cleanup on drop (REQ-5 scenarios).
- [x] 3.2 Implement `gate_audio_config()` in `src/channels/mod.rs`: check enabled + allowed_channels, emit events, send rejection messages. Test with mock context (REQ-2, REQ-7 gate scenarios).
- [x] 3.3 Implement `gate_and_stage_audio()` in `src/channels/mod.rs`: delegate to channel fetch, validate MIME/size/duration, stage to temp file. Test validation pipeline (REQ-3, REQ-4 scenarios).
- [x] 3.4 Implement `transcribe_audio()` and `inject_transcription()` in `src/channels/mod.rs`: semaphore acquire, transcriber call, empty-text guard, replace Audio→Text parts, build `AudioHistoryMeta`. Test injection logic (REQ-8, REQ-14 scenarios).
- [x] 3.5 Wire all four stages into `process_channel_message()` between `extract_user_text()` and `enrich_with_memory()`.
- [x] 3.6 Modify `build_telegram_content_parts()` in `src/channels/telegram.rs` to parse `message.voice` and `message.audio` into `ContentPart::Audio`. Write unit tests with mock Telegram JSON first (REQ-10 scenarios).
- [x] 3.7 Implement `fetch_and_stage_audio()` on `TelegramChannel` in `src/channels/telegram.rs`: pre-flight duration check, getFile→download, streaming size validation, MIME sniffing, SHA-256, temp file write. Test with mock HTTP responses.

## Phase 4: Verification + Doctor

- [x] 4.1 Add audio health checks to `src/doctor/mod.rs`: whisper binary existence, model file existence. Test pass/fail/skip scenarios (REQ-18).
- [x] 4.2 Integration test: full pipeline happy path — mock transcriber returning known text, verify text injection + observability event + temp file cleanup.
- [x] 4.3 Integration test: verify text-only and image-only messages are completely unaffected when audio is enabled (REQ-17 regression).
- [x] 4.4 Integration test: concurrent transcription semaphore — spawn multiple transcribe calls, verify serial execution under default concurrency=1 (REQ-12).
