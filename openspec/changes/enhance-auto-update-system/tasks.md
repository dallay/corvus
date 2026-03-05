# Tasks: Enhance Auto-Update System

## Phase Dependencies and Sequencing

- Phase 1 -> Phase 2: shared update policy/model and env parsing must exist before CLI/state wiring.
- Phase 2 -> Phase 3: canonical status and notification payload must be stable before
  channel/daemon/admin fan-out.
- Phase 3 -> Phase 4: unified surfaces and policy toggles must be in place before verification
  hardening/history UX.
- Phase 4 -> Phase 5: implementation is complete before end-to-end verification and regression.

## Phase 1: Policy and State Foundation (TDD)

- [x] 1.1 Add RED unit tests in `clients/agent-runtime/src/config/schema.rs` for new update
  fields/defaults and env override precedence (`CORVUS_UPDATES_ENABLED`,
  `CORVUS_UPDATE_AUTO_INSTALL`, `CORVUS_UPDATE_CHANNEL_VISIBILITY`, `CORVUS_UPDATE_CLI_NOTICE`,
  `CORVUS_UPDATE_METHOD_OVERRIDE`, `CORVUS_UPDATE_RESTART_POLICY`) including invalid-value fail-safe
  behavior.
- [x] 1.2 Implement GREEN schema updates in `clients/agent-runtime/src/config/schema.rs` for
  `auto_install_enabled`, `channel_visibility_enabled`, `cli_startup_notice_enabled`,
  `install_method_override`, `restart_policy`, and `history_max_entries` with safe defaults and
  validation.
- [x] 1.3 Add RED unit tests in `clients/agent-runtime/src/update/mod.rs` for policy resolution,
  install method precedence (`override -> detected -> unknown`), and install/check state transition
  invariants.
- [x] 1.4 Implement GREEN core model/types in `clients/agent-runtime/src/update/mod.rs` (
  `InstallMethod`, `RestartPolicy`, `UpdatePolicy`, `UpdateStateSnapshot`, `InstallState`,
  `CheckOutcome`, `UpdateStatusView`) and refactor duplicate policy/state mapping helpers (
  REFACTOR).

Verification criteria (Phase 1):

- New config/env tests pass and prove safe-by-default behavior.
- Update model/state tests pass and prove deterministic method and policy resolution.

## Phase 2: CLI Commands, Locking, and Atomic State (TDD)

- [x] 2.1 Add RED CLI command tests for `update status`, `update check`, and `update install`
  deterministic output/exit semantics in runtime command test coverage associated with
  `clients/agent-runtime/src/main.rs`.
- [x] 2.2 Implement GREEN `update status|check|install` command wiring and exit code mapping in
  `clients/agent-runtime/src/main.rs`, routed through `UpdateManager` entrypoints.
- [x] 2.3 Add RED concurrency/resilience tests in `clients/agent-runtime/src/update/mod.rs` (or
  update-focused runtime tests) for cross-process busy outcomes and interrupted-write recovery of
  `workspace/state/version_check.json`.
- [x] 2.4 Implement GREEN file-lock and atomic persistence flow in
  `clients/agent-runtime/src/update/mod.rs` (`update_state.lock`, `update_install.lock`, temp-file +
  fsync + rename + directory sync) plus single-install transaction guard.
- [x] 2.5 Implement deterministic install-method execution routing and unsupported fallback
  messaging in `clients/agent-runtime/src/update/mod.rs` without unsafe generic shell execution.

Dependencies:

- Depends on Phase 1 policy/model contracts.

Verification criteria (Phase 2):

- `update status|check|install` behavior is script-stable and test-covered.
- Concurrent install attempts serialize correctly; state file remains valid after simulated
  interruption.

## Phase 3: Multi-Surface Visibility and Policy Controls (TDD)

- [x] 3.1 Add RED tests in `clients/agent-runtime/src/channels/mod.rs` for channel visibility gating
  and canonical update payload parity with CLI status.
- [x] 3.2 Implement GREEN channel integration in `clients/agent-runtime/src/channels/mod.rs` so
  opportunistic mentions and nonce-confirm flow use canonical status/policy gates.
- [x] 3.3 Add RED daemon watcher tests in `clients/agent-runtime/src/daemon/mod.rs` for check
  interval behavior, deduped notifications, and policy-aware fan-out.
- [x] 3.4 Implement GREEN daemon updater integration in `clients/agent-runtime/src/daemon/mod.rs`
  using canonical update payload and shared manager APIs.
- [x] 3.5 Add RED admin contract tests in `clients/agent-runtime/src/gateway/admin.rs` and
  TypeScript compatibility checks in `clients/web/apps/dashboard/src/types/admin-config.ts` for
  `config.updates` status/policy fields.
- [x] 3.6 Implement GREEN admin API and dashboard type updates in
  `clients/agent-runtime/src/gateway/admin.rs` and
  `clients/web/apps/dashboard/src/types/admin-config.ts`, preserving secret-safe response
  discipline.
- [x] 3.7 Add RED/GREEN tasks in `clients/agent-runtime/src/main.rs` and
  `clients/agent-runtime/src/config/schema.rs` for `update auto-enable` and `update auto-disable`,
  ensuring persisted policy toggles are reflected in same-session `update status`.

Dependencies:

- Depends on Phase 2 canonical status contract and manager entrypoints.

Verification criteria (Phase 3):

- CLI, channel, daemon, and admin surfaces expose consistent version/policy facts.
- Policy toggles (`auto-enable/auto-disable`) persist atomically and reflect immediately.

## Phase 4: Integrity Verification, History, and Restart Safety (TDD)

- [x] 4.1 Add RED verification tests in `clients/agent-runtime/src/update/mod.rs` for
  checksum-required artifact paths, missing metadata failures, digest mismatch failures, and
  fail-closed install blocking.
- [x] 4.2 Implement GREEN verification gate and structured verification/install audit event
  recording in `clients/agent-runtime/src/update/mod.rs`.
- [x] 4.3 Add RED tests for `update history` ordering and schema expectations, then implement GREEN
  command + history reader wiring in `clients/agent-runtime/src/main.rs` and
  `clients/agent-runtime/src/update/mod.rs` backed by `workspace/state/update_history.jsonl`.
- [x] 4.4 Add RED restart-policy integration tests in `clients/agent-runtime/src/service/mod.rs` and
  daemon-facing update handling, then implement GREEN `InstalledPendingRestart` handling for
  `never|prompt|auto_managed_service` behavior.
- [x] 4.5 Refactor duplicated audit/restart decision code in
  `clients/agent-runtime/src/update/mod.rs`, `clients/agent-runtime/src/daemon/mod.rs`, and
  `clients/agent-runtime/src/service/mod.rs` while keeping event taxonomy stable.

Dependencies:

- Depends on Phase 3 fan-out/admin contract completion.

Verification criteria (Phase 4):

- Verification failures block activation and emit auditable failure events.
- `update history` returns chronological, structured check/install events.
- Restart handling avoids mixed-version running state for managed service mode.

## Phase 5: End-to-End Verification and Regression Gate

- [x] 5.1 Add/update focused integration tests under `clients/agent-runtime/tests/` for full command
  contract coverage (`status|check|install|auto-enable|auto-disable|history`) and concurrency
  outcomes.
- [x] 5.2 Add/update integration tests under `clients/agent-runtime/tests/` for cross-surface
  consistency (CLI status vs admin payload vs channel/daemon notification facts).
- [x] 5.3 Run targeted runtime verification (`cargo test -p agent-runtime update`) and dashboard
  type/build checks for `clients/web/apps/dashboard/src/types/admin-config.ts`, fixing regressions
  in touched files.
- [x] 5.4 Run full repository regression (`make test` and `make build`) and confirm every scenario
  in `openspec/changes/enhance-auto-update-system/specs/update-system/spec.md` is mapped to passing
  tests before handoff.

Dependencies:

- Depends on completion of Phases 1-4.

Verification criteria (Phase 5):

- All targeted and full regression suites pass.
- Each spec requirement/scenario has explicit test coverage evidence.
