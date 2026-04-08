# Spec: Channel Image Ingestion Strategy

**Domain**: channels / multimodal
**Status**: active
**Issue**: #266
**Date**: 2026-03-26

## Overview

This specification defines the canonical strategy for how Corvus messaging channels ingest user-sent
images, validate them, stage them to disk, and hand them off to the runtime's provider pipeline. It
codifies the patterns implemented for Telegram, WhatsApp, and Discord, and defines contracts for
remaining channel implementations (Slack and beyond).

## Definitions

- **Ingestion pipeline**: The 5-step flow a channel follows to accept an inbound image: parse →
  emit → fetch → validate → stage.
- **Channel handle**: An opaque, channel-specific identifier for a media asset (e.g., Telegram
  `file_id`, WhatsApp `media_id`, Discord attachment URL).
- **Staged image**: A validated image written to a temp file with metadata, ready for provider
  dispatch.
- **Vision route**: A model route configured with `allow_image_input=true`, resolved via
  `multimodal.vision_model_hint`.

## Requirements

### REQ-1: MVP Channel List

The following channels MUST support image ingestion for MVP:

| Channel  | Status      | Priority |
|----------|-------------|----------|
| Telegram | Implemented | MVP      |
| WhatsApp | Implemented | MVP      |
| Discord  | Implemented | MVP      |
| Slack    | Planned     | Wave 2   |

All other channels (CLI, Matrix, Mattermost, Signal, IRC, Email, DingTalk, Lark, QQ, iMessage) are
explicitly out of scope until a follow-up change promotes them.

### REQ-2: Canonical Ingestion Pipeline

Every channel implementing image ingestion MUST follow this 5-step pipeline:

1. **Parse**: Extract image metadata from the channel's native message format. Produce a
   `ContentPart::Image` with `channel_handle`, `source_channel`, `declared_mime`, `caption_text`,
   `file_name`, and `declared_bytes`.

2. **Emit**: Include the `ContentPart::Image` in the `ChannelMessage.parts` vector. If the image has
   a caption, emit it as a preceding `ContentPart::Text` part AND set `caption_text` on the `Image`
   part.

3. **Fetch**: Download the image bytes using the channel's native API. The fetch MUST:
    - Use channel-specific authentication (bot token, bearer token, etc.)
    - Stream bytes with a per-chunk size check against `MAX_IMAGE_BYTES`
    - Perform early rejection via `Content-Length` header when available
    - Redact credentials from error messages

4. **Validate**: Apply validation in this order:
   a. MIME type via magic-byte sniffing (MUST take precedence over declared MIME)
   b. File size against `MAX_IMAGE_BYTES` (10 MiB)
   c. Image count against `MAX_IMAGES_PER_TURN` (1)

5. **Stage**: Write validated bytes to a temp file and produce a `StagedImage`:
    - Compute SHA-256 hash of the raw bytes
    - Write to `std::env::temp_dir()` with naming:
      `corvus-{channel_abbrev}-img-{sha256_prefix_16}.{ext}`
    - Set `transport_form` to `InlineBytes` (MVP)
    - Set `channel_origin` to the channel name

### REQ-3: Allowed Image Formats

Channels MUST accept only the following MIME types, validated by magic-byte sniffing:

| Format | Magic Bytes                       | MIME       | Extension |
|--------|-----------------------------------|------------|-----------|
| JPEG   | `FF D8 FF`                        | image/jpeg | .jpg      |
| PNG    | `89 50 4E 47 0D 0A 1A 0A`         | image/png  | .png      |
| WebP   | `RIFF....WEBP` (bytes 0-3 + 8-11) | image/webp | .webp     |

GIF, BMP, TIFF, SVG, and all other formats MUST be rejected with
`ImageRejectionReason::MimeRejected`.

### REQ-4: Size and Count Limits

- **Max image payload size**: 10 MiB (`MAX_IMAGE_BYTES = 10 * 1024 * 1024`)
- **Max images per turn**: 1 (`MAX_IMAGES_PER_TURN = 1`)
- Channels SHOULD reject oversized images before fully downloading when `Content-Length` is
  available.
- The `multimodal.max_image_bytes` config field MAY override `MAX_IMAGE_BYTES` in the future.

### REQ-5: Config Gating

Image ingestion MUST be gated by the `[multimodal]` config section:

- `multimodal.enabled` (bool, default: false) — global kill switch. When false, ALL image parts MUST
  be rejected with `ImageRejectionReason::Disabled`.
- `multimodal.allowed_channels` (list of strings) — channel names that MAY accept images. Channels
  not in this list MUST reject with `ImageRejectionReason::ChannelNotAllowed`.
- `multimodal.vision_model_hint` (string) — MUST resolve to a model route with
  `allow_image_input=true`. Missing or non-matching hint MUST reject with
  `ImageRejectionReason::MissingVisionRoute` or `RouteNotImageCapable`.

Config validation at startup MUST enforce:

- If `enabled=true`, then `vision_model_hint` MUST be set and non-empty
- If `enabled=true`, then `allowed_channels` MUST be non-empty
- All entries in `allowed_channels` SHOULD be valid MVP channel names; non-MVP entries are permitted
  but will be fail-closed at runtime (ADR-4) since no staging implementation exists

### REQ-6: Runtime Handoff Format

The runtime handoff MUST follow this chain:

```text
ChannelMessage.parts: [ContentPart::Image { channel_handle, source_channel, ... }]
    ↓ stage_channel_images()
Vec<StagedImage> { sha256, mime_type, byte_len, temp_path, transport_form, channel_origin }
    ↓ passed to Provider::chat() as images slice
Provider reads temp_path, base64-encodes for InlineBytes transport
```

- `stage_channel_images()` MUST dispatch to the correct channel's `fetch_and_stage_image()` based on
  `msg.channel`
- For channels without a staging implementation, `stage_channel_images()` MUST return an empty
  `Vec` (which triggers fail-closed rejection in the caller)

### REQ-7: File Staging and Cleanup

The system MUST preserve request-path staged-image cleanup via `StagedImageGuard` RAII semantics, and
it MUST also perform a one-time startup reaping pass for orphaned staged image temp files.

The staged-image cleanup contract is updated as follows:

- Staged files MUST continue to be cleaned up via `StagedImageGuard` RAII semantics.
- The guard's `Drop` implementation MUST continue to call `StagedImage::cleanup()` for each staged
  image.
- Cleanup on request exit paths MUST remain best-effort and MUST NOT panic on failure.
- In addition to request-path cleanup, the runtime MUST scan `std::env::temp_dir()` once at startup
  from command-level startup entry paths.
- The startup reaper MUST only target files that match Corvus staged-image filename conventions.
- The startup reaper MUST recognize both the current shared staged-image filename format
  (`corvus-{channel}-img-...`) and the legacy Telegram staged-image filename format
  (`corvus-tg-img-...`).
- The startup reaper MUST delete only matching files whose age is older than the effective reaper
  threshold.
- The startup reaper MUST bias toward non-deletion when file age cannot be determined reliably.
- The startup reaper MUST treat missing files and other duplicate-execution races as non-fatal.
- The startup reaper MUST emit an info-level log with the cleaned file count for each execution, and
  it MUST NOT require logging individual file names.
- Startup reaping MUST be invoked from command-level startup paths and MUST NOT be introduced as a
  background or per-message cleanup loop inside deeper gateway or channel processing loops.

#### Scenario: Startup reaper removes stale staged images

- GIVEN the OS temp directory contains Corvus staged-image files older than the effective threshold
- WHEN a runtime command reaches its startup boundary
- THEN the system MUST delete only the matching stale staged-image files
- AND it MUST leave request-path cleanup behavior unchanged for newly staged images
- AND it MUST log the cleaned file count at info level

#### Scenario: Fresh or non-matching temp files are preserved

- GIVEN the OS temp directory contains a mix of Corvus-staged image files newer than the threshold
  and unrelated temp files
- WHEN the startup reaper executes
- THEN the system MUST preserve the newer Corvus-staged image files
- AND it MUST preserve all non-matching temp files

#### Scenario: Legacy Telegram staged-image filenames are still reaped

- GIVEN the OS temp directory contains stale files using the legacy `corvus-tg-img-...` naming
  convention
- WHEN the startup reaper executes
- THEN the system MUST recognize those files as Corvus-staged images
- AND it MUST delete them only when they are older than the effective threshold

#### Scenario: Duplicate startup execution remains safe

- GIVEN one startup execution has already deleted a stale staged-image file
- WHEN a second startup execution encounters the same file set or observes a file disappear during
  deletion
- THEN the system MUST continue startup without failure
- AND it MUST treat the duplicate-delete race as a best-effort cleanup outcome

### REQ-8: Fail-Closed Semantics

- If multimodal is disabled → reject with user message
- If channel not in allowlist → reject with user message
- If vision route missing or not image-capable → reject with user message
- If image count exceeds limit → reject with user message
- If fetch fails → reject with user message
- If MIME validation fails → reject with user message
- If size validation fails → reject with user message
- If channel has no staging implementation → reject with user message ("not yet supported for this
  channel")
- In ALL rejection cases, the runtime MUST emit an `ImageIngressEvent` via the observer

### REQ-9: Observability

Every image ingestion attempt MUST emit an `ImageIngressEvent` with:

- `channel`: source channel name
- `provider` / `model`: resolved vision route (if available)
- `outcome`: `Admitted`, `Rejected`, `ProviderSent`, or `ProviderError`
- `reason`: rejection reason (if rejected)
- `image_count`, `mime_type`, `byte_len`: image metadata

### REQ-10: Deduplication (out of scope for MVP)

Image deduplication is explicitly **out of scope** for MVP. If a user sends the same image twice in
separate messages, each occurrence is independently fetched, validated, staged, and dispatched. The
SHA-256 hash in the staging filename enables future dedup without schema changes, but no dedup logic
SHALL be implemented in the initial channel ingestion pipeline.

## Scenarios

### Scenario 1: Telegram photo accepted

**Given** multimodal is enabled with `allowed_channels: ["telegram"]`
**And** a vision route is configured and image-capable
**When** a Telegram user sends a JPEG photo with caption "What is this?"
**Then** the channel emits `[Text("What is this?"), Image { channel_handle: file_id, ... }]`
**And** `fetch_and_stage_image()` downloads via Telegram Bot API
**And** MIME sniffing confirms JPEG, size is under 10 MiB
**And** a `StagedImage` is produced with SHA-256 hash
**And** the provider receives the image as `InlineBytes`
**And** an `ImageIngressEvent` with outcome `Admitted` is emitted

### Scenario 2: WhatsApp image without caption

**Given** multimodal is enabled with `allowed_channels: ["whatsapp"]`
**When** a WhatsApp user sends a PNG image with no caption
**Then** the channel emits `[Image { channel_handle: media_id, ... }]`
**And** the text projection is empty
**And** `fetch_and_stage_image()` resolves media URL via Graph API with bearer auth
**And** the image is staged and dispatched normally

### Scenario 3: Image rejected — channel not allowed

**Given** multimodal is enabled with `allowed_channels: ["telegram"]`
**When** a Discord user sends an image
**Then** the image is rejected with `ChannelNotAllowed`
**And** the user receives "Image input is not enabled for this channel"
**And** an `ImageIngressEvent` with outcome `Rejected` and reason `ChannelNotAllowed` is emitted

### Scenario 4: Image rejected — oversized

**Given** multimodal is enabled for Telegram
**When** a user sends a 15 MiB image
**Then** the image is rejected with `Oversize` during streaming validation
**And** the user receives "That image is too large to process"
**And** an `ImageIngressEvent` with outcome `Rejected` and reason `Oversize` is emitted

### Scenario 5: Image rejected — unsupported format

**Given** multimodal is enabled for Telegram
**When** a user sends a GIF
**Then** magic-byte sniffing rejects the file with `MimeRejected`
**And** the user receives "That image format is not supported"
**And** an `ImageIngressEvent` with outcome `Rejected` and reason `MimeRejected` is emitted

### Scenario 6: Image rejected — multimodal disabled

**Given** multimodal is disabled (`enabled: false`)
**When** any user sends an image on any channel
**Then** the image is rejected with `Disabled`
**And** the user receives "Image input is currently disabled"

### Scenario 7: Fail-closed for unimplemented channel

**Given** multimodal is enabled with `allowed_channels: ["slack"]`
**When** a Slack user sends an image
**Then** `stage_channel_images()` returns an empty Vec
**And** the caller rejects with "Image input is not yet supported for this channel"

### Scenario 8: Temp file cleanup on timeout

**Given** a valid image is staged to a temp file
**When** the provider call times out after 300 seconds
**Then** the `StagedImageGuard` Drop fires and removes the temp file
**And** the user receives a timeout error message
**And** an `ImageIngressEvent` with outcome `ProviderError` is emitted

### Scenario 9: Image rejected — vision route misconfigured

**Given** multimodal is enabled with `allowed_channels: ["telegram"]`
**And** `vision_model_hint` points to a non-existent or non-image-capable route
**When** a Telegram user sends an image
**Then** the image is rejected with `MissingVisionRoute` or `RouteNotImageCapable`
**And** the user receives an appropriate error message
**And** an `ImageIngressEvent` with outcome `Rejected` and the corresponding reason is emitted

## Channel-Specific Contracts

### Telegram

- **Inbound forms**: `photo` (array, pick last/largest), `document` (if MIME is allowed image type)
- **Channel handle**: `file_id` string
- **Fetch method**: `POST getFile` → resolve `file_path` → `GET /file/bot{token}/{file_path}`
- **Auth**: Bot token in URL path
- **Metadata available**: `file_size` (from photo/document object), MIME (declared for documents,
  assumed `image/jpeg` for photos)
- **Caption**: From `message.caption` field

### WhatsApp

- **Inbound forms**: `type=image` messages
- **Channel handle**: `image.id` (media ID string)
- **Fetch method**: `GET https://graph.facebook.com/v21.0/{media_id}` → resolve `url` → `GET {url}`
- **Auth**: Bearer token (access_token) on both requests
- **Metadata available**: `mime_type`, `caption` from image object
- **Caption**: From `image.caption` field

### Discord (Implemented — MVP)

- **Inbound forms**: Message attachments with image content_type
- **Channel handle**: Attachment URL (direct CDN link)
- **Fetch method**: `GET {attachment_url}` (public CDN, no auth needed for bot-visible messages)
- **Auth**: None for download (attachment URLs are pre-authenticated)
- **Metadata available**: `content_type`, `size`, `filename`, `width`, `height`
- **Caption**: Message text content serves as caption
- **Implementation note**: Filter `message.attachments` by `content_type` starting with `image/`

### Slack (Wave 2 — contract only)

- **Inbound forms**: Files shared in channel (via `file_shared` event or `files` array in message)
- **Channel handle**: `file.id` string
- **Fetch method**: `GET {url_private_download}` with bearer auth
- **Auth**: Bot token as Bearer header
- **Metadata available**: `mimetype`, `size`, `name`, `filetype`
- **Caption**: Message text serves as caption
- **Implementation note**: Requires `files:read` OAuth scope; use `url_private_download` not
  `url_private`
