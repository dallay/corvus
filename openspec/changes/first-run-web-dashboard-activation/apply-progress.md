# Apply Progress: first-run-web-dashboard-activation

## Status

Completed (Phase 6 handoff gate passed).

## Completed in this execution batch

- [x] 5.1 Update onboarding docs with final prompt, accept/decline behavior, and canonical URLs.
- [x] 5.2 Update dashboard activation troubleshooting and resume-later guidance.
- [x] 5.3 Phase 5 verification gate.
- [x] 6.1 Execute targeted test stack for onboarding activation.
- [x] 6.2 Run broader regression gate for touched surfaces.
- [x] 6.3 Traceability and artifact closure gate.

## What was implemented

- Updated Getting Started docs with the final optional activation prompt wording,
  accept/decline behavior, canonical local URLs, and secure resume commands.
- Updated CLI reference with interactive `onboard` activation behavior,
  deterministic `DASH-*` diagnosis codes, safe fallback commands, and `status`
  resume-handoff output details.
- Added explicit troubleshooting/resume guidance that remains token-safe and avoids
  insecure `/web/admin/*` bypass patterns.
- Closed Phase 6 by recording requirement-to-test traceability in `tasks.md` and
  validating docs + targeted activation tests.

## Tests run

- `cargo test dashboard_`
  - Passed: 16 dashboard onboarding tests in `src/lib.rs` and 17 in `src/main.rs` (all green).
- `cargo test dashboard_resume_status_lines_include_help_and_secure_pairing_path`
  - Passed: 1 status resume-handoff unit test in `src/main.rs` (green).
- `pnpm --filter @corvus/docs run check`
  - Passed: Biome check for docs app (`Checked 6 files`, no issues).

## Remaining tasks

- [x] None (Phases 1-6 complete for this change scope).

## Notes

- Security-first constraint maintained: no secret, bearer token, auth header, token hash, or
  insecure `/web/admin/*` guidance is introduced by these docs updates.
- Compatibility remains backward-safe: decline path and CLI-first behavior are documented as
  unchanged while adding optional activation/troubleshooting guidance.
