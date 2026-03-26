# Design: Channel Image Ingestion Strategy

**Change**: channel-image-ingestion-strategy
**Issue**: #266
**Date**: 2026-03-26

## Architecture Overview

The channel image ingestion system follows a **pipeline architecture** with clear separation between
channel-specific parsing and channel-agnostic validation/staging.

```
┌─────────────────────────────────────────────────────────────────┐
│                    Channel Layer (per-channel)                   │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ Telegram  │  │ WhatsApp │  │ Discord  │  │  Slack   │       │
│  │  parser   │  │  parser  │  │  parser  │  │  parser  │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
│       │              │              │              │             │
│       ▼              ▼              ▼              ▼             │
│  ContentPart::Image { channel_handle, source_channel, ... }     │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Config Gating (mod.rs)                         │
│                                                                 │
│  multimodal.enabled? → allowed_channels? → vision_route?        │
│  image_count ≤ MAX_IMAGES_PER_TURN?                             │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│              Fetch & Stage (per-channel dispatch)                │
│                                                                 │
│  stage_channel_images() dispatches to:                          │
│    "telegram" → TelegramChannel::fetch_and_stage_image()        │
│    "whatsapp" → WhatsAppChannel::fetch_and_stage_image()        │
│    "discord"  → DiscordChannel::fetch_and_stage_image() [Wave2] │
│    "slack"    → SlackChannel::fetch_and_stage_image()   [Wave2] │
│    _          → Ok(Vec::new())  [fail-closed]                   │
│                                                                 │
│  Each fetch_and_stage_image():                                  │
│    1. Resolve download URL (channel-specific API)               │
│    2. Stream bytes with size limit                              │
│    3. validate_mime() — magic-byte sniffing                     │
│    4. validate_size() — byte count check                        │
│    5. SHA-256 hash → write to temp file                         │
│    6. Return StagedImage                                        │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│              Provider Dispatch (mod.rs)                          │
│                                                                 │
│  StagedImageGuard wraps Vec<StagedImage>                        │
│  Provider::chat() receives &[StagedImage] as images param       │
│  Provider reads temp_path bytes, base64 encodes for transport   │
│  Guard drops → temp files cleaned up                            │
└─────────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### ADR-1: Channel-Specific Fetch, Shared Validation

**Decision**: Each channel implements its own `fetch_and_stage_image()` for download logic, but ALL
channels use the shared `media::validate_mime()`, `media::validate_size()`, and
`media::validate_image_count()` functions.

**Rationale**: Channel APIs differ fundamentally in authentication, URL resolution, and response
format. But validation rules (MIME, size, count) are universal and security-critical — they MUST NOT
be reimplemented per channel.

### ADR-2: Magic-Byte Sniffing Over Declared MIME

**Decision**: MIME type is determined by magic-byte sniffing of the actual payload, NOT by the
declared MIME type from the channel API.

**Rationale**: Channel APIs can declare incorrect MIME types (e.g., Telegram always declares
`image/jpeg` for photos even if the user sent PNG). Declared MIME is a hint only. Sniffing prevents
MIME confusion attacks where a malicious file is disguised with an image MIME type.

### ADR-3: RAII Temp File Cleanup

**Decision**: Use `StagedImageGuard` (Drop-based) for temp file cleanup rather than explicit cleanup
calls or TTL-based reapers.

**Rationale**: RAII guarantees cleanup on ALL exit paths including panics, timeouts, and early
returns. This is more reliable than manual cleanup and doesn't require a background reaper thread.
The minor risk of orphaned files on process crash is acceptable for MVP — a startup reaper can be
added later.

### ADR-4: Fail-Closed Default

**Decision**: Any channel that hasn't implemented `fetch_and_stage_image()` returns an empty Vec,
which triggers rejection at the caller level.

**Rationale**: Security-first. A new channel added to `allowed_channels` config should not silently
pass through image parts without actual staging. The empty Vec return is the signal that staging is
not implemented, triggering a user-facing "not yet supported" message.

### ADR-5: Single Dispatch Point

**Decision**: `stage_channel_images()` in `mod.rs` is the single dispatch point for all channel
image staging. Channels are dispatched by `msg.channel` string match.

**Rationale**: Centralizes the channel→staging mapping, makes it easy to audit which channels have
staging support, and ensures consistent error handling. Adding a new channel requires exactly one
new match arm.

## Observability (REQ-9)

Every image ingestion attempt emits an `ImageIngressEvent` through the `Observer` trait. Events are
fired at the **single dispatch point** (`stage_channel_images()` in `mod.rs`) so that every path —
success, validation failure, channel-not-allowed, provider error — is captured uniformly.

### Event structure

| Field          | Source                                       |
|----------------|----------------------------------------------|
| `channel`      | `msg.channel` string                         |
| `provider`     | Resolved vision route name (if available)    |
| `model`        | Resolved vision model (if available)         |
| `outcome`      | `Admitted` · `Rejected` · `ProviderSent` · `ProviderError` |
| `reason`       | Rejection reason (if rejected)               |
| `image_count`  | Number of images in the turn                 |
| `mime_type`    | Sniffed MIME (if fetch succeeded)            |
| `byte_len`     | Payload size in bytes (if fetch succeeded)   |

### Emission points

1. **After staging succeeds** → `Admitted` (one event per image)
2. **After provider dispatch succeeds** → `ProviderSent`
3. **On validation failure** (MIME, size, count) → `Rejected` with reason
4. **On channel-not-allowed** → `Rejected` with `ChannelNotAllowed`
5. **On provider error** → `ProviderError`

### Implementation note

The `Observer` trait already supports structured event emission. `ImageIngressEvent` is a new event
variant registered alongside existing observer events. No new infrastructure is required — channel
implementors call `observer.emit(ImageIngressEvent { ... })` at the appropriate point in the
pipeline.

## Staging File Naming Convention

```
{temp_dir}/corvus-{channel_abbrev}-img-{sha256_prefix}.{ext}

Examples:
  /tmp/corvus-tg-img-a1b2c3d4e5f6g7h8.jpg
  /tmp/corvus-wa-img-f8e7d6c5b4a3928d.png
  /tmp/corvus-dc-img-1234567890abcdef.webp
  /tmp/corvus-sl-img-abcdef1234567890.jpg
```

Channel abbreviations:

- `tg` = Telegram
- `wa` = WhatsApp
- `dc` = Discord
- `sl` = Slack

## Sequence: Telegram Image Ingestion

```
User                Telegram API        TelegramChannel       media.rs        Provider
 │                      │                    │                   │               │
 │──send photo──────────▶                    │                   │               │
 │                      │──webhook update────▶                   │               │
 │                      │                    │                   │               │
 │                      │                    │ parse_update_message()            │
 │                      │                    │ emit ContentPart::Image           │
 │                      │                    │                   │               │
 │                      │                    │ fetch_and_stage_image()           │
 │                      │◀──POST getFile─────│                   │               │
 │                      │──file_path────────▶│                   │               │
 │                      │◀──GET file─────────│                   │               │
 │                      │──bytes────────────▶│                   │               │
 │                      │                    │                   │               │
 │                      │                    │──validate_mime()──▶               │
 │                      │                    │◀──Ok(Jpeg)────────│               │
 │                      │                    │──validate_size()──▶               │
 │                      │                    │◀──Ok(())──────────│               │
 │                      │                    │                   │               │
 │                      │                    │ write temp file                   │
 │                      │                    │ return StagedImage                │
 │                      │                    │                   │               │
 │                      │                    │───────────────────────────────────▶
 │                      │                    │                   │   chat(images) │
 │◀─────────────────────────────────────────────────────────────── response      │
```

## Future Considerations

1. **Multi-image support**: Increase `MAX_IMAGES_PER_TURN` and update provider payloads
2. **GIF support**: Could add animated→static conversion or accept GIF MIME with first-frame
   extraction
3. **Startup reaper**: Glob for `corvus-*-img-*` in temp dir on startup, delete files older than N
   minutes
4. **URL-based transport**: For channels with public CDN URLs (Discord), skip download and pass URL
   directly to provider (if provider supports URL-based image input)
5. **Deduplication**: SHA-256 in filename enables dedup if the same image is sent multiple times
   within a turn
