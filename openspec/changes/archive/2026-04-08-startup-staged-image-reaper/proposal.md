# Proposal: Startup Staged Image Reaper

**Change**: `startup-staged-image-reaper`
**Issue**: #330 — startup temp file reaper for staged images
**Date**: 2026-04-08

## Intent

Corvus already stages inbound images into `std::env::temp_dir()` and relies on request-path cleanup
(`StagedImageGuard`) to remove them. That covers the happy path, but it does not clean up orphaned
 temp files left behind after crashes, forced shutdowns, or interrupted startup flows. Over time,
those stale files accumulate in the system temp directory.

This change adds a startup reaper that scans the OS temp directory for Corvus-staged image files,
deletes files older than a configurable threshold, and reports how many files were cleaned. The goal
is to restore temp-dir hygiene without widening the deletion surface beyond Corvus-owned naming
patterns.

## Scope

### In Scope

- Add a staged-image startup reaper owned by `clients/agent-runtime/src/channels/media.rs`
- Support both the current shared filename format (`corvus-{channel}-img-...`) and the legacy
  Telegram format (`corvus-tg-img-...`)
- Run the reaper from command-level startup hooks in `clients/agent-runtime/src/main.rs` for
  `gateway`, `daemon`, channel start paths, and onboarding autostart
- Add optional config `multimodal.staged_image_reaper_threshold_minutes` with a safe default of 30
- Log cleaned file count at info level
- Add focused tests for filename matching, age threshold behavior, legacy compatibility, and
  duplicate execution safety

### Out of Scope

- Reaping non-image temp files (audio, gateway uploads, provider caches, generic temp artifacts)
- Background or periodic cleanup loops after startup
- New temp-file directory layout outside `std::env::temp_dir()`
- Cross-process locking or distributed coordination beyond local best-effort cleanup

## Approach

Implement a small, file-pattern-constrained reaper in `channels/media.rs` so the ownership stays
close to the staged-image lifecycle code. The reaper will enumerate `std::env::temp_dir()`, keep
only files matching Corvus staged-image naming conventions, inspect file modification time, and
best-effort delete entries older than the configured threshold.

`main.rs` will invoke this reaper at command startup boundaries where staged-image temp files are
relevant: gateway startup, daemon startup, explicit channel start, and onboarding autostart.
Configuration remains intentionally small: `multimodal.staged_image_reaper_threshold_minutes` is
optional, defaults to 30, and acts only as an age threshold. Logging should emit the cleaned file
count at info level so operators can confirm cleanup happened without exposing file names.

Because temp-file cleanup is already best-effort, duplicate execution should be handled safely:
missing files during deletion must not fail startup, and stale timestamp edge cases should bias
toward non-deletion rather than aggressive cleanup.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/channels/media.rs` | Modified | Add staged-image filename matching, age threshold evaluation, and best-effort startup reaper implementation |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Add optional `multimodal.staged_image_reaper_threshold_minutes` config with defaulted behavior and validation |
| `clients/agent-runtime/src/main.rs` | Modified | Invoke reaper in command-level startup hooks for gateway, daemon, channel start, and onboarding autostart |
| `clients/agent-runtime/src/channels/telegram.rs` | Reference only | Legacy filename format remains supported by reaper matching rules |
| `openspec/changes/startup-staged-image-reaper/` | New | Proposal, followed by spec/design/tasks artifacts for the change |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| False-positive deletion removes non-Corvus files | Medium | Match only explicit Corvus staged-image filename patterns and known image extensions; if uncertain, skip deletion |
| Duplicate execution races on the same file | Medium | Treat `NotFound`/best-effort delete failures as non-fatal and keep the operation idempotent |
| Cross-platform `mtime` behavior is inconsistent or unavailable | Medium | Use conservative age checks, skip files whose age cannot be determined reliably, and cover edge cases with tests |
| Startup hook coverage misses one entry path | Low | Centralize invocation in command-level startup flow and verify all required commands in tests/review |

## Rollback Plan

1. Revert the startup hook calls in `clients/agent-runtime/src/main.rs` to stop invoking the reaper.
2. Revert the reaper helpers in `clients/agent-runtime/src/channels/media.rs`; request-path RAII
   cleanup remains unchanged.
3. Remove `multimodal.staged_image_reaper_threshold_minutes` from config parsing/validation.
4. If needed before code revert, operators can effectively disable cleanup by setting the threshold
   to a very large value once the config contract is finalized.

## Dependencies

- Existing staged-image lifecycle in `clients/agent-runtime/src/channels/media.rs`
- Existing legacy Telegram staged-image filename convention in
  `clients/agent-runtime/src/channels/telegram.rs`
- No new external services or third-party runtime dependencies expected

## Success Criteria

- [ ] Corvus deletes orphaned staged image temp files older than the effective threshold during
      startup
- [ ] The reaper recognizes both current shared staged-image filenames and legacy Telegram staged
      image filenames
- [ ] Files that do not match Corvus staged-image conventions are never targeted for deletion
- [ ] Cleaned file count is logged at info level on startup reaper execution
- [ ] Duplicate or repeated startup execution remains safe and non-fatal
- [ ] Focused tests cover threshold handling, filename matching, legacy compatibility, and safe
      failure behavior
