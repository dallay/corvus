# Runtime Image Normalization Pipeline Specification

**Domain**: runtime / multimodal
**Status**: draft
**Issue**: #267
**Date**: 2026-03-26
**Depends on**: `channel-image-ingestion` spec (#266)

## Overview

This specification defines the canonical runtime contract for multimodal image normalization in
Corvus. It covers the full lifecycle from the `StagedImage` handoff boundary (where the
channel-ingestion spec ends) through provider dispatch, conversation history persistence, and error
taxonomy.

This spec layers on top of — and cross-references — the `channel-image-ingestion` spec
(`openspec/specs/channel-image-ingestion/spec.md`). The `StagedImage` type is the shared interface
between the two specs.

## Definitions

- **Normalization pipeline**: The 5-step runtime flow for accepting an inbound image: parse → gate →
  fetch+stage → validate → handoff. Steps 1-4 are defined by the channel-ingestion spec; this spec
  formalizes the end-to-end contract and owns step 5 (handoff) and beyond.
- **Staged image**: A validated image written to a temp file with metadata (`StagedImage`), produced
  by the channel-ingestion pipeline. See channel-ingestion spec REQ-2 step 5.
- **Provider dispatch**: The process of encoding a `StagedImage` into a provider-compatible content
  block (e.g., base64 data URL for OpenAI-compatible APIs) and attaching it to a `ChatRequest`.
- **Image context**: Metadata about an image that was part of a conversation turn, preserved in
  history so that subsequent turns can reference prior images.
- **Transport form**: The encoding strategy used to deliver image bytes to a provider. MVP supports
  only `InlineBytes` (base64 data URL). The `ImageTransportForm` enum is extensible for future
  strategies (e.g., provider-managed upload, URL reference).
- **Vision route**: A model route configured with `allow_image_input=true`, resolved via
  `multimodal.vision_model_hint`. See channel-ingestion spec REQ-5.

## Requirements

### REQ-1: Canonical Runtime Representation

The runtime MUST represent images as structured enum variants throughout the pipeline. Specifically:

- **Pre-fetch**: `ContentPart::Image` within `ChannelMessage.parts` — carries `channel_handle`,
  `source_channel`, `declared_mime`, `caption_text`, `file_name`, and `declared_bytes`.
- **Post-validation**: `StagedImage` — carries `sha256`, `mime_type` (enum), `byte_len`,
  `temp_path`, `transport_form`, and `channel_origin`.
- **Provider dispatch**: `ChatRequest.images` — a `&[StagedImage]` slice passed to the provider's
  `chat()` or `chat_multimodal()` method.

The runtime MUST NOT use marker syntax (e.g., `[IMAGE:<source>]`) for inbound or runtime image
representation. Marker syntax MAY exist only in outbound delivery layers (e.g., channel delivery
instructions for Telegram media).

The `ContentPart::Image` → `StagedImage` → `ChatRequest.images` chain is the canonical pipeline.
All image processing MUST flow through this chain.

#### Scenario: Image flows through canonical pipeline

- GIVEN multimodal is enabled and a vision route is configured
- WHEN a user sends a valid JPEG image on an allowed channel
- THEN the channel layer produces a `ContentPart::Image` in `ChannelMessage.parts`
- AND `stage_channel_images()` produces a `StagedImage` with validated metadata
- AND the provider receives the image via `ChatRequest.images` as a `&[StagedImage]` slice
- AND no marker syntax is used at any point in the pipeline

#### Scenario: Marker syntax rejected for inbound representation

- GIVEN a developer proposes using `[IMAGE:telegram:abc123]` markers in message text
- WHEN the runtime processes an inbound image
- THEN the image MUST be represented as a `ContentPart::Image` enum variant
- AND the message text MUST NOT contain image marker strings

### REQ-2: Normalization Pipeline

The runtime MUST process every inbound image through a 5-step normalization pipeline:

1. **Parse**: Extract image metadata from the channel's native message format into a
   `ContentPart::Image`. (Owned by channel-ingestion spec REQ-2 step 1.)

2. **Gate**: Apply config-driven admission control before any fetch:
   - `multimodal.enabled` MUST be `true` (reject with `Disabled` otherwise)
   - Channel MUST be in `multimodal.allowed_channels` (reject with `ChannelNotAllowed` otherwise)
   - `multimodal.vision_model_hint` MUST resolve to a route with `allow_image_input=true` (reject
     with `MissingVisionRoute` or `RouteNotImageCapable` otherwise)
   - Image count MUST NOT exceed `MAX_IMAGES_PER_TURN` (reject with `TooManyImages` otherwise)

3. **Fetch + Stage**: Download image bytes from the channel's platform CDN/API and write to a
   validated temp file. (Owned by channel-ingestion spec REQ-2 steps 3-5.)

4. **Validate**: Apply MIME and size validation per REQ-3 and REQ-4 of this spec. Validation MUST
   occur during streaming — the runtime MUST NOT buffer the entire image before checking limits.

5. **Handoff**: Deliver the `StagedImage` to the provider layer:
   - The provider MUST read bytes from `StagedImage.temp_path`
   - For `InlineBytes` transport, the provider MUST base64-encode the bytes and construct a
     `data:{mime};base64,{b64}` data URL
   - The image content block MUST be attached to the last user message in the provider request
   - After provider dispatch (success or failure), `StagedImageGuard` MUST clean up temp files via
     RAII semantics (see channel-ingestion spec REQ-7)

The pipeline MUST be fail-closed: any step that cannot be completed MUST reject the image with an
appropriate `ImageRejectionReason` and emit an `ImageIngressEvent`.

#### Scenario: Full pipeline happy path

- GIVEN multimodal is enabled with `allowed_channels: ["telegram"]` and a valid vision route
- WHEN a Telegram user sends a 2 MiB PNG image with caption "Describe this"
- THEN step 1 (parse) produces `ContentPart::Image` with `channel_handle` and `declared_mime`
- AND step 2 (gate) passes all config checks
- AND step 3 (fetch+stage) downloads bytes via Telegram Bot API and writes to temp file
- AND step 4 (validate) confirms PNG magic bytes and size under limit
- AND step 5 (handoff) base64-encodes the image and attaches it to the provider request
- AND the provider responds with a description of the image
- AND the temp file is cleaned up after provider response

#### Scenario: Pipeline short-circuits at gate step

- GIVEN multimodal is enabled but `allowed_channels` does not include "discord"
- WHEN a Discord user sends an image
- THEN step 2 (gate) rejects with `ChannelNotAllowed`
- AND steps 3-5 are NOT executed
- AND no fetch request is made to any external service
- AND an `ImageIngressEvent` with outcome `Rejected` is emitted

### REQ-3: MIME Validation Rules

The runtime MUST validate image MIME types using magic-byte sniffing. Magic-byte sniffing MUST take
strict precedence over any declared MIME type from the channel.

The following formats MUST be accepted:

| Format | Magic Bytes                       | MIME       | Extension |
|--------|-----------------------------------|------------|-----------|
| JPEG   | `FF D8 FF`                        | image/jpeg | .jpg      |
| PNG    | `89 50 4E 47 0D 0A 1A 0A`         | image/png  | .png      |
| WebP   | `RIFF....WEBP` (bytes 0-3 + 8-11) | image/webp | .webp     |

All other formats (GIF, BMP, TIFF, SVG, HEIC, AVIF, etc.) MUST be rejected with
`ImageRejectionReason::MimeRejected`.

If the declared MIME type conflicts with the sniffed MIME type, the sniffed type MUST be used and
the declared type MUST be ignored. The runtime SHOULD log a warning when declared and sniffed types
disagree.

#### Scenario: Magic bytes override declared MIME

- GIVEN a channel declares an image as `image/png`
- WHEN the first bytes are `FF D8 FF` (JPEG magic bytes)
- THEN the runtime classifies the image as `image/jpeg`
- AND the declared `image/png` MIME is ignored
- AND the staged image uses extension `.jpg`

#### Scenario: Unsupported format rejected

- GIVEN a user sends a GIF file (magic bytes `47 49 46 38`)
- WHEN MIME validation runs
- THEN the image is rejected with `MimeRejected`
- AND the user receives "That image format is not supported."

#### Scenario: SVG rejected despite image MIME prefix

- GIVEN a user sends a file with declared MIME `image/svg+xml`
- WHEN magic-byte sniffing does not match any allowed format
- THEN the image is rejected with `MimeRejected`

### REQ-4: Size and Count Limits

The runtime MUST enforce the following limits:

- **Max image payload size**: 10 MiB (`MAX_IMAGE_BYTES = 10 * 1024 * 1024`) by default
- **Max images per turn**: 1 (`MAX_IMAGES_PER_TURN = 1`)

The `multimodal.max_image_bytes` configuration field MUST override `MAX_IMAGE_BYTES` when set.
If `multimodal.max_image_bytes` is not set or is `null`, the runtime MUST fall back to the
hardcoded `MAX_IMAGE_BYTES` constant.

Config validation for `max_image_bytes` (see REQ-8):
- The value MUST be greater than 0
- The value MUST NOT exceed 50 MiB (hardcoded ceiling)
- Invalid values MUST cause a startup validation error

Size validation MUST occur during streaming — the runtime SHOULD reject oversized images before
fully downloading them when `Content-Length` is available, and MUST reject during streaming when
accumulated bytes exceed the limit.

#### Scenario: Default size limit applied

- GIVEN `multimodal.max_image_bytes` is not set in config
- WHEN a user sends a 12 MiB image
- THEN the image is rejected with `Oversize`
- AND the effective limit is 10 MiB

#### Scenario: Config override reduces limit

- GIVEN `multimodal.max_image_bytes` is set to `5242880` (5 MiB)
- WHEN a user sends a 7 MiB image
- THEN the image is rejected with `Oversize`
- AND the effective limit is 5 MiB

#### Scenario: Config override increases limit

- GIVEN `multimodal.max_image_bytes` is set to `20971520` (20 MiB)
- WHEN a user sends a 15 MiB PNG image
- THEN the image is accepted and staged
- AND the effective limit is 20 MiB

#### Scenario: Too many images in one turn

- GIVEN `MAX_IMAGES_PER_TURN` is 1
- WHEN a user sends a message with 2 images
- THEN the images are rejected with `TooManyImages`
- AND the user receives "Too many images (2). Maximum 1 per message."

#### Scenario: Early rejection via Content-Length

- GIVEN the effective size limit is 10 MiB
- WHEN the channel API returns `Content-Length: 15728640` (15 MiB)
- THEN the runtime rejects the image with `Oversize` before downloading any bytes

### REQ-5: Remote Fetch Safety

The runtime MUST NOT fetch images from arbitrary user-supplied URLs. All image fetches MUST be
mediated by the channel's platform API:

- **Telegram**: Fetch via Bot API `getFile` endpoint using bot token
- **WhatsApp**: Fetch via Graph API media endpoint using bearer token
- **Discord**: Fetch via pre-authenticated CDN attachment URL

The following constraints MUST apply to all fetch operations:

- Fetch MUST use channel-specific authentication (bot token, bearer token, or pre-authenticated URL)
- Fetch MUST stream bytes with per-chunk size validation
- Fetch MUST redact credentials from error messages and logs
- Fetch MUST NOT follow redirects to hosts outside the channel's known CDN domains
- The runtime MUST NOT implement a generic URL fetch path for user-supplied image URLs

This constraint is a deliberate security boundary: the runtime acts as a controlled proxy, not an
open fetcher.

#### Scenario: Platform-mediated fetch succeeds

- GIVEN a Telegram user sends a photo
- WHEN the runtime fetches the image
- THEN the fetch uses the Telegram Bot API with the bot token
- AND the image bytes are streamed with size validation
- AND credentials are not present in any error messages

#### Scenario: Arbitrary URL rejected

- GIVEN a user provides a raw URL `https://evil.com/image.jpg` in message text
- WHEN the runtime processes the message
- THEN the URL is treated as text, NOT as an image to fetch
- AND no HTTP request is made to `https://evil.com`

### REQ-6: Conversation History Image Representation

When a turn includes an image that was successfully processed by the provider, the runtime MUST
store image context in conversation history so that subsequent turns retain awareness of prior
images.

The history entry for an image turn MUST include:

- The text content of the turn (caption or user message)
- An image context marker that records: MIME type, byte length, SHA-256 hash, channel origin,
  caption (if provided), and description (if available)
- Caption and description values MUST be sanitized before storage and context injection: newlines
  stripped and content truncated to 200 characters
- The assistant's response to the image

The history representation MUST NOT store raw image bytes in conversation history. Image bytes are
ephemeral (temp file, cleaned up after provider dispatch). History stores metadata only.

On subsequent turns, the model MUST receive the image context metadata as part of the conversation
history. This allows the model to understand that a prior turn included an image, even though the
raw bytes are no longer available.

The image context in history SHOULD be structured such that:

- The model can distinguish image turns from text-only turns
- Multiple image turns in a conversation are each independently identifiable by their SHA-256 hash
- The representation is compact (metadata only, not base64 bytes)

#### Scenario: Follow-up question about a previous image

- GIVEN a user sent a PNG image with caption "What is this?" on turn 1
- AND the provider responded with "This is a photo of a sunset."
- WHEN the user sends "What colors are in it?" on turn 2 (text only)
- THEN the conversation history sent to the provider includes image context from turn 1
- AND the model can reference the prior image in its response
- AND the raw image bytes are NOT re-sent to the provider on turn 2

#### Scenario: Image context distinguishes multiple image turns

- GIVEN a user sent image A (SHA-256: `abc123...`) on turn 1
- AND a user sent image B (SHA-256: `def456...`) on turn 3
- WHEN the user asks "Compare the two images" on turn 5
- THEN the conversation history includes distinct image context entries for both images
- AND each entry is identifiable by its SHA-256 hash

#### Scenario: History does not store raw bytes

- GIVEN a 5 MiB JPEG image was successfully processed on turn 1
- WHEN the runtime stores the turn in conversation history
- THEN the history entry contains image metadata (mime, size, hash, origin)
- AND the history entry does NOT contain base64-encoded image bytes
- AND the temp file has been cleaned up by `StagedImageGuard`

### REQ-7: Error Taxonomy

The runtime MUST use the following rejection reasons as a stable contract. Each rejection reason
MUST map to exactly one user-facing message and one observability event.

| Rejection Reason       | User-Facing Message                                                      | Emitted When                                                              |
|------------------------|--------------------------------------------------------------------------|---------------------------------------------------------------------------|
| `Disabled`             | "Image input is currently disabled."                                     | `multimodal.enabled` is `false`                                           |
| `ChannelNotAllowed`    | "Image input is not enabled for this channel."                           | Channel not in `multimodal.allowed_channels`                              |
| `MissingVisionRoute`   | "Image input is not configured with a vision route."                     | `vision_model_hint` is unset or resolves to no route                      |
| `RouteNotImageCapable` | "The configured vision route does not allow image input."                | Resolved route has `allow_image_input=false`                              |
| `TooManyImages`        | "Too many images ({count}). Maximum {limit} per message."                | Image count exceeds `MAX_IMAGES_PER_TURN`                                 |
| `FetchFailed`          | "I couldn't download that image safely. Please try again."               | Channel fetch fails (network error, auth error, timeout)                  |
| `MimeRejected`         | "That image format is not supported."                                    | Magic-byte sniffing does not match JPEG, PNG, or WebP                     |
| `Oversize`             | "That image is too large to process."                                    | Image bytes exceed effective size limit                                   |
| `ChannelNotSupported`  | "Image input is not yet supported for this channel."                     | Channel has no `fetch_and_stage_image()` implementation                   |
| `ProviderError`        | "Image processing failed — please try again."                            | Provider returned an error during image-bearing request                   |

This taxonomy (10 variants) MUST be exhaustive for MVP — every image rejection MUST map to exactly
one of these reasons.

All rejection reasons MUST:
- Be variants of `ImageRejectionReason` enum
- Implement `Display` producing a stable snake_case identifier (e.g., `disabled`, `mime_rejected`)
- Emit an `ImageIngressEvent` with outcome `Rejected` and the corresponding reason

User-facing messages MUST be static strings (with parameter substitution for `TooManyImages` only).
The runtime MUST NOT expose internal error details (stack traces, URLs, credentials) in user-facing
messages.

#### Scenario: Disabled rejection

- GIVEN `multimodal.enabled` is `false`
- WHEN any user sends an image on any channel
- THEN the image is rejected with reason `Disabled`
- AND the user receives "Image input is currently disabled."
- AND an `ImageIngressEvent` with outcome `Rejected` and reason `Disabled` is emitted

#### Scenario: Channel not allowed rejection

- GIVEN `multimodal.allowed_channels` is `["telegram"]`
- WHEN a WhatsApp user sends an image
- THEN the image is rejected with reason `ChannelNotAllowed`
- AND the user receives "Image input is not enabled for this channel."

#### Scenario: Missing vision route rejection

- GIVEN `multimodal.vision_model_hint` is not set
- WHEN a user sends an image on an allowed channel
- THEN the image is rejected with reason `MissingVisionRoute`
- AND the user receives "Image input is not configured with a vision route."

#### Scenario: Route not image-capable rejection

- GIVEN `vision_model_hint` resolves to a route with `allow_image_input=false`
- WHEN a user sends an image on an allowed channel
- THEN the image is rejected with reason `RouteNotImageCapable`
- AND the user receives "The configured vision route does not allow image input."

#### Scenario: Fetch failure rejection

- GIVEN the Telegram Bot API is unreachable (network timeout)
- WHEN a user sends an image on Telegram
- THEN the image is rejected with reason `FetchFailed`
- AND the user receives "I couldn't download that image safely. Please try again."
- AND no credentials or internal URLs appear in the user message

#### Scenario: Channel not supported rejection

- GIVEN `allowed_channels` includes "slack" but Slack has no staging implementation
- WHEN a Slack user sends an image
- THEN the image is rejected with reason `ChannelNotSupported`
- AND the user receives "Image input is not yet supported for this channel."

### REQ-8: Configuration Contract

The `[multimodal]` config section MUST conform to the following contract:

```toml
[multimodal]
enabled = false                    # bool, default: false — global kill switch
allowed_channels = []              # list of strings — channel allowlist
vision_model_hint = ""             # string — model route selector
max_image_bytes = null             # optional integer — override MAX_IMAGE_BYTES
```

Startup validation MUST enforce:

- If `enabled=true`, then `vision_model_hint` MUST be set and non-empty. Violation MUST produce a
  startup error.
- If `enabled=true`, then `allowed_channels` MUST be non-empty. Violation MUST produce a startup
  error.
- If `max_image_bytes` is set, it MUST be > 0 and <= 52428800 (50 MiB). Violation MUST produce a
  startup error.
- Non-MVP channel names in `allowed_channels` SHOULD produce a startup warning (not an error).
  These channels will be fail-closed at runtime per the channel-ingestion spec (REQ-8 / ADR-4).

The runtime MUST log the effective `max_image_bytes` value at startup when multimodal is enabled,
indicating whether the value comes from config override or the hardcoded default.

#### Scenario: Valid config with custom size limit

- GIVEN a config file with `multimodal.enabled=true`, `vision_model_hint="gpt-4o"`,
  `allowed_channels=["telegram"]`, `max_image_bytes=5242880`
- WHEN the runtime starts
- THEN config validation passes
- AND the runtime logs "Multimodal enabled: max_image_bytes=5242880 (config override)"

#### Scenario: Invalid config — enabled without vision route

- GIVEN a config file with `multimodal.enabled=true` and `vision_model_hint=""`
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error
- AND the error message indicates that `vision_model_hint` is required when multimodal is enabled

#### Scenario: Invalid config — max_image_bytes too large

- GIVEN a config file with `max_image_bytes=104857600` (100 MiB)
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error
- AND the error message indicates the 50 MiB ceiling

#### Scenario: Invalid config — max_image_bytes is zero

- GIVEN a config file with `max_image_bytes=0`
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error

#### Scenario: Warning for non-MVP channel in allowlist

- GIVEN a config file with `allowed_channels=["telegram", "slack"]`
- WHEN the runtime starts
- THEN the runtime logs a warning that "slack" is not an MVP channel
- AND startup succeeds (not a fatal error)
- AND Slack image requests are fail-closed at runtime

## Cross-References

- **Channel Image Ingestion Spec** (`openspec/specs/channel-image-ingestion/spec.md`, #266):
  Defines the channel-layer pipeline (parse → emit → fetch → validate → stage) and produces the
  `StagedImage` that this spec consumes. REQ-2 through REQ-5 of the channel spec define the
  pre-handoff contract.

- **Provider Adapters** (#268): Future work to support non-OpenAI-compatible content block formats
  (Anthropic, Gemini). This spec's `ImageTransportForm` extensibility point enables that work
  without modifying the normalization pipeline.

## Appendix: Transport Form Extensibility

The `ImageTransportForm` enum currently has a single variant:

- `InlineBytes` — Provider reads bytes from `temp_path` and base64-encodes them into a data URL.

Future variants MAY include:

- `UrlReference` — Provider passes a pre-signed URL to the model API (avoids base64 overhead).
- `ProviderUpload` — Provider uploads bytes to a provider-specific storage endpoint and references
  the upload ID.

Adding new transport forms MUST NOT require changes to the normalization pipeline (REQ-2). Transport
form selection is a provider-layer concern, not a runtime-layer concern.
