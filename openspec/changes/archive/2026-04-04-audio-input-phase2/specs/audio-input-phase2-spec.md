# Delta for Audio Input — Phase 2: HTTP Gateway + CLI

**Parent spec**: `openspec/specs/audio-input/spec.md` (REQ-1 through REQ-18, final)
**Change**: `audio-input-phase2`
**Issue**: #412
**Date**: 2026-04-04
**Status**: draft

## Overview

This delta spec extends the Audio Input Specification (Phase 1) with six new requirements
(REQ-19 through REQ-24) covering two new entry points — HTTP Gateway and CLI — plus shared
staging infrastructure, config integration, observability, and cross-channel concurrency.

All Phase 1 requirements (REQ-1 through REQ-18) remain unchanged and in force. This delta
adds new requirements only; no existing requirements are modified or removed.

---

## ADDED Requirements

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

---

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

---

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

---

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

---

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

---

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

---

## MODIFIED Requirements

### REQ-7: Audio Configuration (amended)

(Previously: `allowed_channels` only recognized `"telegram"` as a Phase 1 channel name.
Non-`"telegram"` names produced a startup warning.)

The `audio.allowed_channels` field MUST now recognize the following as valid channel names
without producing startup warnings:

| Channel Name | Phase | Description                            |
|--------------|-------|----------------------------------------|
| `"telegram"` | 1     | Telegram Bot API channel               |
| `"gateway"`  | 2     | HTTP Gateway multipart upload endpoint |
| `"cli"`      | 2     | CLI `/audio` slash command             |

Any channel name not in the above list SHOULD produce a startup warning (unchanged behavior
for truly unknown channels).

All other REQ-7 validation rules remain unchanged.

#### Scenario: Gateway and CLI recognized as valid channels

- GIVEN `audio.allowed_channels: ["telegram", "gateway", "cli"]`
- WHEN the runtime starts
- THEN config validation MUST pass with no warnings

#### Scenario: Unknown channel still produces warning

- GIVEN `audio.allowed_channels: ["telegram", "gateway", "slack"]`
- WHEN the runtime starts
- THEN the runtime MUST log a warning for `"slack"` (not a recognized channel)
- AND MUST NOT log warnings for `"telegram"` or `"gateway"`

---

## REMOVED Requirements

(None — no existing requirements are removed in Phase 2.)

---

## Cross-References

- **Parent spec**: `openspec/specs/audio-input/spec.md` — REQ-1 through REQ-18 remain in
  force and unchanged (except the REQ-7 amendment above)
- **Phase 1 design**: `openspec/changes/archive/2026-04-03-audio-input-support/design.md`
- **Phase 2 proposal**: `openspec/changes/audio-input-phase2/proposal.md`
- **REQ-3** (MIME validation): Reused by `stage_audio_from_bytes()` in REQ-19
- **REQ-5** (File staging / RAII): Reused by `stage_audio_from_bytes()` in REQ-19
- **REQ-6** (Transcriber trait): Used by gateway handler (REQ-20) and CLI command (REQ-21)
- **REQ-11** (Error taxonomy): All 11 rejection reasons reused by Phase 2 channels; no new
  variants added
- **REQ-12** (Concurrency control): Extended to cross-channel scope in REQ-24
- **REQ-13** (Observability): Extended to gateway and CLI events in REQ-23
- **REQ-14** (Empty transcription guard): Applied in gateway handler (REQ-20) and CLI (REQ-21)
