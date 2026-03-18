# Tasks: First-Run Web Dashboard Activation

## Execution Order and Dependency Graph

- Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5 -> Phase 6
- Within each phase, tasks are ordered; a task may start only when its listed dependencies are done.
- Verification gates at the end of each phase are mandatory before moving forward.

## Phase 1: Baseline and Test Harness (RED)

- [x] 1.1 Capture current onboarding output/order baseline for decline-parity protection.
  - Depends on: none
  - Covers: RF3, NFR-C1, AC3
  - Tests: add/adjust onboarding snapshot/behavior tests proving current post-summary ordering
    before new prompt insertion.
  - Done when: tests fail for missing final prompt but pass for unchanged existing ordering up to
    insertion point.

- [x] 1.2 Add dashboard activation domain test scaffolding (decision, diagnosis, rendering).
  - Depends on: 1.1
  - Covers: RF1, RF2, RF4, RF5, NFR-R1, AC1, AC2, AC4, AC5
  - Tests: add unit test modules for decision branch, diagnosis mapping, and command-block rendering
    with stable assertions.
  - Done when: RED tests exist for accept/decline flow, diagnosis codes, and resume block presence.

- [x] 1.3 Add security-output guard tests for onboarding activation messaging.
  - Depends on: 1.2
  - Covers: NFR-S1, AC6, AC7
  - Tests: assertions that output never includes bearer tokens, token hashes, auth headers, or
    insecure admin API curl guidance.
  - Done when: tests fail until safe output implementation is present.

- [x] 1.4 Phase 1 verification gate.
  - Depends on: 1.1, 1.2, 1.3
  - Covers: AC1-AC7 (baseline readiness)
  - Tests: run targeted agent-runtime onboarding tests.
  - Done when: RED tests are intentional, deterministic, and isolated to unimplemented activation
    behavior.

## Phase 2: Final Prompt and Decline-Parity Flow

- [x] 2.1 Insert final optional prompt in onboarding at the designed insertion point.
  - Depends on: 1.4
  - Covers: RF1, AC1
  - Tests: prompt-order tests validate prompt appears after summary and optional channel flow.
  - Done when: prompt wording is optional and appears in correct sequence.

- [x] 2.2 Implement decline branch as no-mutation, CLI-only parity path.
  - Depends on: 2.1
  - Covers: RF3, NFR-C1, AC3
  - Tests: decline scenario tests validate no new mandatory actions and equivalent completion
    behavior.
  - Done when: decline path preserves existing next-step semantics and side-effect profile.

- [x] 2.3 Render concise resume-later block for decline path.
  - Depends on: 2.2
  - Covers: RF5, AC5
  - Tests: rendering tests assert commands include `corvus status`, `corvus gateway`,
    `make dev-up`, `./dev/cli.sh up-dashboard`, and proxied URL/pair reminder.
  - Done when: resume block is always shown for decline and commands are copy-paste ready.

- [x] 2.4 Phase 2 verification gate.
  - Depends on: 2.1, 2.2, 2.3
  - Covers: AC1, AC3, AC5
  - Tests: run targeted onboarding tests for prompt + decline scenarios.
  - Done when: all Phase 2 tests are green with no regressions in onboarding ordering.

## Phase 3: Accept Path Activation Guidance and Browser-Open Behavior

- [x] 3.1 Implement one-screen 3-5 step accept-path activation guide.
  - Depends on: 2.4
  - Covers: RF2, NFR-U1, AC2, AC7
  - Tests: output-format tests assert step count, canonical URLs, and pairing instructions via
    secure flow.
  - Done when: guidance is compact, actionable, and uses `http://corvus.localhost` + `/api`
    consistently.

- [x] 3.2 Add optional browser-open attempt targeting dashboard URL with non-fatal fallback.
  - Depends on: 3.1
  - Covers: RF2, NFR-R1, AC2
  - Tests: unit tests for opened/unsupported/failed-nonfatal outcomes and stable user messaging.
  - Done when: onboarding never fails due to browser-open limitations and always provides manual URL
    path.

- [x] 3.3 Ensure accept path always includes resume-later block.
  - Depends on: 3.1
  - Covers: RF5, AC5
  - Tests: accept-path render tests verify resume commands appear for both success and degraded
    outcomes.
  - Done when: resume instructions are emitted consistently and independently executable.

- [x] 3.4 Phase 3 verification gate.
  - Depends on: 3.1, 3.2, 3.3
  - Covers: AC2, AC5, AC7
  - Tests: run targeted onboarding accept-path unit tests.
  - Done when: accept-path presentation and browser fallback behavior are fully green.

## Phase 4: Deterministic Diagnosis Engine and Fallback Command Mapping

- [x] 4.1 Implement bounded local diagnosis probes and deterministic state mapping.
  - Depends on: 3.4
  - Covers: RF4, NFR-R1, AC4
  - Tests: mapper tests for gateway down, gateway up+unpaired, gateway up+paired, dashboard UI
    unavailable, unknown failure.
  - Done when: fixed probe order/timeouts (500 ms request timeout, one retry, <= 1.5 s budget)
    produce stable diagnosis results.

- [x] 4.2 Emit stable DASH status codes and concise cause line.
  - Depends on: 4.1
  - Covers: RF4, NFR-U1, AC4
  - Tests: output tests assert exact status code labels `DASH-001/002/003/004/999` and deterministic
    message shape.
  - Done when: each diagnosis prints one status line + one cause line in a grep-friendly format.

- [x] 4.3 Implement state-specific fallback command blocks (secure-only guidance).
  - Depends on: 4.2
  - Covers: RF4, NFR-S1, AC4, AC7
  - Tests: command-block tests assert per-state fallback commands and explicit exclusion of insecure
    direct `/web/admin/*` API usage.
  - Done when: each diagnosis has exact copy-paste recovery commands matching design mapping.

- [x] 4.4 Enforce security invariants in activation output path.
  - Depends on: 4.3
  - Covers: NFR-S1, AC6
  - Tests: redaction/no-secrets tests for all diagnosis and fallback branches.
  - Done when: no secret-bearing data can reach onboarding output in any tested branch.

- [x] 4.5 Phase 4 verification gate.
  - Depends on: 4.1, 4.2, 4.3, 4.4
  - Covers: AC4, AC6, AC7
  - Tests: run targeted onboarding diagnosis suite and integration-like local-state simulations.
  - Done when: all diagnosis states are deterministic, secure, and fully mapped to fallback
    commands.

## Phase 5: Documentation Updates

- [x] 5.1 Update onboarding docs with final prompt, accept/decline behavior, and canonical URLs.
  - Depends on: 4.5
  - Covers: RF1, RF2, RF3, NFR-U1, AC1, AC2, AC3
  - Tests: docs review checklist and command/link validation.
  - Done when: docs reflect implemented behavior and terminology exactly.

- [x] 5.2 Update dashboard activation troubleshooting and resume-later guidance.
  - Depends on: 5.1
  - Covers: RF4, RF5, AC4, AC5, AC7
  - Tests: verify each diagnosis state has corresponding manual fallback commands and resume steps
    in docs.
  - Done when: docs include deterministic diagnosis codes and secure recovery guidance.

- [x] 5.3 Phase 5 verification gate.
  - Depends on: 5.1, 5.2
  - Covers: AC1-AC5, AC7
  - Tests: markdown lint/check + manual command walkthrough review.
  - Done when: documentation is complete, consistent, and executable by a first-run user.

## Phase 6: End-to-End Verification and Handoff Gates

- [x] 6.1 Execute targeted test stack for onboarding activation.
  - Depends on: 5.3
  - Covers: AC1-AC7
  - Tests: agent-runtime onboarding unit/integration/E2E scenario coverage (A-D) per spec.
  - Done when: all activation-related tests are green in CI-equivalent local run.

- [x] 6.2 Run broader regression gate for touched surfaces.
  - Depends on: 6.1
  - Covers: NFR-C1, NFR-R1
  - Tests: relevant module test suite and project-standard checks for modified areas.
  - Done when: no regressions are introduced outside activation flow.

- [x] 6.3 Traceability and artifact closure gate.
  - Depends on: 6.1, 6.2
  - Covers: RF1-RF5, NFR-S1/U1/R1/C1, AC1-AC7
  - Tests: update this tasks file with completion marks and confirm each requirement has a passing
    test reference.
  - Done when: requirement-to-task-to-test trace is complete and implementation is ready for
    verify/archive phases.

## Requirement and Acceptance Coverage Index

- RF1 / AC1 -> 2.1, 5.1, 6.1
- RF2 / AC2 -> 3.1, 3.2, 5.1, 6.1
- RF3 / AC3 -> 1.1, 2.2, 5.1, 6.1
- RF4 / AC4 -> 4.1, 4.2, 4.3, 5.2, 6.1
- RF5 / AC5 -> 2.3, 3.3, 5.2, 6.1
- NFR-S1 / AC6 -> 1.3, 4.4, 6.1
- AC7 (secure canonical guidance) -> 3.1, 4.3, 5.2, 6.1

## Phase 6 Verification Evidence

- Targeted activation tests: `cargo test dashboard_` and
  `cargo test dashboard_resume_status_lines_include_help_and_secure_pairing_path`.
- Broader touched-surface regression: `pnpm --filter @corvus/docs run check`.
- Requirement traceability check (RF1-RF5, NFR-S1/U1/R1/C1, AC1-AC7):
  - Prompt + accept/decline + canonical URLs: onboarding/dashboard wizard tests covered by
    `cargo test dashboard_`.
  - Deterministic diagnosis (`DASH-001/002/003/004/999`) + secure fallback blocks + resume path:
    onboarding/dashboard wizard tests covered by `cargo test dashboard_`.
  - Security invariants (no token/bearer/admin bypass leakage): dashboard security-output tests in
    onboarding wizard suite covered by `cargo test dashboard_`.
  - Status resume handoff output:
    `cargo test dashboard_resume_status_lines_include_help_and_secure_pairing_path`.
  - Docs correctness for Getting Started and CLI reference updates: `pnpm --filter @corvus/docs run check`.
