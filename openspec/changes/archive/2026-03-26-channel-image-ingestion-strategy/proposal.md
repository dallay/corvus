# Proposal: Channel Image Ingestion Strategy

**Change**: channel-image-ingestion-strategy
**Issue**: #266
**Date**: 2026-03-26
**Status**: archived

## Intent

Define a consistent, documented strategy for how Corvus channels ingest user-sent images and
translate them into the runtime's multimodal contract. This closes the design gap identified in #266
and enables clean follow-up implementation issues for additional channels.

## Scope

### In Scope

- Codify the Telegram and WhatsApp ingestion patterns as the canonical reference
- Define ingestion contracts for Discord and Slack (next-wave channels)
- Specify file staging location, naming, retention, and cleanup semantics
- Specify size limits, MIME validation, and per-turn image count constraints
- Document the exact runtime handoff format (`ContentPart::Image` → `StagedImage` →
  `Provider::chat()`)
- Define the config gating model (`multimodal.*` section)

### Out of Scope

- Actual implementation of Discord/Slack image ingestion (follow-up issues)
- Multi-image per turn support (future enhancement)
- GIF support or image format conversion
- Outbound image generation or sending
- Video, audio, or document ingestion

## Approach

1. **Document the existing pattern** — Telegram and WhatsApp already follow a consistent 5-step
   flow: parse → emit ContentPart::Image → fetch bytes → validate (MIME sniff + size) → stage to
   temp. Codify this as the canonical ingestion pipeline.

2. **Define channel-specific contracts** — For each MVP and next-wave channel, document: what
   inbound message types carry images, what metadata is available, how to fetch bytes, and what auth
   is required.

3. **Standardize staging** — All channels MUST use the same temp directory, naming convention, RAII
   cleanup, and SHA-256 dedup-ready naming.

4. **Config gating** — The `multimodal.allowed_channels` allowlist is the single point of channel
   enablement. Channels not in the list fail-closed with a user-facing message.

## Affected Modules

- `clients/agent-runtime/src/channels/traits.rs` — contract types (no changes needed)
- `clients/agent-runtime/src/channels/media.rs` — validation functions (no changes needed)
- `clients/agent-runtime/src/channels/mod.rs` — `stage_channel_images()` dispatch (extend for new
  channels)
- `clients/agent-runtime/src/channels/discord.rs` — needs `fetch_and_stage_image()` (follow-up)
- `clients/agent-runtime/src/channels/slack.rs` — needs `fetch_and_stage_image()` (follow-up)
- `clients/agent-runtime/src/config/schema.rs` — multimodal config (may need allowed_channels
  update)

## Rollback Plan

This change is documentation/specification only. No code changes. Rollback = revert the openspec
artifacts.

## Risks

| Risk                                                    | Likelihood | Impact | Mitigation                                                         |
|---------------------------------------------------------|------------|--------|--------------------------------------------------------------------|
| Discord/Slack APIs change image access patterns         | Low        | Medium | Contracts reference API version; update spec when implementing     |
| Orphaned temp files on crash                            | Medium     | Low    | Document startup reaper as follow-up enhancement                   |
| MAX_IMAGES_PER_TURN=1 too restrictive for some channels | Medium     | Low    | Spec explicitly marks this as MVP constraint with planned increase |
