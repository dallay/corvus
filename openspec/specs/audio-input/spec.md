# Audio Input Specification

**Domain**: channels / audio / transcription
**Status**: final
**Issue**: #246 / DALLAY-150
**Date**: 2026-04-03
**Depends on**: `channel-image-ingestion` spec (#266), `runtime-image-pipeline` spec (#267)

## Overview

This specification defines the behavioral contract for audio input support in the Corvus runtime
(Phase 1: Core infrastructure + Telegram). Users send voice notes or audio files through a channel;
the runtime receives them, validates format/size/duration, transcribes the audio locally to text
using a `Transcriber` implementation (whisper.cpp CLI), and injects the transcription into the
normal agent conversation flow as if the user had typed the text.

Audio differs from image input in one critical way: **audio is never forwarded to the provider**.
It is transcribed to text pre-loop, and the provider receives only the resulting text. The
`ChatRequest` struct is NOT modified.

## Definitions

- **Audio ingestion pipeline**: The flow a channel follows to accept inbound audio: parse → gate →
  fetch → validate → stage → transcribe → inject text.
- **Staged audio**: A validated audio file written to a temp file with metadata (`StagedAudio`),
  ready for transcription.
- **Transcription**: The process of converting audio speech to text using a local STT engine.
- **Transcriber**: A trait-based extension point for speech-to-text engines.
- **Channel handle**: An opaque, channel-specific identifier for a media asset (e.g., Telegram
  `file_id` for voice notes or audio files).
- **Audio history metadata**: Compact metadata stored in conversation history recording that a turn
  originated from audio input, including the transcription text.

## Requirements

### REQ-1: Audio Detection (FR1)

The runtime MUST distinguish audio content from text content in inbound channel messages. When a
channel message contains audio (voice note or audio file), the channel layer MUST parse it into a
`ContentPart::Audio` variant and include it in `ChannelMessage.parts`.

The `ContentPart::Audio` variant MUST carry the following fields:

| Field                    | Type             | Required | Description                                    |
|--------------------------|------------------|----------|------------------------------------------------|
| `channel_handle`         | `String`         | Yes      | Channel-specific media identifier              |
| `source_channel`         | `String`         | Yes      | Channel name (e.g., "telegram")                |
| `declared_mime`          | `Option<String>` | No       | MIME type declared by the channel               |
| `caption_text`           | `Option<String>` | No       | Accompanying text/caption from the user         |
| `file_name`              | `Option<String>` | No       | Original file name (audio files only)           |
| `declared_bytes`         | `Option<u64>`    | No       | File size declared by the channel               |
| `declared_duration_secs` | `Option<u64>`    | No       | Duration in seconds declared by the channel     |

The runtime MUST also provide helper methods on `ChannelMessage`:
- `has_audio_parts()` — returns `true` if any part is `ContentPart::Audio`
- `audio_parts()` — returns an iterator over `ContentPart::Audio` parts

A message containing only audio (no text, no caption) MUST still be processed. The text projection
for such messages MUST be empty until transcription injects the text.

A message containing both text and audio MUST preserve the text as a `ContentPart::Text` part
alongside the `ContentPart::Audio` part.

#### Scenario: Voice note detected as audio

- GIVEN a Telegram user sends a voice note (no text)
- WHEN the channel layer parses the message
- THEN `ChannelMessage.parts` MUST contain exactly one `ContentPart::Audio`
- AND `has_audio_parts()` MUST return `true`
- AND `source_channel` MUST be `"telegram"`
- AND `declared_mime` SHOULD be `Some("audio/ogg")`

#### Scenario: Audio file with caption detected

- GIVEN a Telegram user sends an MP3 audio file with caption "translate this"
- WHEN the channel layer parses the message
- THEN `ChannelMessage.parts` MUST contain `ContentPart::Text { text: "translate this" }` and
  `ContentPart::Audio { caption_text: Some("translate this"), .. }`
- AND `has_audio_parts()` MUST return `true`

#### Scenario: Text-only message has no audio parts

- GIVEN a Telegram user sends a plain text message "hello"
- WHEN the channel layer parses the message
- THEN `has_audio_parts()` MUST return `false`
- AND audio processing MUST NOT be triggered

#### Scenario: Image message is not treated as audio

- GIVEN a Telegram user sends a photo (no audio)
- WHEN the channel layer parses the message
- THEN `has_audio_parts()` MUST return `false`
- AND the image pipeline handles the message (not the audio pipeline)

### REQ-2: Audio Processing Pipeline (FR2)

The runtime MUST process every inbound audio through a 7-step pipeline inserted into
`process_channel_message()` before `extract_user_text()` and `enrich_with_memory()`:

1. **Parse**: Channel extracts audio metadata into `ContentPart::Audio` (REQ-1)
2. **Gate config**: Check `[audio]` config — `enabled` and `allowed_channels` (REQ-7)
3. **Fetch**: Download audio bytes from the channel's platform API (REQ-10)
4. **Validate**: Apply MIME sniffing, size limit, and duration limit (REQ-3, REQ-4)
5. **Stage**: Write validated bytes to temp file as `StagedAudio`, protected by `StagedAudioGuard`
   RAII cleanup (REQ-5)
6. **Transcribe**: Invoke `Transcriber::transcribe()` to produce text (REQ-6)
7. **Inject**: Replace `ContentPart::Audio` with `ContentPart::Text` containing the transcription;
   store `AudioHistoryMeta` (REQ-8)

After injection, the message continues through the normal text-only flow (`enrich_with_memory()` →
`run_unified_channel_tool_loop()` → provider). The provider MUST NOT receive audio bytes or any
audio-specific payload.

The pipeline MUST be fail-closed: any step that cannot be completed MUST reject the audio with an
appropriate `AudioRejectionReason` and emit an `AudioIngressEvent`.

#### Scenario: Full pipeline happy path — voice note

- GIVEN `[audio]` is enabled with `allowed_channels: ["telegram"]`
- AND a Transcriber is available and healthy
- WHEN a Telegram user sends a 15-second OGG/Opus voice note saying "¿qué tiempo hace hoy?"
- THEN step 1 (parse) produces `ContentPart::Audio` with `declared_mime: Some("audio/ogg")`
- AND step 2 (gate) passes config checks
- AND step 3 (fetch) downloads bytes via Telegram Bot API `getFile`
- AND step 4 (validate) confirms OGG/Opus magic bytes and size/duration within limits
- AND step 5 (stage) writes to temp file and creates `StagedAudioGuard`
- AND step 6 (transcribe) produces `TranscriptionResult { text: "¿Qué tiempo hace hoy?", .. }`
- AND step 7 (inject) replaces the Audio part with `ContentPart::Text { text: "¿Qué tiempo hace hoy?" }`
- AND the provider receives only text (no audio reference)
- AND an `AudioIngressEvent` with outcome `Admitted` is emitted
- AND the temp file is cleaned up after the turn completes

#### Scenario: Pipeline short-circuits at gate — audio disabled

- GIVEN `[audio]` has `enabled: false`
- WHEN a user sends a voice note on any channel
- THEN step 2 (gate) rejects with `AudioRejectionReason::Disabled`
- AND steps 3–7 are NOT executed
- AND no fetch request is made
- AND the user receives a friendly error message
- AND an `AudioIngressEvent` with outcome `Rejected` and reason `Disabled` is emitted

#### Scenario: Pipeline short-circuits at gate — channel not allowed

- GIVEN `[audio]` is enabled with `allowed_channels: ["telegram"]`
- WHEN an audio message arrives from a channel not in the allowlist
- THEN step 2 (gate) rejects with `AudioRejectionReason::ChannelNotAllowed`
- AND steps 3–7 are NOT executed

### REQ-3: Audio MIME Validation

The runtime MUST validate audio MIME types using magic-byte sniffing. Magic-byte sniffing MUST take
strict precedence over any declared MIME type from the channel.

The following formats MUST be accepted:

| Format   | Magic Bytes                              | MIME             | Extension |
|----------|------------------------------------------|------------------|-----------|
| OGG/Opus | `4F 67 67 53` ("OggS")                   | audio/ogg        | .ogg      |
| MP3      | `FF FB`, `FF F3`, `FF F2`, or `49 44 33` | audio/mpeg       | .mp3      |
| WAV      | `52 49 46 46....57 41 56 45` (RIFF+WAVE) | audio/wav        | .wav      |
| M4A/AAC  | `....66 74 79 70` (ftyp at offset 4)     | audio/mp4        | .m4a      |

All other audio formats MUST be rejected with `AudioRejectionReason::MimeRejected`.

If the declared MIME type conflicts with the sniffed MIME type, the sniffed type MUST be used and
the declared type MUST be ignored. The runtime SHOULD log a warning when declared and sniffed types
disagree.

#### Scenario: OGG/Opus voice note accepted

- GIVEN a Telegram voice note with declared MIME `audio/ogg`
- WHEN magic-byte sniffing finds `4F 67 67 53` at offset 0
- THEN the audio is classified as `AllowedAudioMime::OggOpus`
- AND validation passes

#### Scenario: MP3 audio file accepted

- GIVEN an audio file with first bytes `49 44 33` (ID3 tag header)
- WHEN MIME validation runs
- THEN the audio is classified as `AllowedAudioMime::Mp3`

#### Scenario: Magic bytes override declared MIME

- GIVEN a channel declares an audio file as `audio/mpeg`
- WHEN the first bytes are `4F 67 67 53` (OGG magic bytes)
- THEN the runtime classifies the audio as `audio/ogg`
- AND the declared `audio/mpeg` MIME is ignored
- AND the runtime SHOULD log a warning about the mismatch

#### Scenario: Unsupported format rejected — FLAC

- GIVEN a user sends a FLAC file (magic bytes `66 4C 61 43`)
- WHEN MIME validation runs
- THEN the audio is rejected with `AudioRejectionReason::MimeRejected`
- AND the user receives "That audio format is not supported. Supported formats: OGG, MP3, WAV, M4A."

#### Scenario: Unsupported format rejected — MIDI

- GIVEN a user sends a MIDI file
- WHEN magic-byte sniffing does not match any allowed format
- THEN the audio is rejected with `AudioRejectionReason::MimeRejected`

### REQ-4: Size and Duration Limits (NFR3)

The runtime MUST enforce the following limits on audio input:

- **Max audio payload size**: 25 MiB (`MAX_AUDIO_BYTES = 25 * 1024 * 1024 = 26214400`) by default
- **Max audio duration**: 10 minutes (`MAX_AUDIO_DURATION_SECS = 600`) by default

The `audio.max_audio_bytes` configuration field MUST override `MAX_AUDIO_BYTES` when set.
The `audio.max_audio_duration_secs` configuration field MUST override `MAX_AUDIO_DURATION_SECS`
when set.

Size validation MUST occur during streaming — the runtime SHOULD reject oversized audio before
fully downloading when `Content-Length` is available, and MUST reject during streaming when
accumulated bytes exceed the limit.

Duration validation MUST use the channel-declared duration (`declared_duration_secs`) when available
for pre-fetch gating. If the channel does not provide duration, duration validation MAY be deferred
to post-transcription (whisper.cpp reports actual duration).

Config validation for size/duration limits (see REQ-7):
- `max_audio_bytes` MUST be > 0 and MUST NOT exceed 100 MiB (hardcoded ceiling)
- `max_audio_duration_secs` MUST be > 0 and MUST NOT exceed 3600 (1 hour, hardcoded ceiling)
- Invalid values MUST cause a startup validation error

#### Scenario: Audio within size limit accepted

- GIVEN `max_audio_bytes` is 26214400 (25 MiB)
- WHEN a user sends a 5 MiB voice note
- THEN size validation passes

#### Scenario: Audio exactly at size limit accepted

- GIVEN `max_audio_bytes` is 26214400 (25 MiB)
- WHEN a user sends an audio file of exactly 26214400 bytes
- THEN size validation passes (limit is inclusive)

#### Scenario: Audio exceeding size limit rejected

- GIVEN `max_audio_bytes` is 26214400 (25 MiB)
- WHEN a user sends a 30 MiB audio file
- THEN the audio is rejected with `AudioRejectionReason::Oversize`
- AND the user receives "That audio file is too large to process. Maximum size: 25 MB."
- AND an `AudioIngressEvent` with outcome `Rejected` and reason `Oversize` is emitted

#### Scenario: Early rejection via Content-Length

- GIVEN `max_audio_bytes` is 26214400 (25 MiB)
- WHEN the channel API returns `Content-Length: 31457280` (30 MiB)
- THEN the runtime rejects the audio with `Oversize` before downloading any bytes

#### Scenario: Audio within duration limit accepted

- GIVEN `max_audio_duration_secs` is 600 (10 minutes)
- WHEN a Telegram user sends a voice note with `duration: 120` (2 minutes)
- THEN duration validation passes

#### Scenario: Audio exactly at duration limit accepted

- GIVEN `max_audio_duration_secs` is 600 (10 minutes)
- WHEN a Telegram user sends a voice note with `duration: 600`
- THEN duration validation passes (limit is inclusive)

#### Scenario: Audio exceeding duration limit rejected

- GIVEN `max_audio_duration_secs` is 600 (10 minutes)
- WHEN a Telegram user sends a voice note with `duration: 900` (15 minutes)
- THEN the audio is rejected with `AudioRejectionReason::TooLong`
- AND the user receives "That audio is too long to process. Maximum duration: 10 minutes."
- AND steps 3–7 of the pipeline are NOT executed (no fetch)

#### Scenario: Duration unknown — deferred validation

- GIVEN a channel does not provide a duration value (`declared_duration_secs: None`)
- WHEN the audio passes size and MIME validation
- THEN the runtime MUST proceed to transcription
- AND if the transcriber reports a duration exceeding `max_audio_duration_secs`, the runtime SHOULD
  log a warning but MUST NOT reject (transcription already completed)

#### Scenario: Config override reduces size limit

- GIVEN `audio.max_audio_bytes` is set to `5242880` (5 MiB) in config
- WHEN a user sends a 7 MiB audio file
- THEN the audio is rejected with `Oversize`
- AND the effective limit is 5 MiB

### REQ-5: File Staging and RAII Cleanup

Validated audio bytes MUST be written to a temp file as a `StagedAudio` struct with the following
fields:

| Field            | Type              | Description                              |
|------------------|-------------------|------------------------------------------|
| `sha256`         | `String`          | SHA-256 hash of the raw audio bytes      |
| `mime_type`      | `AllowedAudioMime`| Validated MIME type from sniffing         |
| `byte_len`       | `u64`             | Total byte size of the staged file       |
| `duration_secs`  | `Option<f64>`     | Duration if known (channel or post-transcription) |
| `temp_path`      | `PathBuf`         | Path to the temp file on disk            |
| `channel_origin` | `String`          | Channel name that sourced the audio      |

Temp file naming MUST follow the pattern:
`corvus-{channel_abbrev}-aud-{sha256_prefix_16}.{ext}`

Staged files MUST be cleaned up via `StagedAudioGuard` RAII semantics:
- The guard's `Drop` implementation MUST call `StagedAudio::cleanup()` for each staged audio
- Cleanup MUST be best-effort (log warning on failure, do not panic)
- Cleanup MUST occur on all exit paths: success, error, timeout, transcription failure, early return

#### Scenario: Temp file created with correct naming

- GIVEN a valid OGG/Opus voice note from Telegram with SHA-256 starting with `a1b2c3d4e5f6g7h8`
- WHEN the audio is staged to disk
- THEN the temp file path MUST match `corvus-tg-aud-a1b2c3d4e5f6g7h8.ogg`
- AND the file is written to `std::env::temp_dir()`

#### Scenario: Cleanup on successful transcription

- GIVEN a valid audio file is staged and transcribed successfully
- WHEN the turn completes (agent responds)
- THEN `StagedAudioGuard::drop()` fires and removes the temp file
- AND no orphaned audio files remain

#### Scenario: Cleanup on transcription failure

- GIVEN a valid audio file is staged but transcription fails
- WHEN the error is returned to the user
- THEN `StagedAudioGuard::drop()` fires and removes the temp file

#### Scenario: Cleanup on timeout

- GIVEN a valid audio file is staged and transcription is in progress
- WHEN the turn times out
- THEN `StagedAudioGuard::drop()` fires and removes the temp file

### REQ-6: Transcriber Trait and Transcription (FR2)

The runtime MUST define a `Transcriber` trait as a new extension point for speech-to-text engines:

```rust
trait Transcriber: Send + Sync {
    fn name(&self) -> &str;
    async fn transcribe(&self, audio: &StagedAudio) -> Result<TranscriptionResult, AudioRejectionReason>;
    async fn health_check(&self) -> Result<(), String>;
}
```

`TranscriptionResult` MUST contain:

| Field           | Type             | Description                                  |
|-----------------|------------------|----------------------------------------------|
| `text`          | `String`         | The transcribed text                         |
| `language`      | `Option<String>` | Detected or configured language              |
| `duration_secs` | `Option<f64>`    | Actual audio duration as reported by engine (None if not reported) |
| `confidence`    | `Option<f64>`    | Confidence score if available (0.0–1.0)      |

The Phase 1 implementation MUST be a whisper.cpp CLI wrapper that:
- Spawns `whisper-cli` (or configured binary path) as an external process
- Passes the staged audio file path and configured model/language
- Parses stdout for transcription text
- Returns structured errors on non-zero exit, timeout, or unparseable output
- MUST NOT block the async runtime (use `tokio::process::Command`)

Transcription MUST be bounded by a concurrency semaphore (REQ-12) and MUST complete within the
turn's overall timeout budget.

The `Transcriber::health_check()` method MUST verify:
- The whisper binary is accessible and executable
- The configured model file exists at the expected path

#### Scenario: Successful transcription of Spanish voice note

- GIVEN a healthy whisper.cpp transcriber with `base` model
- AND `transcription_language` is `"es"`
- WHEN a staged OGG/Opus file containing "Hola, ¿cómo estás?" is transcribed
- THEN `TranscriptionResult.text` MUST be a non-empty string containing the spoken words
- AND `TranscriptionResult.duration_secs` SHOULD be `Some(d)` where `d > 0`
- AND `TranscriptionResult.language` SHOULD be `Some("es")`

#### Scenario: Transcription failure — whisper binary not found

- GIVEN the whisper binary is not installed or not in PATH
- WHEN `transcribe()` is called
- THEN it MUST return `Err` with a descriptive error
- AND the audio MUST be rejected with `AudioRejectionReason::TranscriptionFailed`
- AND the user receives "Audio transcription failed. Please try again or send text instead."

#### Scenario: Transcription failure — corrupt audio

- GIVEN a staged audio file that passes MIME sniffing but has corrupted content
- WHEN whisper.cpp attempts to decode it
- THEN the process exits with non-zero status
- AND the audio MUST be rejected with `AudioRejectionReason::Corrupted`
- AND the user receives "That audio file appears to be corrupted and cannot be processed."

#### Scenario: Transcription failure — process timeout

- GIVEN a very large audio file near the duration limit
- WHEN the whisper.cpp process does not complete within the turn timeout
- THEN the process MUST be killed
- AND the audio MUST be rejected with `AudioRejectionReason::TranscriptionFailed`

#### Scenario: Health check — healthy

- GIVEN whisper binary exists at the configured path
- AND the configured model file exists at `~/.corvus/models/whisper/{model}.bin`
- WHEN `health_check()` is called
- THEN it MUST return `Ok(())`

#### Scenario: Health check — unhealthy (missing model)

- GIVEN whisper binary exists but the configured model file does not exist
- WHEN `health_check()` is called
- THEN it MUST return `Err(String)` with a descriptive message about the missing model

### REQ-7: Audio Configuration

Audio input MUST be gated by a separate `[audio]` config section, independent from `[multimodal]`:

```toml
[audio]
enabled = false                      # bool, default: false — global kill switch
allowed_channels = []                # list of strings — channel allowlist
max_audio_bytes = 26214400           # u64, default: 25 MiB
max_audio_duration_secs = 600        # u64, default: 10 minutes
transcription_model = "base"         # string, default: "base"
transcription_language = "es"        # string, default: "es"
whisper_binary = "whisper-cli"       # string, default: "whisper-cli"
max_concurrent_transcriptions = 1    # usize, default: 1
transcription_timeout_secs = 120     # u64, default: 120
```

Startup validation MUST enforce:

- If `enabled=true`, then `allowed_channels` MUST be non-empty. Violation MUST produce a startup
  error: "audio.allowed_channels must be non-empty when audio is enabled"
- If `max_audio_bytes` is set, it MUST be > 0 and <= 104857600 (100 MiB). Violation MUST produce a
  startup error.
- If `max_audio_duration_secs` is set, it MUST be > 0 and <= 3600. Violation MUST produce a startup
  error.
- Non-Phase-1 channel names in `allowed_channels` (anything other than `"telegram"`) SHOULD produce
  a startup warning. These channels will be fail-closed at runtime since no audio parsing
  implementation exists.

When `[audio]` is absent from the config file, all audio fields MUST default to their documented
defaults. With defaults, audio is disabled (`enabled = false`).

The runtime MUST log effective audio config at startup when audio is enabled:
`"Audio enabled: allowed_channels={:?}, max_bytes={}, max_duration={}s, model={}, language={}"`

#### Scenario: Valid audio config

- GIVEN a config file with `audio.enabled=true`, `audio.allowed_channels=["telegram"]`,
  `audio.transcription_model="base"`, `audio.transcription_language="es"`
- WHEN the runtime starts
- THEN config validation passes
- AND the runtime logs effective audio configuration

#### Scenario: Invalid config — enabled without allowed_channels

- GIVEN a config file with `audio.enabled=true` and `audio.allowed_channels=[]`
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error

#### Scenario: Invalid config — max_audio_bytes is zero

- GIVEN a config file with `audio.max_audio_bytes=0`
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error

#### Scenario: Invalid config — max_audio_bytes exceeds ceiling

- GIVEN a config file with `audio.max_audio_bytes=209715200` (200 MiB)
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error indicating the 100 MiB ceiling

#### Scenario: Invalid config — max_audio_duration_secs exceeds ceiling

- GIVEN a config file with `audio.max_audio_duration_secs=7200` (2 hours)
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error indicating the 1 hour ceiling

#### Scenario: Missing audio section uses defaults

- GIVEN a config file with no `[audio]` section
- WHEN the runtime starts
- THEN audio is disabled (`enabled = false`)
- AND no startup error is produced

#### Scenario: Warning for non-Phase-1 channel

- GIVEN a config file with `audio.allowed_channels=["telegram", "discord"]`
- WHEN the runtime starts
- THEN the runtime logs a warning that "discord" is not a Phase 1 audio channel
- AND startup succeeds (not a fatal error)

### REQ-8: Conversational Integration and History (FR3, FR5)

When audio is successfully transcribed, the transcription text MUST enter the agent conversation
flow as if the user had typed it. The provider MUST receive the transcription as a normal user text
message.

The runtime MUST store audio metadata in conversation history as `AudioHistoryMeta`:

| Field            | Type             | Description                              |
|------------------|------------------|------------------------------------------|
| `mime`           | `String`         | Validated MIME type string               |
| `sha256`         | `String`         | SHA-256 hash of the audio bytes          |
| `byte_len`       | `u64`            | File size in bytes                       |
| `duration_secs`  | `Option<f64>`    | Audio duration                           |
| `channel_origin` | `String`         | Source channel name                      |
| `transcription`  | `String`         | The transcribed text                     |
| `caption`        | `Option<String>` | Original caption if provided             |

The history representation MUST NOT store raw audio bytes. Audio bytes are ephemeral (temp file,
cleaned up after transcription).

On subsequent turns, the model MUST receive the transcription text as part of conversation history.
The history entry SHOULD indicate audio origin so the model can distinguish transcribed turns from
typed turns.

#### Scenario: Transcription enters agent flow as text

- GIVEN a voice note is transcribed to "Schedule a meeting for tomorrow"
- WHEN the transcription is injected into the message
- THEN the provider receives a user message containing "Schedule a meeting for tomorrow"
- AND the provider response is based on this text
- AND the provider has no knowledge that the input was originally audio

#### Scenario: Audio metadata stored in history

- GIVEN a voice note is successfully transcribed
- WHEN the turn is stored in conversation history
- THEN the history entry contains `AudioHistoryMeta` with transcription text, MIME, hash, and
  duration
- AND the history entry does NOT contain raw audio bytes

#### Scenario: Follow-up references transcribed content

- GIVEN turn 1 was a voice note transcribed to "I need to book a flight to Madrid"
- AND the agent responded with flight options
- WHEN the user sends "What about the second option?" on turn 2 (text)
- THEN the conversation history includes the transcription from turn 1
- AND the model can reference the prior transcribed content

#### Scenario: Audio with caption combines both in context

- GIVEN a user sends an audio file with caption "translate this"
- AND the transcription produces "Buenos días, ¿cómo estás?"
- WHEN the transcription is injected
- THEN the provider receives text that includes both the caption context and the transcription
- AND `AudioHistoryMeta.caption` is `Some("translate this")`

### REQ-9: User Response Through Same Channel (FR4)

The agent's response to a transcribed audio message MUST be delivered through the same channel that
received the audio. The response format MUST be text (not audio). The runtime MUST NOT generate
audio output (text-to-speech is out of scope).

#### Scenario: Response on Telegram

- GIVEN a Telegram user sends a voice note
- AND it is transcribed and processed
- WHEN the agent generates a response
- THEN the response MUST be sent back via Telegram as a text message
- AND the response MUST NOT be sent as a voice note or audio file

### REQ-10: Telegram Channel Support (FR7)

The Telegram channel MUST parse the following message types as `ContentPart::Audio`:

| Telegram Field    | Audio Type  | Expected MIME  | Duration Source        |
|-------------------|-------------|----------------|------------------------|
| `message.voice`   | Voice note  | `audio/ogg`    | `voice.duration`       |
| `message.audio`   | Audio file  | Varies         | `audio.duration`       |

For `message.voice`:
- `channel_handle` MUST be `voice.file_id`
- `declared_mime` SHOULD be `Some("audio/ogg")` (Telegram voice notes are always OGG/Opus)
- `declared_duration_secs` MUST be `Some(voice.duration)`
- `declared_bytes` SHOULD be `voice.file_size` when available

For `message.audio`:
- `channel_handle` MUST be `audio.file_id`
- `declared_mime` SHOULD be `audio.mime_type` when available
- `declared_duration_secs` MUST be `Some(audio.duration)`
- `declared_bytes` SHOULD be `audio.file_size` when available
- `file_name` SHOULD be `audio.file_name` when available

Audio fetch MUST use the same Telegram Bot API pattern as image fetch:
`POST getFile` → resolve `file_path` → `GET /file/bot{token}/{file_path}` with streaming
download and size validation.

Authentication credentials MUST NOT appear in error messages or logs.

#### Scenario: Telegram voice note parsed

- GIVEN a Telegram message with `voice: { file_id: "abc123", duration: 5, file_size: 12345 }`
- WHEN `build_telegram_content_parts()` processes the message
- THEN it MUST produce `ContentPart::Audio { channel_handle: "abc123", source_channel: "telegram",
  declared_mime: Some("audio/ogg"), declared_duration_secs: Some(5), declared_bytes: Some(12345) }`

#### Scenario: Telegram audio file parsed

- GIVEN a Telegram message with `audio: { file_id: "xyz789", duration: 120,
  mime_type: "audio/mpeg", file_size: 500000, file_name: "recording.mp3" }`
- WHEN `build_telegram_content_parts()` processes the message
- THEN it MUST produce `ContentPart::Audio { channel_handle: "xyz789", source_channel: "telegram",
  declared_mime: Some("audio/mpeg"), declared_duration_secs: Some(120),
  declared_bytes: Some(500000), file_name: Some("recording.mp3") }`

#### Scenario: Telegram message with voice and text

- GIVEN a Telegram message with a voice note and `caption: "what does this say?"`
- WHEN the channel layer parses the message
- THEN `ChannelMessage.parts` MUST contain both a `ContentPart::Text` and `ContentPart::Audio`
- AND `caption_text` on the Audio part MUST be `Some("what does this say?")`

#### Scenario: Telegram message with only text — no audio parsing

- GIVEN a Telegram text message with no `voice` or `audio` field
- WHEN `build_telegram_content_parts()` processes the message
- THEN no `ContentPart::Audio` is produced
- AND existing text behavior is unchanged

### REQ-11: Error Taxonomy (FR6)

The runtime MUST use the following rejection reasons as a stable contract. Each rejection reason
MUST map to exactly one user-facing message and one observability event.

| Rejection Reason        | User-Facing Message                                                                  | Emitted When                                              |
|-------------------------|--------------------------------------------------------------------------------------|-----------------------------------------------------------|
| `Disabled`              | "Audio input is currently disabled."                                                 | `audio.enabled` is `false`                                |
| `ChannelNotAllowed`     | "Audio input is not enabled for this channel."                                       | Channel not in `audio.allowed_channels`                   |
| `FetchFailed`           | "I couldn't download that audio safely. Please try again."                           | Channel fetch fails (network, auth, timeout)              |
| `MimeRejected`          | "That audio format is not supported. Supported formats: OGG, MP3, WAV, M4A."        | Magic-byte sniffing does not match allowed formats        |
| `Oversize`              | "That audio file is too large to process. Maximum size: 25 MB."                      | Audio bytes exceed effective size limit                   |
| `TooLong`               | "That audio is too long to process. Maximum duration: 10 minutes."                   | Duration exceeds effective duration limit                 |
| `Corrupted`             | "That audio file appears to be corrupted and cannot be processed."                   | Transcription engine cannot decode the audio              |
| `TranscriptionFailed`   | "Audio transcription failed. Please try again or send text instead."                 | Transcriber returns error (process crash, timeout, etc.)  |
| `NoSpeechDetected`      | "No speech was detected in that audio. Please try again with a clearer recording."   | Transcription produces empty/whitespace-only text         |
| `TranscriberUnavailable`| "Audio transcription is not available on this agent. Please send text instead."      | No healthy Transcriber is registered or health check fails|
| `SystemError`           | "An internal error occurred while processing audio. Please try again."               | Unexpected internal error (e.g., temp file I/O failure, semaphore poisoning) |

This taxonomy (11 variants) MUST be exhaustive for Phase 1. Every audio rejection MUST map to
exactly one of these reasons.

All rejection reasons MUST:
- Be variants of `AudioRejectionReason` enum
- Implement `Display` producing a stable snake_case identifier (e.g., `disabled`, `mime_rejected`)
- Emit an `AudioIngressEvent` with outcome `Rejected` and the corresponding reason

User-facing messages MUST be static strings (with parameter substitution for `Oversize` and
`TooLong` only, reflecting effective limits). The runtime MUST NOT expose internal error details
(stack traces, file paths, binary paths, credentials) in user-facing messages.

#### Scenario: Disabled rejection

- GIVEN `audio.enabled` is `false`
- WHEN any user sends audio on any channel
- THEN the audio is rejected with reason `Disabled`
- AND the user receives "Audio input is currently disabled."

#### Scenario: Unsupported format rejection

- GIVEN audio is enabled for Telegram
- WHEN a user sends a FLAC file
- THEN the audio is rejected with reason `MimeRejected`
- AND the user receives the message listing supported formats

#### Scenario: Oversize rejection

- GIVEN `max_audio_bytes` is 26214400
- WHEN a user sends a 30 MiB audio file
- THEN the audio is rejected with reason `Oversize`

#### Scenario: Too long rejection

- GIVEN `max_audio_duration_secs` is 600
- WHEN Telegram declares `duration: 900`
- THEN the audio is rejected with reason `TooLong` before fetch

#### Scenario: Corrupted audio rejection

- GIVEN a file passes MIME sniffing (valid OGG header) but has truncated/corrupted content
- WHEN whisper.cpp fails to decode it
- THEN the audio is rejected with reason `Corrupted`

#### Scenario: No speech detected

- GIVEN a valid audio file containing only silence or background noise
- WHEN whisper.cpp produces an empty or whitespace-only transcription
- THEN the audio is rejected with reason `NoSpeechDetected`
- AND the user receives the no-speech message
- AND no empty message is sent to the agent

#### Scenario: Transcriber unavailable

- GIVEN whisper.cpp is not installed
- WHEN a user sends a voice note on an enabled channel
- THEN the runtime detects that `health_check()` returns `Err(..)`
- AND the audio is rejected with reason `TranscriberUnavailable`
- AND the user receives "Audio transcription is not available on this agent. Please send text
  instead."

#### Scenario: Fetch failure

- GIVEN the Telegram Bot API is unreachable (network timeout)
- WHEN a user sends a voice note
- THEN the audio is rejected with reason `FetchFailed`
- AND no credentials or internal URLs appear in the user message

### REQ-12: Concurrency Control

Transcription MUST be bounded by a concurrency semaphore to prevent CPU overload from multiple
simultaneous audio messages. The semaphore MUST have a configurable limit with a default of 1
concurrent transcription.

When the semaphore is full, incoming audio transcription requests MUST wait (up to the turn
timeout). If the timeout expires while waiting for the semaphore, the audio MUST be rejected with
`AudioRejectionReason::TranscriptionFailed`.

#### Scenario: Sequential transcription under default concurrency

- GIVEN the concurrency limit is 1 (default)
- AND user A sends a voice note at time T
- WHEN user B sends a voice note at time T+1 (while A's transcription is running)
- THEN user B's transcription waits for user A's to complete
- AND both users eventually receive responses

#### Scenario: Timeout while waiting for semaphore

- GIVEN the concurrency limit is 1
- AND a long-running transcription holds the semaphore
- WHEN a second audio message arrives and the turn timeout expires while waiting
- THEN the second audio is rejected with `TranscriptionFailed`
- AND the user receives the transcription-failed error message

### REQ-13: Observability (NFR4)

Every audio ingestion attempt MUST emit an `AudioIngressEvent` via the observer pattern
(`Observer::on_audio_ingress()`).

The `AudioIngressEvent` MUST contain:

| Field           | Type                   | Description                                    |
|-----------------|------------------------|------------------------------------------------|
| `channel`       | `String`               | Source channel name                            |
| `outcome`       | `AudioIngressOutcome`  | Admitted, Rejected                             |
| `reason`        | `Option<AudioIngressReason>` | Rejection reason (if rejected)           |
| `mime_type`     | `Option<String>`       | Detected MIME type (if validation reached)     |
| `byte_len`      | `Option<u64>`          | File size (if known)                           |
| `duration_secs` | `Option<f64>`          | Duration (if known)                            |
| `transcription_duration_ms` | `Option<u64>` | Wall-clock time for transcription (if completed) |

`AudioIngressOutcome` MUST have at least these variants:
- `Admitted` — audio was transcribed and injected into the agent flow
- `Rejected` — audio was rejected at any pipeline step

`AudioIngressReason` MUST mirror the `AudioRejectionReason` variants for the `reason` field.

#### Scenario: Admitted event emitted on success

- GIVEN a voice note is successfully transcribed
- WHEN the transcription is injected
- THEN an `AudioIngressEvent` with outcome `Admitted` is emitted
- AND `transcription_duration_ms` records the wall-clock transcription time
- AND `mime_type`, `byte_len`, and `duration_secs` are populated

#### Scenario: Rejected event emitted on failure

- GIVEN a voice note is rejected for being oversized
- WHEN the rejection occurs
- THEN an `AudioIngressEvent` with outcome `Rejected` and reason `Oversize` is emitted
- AND `byte_len` records the declared or detected size

### REQ-14: Empty Transcription Guard (FR8)

The runtime MUST NOT send an empty or whitespace-only transcription to the agent. If
`TranscriptionResult.text` is empty or contains only whitespace after trimming, the audio MUST be
rejected with `AudioRejectionReason::NoSpeechDetected`.

#### Scenario: Empty transcription blocked

- GIVEN whisper.cpp returns `text: ""`
- WHEN the runtime processes the transcription result
- THEN the audio is rejected with `NoSpeechDetected`
- AND no message is sent to the provider
- AND the user receives the no-speech error message

#### Scenario: Whitespace-only transcription blocked

- GIVEN whisper.cpp returns `text: "   \n  \t  "`
- WHEN the runtime trims and checks the transcription
- THEN the audio is rejected with `NoSpeechDetected`

#### Scenario: Valid transcription with leading/trailing whitespace accepted

- GIVEN whisper.cpp returns `text: "  Hello world  "`
- WHEN the runtime trims and checks the transcription
- THEN the trimmed text `"Hello world"` is injected into the message
- AND processing continues normally

### REQ-15: Privacy — Local Processing Only (NFR1)

All audio transcription MUST be performed locally. The runtime MUST NOT send audio data to any
external third-party service for processing. This includes cloud STT APIs (OpenAI Whisper API,
Google Cloud Speech-to-Text, AWS Transcribe, Azure Speech Services, etc.).

The `Transcriber` implementation MUST NOT make any outbound network requests during transcription.

Audio bytes MUST NOT be logged, traced, or persisted beyond the ephemeral temp file used for
transcription. The temp file MUST be cleaned up via RAII (REQ-5).

#### Scenario: No network calls during transcription

- GIVEN audio transcription is in progress
- WHEN the `Transcriber::transcribe()` method executes
- THEN zero outbound network requests are made
- AND all processing occurs on the local machine

#### Scenario: Audio bytes not logged

- GIVEN a voice note is being processed
- WHEN the runtime logs events related to the audio
- THEN log entries MUST NOT contain raw audio bytes or base64-encoded audio
- AND log entries MAY contain metadata (size, MIME, duration, hash)

### REQ-16: Reliability — Audio Failure Isolation (NFR2)

Audio processing failures MUST NOT break the user's session or prevent subsequent text messages.
If any step of the audio pipeline fails, the runtime MUST:

1. Reject the audio with an appropriate error message
2. Clean up any staged temp files
3. Continue accepting messages on the same session

The audio pipeline MUST NOT panic or crash on any input, including:
- Zero-byte audio files
- Extremely large files (rejected by size limit)
- Files with valid headers but corrupted content
- Non-audio files disguised with audio extensions
- Concurrent audio messages from the same user

#### Scenario: Session continues after audio failure

- GIVEN a user sends a corrupted audio file that fails transcription
- AND the user receives an error message
- WHEN the same user sends a text message "hello" afterwards
- THEN the text message is processed normally
- AND the session state is intact

#### Scenario: Zero-byte audio file handled gracefully

- GIVEN a user sends a file with 0 bytes
- WHEN the runtime validates it
- THEN it is rejected (MIME sniffing fails on empty input) with `MimeRejected` or `Corrupted`
- AND no panic occurs
- AND the session continues

### REQ-17: Progressive Compatibility — No Text Regression (NFR5)

Adding audio support MUST NOT change any existing behavior for text-only or image-only messages.
Specifically:

- Text messages MUST continue to be processed identically whether audio is enabled or disabled
- Image messages MUST continue to flow through the existing image pipeline unchanged
- The `ChatRequest` struct MUST NOT be modified
- No existing config sections MUST be modified (audio uses a new `[audio]` section)
- No existing `ContentPart` variants MUST be modified (audio adds a new variant)

#### Scenario: Text flow unchanged with audio enabled

- GIVEN `[audio]` is enabled with `allowed_channels: ["telegram"]`
- WHEN a Telegram user sends a plain text message "hello"
- THEN the message is processed through the existing text path
- AND the audio pipeline is NOT invoked
- AND the response is identical to what it would be with audio disabled

#### Scenario: Image flow unchanged with audio enabled

- GIVEN both `[multimodal]` and `[audio]` are enabled
- WHEN a Telegram user sends a photo
- THEN the image pipeline handles it (not the audio pipeline)
- AND the image flow is identical to behavior before audio support was added

### REQ-18: Doctor Health Check

`corvus doctor` MUST include audio-related health checks when `[audio]` is enabled:

| Check                | Pass Condition                                        | Fail Message                                           |
|----------------------|-------------------------------------------------------|--------------------------------------------------------|
| Whisper binary       | Binary exists and is executable at configured path     | "whisper binary not found at {path}"                   |
| Whisper model        | Model file exists at `~/.corvus/models/whisper/{model}.bin` | "whisper model '{model}' not found"              |

When `[audio]` is disabled, these checks SHOULD be skipped (or marked as "skipped — audio
disabled").

#### Scenario: Doctor passes with healthy setup

- GIVEN `[audio]` is enabled with `transcription_model: "base"`
- AND whisper binary is installed
- AND `~/.corvus/models/whisper/base.bin` exists
- WHEN `corvus doctor` runs
- THEN both audio checks pass

#### Scenario: Doctor warns on missing model

- GIVEN `[audio]` is enabled with `transcription_model: "small"`
- AND whisper binary is installed
- BUT `~/.corvus/models/whisper/small.bin` does not exist
- WHEN `corvus doctor` runs
- THEN the model check fails with "whisper model 'small' not found"

#### Scenario: Doctor skips when audio disabled

- GIVEN `[audio]` is disabled
- WHEN `corvus doctor` runs
- THEN audio health checks are skipped or marked "skipped — audio disabled"

## Cross-References

- **Channel Image Ingestion Spec** (`openspec/specs/channel-image-ingestion/spec.md`, #266):
  Audio mirrors the image ingestion patterns (parse → gate → fetch → validate → stage) but adds
  transcription and text injection stages. Audio does NOT modify image specs.

- **Runtime Image Pipeline Spec** (`openspec/specs/runtime-image-pipeline/spec.md`, #267):
  Audio mirrors pipeline architecture and RAII cleanup but differs in that audio is transcribed
  pre-loop while images are forwarded to the provider. No image pipeline changes.

- **Agent Loop Spec** (`openspec/specs/agent-loop/spec.md`):
  The agent loop receives the transcribed text as a normal user message. No agent loop changes
  are required — audio is transparent to the loop after transcription.
