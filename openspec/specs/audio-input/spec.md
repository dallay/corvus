# Audio Input Specification

**Domain**: channels / audio / transcription
**Status**: final
**Issue**: #246 / DALLAY-150 (Phase 1), #412 (Phase 2)
**Date**: 2026-04-04
**Depends on**: `channel-image-ingestion` spec (#266), `runtime-image-pipeline` spec (#267)

## Overview

This specification defines the behavioral contract for audio input support in the Corvus runtime
(Phase 1: Core infrastructure + Telegram; Phase 2: HTTP Gateway + CLI). Users send voice notes or
audio files through a channel;
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

| Field                    | Type             | Required | Description                                 |
|--------------------------|------------------|----------|---------------------------------------------|
| `channel_handle`         | `String`         | Yes      | Channel-specific media identifier           |
| `source_channel`         | `String`         | Yes      | Channel name (e.g., "telegram")             |
| `declared_mime`          | `Option<String>` | No       | MIME type declared by the channel           |
| `caption_text`           | `Option<String>` | No       | Accompanying text/caption from the user     |
| `file_name`              | `Option<String>` | No       | Original file name (audio files only)       |
| `declared_bytes`         | `Option<u64>`    | No       | File size declared by the channel           |
| `declared_duration_secs` | `Option<u64>`    | No       | Duration in seconds declared by the channel |

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
- AND step 7 (inject) replaces the Audio part with
  `ContentPart::Text { text: "¿Qué tiempo hace hoy?" }`
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

| Format   | Magic Bytes                              | MIME       | Extension |
|----------|------------------------------------------|------------|-----------|
| OGG/Opus | `4F 67 67 53` ("OggS")                   | audio/ogg  | .ogg      |
| MP3      | `FF FB`, `FF F3`, `FF F2`, or `49 44 33` | audio/mpeg | .mp3      |
| WAV      | `52 49 46 46....57 41 56 45` (RIFF+WAVE) | audio/wav  | .wav      |
| M4A/AAC  | `....66 74 79 70` (ftyp at offset 4)     | audio/mp4  | .m4a      |

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

| Field            | Type               | Description                                       |
|------------------|--------------------|---------------------------------------------------|
| `sha256`         | `String`           | SHA-256 hash of the raw audio bytes               |
| `mime_type`      | `AllowedAudioMime` | Validated MIME type from sniffing                 |
| `byte_len`       | `u64`              | Total byte size of the staged file                |
| `duration_secs`  | `Option<f64>`      | Duration if known (channel or post-transcription) |
| `temp_path`      | `PathBuf`          | Path to the temp file on disk                     |
| `channel_origin` | `String`           | Channel name that sourced the audio               |

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

| Field           | Type             | Description                                                        |
|-----------------|------------------|--------------------------------------------------------------------|
| `text`          | `String`         | The transcribed text                                               |
| `language`      | `Option<String>` | Detected or configured language                                    |
| `duration_secs` | `Option<f64>`    | Actual audio duration as reported by engine (None if not reported) |
| `confidence`    | `Option<f64>`    | Confidence score if available (0.0–1.0)                            |

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
- Unrecognized channel names in `allowed_channels` (anything other than `"telegram"`, `"gateway"`,
  `"cli"`) SHOULD produce a startup warning. These channels will be fail-closed at runtime since
  no audio implementation exists for them.

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

#### Scenario: Warning for unrecognized channel

- GIVEN a config file with `audio.allowed_channels=["telegram", "discord"]`
- WHEN the runtime starts
- THEN the runtime logs a warning that "discord" is not a recognized audio channel
- AND startup succeeds (not a fatal error)

#### Scenario: Gateway and CLI recognized as valid channels

- GIVEN `audio.allowed_channels: ["telegram", "gateway", "cli"]`
- WHEN the runtime starts
- THEN config validation MUST pass with no warnings

#### Scenario: Unknown channel still produces warning

- GIVEN `audio.allowed_channels: ["telegram", "gateway", "slack"]`
- WHEN the runtime starts
- THEN the runtime MUST log a warning for `"slack"` (not a recognized channel)
- AND MUST NOT log warnings for `"telegram"` or `"gateway"`

### REQ-8: Conversational Integration and History (FR3, FR5)

When audio is successfully transcribed, the transcription text MUST enter the agent conversation
flow as if the user had typed it. The provider MUST receive the transcription as a normal user text
message.

The runtime MUST store audio metadata in conversation history as `AudioHistoryMeta`:

| Field            | Type             | Description                     |
|------------------|------------------|---------------------------------|
| `mime`           | `String`         | Validated MIME type string      |
| `sha256`         | `String`         | SHA-256 hash of the audio bytes |
| `byte_len`       | `u64`            | File size in bytes              |
| `duration_secs`  | `Option<f64>`    | Audio duration                  |
| `channel_origin` | `String`         | Source channel name             |
| `transcription`  | `String`         | The transcribed text            |
| `caption`        | `Option<String>` | Original caption if provided    |

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

| Telegram Field  | Audio Type | Expected MIME | Duration Source  |
|-----------------|------------|---------------|------------------|
| `message.voice` | Voice note | `audio/ogg`   | `voice.duration` |
| `message.audio` | Audio file | Varies        | `audio.duration` |

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

| Rejection Reason         | User-Facing Message                                                                | Emitted When                                                                 |
|--------------------------|------------------------------------------------------------------------------------|------------------------------------------------------------------------------|
| `Disabled`               | "Audio input is currently disabled."                                               | `audio.enabled` is `false`                                                   |
| `ChannelNotAllowed`      | "Audio input is not enabled for this channel."                                     | Channel not in `audio.allowed_channels`                                      |
| `FetchFailed`            | "I couldn't download that audio safely. Please try again."                         | Channel fetch fails (network, auth, timeout)                                 |
| `MimeRejected`           | "That audio format is not supported. Supported formats: OGG, MP3, WAV, M4A."       | Magic-byte sniffing does not match allowed formats                           |
| `Oversize`               | "That audio file is too large to process. Maximum size: 25 MB."                    | Audio bytes exceed effective size limit                                      |
| `TooLong`                | "That audio is too long to process. Maximum duration: 10 minutes."                 | Duration exceeds effective duration limit                                    |
| `Corrupted`              | "That audio file appears to be corrupted and cannot be processed."                 | Transcription engine cannot decode the audio                                 |
| `TranscriptionFailed`    | "Audio transcription failed. Please try again or send text instead."               | Transcriber returns error (process crash, timeout, etc.)                     |
| `NoSpeechDetected`       | "No speech was detected in that audio. Please try again with a clearer recording." | Transcription produces empty/whitespace-only text                            |
| `TranscriberUnavailable` | "Audio transcription is not available on this agent. Please send text instead."    | No healthy Transcriber is registered or health check fails                   |
| `SystemError`            | "An internal error occurred while processing audio. Please try again."             | Unexpected internal error (e.g., temp file I/O failure, semaphore poisoning) |

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

| Field                       | Type                         | Description                                      |
|-----------------------------|------------------------------|--------------------------------------------------|
| `channel`                   | `String`                     | Source channel name                              |
| `outcome`                   | `AudioIngressOutcome`        | Admitted, Rejected                               |
| `reason`                    | `Option<AudioIngressReason>` | Rejection reason (if rejected)                   |
| `mime_type`                 | `Option<String>`             | Detected MIME type (if validation reached)       |
| `byte_len`                  | `Option<u64>`                | File size (if known)                             |
| `duration_secs`             | `Option<f64>`                | Duration (if known)                              |
| `transcription_duration_ms` | `Option<u64>`                | Wall-clock time for transcription (if completed) |

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

| Check          | Pass Condition                                              | Fail Message                         |
|----------------|-------------------------------------------------------------|--------------------------------------|
| Whisper binary | Binary exists and is executable at configured path          | "whisper binary not found at {path}" |
| Whisper model  | Model file exists at `~/.corvus/models/whisper/{model}.bin` | "whisper model '{model}' not found"  |

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

### REQ-19: Shared Audio Staging from Bytes

The runtime MUST provide a shared `stage_audio_from_bytes()` function that accepts raw audio
bytes and metadata, performs validation and staging, and returns a `StagedAudio` struct. This
function MUST be usable by all channels — Telegram (post-download), gateway (post-multipart
extraction), and CLI (post-file-read).

**Signature**:

```
stage_audio_from_bytes(
    bytes: &[u8],
    declared_mime: Option<&str>,
    declared_duration_secs: Option<u64>,
    max_bytes: u64,
    max_duration_secs: u64,
    channel_origin: &str,
) -> Result<StagedAudio, AudioRejectionReason>
```

The function MUST perform the following steps in order:

1. **Size validation**: If `bytes.len()` exceeds `max_bytes`, reject with `Oversize`
2. **Duration pre-check**: If `declared_duration_secs` exceeds `max_duration_secs`, reject
   with `TooLong`
3. **MIME validation**: Sniff magic bytes per REQ-3 rules. If no allowed format matches,
   reject with `MimeRejected`
4. **SHA-256 computation**: Compute SHA-256 hash of the raw bytes
5. **Temp file write**: Write bytes to a temp file following the naming convention from REQ-5
6. **Return `StagedAudio`**: Populate all fields per REQ-5

The function MUST reject zero-length input with `MimeRejected` (magic-byte sniffing fails on
empty input).

The existing Telegram audio staging logic MUST be refactored to call this function
post-download. This refactoring MUST NOT change any observable Telegram behavior.

#### Scenario: Valid OGG bytes from gateway staged successfully

- GIVEN `stage_audio_from_bytes()` is called with 50,000 bytes of valid OGG/Opus audio
- AND `channel_origin` is `"gateway"`
- AND `max_bytes` is 26214400 and `max_duration_secs` is 600
- WHEN the function executes
- THEN it MUST return `Ok(StagedAudio)` with `mime_type: OggOpus`
- AND `channel_origin` MUST be `"gateway"`
- AND `sha256` MUST be the SHA-256 hex digest of the input bytes
- AND a temp file MUST exist at the returned `temp_path`

#### Scenario: Valid MP3 bytes from CLI staged successfully

- GIVEN `stage_audio_from_bytes()` is called with valid MP3 bytes (ID3 header `49 44 33`)
- AND `channel_origin` is `"cli"`
- WHEN the function executes
- THEN it MUST return `Ok(StagedAudio)` with `mime_type: Mp3`
- AND `channel_origin` MUST be `"cli"`

#### Scenario: Bytes exceeding max_bytes rejected

- GIVEN `stage_audio_from_bytes()` is called with 30,000,000 bytes
- AND `max_bytes` is 26214400
- WHEN the function executes
- THEN it MUST return `Err(AudioRejectionReason::Oversize)`
- AND no temp file MUST be written

#### Scenario: Invalid magic bytes rejected

- GIVEN `stage_audio_from_bytes()` is called with bytes whose first 16 bytes do not match
  any allowed audio format (REQ-3)
- WHEN the function executes
- THEN it MUST return `Err(AudioRejectionReason::MimeRejected)`
- AND no temp file MUST be written

#### Scenario: Zero-length bytes rejected

- GIVEN `stage_audio_from_bytes()` is called with an empty byte slice (`bytes.len() == 0`)
- WHEN the function executes
- THEN it MUST return `Err(AudioRejectionReason::MimeRejected)`
- AND no temp file MUST be written

#### Scenario: Telegram refactored to use shared function — no behavior change

- GIVEN the Telegram channel previously used inline staging logic in `fetch_and_stage_audio()`
- WHEN the Telegram fetch logic is refactored to call `stage_audio_from_bytes()` post-download
- THEN all existing Telegram audio scenarios (REQ-10) MUST produce identical results
- AND no observable behavior change MUST occur for Telegram users

#### Scenario: Declared duration exceeds limit rejected before staging

- GIVEN `stage_audio_from_bytes()` is called with `declared_duration_secs: Some(900)`
- AND `max_duration_secs` is 600
- WHEN the function executes
- THEN it MUST return `Err(AudioRejectionReason::TooLong)`
- AND no temp file MUST be written
- AND no SHA-256 computation MUST occur

### REQ-20: HTTP Gateway Audio Endpoint

The HTTP Gateway MUST expose a `POST /web/chat/audio` endpoint accepting
`multipart/form-data` for audio file uploads.

#### Multipart Fields

| Field        | Type | Required | Description                                    |
|--------------|------|----------|------------------------------------------------|
| `audio`      | file | Yes      | The audio file to transcribe                   |
| `session_id` | text | No       | Session identifier for conversation continuity |
| `language`   | text | No       | Language hint override for transcription       |

#### Request Constraints

- **Body limit**: The endpoint MUST accept request bodies up to 25 MiB. This MUST override
  the global gateway body limit (64 KB) for this route only.
- **Authentication**: The endpoint MUST use the same bearer token pairing mechanism as
  existing gateway endpoints (`/web/chat/stream`, `/web/chat/message`).
- **Timeout**: The endpoint MUST use a per-request timeout of
  `audio.transcription_timeout_secs + 60` seconds (default: 180s). This MUST override the
  global gateway request timeout for this route.

#### Processing Flow

The handler MUST:

1. Extract the `audio` file field from the multipart body
2. Extract optional `session_id` and `language` text fields
3. Call `stage_audio_from_bytes()` with the extracted audio bytes (REQ-19)
4. Transcribe the staged audio via the runtime's `Transcriber` (REQ-6)
5. Guard against empty transcription per REQ-14
6. Dispatch the transcription text through the existing chat pipeline
7. Return the response as JSON with `transcription` and `response` fields, OR as an SSE
   stream matching the `/web/chat/stream` pattern

#### Channel Gating

- The `audio.allowed_channels` config MUST include `"gateway"` for this endpoint to accept
  audio uploads
- If `"gateway"` is NOT in `allowed_channels` or audio is globally disabled, the endpoint
  MUST return HTTP 403

#### Error Responses

| HTTP Status | Condition                                                                                |
|-------------|------------------------------------------------------------------------------------------|
| 400         | Missing `audio` field, unsupported format (`MimeRejected`), corrupted file (`Corrupted`) |
| 403         | Audio disabled globally OR `"gateway"` not in `allowed_channels`                         |
| 413         | File exceeds body limit or `max_audio_bytes` (`Oversize`)                                |
| 422         | No speech detected (`NoSpeechDetected`), transcription failed (`TranscriptionFailed`)    |
| 500         | System error (`SystemError`)                                                             |
| 503         | Transcriber unavailable (`TranscriberUnavailable`)                                       |

Error response bodies MUST be JSON with at minimum an `error` field containing a
machine-readable code and a `message` field containing the user-facing message from REQ-11.

The endpoint MUST NOT expose internal error details (stack traces, file paths, binary paths,
credentials) in error responses.

#### Scenario: Upload valid OGG file — success

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- AND a healthy Transcriber is available
- WHEN a client sends `POST /web/chat/audio` with a valid OGG/Opus file in the `audio` field
- THEN the response status MUST be 200
- AND the response MUST contain the transcription text
- AND the response MUST contain the agent's response to the transcription
- AND an `AudioIngressEvent` with `channel: "gateway"` and outcome `Admitted` MUST be emitted

#### Scenario: Upload valid MP3 with session_id — session preserved

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- AND a healthy Transcriber is available
- WHEN a client sends `POST /web/chat/audio` with a valid MP3 file and
  `session_id: "sess-abc-123"`
- THEN the response status MUST be 200
- AND the conversation MUST be associated with session `"sess-abc-123"`
- AND subsequent requests with the same `session_id` MUST share conversation context

#### Scenario: Upload FLAC file — rejected

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- WHEN a client sends `POST /web/chat/audio` with a FLAC file
- THEN the response status MUST be 400
- AND the error body MUST indicate `MimeRejected`
- AND the message MUST list supported formats

#### Scenario: Upload 30 MB file — rejected as oversize

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- AND `max_audio_bytes` is 26214400 (25 MiB)
- WHEN a client sends `POST /web/chat/audio` with a 30 MB file
- THEN the response status MUST be 413
- AND the error body MUST indicate `Oversize`

#### Scenario: Missing audio field in multipart — rejected

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- WHEN a client sends `POST /web/chat/audio` with multipart body but no `audio` file field
- THEN the response status MUST be 400
- AND the error body MUST indicate that the `audio` field is missing

#### Scenario: Gateway not in allowed_channels — rejected

- GIVEN audio is enabled with `allowed_channels: ["telegram"]` (gateway NOT included)
- WHEN a client sends `POST /web/chat/audio` with a valid audio file
- THEN the response status MUST be 403
- AND the error body MUST indicate that audio is not enabled for this channel

#### Scenario: Audio disabled globally — rejected

- GIVEN `audio.enabled` is `false`
- WHEN a client sends `POST /web/chat/audio` with a valid audio file
- THEN the response status MUST be 403
- AND the error body MUST indicate that audio is disabled

#### Scenario: Transcriber unavailable — service unavailable

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- AND no healthy Transcriber is registered (health check fails)
- WHEN a client sends `POST /web/chat/audio` with a valid audio file
- THEN the response status MUST be 503
- AND the error body MUST indicate `TranscriberUnavailable`

#### Scenario: Empty audio file — rejected

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- WHEN a client sends `POST /web/chat/audio` with a 0-byte file in the `audio` field
- THEN the response status MUST be 400
- AND the error body MUST indicate `MimeRejected`

#### Scenario: Upload with language hint — language forwarded to transcriber

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- AND the default `transcription_language` is `"es"`
- WHEN a client sends `POST /web/chat/audio` with a valid OGG file and `language: "en"`
- THEN the Transcriber MUST receive the language hint `"en"` instead of the default `"es"`
- AND the transcription SHOULD use the specified language

#### Scenario: Valid bearer token required

- GIVEN audio is enabled with `allowed_channels` including `"gateway"`
- WHEN a client sends `POST /web/chat/audio` without a valid bearer token
- THEN the response status MUST be 401 or 403 (per existing gateway auth behavior)
- AND no audio processing MUST occur

### REQ-21: CLI Audio Command

The CLI MUST support an `/audio <path>` slash command for local audio file transcription and
agent conversation.

#### Command Syntax

```
/audio <file-path>
```

Where `<file-path>` is an absolute path, a relative path, or a path using `~` expansion
(home directory).

If `/audio` is invoked with no path argument, the CLI MUST print a usage hint:
`"Usage: /audio <file-path>"`

#### Processing Flow

The CLI MUST:

1. Parse the `/audio` command and extract the file path
2. Expand `~` to the user's home directory
3. Verify the file exists and is readable
4. Read the file bytes from the local filesystem
5. Call `stage_audio_from_bytes()` with the file bytes (REQ-19)
6. Build a `ChannelMessage` with `ContentPart::Audio` and route through
   `process_channel_message()` to ensure the full audio pipeline runs
7. Transcribe via the runtime's `Transcriber` (REQ-6)
8. Display the transcription to the user before the agent response:
   `[Transcription]: "{transcribed text}"`
9. Continue through the agent conversation flow and display the agent response

#### Channel Gating

- The `audio.allowed_channels` config MUST include `"cli"` for this command to be active
- If audio is globally disabled, the CLI MUST print:
  `"Audio input is currently disabled."`
- If `"cli"` is NOT in `allowed_channels`, the CLI MUST print:
  `"Audio input is not enabled for CLI."`

#### Error Messages

| Condition               | Error Message                                                    |
|-------------------------|------------------------------------------------------------------|
| File not found          | `"File not found: {path}"`                                       |
| File not readable       | `"Cannot read file: {path}"`                                     |
| Unsupported format      | Same `MimeRejected` message as other channels (REQ-11)           |
| File too large          | Same `Oversize` message as other channels (REQ-11)               |
| Transcription failed    | Same `TranscriptionFailed` message as other channels (REQ-11)    |
| No speech detected      | Same `NoSpeechDetected` message as other channels (REQ-11)       |
| Audio disabled          | `"Audio input is currently disabled."`                           |
| CLI not allowed         | `"Audio input is not enabled for CLI."`                          |
| Transcriber unavailable | Same `TranscriberUnavailable` message as other channels (REQ-11) |
| No path argument        | `"Usage: /audio <file-path>"`                                    |

#### Scenario: Valid OGG file transcribed and processed

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- AND a healthy Transcriber is available
- WHEN a user types `/audio ~/recording.ogg` and the file exists and contains valid OGG audio
- THEN the CLI MUST display `[Transcription]: "{transcribed text}"`
- AND the agent MUST process the transcription and produce a response
- AND the response MUST be displayed to the user
- AND an `AudioIngressEvent` with `channel: "cli"` and outcome `Admitted` MUST be emitted

#### Scenario: Valid MP3 file transcribed and processed

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- AND a healthy Transcriber is available
- WHEN a user types `/audio /tmp/test.mp3` and the file exists and contains valid MP3 audio
- THEN the CLI MUST display `[Transcription]: "{transcribed text}"`
- AND the agent MUST process the transcription and produce a response

#### Scenario: Unsupported format rejected

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- WHEN a user types `/audio ~/music.flac` and the file exists but contains FLAC audio
- THEN the CLI MUST display the `MimeRejected` error message from REQ-11
- AND no transcription MUST be attempted

#### Scenario: File not found

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- WHEN a user types `/audio /nonexistent.ogg` and the file does not exist
- THEN the CLI MUST display `"File not found: /nonexistent.ogg"`

#### Scenario: No path argument — usage hint

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- WHEN a user types `/audio` with no file path
- THEN the CLI MUST display `"Usage: /audio <file-path>"`

#### Scenario: Audio globally disabled

- GIVEN `audio.enabled` is `false`
- WHEN a user types `/audio ~/recording.ogg`
- THEN the CLI MUST display `"Audio input is currently disabled."`
- AND no file I/O MUST occur

#### Scenario: CLI not in allowed_channels

- GIVEN audio is enabled with `allowed_channels: ["telegram"]` (cli NOT included)
- WHEN a user types `/audio ~/recording.ogg`
- THEN the CLI MUST display `"Audio input is not enabled for CLI."`
- AND no file I/O MUST occur

#### Scenario: Transcriber not available

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- AND no healthy Transcriber is registered
- WHEN a user types `/audio ~/recording.ogg` and the file is valid
- THEN the CLI MUST display the `TranscriberUnavailable` error message from REQ-11

#### Scenario: Tilde expansion works correctly

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- AND the user's home directory is `/home/user`
- WHEN a user types `/audio ~/recordings/note.ogg`
- THEN the CLI MUST resolve the path to `/home/user/recordings/note.ogg`
- AND process the file at that resolved path

#### Scenario: Relative path resolved correctly

- GIVEN audio is enabled with `allowed_channels` including `"cli"`
- AND the current working directory is `/home/user/projects`
- WHEN a user types `/audio ../recordings/note.ogg`
- THEN the CLI MUST resolve the path to `/home/user/recordings/note.ogg`
- AND process the file at that resolved path

### REQ-22: Gateway Audio Config Integration

#### Route Registration

When `audio.allowed_channels` includes `"gateway"`:

- The HTTP Gateway MUST register the `POST /web/chat/audio` route as active
- The gateway `AppState` MUST carry a reference to the runtime's `Transcriber`
  (`Option<Arc<dyn Transcriber>>`)
- The gateway MUST use the same `AudioConfig` limits (`max_audio_bytes`,
  `max_audio_duration_secs`, `transcription_timeout_secs`) as other channels

When `audio.allowed_channels` does NOT include `"gateway"`:

- The `POST /web/chat/audio` route SHOULD still be registered but MUST return HTTP 403 with
  an error message indicating audio is not enabled
- This allows clients to detect the endpoint exists but is disabled (rather than receiving 404)

#### Config Validation

`"gateway"` and `"cli"` MUST be recognized as valid Phase 2 channel names in the
`allowed_channels` validation.

Non-Phase-2 channel names (anything other than `"telegram"`, `"gateway"`, `"cli"`) SHOULD
produce a startup warning. These channels will be fail-closed at runtime since no audio
implementation exists for them.

The existing Phase 1 warning for non-`"telegram"` channels (REQ-7) MUST be updated to
recognize `"gateway"` and `"cli"` as valid channel names that do not trigger warnings.

#### Scenario: Gateway active with audio enabled

- GIVEN `audio.enabled: true` and `allowed_channels: ["telegram", "gateway"]`
- WHEN the runtime starts
- THEN the gateway MUST register `POST /web/chat/audio` as an active route
- AND `AppState` MUST contain a reference to the `Transcriber`
- AND the gateway MUST log that audio is enabled for the gateway channel

#### Scenario: Gateway returns 403 when not in allowed_channels

- GIVEN `audio.enabled: true` and `allowed_channels: ["telegram"]`
- WHEN a client sends `POST /web/chat/audio`
- THEN the response MUST be HTTP 403
- AND the response body MUST indicate audio is not enabled for this channel

#### Scenario: Config validation accepts gateway and cli channel names

- GIVEN `audio.allowed_channels: ["telegram", "gateway", "cli"]`
- WHEN the runtime starts
- THEN config validation MUST pass without warnings for these channel names

#### Scenario: Config validation warns on unknown channel names

- GIVEN `audio.allowed_channels: ["telegram", "gateway", "discord"]`
- WHEN the runtime starts
- THEN the runtime MUST log a warning for `"discord"` (not a recognized channel)
- AND the runtime MUST NOT log warnings for `"telegram"` or `"gateway"`
- AND startup MUST succeed (warning is non-fatal)

#### Scenario: Gateway shares AudioConfig limits with other channels

- GIVEN `audio.max_audio_bytes: 10485760` (10 MiB) and `allowed_channels: ["telegram", "gateway"]`
- WHEN a client uploads a 15 MiB file via `POST /web/chat/audio`
- THEN the upload MUST be rejected with `Oversize`
- AND the effective limit MUST be 10 MiB (same as Telegram would enforce)

### REQ-23: Gateway and CLI Audio Observability

All gateway audio ingestion attempts MUST emit an `AudioIngressEvent` (REQ-13) with
`channel: "gateway"`.

All CLI audio ingestion attempts MUST emit an `AudioIngressEvent` (REQ-13) with
`channel: "cli"`.

The events MUST use the same `AudioIngressOutcome` and `AudioIngressReason` taxonomy defined
in REQ-13. No new outcome or reason variants are required — the Phase 1 taxonomy (REQ-11,
REQ-13) is sufficient for Phase 2 channels.

#### Scenario: Gateway admitted event

- GIVEN a valid audio file is uploaded via `POST /web/chat/audio` and transcribed successfully
- WHEN the transcription is complete
- THEN an `AudioIngressEvent` MUST be emitted with:
    - `channel: "gateway"`
    - `outcome: Admitted`
    - `mime_type: Some({detected MIME})`
    - `byte_len: Some({file size})`
    - `transcription_duration_ms: Some({wall-clock ms})`

#### Scenario: Gateway rejected event

- GIVEN a FLAC file is uploaded via `POST /web/chat/audio`
- WHEN MIME validation rejects it
- THEN an `AudioIngressEvent` MUST be emitted with:
    - `channel: "gateway"`
    - `outcome: Rejected`
    - `reason: Some(MimeRejected)`

#### Scenario: CLI admitted event

- GIVEN a valid OGG file is processed via `/audio ~/recording.ogg`
- WHEN the transcription completes successfully
- THEN an `AudioIngressEvent` MUST be emitted with:
    - `channel: "cli"`
    - `outcome: Admitted`
    - `transcription_duration_ms: Some({wall-clock ms})`

#### Scenario: CLI rejected event — file too large

- GIVEN a user types `/audio ~/huge-recording.ogg` and the file exceeds `max_audio_bytes`
- WHEN size validation rejects it
- THEN an `AudioIngressEvent` MUST be emitted with:
    - `channel: "cli"`
    - `outcome: Rejected`
    - `reason: Some(Oversize)`
    - `byte_len: Some({file size})`

#### Scenario: CLI rejected event — audio disabled

- GIVEN `audio.enabled` is `false`
- WHEN a user types `/audio ~/recording.ogg`
- THEN an `AudioIngressEvent` MUST be emitted with:
    - `channel: "cli"`
    - `outcome: Rejected`
    - `reason: Some(Disabled)`

### REQ-24: Cross-Channel Concurrency

The transcription semaphore defined in REQ-12 MUST be shared across ALL channels. A Telegram
voice note, a gateway audio upload, and a CLI audio file MUST compete for the same semaphore
permits. This ensures the configured `max_concurrent_transcriptions` limit is respected as a
global system-wide ceiling, not a per-channel limit.

The semaphore MUST be a single instance (e.g., `Arc<Semaphore>`) shared via runtime state and
passed to all channel handlers.

#### Scenario: Cross-channel semaphore contention

- GIVEN `max_concurrent_transcriptions` is 1
- AND a Telegram voice note transcription is in progress (holding the semaphore)
- WHEN a gateway audio upload arrives simultaneously
- THEN the gateway request MUST wait for the Telegram transcription to complete
- AND then proceed with its own transcription
- AND both requests MUST eventually receive responses

#### Scenario: Three channels competing for semaphore

- GIVEN `max_concurrent_transcriptions` is 2
- AND two transcriptions are already in progress (one Telegram, one CLI)
- WHEN a gateway audio upload arrives
- THEN the gateway request MUST wait for one of the two active transcriptions to complete
- AND then proceed with its own transcription

#### Scenario: Semaphore timeout across channels

- GIVEN `max_concurrent_transcriptions` is 1
- AND a long-running Telegram transcription holds the semaphore
- WHEN a CLI audio request arrives and the turn timeout expires while waiting
- THEN the CLI audio MUST be rejected with `AudioRejectionReason::TranscriptionFailed`
- AND the user MUST receive the `TranscriptionFailed` error message
- AND the Telegram transcription MUST NOT be affected

#### Scenario: Semaphore released on failure

- GIVEN `max_concurrent_transcriptions` is 1
- AND a gateway transcription is in progress and fails (e.g., corrupted audio)
- WHEN the failure is processed
- THEN the semaphore permit MUST be released
- AND the next queued transcription (from any channel) MUST proceed immediately

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

- **Phase 1 Archive** (`openspec/changes/archive/2026-04-03-audio-input-support/`):
  Design, proposal, and tasks for Phase 1 (Telegram + core infrastructure).

- **Phase 2 Archive** (`openspec/changes/archive/2026-04-04-audio-input-phase2/`):
  Design, proposal, and tasks for Phase 2 (HTTP Gateway + CLI). Issue #412.
