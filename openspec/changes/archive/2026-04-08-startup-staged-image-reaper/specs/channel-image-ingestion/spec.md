# Delta for channel-image-ingestion

## MODIFIED Requirements

### Requirement: REQ-7: File Staging and Cleanup

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

(Previously: staged-image cleanup was defined only in terms of `StagedImageGuard` RAII semantics on
request exit paths.)

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
