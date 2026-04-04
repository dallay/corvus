# Exploration: Audio Input Support for Agents

**Change**: `audio-input-support`
**Issue**: #246 / DALLAY-150
**Date**: 2026-04-03

---

## 1. Current Architecture Findings

### 1.1 ContentPart Enum & ChannelMessage

**File**: `src/channels/traits.rs` (lines 4–78)

The `ContentPart` enum currently has two variants:

```rust
pub enum ContentPart {
    Text { text: String },
    Image {
        channel_handle: String,
        source_channel: String,
        declared_mime: Option<String>,
        caption_text: Option<String>,
        file_name: Option<String>,
        declared_bytes: Option<u64>,
    },
}
```

`ChannelMessage` carries `parts: Vec<ContentPart>` as the canonical multimodal payload. Helper methods exist: `text_projection()`, `has_image_parts()`, `image_parts()`. These are image-specific and will need audio counterparts.

**Key insight**: Audio needs a new `ContentPart::Audio { .. }` variant. Unlike `Image`, audio will NOT be forwarded to the provider — it's transcribed to text pre-loop.

### 1.2 Media Module (Shared Validation)

**File**: `src/channels/media.rs` (851 lines)

Contains the image pipeline infrastructure:
- `AllowedImageMime` — enum for MIME validation via magic-byte sniffing
- `ImageRejectionReason` — 10-variant error enum with `thiserror`
- `StagedImage` — validated image ready for provider dispatch, with RAII cleanup
- `ImageHistoryMeta` — compact metadata stored in conversation history
- `validate_mime()` — magic-byte sniffing (JPEG, PNG, WebP)
- `validate_size()` — byte limit validation
- `stream_validate_and_stage()` — shared HTTP response → staged file pipeline

**Reuse opportunity**: The `stream_validate_and_stage()` pattern is directly applicable for audio. We need an `AllowedAudioMime`, `AudioRejectionReason`, `StagedAudio`, and `AudioHistoryMeta` following the same patterns.

### 1.3 StagedImageGuard (RAII Cleanup)

**File**: `src/channels/mod.rs` (lines 127–135)

```rust
struct StagedImageGuard(Vec<media::StagedImage>);
impl Drop for StagedImageGuard {
    fn drop(&mut self) {
        for img in &self.0 { img.cleanup(); }
    }
}
```

This pattern MUST be replicated for audio files. A `StagedAudioGuard` is needed.

### 1.4 Image Pipeline Flow (End-to-End)

**File**: `src/channels/mod.rs` — `process_channel_message()` (line 604)

The full flow is:

```text
Channel.listen() → parse message → build ContentPart::Image
  → process_channel_message()
    → extract_user_text()           // text projection
    → enrich_with_memory()          // memory context
    → gate_multimodal_config()      // check enabled, allowed channels, vision route
    → gate_and_stage_images()       // count validation, fetch+stage per channel
    → StagedImageGuard              // RAII cleanup
    → build_history()               // inject prior image metadata
    → run_unified_channel_tool_loop()
      → provider.chat(ChatRequest { images: &staged_guard.0, .. })
    → handle_successful_response()  // store ImageHistoryMeta in history
```

**Critical difference for audio**: Audio does NOT go to the provider. The pipeline must:
1. Parse `ContentPart::Audio` from channel
2. Gate audio config (enabled, allowed channels)
3. Fetch + stage audio file
4. **Transcribe locally** → produce text
5. Replace/inject transcription as `ContentPart::Text` in the message
6. Continue normal text-only processing (no `images` in `ChatRequest`)
7. Store `AudioHistoryMeta` with transcription text

### 1.5 Provider Trait & ChatRequest

**File**: `src/providers/traits.rs` (lines 93–101)

```rust
pub struct ChatRequest<'a> {
    pub messages: &'a [ChatMessage],
    pub tools: Option<&'a [ToolSpec]>,
    pub images: &'a [StagedImage],
}
```

Audio does NOT need a field here — transcription happens before provider dispatch. The transcribed text is injected as a normal user message.

### 1.6 Observability Pattern

**File**: `src/observability/traits.rs`

Image ingress has a full observability contract:
- `ImageIngressOutcome` (Admitted, Rejected, ProviderSent, ProviderError)
- `ImageIngressReason` (Disabled, ChannelNotAllowed, MimeRejected, Oversize, etc.)
- `ImageIngressEvent` struct
- `Observer::on_image_ingress()` method

We need an equivalent `AudioIngressOutcome`, `AudioIngressReason`, `AudioIngressEvent`, and `Observer::on_audio_ingress()`.

### 1.7 Config Structure

**File**: `src/config/schema.rs` (lines 280–294)

```rust
pub struct MultimodalConfig {
    pub enabled: bool,
    pub allowed_channels: Vec<String>,
    pub vision_model_hint: Option<String>,
    pub max_image_bytes: Option<u64>,
}
```

Audio needs a separate config section (e.g., `AudioConfig` or extending `MultimodalConfig`):
- `audio_enabled: bool`
- `audio_allowed_channels: Vec<String>`
- `max_audio_bytes: Option<u64>` (default 25 MiB)
- `max_audio_duration_secs: Option<u64>` (default 600)
- `transcription_model: Option<String>` (whisper model name, default "base")
- `transcription_language: Option<String>` (default "es")

---

## 2. Multimodal Pipeline Analysis

### What Can Be Reused

| Component | Reuse Level | Notes |
|-----------|------------|-------|
| `ContentPart` enum | Extend | Add `Audio` variant |
| `ChannelMessage` helpers | Mirror | Add `has_audio_parts()`, `audio_parts()` |
| `media.rs` validation pattern | Mirror | New `AllowedAudioMime`, `validate_audio_mime()` |
| `stream_validate_and_stage()` | Fork/adapt | Audio needs duration check, different MIME sniffing |
| `StagedImageGuard` RAII | Mirror | `StagedAudioGuard` |
| Config gating pattern | Mirror | `gate_audio_config()` |
| Observability events | Mirror | `AudioIngressEvent` |
| Telegram `getFile` download | Reuse directly | Same API for voice/audio files |
| History metadata pattern | Mirror | `AudioHistoryMeta` |

### What Needs New Work

| Component | Reason |
|-----------|--------|
| `Transcriber` trait | New extension point for STT engines |
| Audio MIME sniffing | Different magic bytes (OGG, MP3, WAV, M4A) |
| Duration validation | Images don't have duration; audio needs ffprobe or header parsing |
| Transcription pipeline stage | New: between staging and agent loop |
| Audio-to-text injection | New: replace Audio part with Text part containing transcription |
| Whisper.cpp integration | New Rust binding or CLI wrapper |
| Model management | Download, cache, and locate whisper models |

---

## 3. Channel-Specific Findings

### 3.1 Telegram

**File**: `src/channels/telegram.rs` (3241 lines)

**Current state**:
- `build_telegram_content_parts()` (line 21) parses `photo` and `document` (image MIME only)
- Voice notes (`voice` field) and audio files (`audio` field) are **completely ignored** — messages with only voice/audio return empty parts → `parse_update_message()` returns `None` (line 886–888)
- Telegram already has `send_voice()` and `send_audio()` for outbound (lines 1255–1338)
- `fetch_and_stage_image()` (line 1566) uses `getFile` → download URL → stream validate — **this pattern works for audio too**, just different MIME validation
- `TelegramAttachmentKind` enum (line 177) already has `Audio` and `Voice` variants

**What needs to change**:
- Add voice/audio parsing to `build_telegram_content_parts()`: check `message.voice` and `message.audio` fields
- Telegram voice notes are always OGG/Opus; audio files have `mime_type` field
- Add `fetch_and_stage_audio()` method (similar to `fetch_and_stage_image()`)

**Telegram API reference**:
- Voice note: `{ "voice": { "file_id": "...", "file_unique_id": "...", "duration": 5, "mime_type": "audio/ogg", "file_size": 12345 } }`
- Audio file: `{ "audio": { "file_id": "...", "duration": 120, "mime_type": "audio/mpeg", "file_size": 500000, "title": "...", "performer": "..." } }`

### 3.2 HTTP Gateway

**File**: `src/gateway/mod.rs` (6016 lines)

**Current state**:
- `POST /web/chat/stream` accepts JSON body (`WebhookJsonBody`) with a `message` string field
- No multipart support — all payloads are JSON
- The gateway dispatches via `webhook_dispatch::execute()` which takes a text message
- Body limit is 64KB (line 7) — far too small for audio

**What needs to change**:
- Add a new endpoint: `POST /web/chat/audio` accepting `multipart/form-data`
- Fields: `audio` (file), `session_id` (optional), `language` (optional)
- Increase body limit for this endpoint only (25 MiB)
- The endpoint must: validate file, stage, transcribe, then dispatch text through existing path
- Return transcription + agent response via SSE or JSON

### 3.3 CLI

**File**: `src/channels/cli.rs` (136 lines)

**Current state**:
- Reads lines from stdin, creates text-only `ChannelMessage`
- No file path handling at all

**What needs to change**:
- Detect a special prefix (e.g., `/audio <path>` or `@audio:<path>`)
- Read the local file, validate format/size/duration
- Stage as `StagedAudio`, transcribe, inject text
- Minimal change — CLI is the simplest entry point

---

## 4. Transcription Engine Evaluation

### NFR1: No External Third-Party Services

All processing MUST be local. This eliminates cloud APIs (OpenAI Whisper API, Google STT, AWS Transcribe).

### Candidates

| Engine | Type | Spanish Quality | Binary Size Impact | Memory | Startup | Maturity |
|--------|------|----------------|-------------------|--------|---------|----------|
| **whisper.cpp (CLI)** | External binary | Excellent (multilingual) | None (separate binary) | 500MB–1.5GB (model) | Fast (pre-loaded) | Very mature |
| **whisper-rs** | Rust bindings to whisper.cpp | Excellent | +5–10MB (C lib) | 500MB–1.5GB (model) | ~2s model load | Mature |
| **candle-whisper** | Pure Rust via candle ML | Good | +15–20MB | 500MB–1.5GB (model) | Slower (no GGML optimization) | Experimental |
| **vosk-rs** | Rust bindings to Vosk | Good for Spanish | +20MB (C++ lib) | 50–300MB (model) | Fast | Mature |

### Recommendation: whisper.cpp CLI (Phase 1) → whisper-rs (Phase 2)

**Phase 1 — whisper.cpp CLI wrapper** (like robot-kit already does):
- The robot-kit crate (`crates/robot-kit/src/listen.rs`, line 70) already uses whisper.cpp as an external binary
- Zero additional Rust dependencies — no binary size impact
- Operator installs whisper.cpp + model separately (documented in setup)
- Proven pattern already in the codebase
- Best Spanish quality via multilingual models

**Phase 2 — whisper-rs integration** (optional future):
- Embed whisper.cpp as a Rust library for zero external dependencies
- Feature-gated (`--features audio-transcription`) to avoid binary bloat for users who don't need it
- Adds ~5–10MB to binary but removes external dependency

**Model strategy**:
- Default model: `base` (~150MB, good speed/quality for Spanish)
- Models stored in `~/.corvus/models/whisper/`
- `corvus doctor` checks for model availability
- Config: `transcription_model = "base"` (overridable to "small", "medium", "large-v3")

---

## 5. Proposed Extension Points

### 5.1 Transcriber Trait

New file: `src/transcription/traits.rs`

```rust
#[async_trait]
pub trait Transcriber: Send + Sync {
    /// Human-readable name of the transcription engine.
    fn name(&self) -> &str;

    /// Transcribe audio file to text.
    async fn transcribe(&self, audio: &StagedAudio) -> Result<TranscriptionResult>;

    /// Whether the engine is ready (model loaded, binary available).
    async fn health_check(&self) -> bool;
}

pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_secs: f64,
    pub confidence: Option<f64>,
}
```

### 5.2 ContentPart::Audio Variant

```rust
pub enum ContentPart {
    Text { text: String },
    Image { /* existing */ },
    Audio {
        channel_handle: String,
        source_channel: String,
        declared_mime: Option<String>,
        caption_text: Option<String>,
        file_name: Option<String>,
        declared_bytes: Option<u64>,
        declared_duration_secs: Option<u64>,
    },
}
```

### 5.3 Audio Media Types

In `src/channels/media.rs` (or new `src/channels/audio_media.rs`):

```rust
pub enum AllowedAudioMime {
    OggOpus,  // voice notes (Telegram)
    Mp3,
    Wav,
    M4a,
}

pub enum AudioRejectionReason {
    Disabled,
    ChannelNotAllowed,
    FetchFailed,
    MimeRejected,
    Oversize,
    TooLong,           // duration > 10 min
    Corrupted,
    TranscriptionFailed,
    NoSpeechDetected,
    SystemError,
}

pub struct StagedAudio {
    pub sha256: String,
    pub mime_type: AllowedAudioMime,
    pub byte_len: u64,
    pub duration_secs: Option<f64>,
    pub temp_path: PathBuf,
    pub channel_origin: String,
}

pub struct AudioHistoryMeta {
    pub mime: String,
    pub sha256: String,
    pub byte_len: u64,
    pub duration_secs: Option<f64>,
    pub channel_origin: String,
    pub transcription: String,
    pub caption: Option<String>,
}
```

### 5.4 Pipeline Insertion Point

In `process_channel_message()` (`src/channels/mod.rs`, line 604), audio processing inserts **between** `extract_user_text()` and `enrich_with_memory()`:

```text
extract_user_text()
→ NEW: gate_audio_config()         // check enabled, allowed channels
→ NEW: gate_and_stage_audio()      // fetch, validate MIME/size/duration
→ NEW: transcribe_audio()          // Transcriber::transcribe()
→ NEW: inject_transcription()      // replace Audio parts with Text, store metadata
→ enrich_with_memory()             // existing flow continues with text
```

### 5.5 Module Structure

```text
src/
├── transcription/
│   ├── mod.rs              // module exports
│   ├── traits.rs           // Transcriber trait
│   └── whisper_cli.rs      // whisper.cpp CLI wrapper
├── channels/
│   ├── media.rs            // existing image media (unchanged)
│   ├── audio_media.rs      // NEW: audio validation, staging, MIME sniffing
│   ├── traits.rs           // extend ContentPart with Audio variant
│   └── mod.rs              // add audio pipeline stages
```

---

## 6. Risks and Open Questions

### Risks

1. **Binary size (Medium)**: whisper-rs would add ~5–10MB. Mitigated by using CLI wrapper in Phase 1 and feature-gating in Phase 2.

2. **Model distribution (Medium)**: Whisper models are 150MB–3GB. Operator must download separately. Need clear `corvus doctor` check and setup docs.

3. **Memory footprint (Medium)**: Whisper model inference uses 500MB–1.5GB RAM. On constrained devices (Raspberry Pi), this could be an issue. Consider model size recommendations per platform.

4. **OGG/Opus duration parsing (Low)**: Getting duration from OGG headers is non-trivial without a dependency. Options: (a) trust Telegram's `duration` field, (b) let whisper.cpp report it, (c) add an `ogg` crate.

5. **Concurrent transcription (Medium)**: Whisper is CPU-intensive. Multiple simultaneous audio messages could overwhelm the system. Need a transcription semaphore or queue.

6. **Audio format conversion (Low)**: whisper.cpp natively supports WAV 16kHz mono. OGG/Opus and M4A may need conversion. `ffmpeg` or `sox` could be required as an external dependency.

### Open Questions

1. **Should audio config be a separate TOML section or nested under `[multimodal]`?**
   - Recommendation: Separate `[audio]` section — audio has different concerns (transcription model, language, duration limits) vs images (vision route, provider routing).

2. **Should the Transcriber trait live under `src/transcription/` or `src/providers/`?**
   - Recommendation: New `src/transcription/` module — it's not a provider (doesn't do LLM inference), it's a preprocessing stage.

3. **What's the error UX for "whisper not installed"?**
   - Recommendation: `corvus doctor` warns. At runtime, audio messages get a friendly "Audio transcription is not available on this agent. Please send text instead." reply.

4. **Should transcription be synchronous (block the message) or async (reply when ready)?**
   - Recommendation: Synchronous within the message processing timeout (300s). Transcription of a 10-min audio takes ~30s on decent hardware with base model.

5. **Should we support audio in the config `allowed_channels` separately from images?**
   - Yes. An operator might enable image input for Telegram but not audio (or vice versa).

---

## 7. Recommendations

### Approach: Incremental Extension of Existing Multimodal Pipeline

1. **Add `ContentPart::Audio` variant** — extends the existing enum, follows the image precedent
2. **Create `src/channels/audio_media.rs`** — audio-specific validation, MIME sniffing, staging (mirrors `media.rs`)
3. **Create `src/transcription/` module** — `Transcriber` trait + whisper.cpp CLI implementation
4. **Extend Telegram channel** — parse `voice` and `audio` fields in `build_telegram_content_parts()`
5. **Add audio pipeline stages in `mod.rs`** — gate, stage, transcribe, inject text
6. **Add `[audio]` config section** — separate from multimodal image config
7. **Add audio observability** — `AudioIngressEvent`, `on_audio_ingress()`
8. **HTTP Gateway** — new `POST /web/chat/audio` multipart endpoint
9. **CLI** — `/audio <path>` command for local file transcription


### Phase 1 Scope (MVP)

- Telegram voice notes + audio files
- whisper.cpp CLI wrapper (proven pattern from robot-kit)
- `[audio]` config with enabled/allowed_channels/max_bytes/max_duration
- Audio observability events
- 6 error types from PRD


### Phase 2 (Follow-up)

- HTTP Gateway multipart endpoint
- CLI `/audio` command
- whisper-rs embedded (feature-gated)
- Model auto-download


### Effort Estimate

- Phase 1: **Medium-High** (~15–20 tasks across infrastructure, implementation, testing)
- Phase 2: **Medium** (~8–12 additional tasks)

---

## Ready for Proposal

**Yes** — the codebase investigation is complete. The image multimodal pipeline provides a clear precedent for all audio pipeline components. The transcription engine choice (whisper.cpp CLI) is proven in the robot-kit crate. All extension points are identified with exact file paths and line numbers.

The orchestrator should proceed to `sdd-propose` to formalize scope, approach, and rollback plan.
