# Tasks: Channel Image Ingestion Strategy

**Change**: channel-image-ingestion-strategy
**Issue**: #266
**Date**: 2026-03-26

## Phase 1: Documentation (this change — spec-only)

- [x] 1.1 Explore current codebase state for multimodal image ingestion
- [x] 1.2 Write proposal with scope, approach, and affected modules
- [x] 1.3 Write spec with requirements, scenarios, and channel contracts
- [x] 1.4 Write design with architecture, ADRs, and sequence diagrams
- [x] 1.5 Write tasks breakdown with follow-up issue definitions

## Phase 2: Follow-Up Implementation Issues (to be created)

These are discrete implementation issues that can be created from this strategy:

### Issue: Discord image ingestion (#next)

- [ ] 2.1 Parse Discord message attachments with image content_type into `ContentPart::Image`
- [ ] 2.2 Implement `DiscordChannel::fetch_and_stage_image()` (direct CDN download, no auth)
- [ ] 2.3 Add `"discord"` match arm in `stage_channel_images()`
- [ ] 2.4 Add `"discord"` to valid MVP channel names in config validation
- [ ] 2.5 Add tests for Discord image parsing and staging

### Issue: Slack image ingestion (#next+1)

- [ ] 2.6 Parse Slack file_shared events / message files array into `ContentPart::Image`
- [ ] 2.7 Implement `SlackChannel::fetch_and_stage_image()` (bearer auth download)
- [ ] 2.8 Add `"slack"` match arm in `stage_channel_images()`
- [ ] 2.9 Add `"slack"` to valid MVP channel names in config validation
- [ ] 2.10 Add tests for Slack image parsing and staging
- [ ] 2.11 Ensure `files:read` OAuth scope is documented in Slack setup guide

### Issue: Startup temp file reaper (#next+2)

- [ ] 2.12 On startup, glob `corvus-*-img-*` in `std::env::temp_dir()`
- [ ] 2.13 Delete files older than configurable threshold (default: 30 minutes)
- [ ] 2.14 Log count of cleaned files at info level
- [ ] 2.15 Add tests for reaper logic

### Issue: Multi-image per turn (#future)

- [ ] 2.16 Increase `MAX_IMAGES_PER_TURN` (configurable, default: 4)
- [ ] 2.17 Update provider payloads to handle multiple images
- [ ] 2.18 Update observability events for multi-image turns
- [ ] 2.19 Add tests for multi-image scenarios

## Acceptance Criteria Mapping

| Acceptance Criterion                                           | Addressed By                                          |
|----------------------------------------------------------------|-------------------------------------------------------|
| Initial channel list is defined                                | REQ-1 (Telegram, WhatsApp MVP; Discord, Slack Wave 2) |
| Channel-specific ingest behavior is defined                    | Channel-Specific Contracts section + REQ-2 pipeline   |
| File staging and retention expectations are defined            | REQ-7, Design ADR-3, staging naming convention        |
| Follow-up channel implementation issues can be created cleanly | Phase 2 task breakdown above                          |
