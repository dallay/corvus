# Design: Runtime Image Normalization Pipeline

## Technical Approach

Formalize the existing `ContentPart::Image` → `StagedImage` → `ChatRequest.images` → provider
content blocks pipeline as the canonical runtime contract. This design layers on top of the
channel-ingestion spec (`openspec/specs/channel-image-ingestion/spec.md`, #266) and addresses three
gaps identified in the exploration: conversation-history image representation, `max_image_bytes`
config wiring, and error taxonomy stabilization.

The approach is deliberately conservative — codify what works, close the gaps, avoid new
abstractions. No new intermediate types are introduced; instead, we extend `ChatMessage` with an
optional metadata field for image context in history turns.

## Architecture Overview

The full runtime pipeline from channel receipt to provider dispatch and history storage:

```
    ┌─────────────────────── Channel Layer (spec: channel-image-ingestion) ───────────────────────┐
    │                                                                                              │
    │  Telegram/WhatsApp/Discord                                                                   │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  ChannelMessage { parts: [ContentPart::Image {...}] }                                        │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  process_channel_message()  ── gate checks ──┐                                               │
    │       │                      enabled?         │ reject → user message + ImageIngressEvent     │
    │       │                      channel allowed? │                                               │
    │       │                      vision route?    │                                               │
    │       │                      image count?     │                                               │
    │       │                      ◄────────────────┘                                               │
    └───────┼──────────────────────────────────────────────────────────────────────────────────────┘
            │
    ┌───────┼────────────────────── Runtime Layer (this spec) ─────────────────────────────────────┐
    │       ▼                                                                                      │
    │  stage_channel_images()                                                                      │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  fetch_and_stage_image() → stream_validate_and_stage()                                       │
    │       │   HTTP stream │ size check (max_image_bytes) │ MIME sniff │ SHA-256 │ temp file       │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  StagedImage { sha256, mime_type, byte_len, temp_path, transport_form, channel_origin }       │
    │       │                                              ▲                                        │
    │       │                                              │ RAII cleanup: StagedImageGuard         │
    │       ▼                                                                                      │
    │  ChatRequest { messages, tools, images: &[StagedImage] }                                     │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  Provider::chat() / chat_multimodal()                                                        │
    │       │   read temp_path → base64 → data:{mime};base64,{b64} → image_url content block       │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  ChatResponse { text, tool_calls }                                                           │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  handle_successful_response()                                                                │
    │       │   store user turn with image metadata → store assistant turn                          │
    │       │                                                                                      │
    │       ▼                                                                                      │
    │  ConversationHistory [ChatMessage { role, content, image_metadata? }]                         │
    │                                                                                              │
    └──────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Architecture Decisions

### ADR-1: Structured Enum over Markers

**Choice**: `ContentPart::Image` enum variant as the canonical inbound representation. No marker
syntax like `[IMAGE:<source>]` for inbound/runtime image handling.

**Alternatives considered**:
- **Marker syntax** (`[IMAGE:telegram:file_123]`) — Text-based markers embedded in `content` string.
  Requires parsing, is fragile to user input collision, and cannot carry typed metadata (MIME,
  byte count, channel handle) without inventing an encoding scheme.
- **Hybrid** — Structured enum for runtime, markers for serialization. Adds a translation layer
  with no benefit; the enum is already serializable.

**Rationale**: The structured enum is already implemented and battle-tested across three channels.
It carries typed metadata (`declared_mime`, `declared_bytes`, `channel_handle`) that a marker string
cannot express without fragile encoding. Markers exist only in the *outbound* delivery layer
(`channel_delivery_instructions()` at `channels/mod.rs:151-158` for Telegram media attachments) —
this is a different concern (instructing the LLM to emit attachments) and does not apply to inbound
image representation. The existing pattern aligns with the Rust type system and catches errors at
compile time rather than parse time.

**Cross-reference**: channel-ingestion spec REQ-2 step 1 ("Produce a `ContentPart::Image`").

### ADR-2: Conversation History Image Representation

**Choice**: Compact metadata struct + model-generated description, stored as an optional field on
`ChatMessage`.

**Alternatives considered**:
1. **Store full image bytes in history** — Unbounded memory growth. A 10 MiB image × 50-turn history
   = 500 MiB per conversation. Rejected.
2. **Placeholder text** (e.g., `[An image was shared: photo.jpg, 2.4 MB JPEG]`) — Loses structured
   queryability. The model sees it but operators cannot filter/aggregate image metadata from history.
   Partially viable as a fallback but insufficient alone.
3. **Image description from model only** — Relies on the LLM's vision response to describe the
   image. Good for model context continuity but not available until after the provider responds.
   Cannot be computed at ingestion time.
4. **Compact metadata + model-generated description** (chosen) — At ingestion time, store
   `ImageHistoryMeta { mime, sha256, byte_len, channel_origin, caption }`. After the provider
   responds, extract a brief description from the model's response and attach it to the history
   entry. On subsequent turns, inject a synthetic text block:
   `[Prior image: {mime}, {byte_len} bytes, sha256:{sha256_prefix}. Description: {description}]`.

**Rationale**: Option 4 gives the model enough context to reason about prior images without storing
bytes. The compact metadata is cheap (~200 bytes per image turn), provides operator-queryable
fields, and the model-generated description preserves semantic context across turns. If the model
does not produce a usable description (e.g., tool-call-only response), the metadata alone is still
sufficient for the model to know *that* an image was shared and its basic properties.

**Data model** (see Interfaces section for full definition):

```rust
/// Compact metadata for an image that appeared in a prior conversation turn.
/// Stored in history instead of raw bytes to bound memory usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageHistoryMeta {
    /// MIME type string (e.g. "image/jpeg").
    pub mime: String,
    /// SHA-256 hex digest of the original image bytes.
    pub sha256: String,
    /// Original image size in bytes.
    pub byte_len: u64,
    /// Channel that originated the image.
    pub channel_origin: String,
    /// User-provided caption, if any.
    pub caption: Option<String>,
    /// Model-generated description of image content (populated post-response).
    pub description: Option<String>,
}
```

### ADR-3: `max_image_bytes` Config Wiring

**Choice**: Thread the operator override from `MultimodalConfig.max_image_bytes` to every
`validate_size()` and `stream_validate_and_stage()` call site, falling back to the `MAX_IMAGE_BYTES`
constant when `None`.

**Alternatives considered**:
- **Global mutable state** — Set a global once at startup. Violates Rust safety patterns and makes
  testing harder. Rejected.
- **Environment variable** — `CORVUS_MAX_IMAGE_BYTES` env override. Adds a second config surface
  that can conflict with TOML. Rejected for MVP; can be added later if needed.

**Rationale**: The config field already exists (`config/schema.rs:293`). The `validate_size()`
function already accepts `max_bytes: u64` as a parameter. The only missing piece is passing
`config.multimodal.max_image_bytes.unwrap_or(MAX_IMAGE_BYTES)` at the call sites in
`stream_validate_and_stage()` and `process_channel_message()`.

**Injection point**: `stream_validate_and_stage()` currently hardcodes `MAX_IMAGE_BYTES` at lines
192 and 205 of `channels/media.rs`. Add a `max_bytes: u64` parameter, with callers passing the
resolved value. Validate at config load: reject `max_image_bytes` values ≤ 0 or > 50 MiB
(hard ceiling to prevent operator misconfiguration).

### ADR-4: Error Taxonomy Stability

**Choice**: The 9 `ImageRejectionReason` variants and their snake_case `Display` strings are the
stable public contract. No numeric error codes for MVP.

**Alternatives considered**:
- **Numeric error codes** (`ERR_IMG_001` through `ERR_IMG_009`) — Adds a mapping layer with no
  clear consumer. Operator dashboards and observability already use the snake_case strings. Numeric
  codes are harder to remember and require a lookup table. Rejected for MVP.
- **Structured error type with code + message + details** — Over-engineered for the current 9
  variants. Can be introduced when the error taxonomy grows beyond ~20 variants.

**Rationale**: The snake_case Display output is already used by `ImageIngressEvent` for
observability and is stable across the three implemented channels. Formalizing these strings as the
contract (with a spec requirement that new variants must not rename existing strings) provides the
stability guarantee without adding complexity.

**The 9 stable variants**:

| Variant               | Display string           | Trigger                                    |
|-----------------------|--------------------------|--------------------------------------------|
| `Disabled`            | `disabled`               | `multimodal.enabled = false`               |
| `ChannelNotAllowed`   | `channel_not_allowed`    | Channel not in `allowed_channels`          |
| `MissingVisionRoute`  | `missing_vision_route`   | No `vision_model_hint` or hint not found   |
| `RouteNotImageCapable`| `route_not_image_capable`| Route exists but `allow_image_input=false` |
| `TooManyImages`       | `too_many_images`        | Image count > `MAX_IMAGES_PER_TURN`        |
| `FetchFailed`         | `fetch_failed`           | HTTP error, stream error, or temp write    |
| `MimeRejected`        | `mime_rejected`          | Magic-byte sniff rejects format            |
| `Oversize`            | `oversize`               | Bytes exceed `max_image_bytes` limit       |
| `ProviderError`       | `provider_error`         | Provider rejects or fails on image turn    |

### ADR-5: Provider-Agnostic Handoff via StagedImage

**Choice**: `StagedImage` is the boundary type between the runtime normalization pipeline and
provider adapters. Provider adapters translate `StagedImage` to format-specific payloads
independently.

**Alternatives considered**:
- **Pre-encoded payloads** — Runtime encodes to base64/data-URL before passing to provider. Locks
  all providers into one encoding. Rejected — Anthropic uses a different content block format than
  OpenAI.
- **`NormalizedImageMessage` intermediate type** — Exploration Approach 2. Adds a type between
  `StagedImage` and provider dispatch. Premature — `StagedImage` already carries everything a
  provider adapter needs (`temp_path` for bytes, `mime_type` for encoding, `sha256` for
  deduplication).

**Rationale**: `StagedImage` provides: (1) validated bytes on disk (`temp_path`), (2) verified MIME
(`mime_type: AllowedImageMime`), (3) content hash (`sha256`), (4) size (`byte_len`), (5) transport
hint (`transport_form`). Each provider adapter reads `temp_path`, encodes as needed (base64
data-URL for OpenAI-compatible, base64 `source.data` for Anthropic, GCS upload for Gemini), and
constructs its native content block format. The `ImageTransportForm` enum is the extensibility
point — new variants (e.g., `PresignedUrl`) can be added without changing `StagedImage`.

**Current adapter**: `CompatibleProvider::chat_multimodal()` (`providers/compatible.rs:494-550`)
reads bytes, base64-encodes, builds `image_url` content blocks with `data:{mime};base64,{b64}`.
Future adapters (#268) implement the same pattern with their native format.

## Sequence Diagram

Full pipeline from channel message receipt through provider dispatch and history storage:

```
Channel         process_channel_     stage_channel_     stream_validate_    Provider        handle_successful_
(Telegram/      message()            images()           _and_stage()        ::chat()        response()
 WA/Discord)
    │                │                    │                   │                │                │
    │  ChannelMsg    │                    │                   │                │                │
    │───────────────>│                    │                   │                │                │
    │                │                    │                   │                │                │
    │                │── gate: enabled? ──┤                   │                │                │
    │                │── gate: channel? ──┤                   │                │                │
    │                │── gate: route?  ──┤                   │                │                │
    │                │── gate: count?  ──┤                   │                │                │
    │                │   (any fail →      │                   │                │                │
    │                │    reject + event) │                   │                │                │
    │                │                    │                   │                │                │
    │                │── all gates pass ─>│                   │                │                │
    │                │                    │                   │                │                │
    │                │                    │── per image ─────>│                │                │
    │                │                    │                   │── HTTP fetch   │                │
    │                │                    │                   │── Content-Len  │                │
    │                │                    │                   │── stream+size  │                │
    │                │                    │                   │── MIME sniff   │                │
    │                │                    │                   │── SHA-256      │                │
    │                │                    │                   │── temp file    │                │
    │                │                    │<── StagedImage ───│                │                │
    │                │                    │                   │                │                │
    │                │<── Vec<Staged> ────│                   │                │                │
    │                │                    │                   │                │                │
    │                │── StagedImageGuard (RAII cleanup) ──────────────────────│                │
    │                │                                                        │                │
    │                │── ChatRequest { messages, images: &[StagedImage] } ───>│                │
    │                │                                                        │                │
    │                │                                        read temp_path ─│                │
    │                │                                        base64 encode ──│                │
    │                │                                        content blocks ─│                │
    │                │                                        HTTP POST ──────│                │
    │                │                                                        │                │
    │                │<──────────── ChatResponse { text, tool_calls } ────────│                │
    │                │                                                        │                │
    │                │── (guard drops: temp files cleaned up) ────────────────>│                │
    │                │                                                                         │
    │                │── store user turn + ImageHistoryMeta ───────────────────────────────────>│
    │                │── store assistant turn ─────────────────────────────────────────────────>│
    │                │── extract description from response, update meta ───────────────────────>│
    │                │                                                                         │
    │<── reply ──────│                                                                         │
```

## Data Flow

Conversation history for an image turn after this change:

```
    Turn N (image turn):
    ┌──────────────────────────────────────────────────────────┐
    │ ChatMessage {                                            │
    │   role: "user",                                          │
    │   content: "What is in this photo?",                     │
    │   image_metadata: Some([ImageHistoryMeta {               │
    │     mime: "image/jpeg",                                  │
    │     sha256: "a1b2c3d4...",                               │
    │     byte_len: 245_760,                                   │
    │     channel_origin: "telegram",                          │
    │     caption: Some("My garden"),                          │
    │     description: Some("A photo of a garden with ..."),   │
    │   }]),                                                   │
    │ }                                                        │
    └──────────────────────────────────────────────────────────┘

    Turn N+1 (text follow-up):
    ┌──────────────────────────────────────────────────────────┐
    │ ChatMessage {                                            │
    │   role: "user",                                          │
    │   content: "Can you identify the flowers?",              │
    │ }                                                        │
    └──────────────────────────────────────────────────────────┘
    
    When building ChatRequest for Turn N+1, the runtime injects
    a synthetic context block into the message history:
    
    "[Prior image: image/jpeg, 245760 bytes, sha256:a1b2c3d4.
     Description: A photo of a garden with roses and tulips.]"
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/channels/media.rs` | Modify | Add `ImageHistoryMeta` struct. Add `max_bytes` parameter to `stream_validate_and_stage()`. Add config ceiling validation constant `MAX_IMAGE_BYTES_CEILING = 50 MiB`. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | In `handle_successful_response()` (~line 1210): build `ImageHistoryMeta` from staged images, store with user turn. Add helper to extract description from assistant response. Thread `max_image_bytes` config to staging call sites. |
| `clients/agent-runtime/src/providers/traits.rs` | Modify | Add optional `image_metadata: Option<Vec<ImageHistoryMeta>>` field to `ChatMessage`. Update `ChatMessage::user()` to default `image_metadata: None`. Add `ChatMessage::user_with_images()` constructor. |
| `clients/agent-runtime/src/config/schema.rs` | Unchanged | `MultimodalConfig.max_image_bytes` already exists. No schema change. |
| `clients/agent-runtime/src/providers/compatible.rs` | Unchanged | `chat_multimodal()` reads `StagedImage.temp_path`. No change needed. |
| `openspec/specs/channel-image-ingestion/spec.md` | Unchanged | Cross-referenced, not modified. |
| `openspec/specs/runtime-image-normalization/spec.md` | Create | Runtime-layer spec (produced by `sdd-spec` phase). |

## Interfaces / Contracts

### ImageHistoryMeta (new struct in `channels/media.rs`)

```rust
use serde::{Deserialize, Serialize};

/// Compact metadata for an image in a prior conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageHistoryMeta {
    pub mime: String,
    pub sha256: String,
    pub byte_len: u64,
    pub channel_origin: String,
    pub caption: Option<String>,
    pub description: Option<String>,
}

impl ImageHistoryMeta {
    /// Build from a StagedImage at ingestion time (description populated later).
    pub fn from_staged(staged: &StagedImage, caption: Option<String>) -> Self {
        Self {
            mime: staged.mime_type.as_str().to_string(),
            sha256: staged.sha256.clone(),
            byte_len: staged.byte_len,
            channel_origin: staged.channel_origin.clone(),
            caption,
            description: None,
        }
    }

    /// Render as a synthetic context string for history injection.
    pub fn to_context_string(&self) -> String {
        let mut s = format!(
            "[Prior image: {}, {} bytes, sha256:{}",
            self.mime,
            self.byte_len,
            &self.sha256[..16.min(self.sha256.len())]
        );
        if let Some(desc) = &self.description {
            s.push_str(&format!(". Description: {desc}"));
        }
        s.push(']');
        s
    }
}
```

### Extended ChatMessage (modification in `providers/traits.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Image metadata for history turns (None for text-only or non-history messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_metadata: Option<Vec<ImageHistoryMeta>>,
}

impl ChatMessage {
    pub fn user_with_images(
        content: impl Into<String>,
        metadata: Vec<ImageHistoryMeta>,
    ) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            image_metadata: if metadata.is_empty() {
                None
            } else {
                Some(metadata)
            },
        }
    }
}
```

### Modified `stream_validate_and_stage` signature

```rust
pub async fn stream_validate_and_stage(
    response: reqwest::Response,
    declared_mime: Option<&str>,
    channel_prefix: &str,
    sanitize_url: &str,
    max_bytes: u64,  // NEW: caller passes config.multimodal.max_image_bytes.unwrap_or(MAX_IMAGE_BYTES)
) -> Result<StagedImage, ImageRejectionReason> {
    // ... existing logic, replacing hardcoded MAX_IMAGE_BYTES with max_bytes ...
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `ImageHistoryMeta::from_staged()` produces correct fields | Construct `StagedImage`, assert all fields map correctly |
| Unit | `ImageHistoryMeta::to_context_string()` format | Assert output matches expected format with and without description |
| Unit | `ChatMessage::user_with_images()` constructor | Assert `image_metadata` is `Some` for non-empty, `None` for empty |
| Unit | `stream_validate_and_stage()` respects custom `max_bytes` | Mock response with body between `MAX_IMAGE_BYTES` and custom limit; assert pass/fail |
| Unit | Config validation rejects `max_image_bytes > 50 MiB` | Load config with 200 MiB, assert error |
| Integration | History stores image metadata across turns | Send image turn, assert next history retrieval contains `ImageHistoryMeta` |
| Integration | Follow-up turn sees prior image context | Send image turn, then text follow-up; assert model receives synthetic context block |
| Integration | `max_image_bytes` override is effective end-to-end | Set config override to 1 MiB, send 5 MiB image, assert `Oversize` rejection |
| Regression | Existing `channel-image-ingestion` scenarios still pass | Run full test suite; no existing behavior degrades |

## Migration / Rollout

No migration required. All changes are additive:

- `ChatMessage.image_metadata` is `Option` with `#[serde(default)]` — existing serialized history
  deserializes with `None`, preserving backward compatibility.
- `stream_validate_and_stage()` gains a parameter, but all callers are internal to the crate.
- No database schema changes — history is in-memory `HashMap<String, Vec<ChatMessage>>`.

## Open Questions

- [x] Should `ImageHistoryMeta` live in `channels/media.rs` or a new `channels/history.rs`? →
  Decision: `channels/media.rs` — it's a small struct closely related to `StagedImage`.
- [ ] Should the model-generated description be extracted heuristically from the response, or should
  the runtime append an explicit instruction ("Briefly describe the image") to the system prompt?
  Recommendation: Heuristic first (first sentence of response when image is present), with a
  follow-up to add explicit description extraction if heuristic proves insufficient.
- [ ] Should `ImageHistoryMeta` include a timestamp for TTL-based expiration of image context in
  long conversations? Deferred — can be added as an optional field later without breaking changes.
