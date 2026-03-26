# Follow-Up Issues for Channel Image Ingestion Strategy

These issues should be created on GitHub from the Phase 2 task definitions.
The token used lacked `repo` write permissions — create these manually or with an appropriate token.

---

## Issue 1: Discord image ingestion

**Title**: `feat: Discord image ingestion`
**Labels**: `type|enhancement`

### Goal
Implement image ingestion for the Discord channel following the canonical pipeline defined in the [channel image ingestion strategy](specs/channel-image-ingestion/spec.md).

### Context
- Strategy spec: #266
- Discord messages have `attachments` with `content_type`, `size`, `filename`, `url`
- Attachment URLs are pre-authenticated CDN links (no auth needed for download)
- Caption = message text content

### Tasks
- [ ] Parse Discord message attachments with image `content_type` into `ContentPart::Image`
- [ ] Implement `DiscordChannel::fetch_and_stage_image()` (direct CDN download, no auth)
- [ ] Add `"discord"` match arm in `stage_channel_images()`
- [ ] Add `"discord"` to valid MVP channel names in config validation
- [ ] Add tests for Discord image parsing and staging

### Acceptance Criteria
- Discord image messages produce `ContentPart::Image` parts
- Images are fetched, validated (MIME sniff + size), and staged to temp
- Config gating works (`multimodal.allowed_channels` must include `"discord"`)
- `ImageIngressEvent` observability events are emitted
- Existing text-only Discord behavior is unchanged

---

## Issue 2: Slack image ingestion

**Title**: `feat: Slack image ingestion`
**Labels**: `type|enhancement`

### Goal
Implement image ingestion for the Slack channel following the canonical pipeline defined in the [channel image ingestion strategy](specs/channel-image-ingestion/spec.md).

### Context
- Strategy spec: #266
- Slack files shared in channels have `url_private_download`, `mimetype`, `size`, `name`
- Download requires bearer auth (bot token)
- Requires `files:read` OAuth scope
- Caption = message text

### Tasks
- [ ] Parse Slack `file_shared` events / message `files` array into `ContentPart::Image`
- [ ] Implement `SlackChannel::fetch_and_stage_image()` (bearer auth download)
- [ ] Add `"slack"` match arm in `stage_channel_images()`
- [ ] Add `"slack"` to valid MVP channel names in config validation
- [ ] Add tests for Slack image parsing and staging
- [ ] Ensure `files:read` OAuth scope is documented in Slack setup guide

### Acceptance Criteria
- Slack image file shares produce `ContentPart::Image` parts
- Images are fetched with bearer auth, validated (MIME sniff + size), and staged
- Config gating works (`multimodal.allowed_channels` must include `"slack"`)
- `ImageIngressEvent` observability events are emitted
- Existing text-only Slack behavior is unchanged

---

## Issue 3: Startup temp file reaper for staged images

**Title**: `feat: startup temp file reaper for staged images`
**Labels**: `type|enhancement`

### Goal
Add a startup cleanup routine that removes orphaned staged image temp files from previous process crashes.

### Context
- Strategy spec: #266
- Staged images use RAII cleanup (`StagedImageGuard`), but process crashes leave orphans
- Temp files follow pattern: `corvus-{channel}-img-{hash}.{ext}` in `std::env::temp_dir()`

### Tasks
- [ ] On startup, glob `corvus-*-img-*` in `std::env::temp_dir()`
- [ ] Delete files older than configurable threshold (default: 30 minutes)
- [ ] Log count of cleaned files at info level
- [ ] Add tests for reaper logic

### Acceptance Criteria
- Orphaned temp files from previous crashes are cleaned on startup
- Only files matching the Corvus staging pattern are deleted
- Age threshold is configurable (or uses sensible default)
- Fresh files from a concurrent process are not deleted

---

## Issue 4: Multi-image per turn support

**Title**: `feat: multi-image per turn support`
**Labels**: `type|enhancement`

### Goal
Increase the per-turn image limit from 1 to a configurable value (default: 4) to support channels where users commonly send multiple images.

### Context
- Strategy spec: #266
- Current limit: `MAX_IMAGES_PER_TURN = 1`
- Some providers (GPT-4o, Claude) support multiple images per request

### Tasks
- [ ] Make `MAX_IMAGES_PER_TURN` configurable (default: 4)
- [ ] Update provider payloads to handle multiple images
- [ ] Update observability events for multi-image turns
- [ ] Add tests for multi-image scenarios

### Acceptance Criteria
- Users can send up to N images per message (configurable)
- All images are individually validated and staged
- Provider receives all staged images in a single request
- Observability events reflect actual image count
- Backward compatible: single-image behavior unchanged
