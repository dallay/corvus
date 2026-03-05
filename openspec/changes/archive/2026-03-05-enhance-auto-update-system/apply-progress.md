# Apply Progress: enhance-auto-update-system

## Completed

- Phase 1: update policy/config schema extensions, env override parsing/validation, and core update models.
- Phase 2: `corvus update` CLI command tree (`status|check|install|auto-enable|auto-disable|history`), file-locking, atomic state write path, install-method routing.
- Phase 3: channel visibility gate wiring, daemon watcher interval/dedupe tests, daemon updater manager integration, admin update contract parity, and dashboard type updates.
- Phase 4: checksum verification fail-closed behavior, update audit history persistence/reading, restart policy handling, and shared restart/audit decision helper refactor.
- Phase 5: integration test expansion for command contract, lock/concurrency outcomes, and CLI/admin parity, plus targeted and full regression checks.

## Incomplete

- None.

## Notes

- Existing update flows were preserved where possible; new command/status path is now manager-driven.
- Task checkboxes were updated in `tasks.md`; all tasks for this change are marked complete.
- Runtime integration coverage now includes `clients/agent-runtime/tests/update_system_integration.rs` and updated admin integration assertions.
- Verify-gap follow-up: admin update status now includes `last_check_at_unix` and `last_check_outcome` with CLI parity assertions.
- Verify-gap follow-up: installation-method detection now uses runtime heuristics (npm/pnpm/yarn/bun, homebrew, cargo, script/binary) instead of test-only override.
- Verify-gap follow-up: CLI now supports `corvus update confirm <nonce>` and appends `confirm_install` audit events.
- Dashboard check (`pnpm --filter @corvus/dashboard run check`) still reports pre-existing Biome issues in unrelated config Vue files; update-related type file check passes.
- Final verify-gap follow-up: added explicit verification-success test coverage asserting activation success and audit history entries for both `verification:success` and `install:success`.
