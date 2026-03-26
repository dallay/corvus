## Exploration: Runtime Image Normalization Pipeline

**Change**: 2026-03-26-runtime-image-normalization-pipeline
**Issue**: #267 — Define canonical runtime pipeline for multimodal image normalization and safety
**Date**: 2026-03-26

### Current State

The Corvus runtime already has a substantial multimodal image ingestion pipeline implemented across
the channel and provider layers. Here is what exists today:

**Canonical representation** — Images are represented as `ContentPart::Image` variants within
`ChannelMessage.parts` (`channels/traits.rs:5-17`). Each image part carries: `channel_handle`
(opaque platform ID), `source_channel`, `declared_mime`, `caption_text`, `file_name`, and
`declared_bytes`. There is no marker syntax like `[IMAGE:<source>]` — the runtime uses a structured
enum model exclusively.

**Staging pipeline** — `stage_channel_images()` in `channels/mod.rs:1014-1056` dispatches to
per-channel `fetch_and_stage_image()` methods for Telegram, WhatsApp, and Discord. The shared helper
`media::stream_validate_and_stage()` (`channels/media.rs:178-249`) handles HTTP response streaming,
size checks, MIME sniffing, SHA-256 hashing, and temp file creation.

**Validated artifact** — `StagedImage` (`channels/media.rs:80-88`) is the post-validation struct:
`sha256`, `mime_type` (enum), `byte_len`, `temp_path`, `transport_form` (currently only
`InlineBytes`), `channel_origin`. Cleanup is RAII via `StagedImageGuard` (`channels/mod.rs:127-135`).

**Provider handoff** — `ChatRequest.images` (`providers/traits.rs:79-81`) carries a `&[StagedImage]`
slice. The `CompatibleProvider::chat_multimodal()` (`providers/compatible.rs:494-550`) reads bytes
from `temp_path`, base64-encodes them, and constructs OpenAI-compatible `image_url` content blocks
with `data:{mime};base64,{b64}` data URLs. Images are attached only to the last user message.

**Config gating** — `MultimodalConfig` (`config/schema.rs:280-294`) provides: `enabled` (global kill
switch, default false), `allowed_channels` (allowlist), `vision_model_hint` (route selector),
`max_image_bytes` (operator override, not yet wired). The runtime enforces these in
`process_channel_message()` (`channels/mod.rs:688-877`).

**MIME validation** — Magic-byte sniffing for JPEG (`FF D8 FF`), PNG (8-byte sig), and WebP
(`RIFF....WEBP`). Sniffing takes strict precedence over declared MIME — even if declared as
`image/png`, non-matching bytes are rejected (`channels/media.rs:106-146`).

**Limits** — `MAX_IMAGE_BYTES = 10 MiB`, `MAX_IMAGES_PER_TURN = 1` (`channels/media.rs:7,10`).

**Observability** — Every ingestion attempt emits `ImageIngressEvent` with outcome
(`Admitted`/`Rejected`/`ProviderSent`/`ProviderError`), reason, image metadata
(`channels/mod.rs:213-235`).

**Existing spec** — `openspec/specs/channel-image-ingestion/spec.md` documents the full channel
ingestion contract with 10 requirements and 9 scenarios.

### Answers to Issue Questions

#### 1. What is the canonical runtime representation for image input?

**Answer**: The runtime uses a **structured message part model** — `ContentPart::Image` enum variant
in `ChannelMessage.parts`. After validation, images become `StagedImage` structs. There is no marker
syntax.

- `ContentPart::Image` — pre-fetch representation (channel layer)
- `StagedImage` — post-validation representation (runtime/provider layer)
- `ChatRequest.images: &[StagedImage]` — provider dispatch interface

Code refs: `channels/traits.rs:5-17`, `channels/media.rs:80-88`, `providers/traits.rs:76-82`

#### 2. Should Corvus adopt a marker like `[IMAGE:<source>]`, a structured message part model, or both?

**Answer**: The structured model is already adopted and is the correct choice. Markers like
`[IMAGE:<source>]` exist only in the **outbound** delivery layer (`channel_delivery_instructions()`
in `channels/mod.rs:151-158` for Telegram media markers). Inbound image handling is entirely
structured. **Recommendation**: Formalize the existing structured model as the canonical contract.
Do NOT introduce marker syntax for inbound/runtime representation.

#### 3. Which image MIME types are supported in the MVP?

**Answer**: Three types, validated by magic-byte sniffing:

| Format | Magic Bytes | MIME | Extension |
|--------|-------------|------|-----------|
| JPEG | `FF D8 FF` | image/jpeg | .jpg |
| PNG | 8-byte PNG signature | image/png | .png |
| WebP | `RIFF....WEBP` | image/webp | .webp |

All other formats (GIF, BMP, TIFF, SVG, etc.) are rejected. Code ref: `channels/media.rs:13-39`,
`channels/media.rs:106-146`

#### 4. What quantity and size limits apply?

**Answer**:

- **Max image payload size**: 10 MiB (`MAX_IMAGE_BYTES`, `channels/media.rs:7`)
- **Max images per turn**: 1 (`MAX_IMAGES_PER_TURN`, `channels/media.rs:10`)
- **Config override**: `multimodal.max_image_bytes` exists in config schema
  (`config/schema.rs:293`) but is **not yet wired** to the validation functions

#### 5. Is remote fetch allowed at all in v1, and if so under what conditions?

**Answer**: Yes — remote fetch is the **only** fetch path. All three MVP channels (Telegram,
WhatsApp, Discord) fetch images from their respective platform CDNs/APIs. Conditions:

- Fetch uses channel-specific authentication (bot token, bearer token, or pre-authenticated CDN URL)
- Streaming with per-chunk size validation against `MAX_IMAGE_BYTES`
- Early rejection via `Content-Length` when available
- Credentials redacted from error messages
- No arbitrary user-supplied URLs are fetched — only platform-mediated handles

There is **no local file path** or **user-supplied URL** ingestion path. This is a deliberate
security constraint.

#### 6. What errors should users and operators see when normalization fails?

**Answer**: The runtime already provides user-facing error messages for each rejection case
(`channels/mod.rs:700-870`):

| Rejection Reason | User Message |
|-----------------|--------------|
| `Disabled` | "Image input is currently disabled." |
| `ChannelNotAllowed` | "Image input is not enabled for this channel." |
| `MissingVisionRoute` | "Image input is not configured with a vision route." |
| `RouteNotImageCapable` | "The configured vision route does not allow image input." |
| `TooManyImages` | "Too many images ({count}). Maximum {limit} per message." |
| `FetchFailed` | "I couldn't download that image safely. Please try again." |
| `MimeRejected` | "That image format is not supported." |
| `Oversize` | "That image is too large to process." |
| Unimplemented channel | "Image input is not yet supported for this channel." |

All rejections also emit `ImageIngressEvent` for operator observability.

### Affected Areas

- `clients/agent-runtime/src/channels/traits.rs` — `ContentPart`, `ChannelMessage` (canonical types)
- `clients/agent-runtime/src/channels/media.rs` — validation, staging, limits
- `clients/agent-runtime/src/channels/mod.rs` — orchestration, gating, history storage
- `clients/agent-runtime/src/config/schema.rs` — `MultimodalConfig`, `ModelRouteConfig`
- `clients/agent-runtime/src/providers/traits.rs` — `ChatRequest.images`
- `clients/agent-runtime/src/providers/compatible.rs` — base64 encoding, content blocks
- `openspec/specs/channel-image-ingestion/spec.md` — existing channel-layer spec

### Gaps Identified

1. **No runtime-layer spec exists** — The existing `channel-image-ingestion` spec covers the channel
   ingestion pipeline but does NOT cover the runtime normalization contract (provider dispatch format,
   conversation history representation, error taxonomy as a formal contract). Issue #267 asks for
   this missing layer.

2. **Conversation history loses image context** — In `handle_successful_response()`
   (`channels/mod.rs:1210-1211`), image turns are stored as plain `ChatMessage::user(enriched_message)`.
   The `enriched_message` is the text projection only — **staged images are not represented in
   history**. On subsequent turns, the model has no record that an image was part of a prior
   exchange. This is the most significant gap.

3. **`max_image_bytes` config override not wired** — `MultimodalConfig.max_image_bytes`
   (`config/schema.rs:293`) exists but `validate_size()` and `stream_validate_and_stage()` always
   use the hardcoded `MAX_IMAGE_BYTES` constant. The operator override has no effect.

4. **No formal error taxonomy** — `ImageRejectionReason` is an enum with Display, but there is no
   versioned error code contract (e.g., `ERR_IMG_001`) for API consumers or operator dashboards. The
   snake_case Display output is the de facto contract.

5. **Single transport form** — `ImageTransportForm` only has `InlineBytes`. No URL-reference or
   provider-managed-upload path exists. This limits scalability for large images and multi-image
   turns but is acceptable for MVP.

6. **Multi-image support blocked** — `MAX_IMAGES_PER_TURN = 1` is hardcoded. The provider layer
   (`chat_multimodal`) already supports multiple images in content blocks, but the validation gate
   rejects > 1. No config path exists to raise this limit.

7. **Provider capability negotiation is implicit** — The `CompatibleProvider` hardcodes
   `image_input: true`, but the `Provider` trait's default `chat()` implementation rejects images
   (`providers/traits.rs:327-329`). There is no runtime check that the resolved provider actually
   supports images before dispatching — the fail-closed default catches it, but the error is generic.

### Approaches

1. **Formalize existing patterns as a runtime spec** — Document the current `ContentPart::Image` →
   `StagedImage` → `ChatRequest.images` pipeline as the canonical runtime contract. Add the missing
   conversation-history representation. Wire `max_image_bytes` config.
   - Pros: Low risk, codifies what works, closes the spec gap
   - Cons: Does not add new capabilities
   - Effort: Low

2. **Introduce a `NormalizedImageMessage` intermediate type** — Add a new struct between
   `StagedImage` and provider dispatch that captures the normalized runtime view (including history
   replay metadata). Separate from the channel-layer `ContentPart::Image`.
   - Pros: Cleaner separation of concerns, enables richer history representation
   - Cons: More types to maintain, migration cost for existing code
   - Effort: Medium

3. **Full multimodal message model overhaul** — Replace `ChatMessage` with a multimodal-native
   `RuntimeMessage` that natively supports content parts (text + image + future modalities).
   Eliminate the text-only `ChatMessage` for multimodal turns.
   - Pros: Future-proof, eliminates the history gap structurally
   - Cons: Large refactor across providers and conversation management, high risk
   - Effort: High

### Recommendation

**Approach 1** (Formalize existing patterns) for the proposal phase. The current implementation is
sound and well-tested. The primary deliverable should be:

1. A runtime-layer spec that formalizes `ContentPart::Image` → `StagedImage` → `ChatRequest.images`
   as the canonical pipeline
2. A concrete design for conversation-history image representation (the biggest gap)
3. Wiring the `max_image_bytes` config override
4. Formalizing the error taxonomy

Approach 2 elements (like `NormalizedImageMessage`) can be evaluated during design if the history
gap requires a new type. Approach 3 is premature — defer until multi-modality expands beyond images.

### Risks

- **History gap is user-visible** — Multi-turn image conversations lose context after the first
  turn. Users asking follow-up questions about an image will get confused responses. This should be
  prioritized in the proposal.
- **Config override gap is operator-visible** — Operators who set `max_image_bytes` expect it to
  work. Silent no-op erodes trust.
- **Spec drift between layers** — The channel-ingestion spec and runtime spec must stay aligned.
  Changes to one should reference the other.
- **Provider compatibility assumptions** — The `image_url` / data-URL format works for
  OpenAI-compatible APIs but may not work for all providers (e.g., Anthropic uses a different
  content block format). The runtime spec should acknowledge transport-form extensibility.

### Ready for Proposal

**Yes** — The codebase investigation is complete. All six questions from the issue are answered with
code references. The gaps are clearly identified, and the recommended approach (Approach 1) is low
risk with a clear scope. The orchestrator should proceed to `sdd-propose` to draft a formal proposal
addressing the gaps and formalizing the existing contract.
