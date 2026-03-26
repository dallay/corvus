# Exploration: Channel Image Ingestion Strategy

**Change**: channel-image-ingestion-strategy
**Issue**: #266
**Date**: 2026-03-26

## Context

Issue #266 asks us to define how channels ingest user-sent images and hand them to the runtime
consistently. The runtime multimodal contract already landed in PR #324, so this exploration focuses
on the **channel side** of that contract.

## Current State Analysis

### Runtime Contract (already implemented)

- `ContentPart::Image` in `channels/traits.rs` carries: `channel_handle`, `source_channel`,
  `declared_mime`, `caption_text`, `file_name`, `declared_bytes`
- `media.rs` defines: `StagedImage`, `AllowedImageMime` (JPEG/PNG/WebP), `MAX_IMAGE_BYTES` (10 MiB),
  `MAX_IMAGES_PER_TURN` (1), MIME magic-byte sniffing, size validation
- `stage_channel_images()` in `mod.rs` dispatches to per-channel `fetch_and_stage_image()`
  implementations
- `StagedImageGuard` provides RAII cleanup of temp files
- Config gating: `multimodal.enabled`, `multimodal.allowed_channels`, `multimodal.vision_model_hint`

### Telegram (fully implemented)

- Parses `photo` array (picks largest variant) and `document` (if MIME is allowed image type)
- Emits `ContentPart::Text` for caption + `ContentPart::Image` with `file_id` as `channel_handle`
- `fetch_and_stage_image()`: calls `getFile` API → downloads bytes with streaming size limit → MIME
  sniffing → SHA-256 → writes to `$TMPDIR/corvus-tg-img-{hash}.{ext}`
- Handles bot token redaction in error messages

### WhatsApp (fully implemented)

- Parses `type=image` messages via `extract_whatsapp_parts()`
- Emits `ContentPart::Text` for caption + `ContentPart::Image` with `media_id` as `channel_handle`
- `fetch_and_stage_image()`: resolves media ID → download URL via Graph API → streams bytes with
  bearer auth → MIME sniffing → SHA-256 → writes to `$TMPDIR/corvus-wa-img-{hash}.{ext}`
- Skips all other types (audio, video, document, sticker, location, contacts, reaction)

### Other Channels (no image support)

- **Discord** (`discord.rs`): Uses serenity crate. Messages have `attachments` field with URL,
  content_type, size. No image parsing implemented.
- **Slack** (`slack.rs`): Uses Slack Web API. Files shared in channels have `files` array with
  `url_private_download`, `mimetype`, `size`. No image parsing implemented.
- **CLI** (`cli.rs`): Reads stdin. Could accept file paths. No image parsing.
- **Matrix, Mattermost, Signal, IRC, Email, DingTalk, Lark, QQ, iMessage**: No image support.

### Staging & Retention

- Temp files written to `std::env::temp_dir()` with channel-specific prefixes (`corvus-tg-img-`,
  `corvus-wa-img-`)
- `StagedImageGuard` Drop impl calls `cleanup()` which removes the temp file
- Cleanup happens when the guard goes out of scope (after provider dispatch completes or on
  error/timeout)
- No explicit TTL or reaper — relies on RAII drop semantics
- Risk: if the process crashes mid-turn, orphan files remain in $TMPDIR until OS cleanup

### Config Gating

- `multimodal.enabled` (default: false) — global kill switch
- `multimodal.allowed_channels` — explicit allowlist of channel names
- `multimodal.vision_model_hint` — must resolve to a model route with `allow_image_input=true`
- `multimodal.max_image_bytes` — optional override (falls back to `MAX_IMAGE_BYTES` constant)
- Fail-closed: if channel not in allowlist, image parts are rejected with user-facing message

## Questions Closed by Exploration

### Q1: Which channels are in scope for image-input MVP?

**Telegram** and **WhatsApp** are already implemented and working. Based on codebase maturity and
user demand:

- **MVP (done)**: Telegram, WhatsApp
- **Next wave candidates**: Discord (has attachment metadata), Slack (has file sharing API)
- **Deferred**: All others (CLI is edge case — could accept file paths but low priority)

### Q2: Inbound image types per channel?

| Channel  | Inbound Forms            | How Image Arrives                            |
|----------|--------------------------|----------------------------------------------|
| Telegram | photo, document-as-image | `file_id` via Bot API `getFile`              |
| WhatsApp | image message            | `media_id` via Graph API media endpoint      |
| Discord  | message attachment       | Direct URL with content_type + size metadata |
| Slack    | file share in channel    | `url_private_download` with bearer auth      |

### Q3: File staging location and retention?

- **Location**: `std::env::temp_dir()` with prefix `corvus-{channel}-img-{sha256_prefix}.{ext}`
- **Retention**: Lifetime of the processing turn (RAII guard). Cleaned up on all exit paths.
- **Orphan risk**: Process crash leaves temp files. Mitigation: OS temp cleanup or optional startup
  reaper.

### Q4: Size and content limits?

- **Max size**: 10 MiB (`MAX_IMAGE_BYTES`)
- **Max images per turn**: 1 (`MAX_IMAGES_PER_TURN`)
- **Allowed MIME types**: image/jpeg, image/png, image/webp (validated by magic-byte sniffing, not
  declared MIME)
- **Early rejection**: Content-Length check before streaming where available

### Q5: Runtime handoff format?

Channel emits `ContentPart::Image` → `stage_channel_images()` produces `Vec<StagedImage>` → passed
as `images` slice to `Provider::chat()`. The `StagedImage` carries: sha256, mime_type, byte_len,
temp_path, transport_form (InlineBytes for MVP), channel_origin.

## Risks & Gaps

1. **No startup cleanup** for orphaned temp files from previous crashes
2. **Discord/Slack** need fetch implementations with their respective auth mechanisms
3. **Multi-image support** (MAX_IMAGES_PER_TURN=1) may need to increase for some channels
4. **No deduplication** — same image sent twice gets staged twice
5. **GIF not supported** — common on Telegram/Discord, rejected by MIME sniffing

## Recommendations

1. Document the current Telegram+WhatsApp strategy as the canonical pattern
2. Define the Discord and Slack ingestion contracts (next implementation targets)
3. Add optional startup reaper for orphaned temp files
4. Consider GIF→static frame conversion as a future enhancement
5. Keep MAX_IMAGES_PER_TURN=1 for MVP, plan for increase in follow-up
