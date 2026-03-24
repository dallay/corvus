## Verification Report

**Change**: `2026-03-23-unify-onboarding-pairing-flow`
**Artifact store**: `openspec`
**Date**: 2026-03-24

---

### Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 23 |
| Tasks complete | 23 |
| Tasks incomplete | 0 |

All tasks in `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/tasks.md` are marked complete.

---

### Build & Tests Execution

**Build**: ✅ Passed

Command: `make build`

Result:
- Exit code `0`.
- The prior `:composeApp:spotlessKotlinCheck` blocker is cleared.
- `spotlessCheck`, `qualityCheck`, Compose/KMP JVM tests, Android lint/build work, and web checks all completed successfully inside the configured build stack.

**Configured test stack**: ✅ Passed

Command: `make test`

Result:
- Exit code `0`.
- The configured Gradle `test` task completed successfully for the repository's JVM/KMP test stack.
- Gradle's default console output did not emit a single aggregate passed/failed total, so affected-surface totals were validated with supplemental commands below.

**Supplemental Rust verification**: ✅ Passed

Command: `make rust-test`

Result:
- Exit code `0`.
- No Rust test failures were reported.
- Cargo output included `2545 passed; 0 failed` for the core library shard, with all remaining runtime/integration shards green.

**Supplemental web verification**: ✅ 70 passed / 0 failed / 0 skipped

Command: `make web-test-all`

Result:
- Chat: `22` passed across `4` test files.
- Dashboard: `48` passed across `10` test files.
- Docs and marketing have no web test suites in this target and were skipped by design.

**Targeted remediation proofs**: ✅ Passed

Commands:
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml cli_runtime_ready_state_is_operator_only_and_uses_host_trust`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml dashboard_accept_path_guide_uses_canonical_sequence_and_terms`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml cli_operator_sequence_stops_before_session_creation_and_ends_with_operator_tasks`
- `./gradlew :composeApp:jvmTest --tests "*OnboardingDefaultsTest*"`
- `./gradlew :agent-core-kmp:jvmTest --tests "*RustCliBridgeTest*"`
- `pnpm --dir clients/web/apps/chat exec vitest --run src/onboardingContract.spec.ts --reporter=verbose`
- `pnpm --dir clients/web/apps/chat exec vitest --run src/composables/useGateway.spec.ts --reporter=verbose`
- `pnpm --dir clients/web/apps/dashboard exec vitest --run --environment happy-dom src/composables/useConfig.spec.ts --reporter=verbose`

Result:
- Rust CLI targeted proofs: `3/3` named onboarding tests passed.
- Web chat contract proofs: `5/5` passed.
- Web chat gateway proofs: `7/7` passed.
- Dashboard onboarding proofs: `30/30` passed.
- ComposeApp and shared bridge targeted Gradle test tasks passed.

**Coverage**: ✅ Threshold satisfied

Command: `make test-coverage`

Result:
- Exit code `0`.
- Threshold configured: `60%` in `openspec/config.yaml:36`.
- Chat coverage: `82.76%` statements.
- Dashboard coverage: `87.74%` statements.
- `modules/agent-core-kmp/build/reports/kover/html/index.html` reports `96%` coverage.
- Rust LCOV report emitted to `coverage/agent-runtime-coverage.lcov`.

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Onboarding | New operator follows the canonical sequence from the CLI | `clients/agent-runtime/src/onboard/wizard.rs > cli_runtime_ready_state_is_operator_only_and_uses_host_trust`; `clients/agent-runtime/src/onboard/wizard.rs > dashboard_accept_path_guide_uses_canonical_sequence_and_terms`; `clients/agent-runtime/src/onboard/wizard.rs > cli_operator_sequence_stops_before_session_creation_and_ends_with_operator_tasks` | ✅ COMPLIANT |
| Onboarding | New end-user follows the canonical sequence from a chat surface | `clients/web/apps/chat/src/onboardingContract.spec.ts > keeps operator and chat intent selection explicit before transport checks`; `clients/web/apps/chat/src/App.spec.ts > pairs, gates on session start, and sends chat turns with bearer and session headers`; `clients/web/apps/chat/src/composables/useChat.spec.ts > enters session_pending only after gateway readiness and creates a UUID session`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should keep mobile chat intent explicit through session-first onboarding states` | ✅ COMPLIANT |
| Onboarding | Shared steps map to the same user outcomes across surfaces | `clients/web/apps/chat/src/onboardingContract.spec.ts > maps shared onboarding steps to the same user outcomes across web surfaces`; `clients/web/apps/chat/src/onboardingContract.spec.ts > keeps broader cross-surface same-outcome parity through shared web adapters`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should keep mobile outcomes aligned with the broader shared onboarding matrix` | ✅ COMPLIANT |
| Onboarding | Operator surface stops before session creation | `clients/agent-runtime/src/onboard/wizard.rs > cli_operator_sequence_stops_before_session_creation_and_ends_with_operator_tasks`; `clients/web/apps/dashboard/src/App.spec.ts > renders operator-ready completion copy when the dashboard is ready` | ✅ COMPLIANT |
| Onboarding | HTTP surface completes trust by pairing | `clients/web/apps/chat/src/composables/useGateway.spec.ts > maps /health -> /pair -> ready using the canonical HTTP onboarding states`; `clients/web/apps/dashboard/src/composables/useConfig.spec.ts > supports pairing without auto-connect when requested` | ✅ COMPLIANT |
| Onboarding | Mobile surface completes trust by linking | `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should keep mobile trust copy on linking instead of HTTP pairing` | ✅ COMPLIANT |
| Onboarding | Product copy distinguishes pairing from linking | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_accept_path_guide_uses_canonical_sequence_and_terms`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should keep mobile trust copy on linking instead of HTTP pairing` | ✅ COMPLIANT |
| Onboarding | Transport validation uses the correct connection term | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_http_activation_always_uses_gateway_connection_term`; `clients/web/apps/chat/src/composables/useGateway.spec.ts > maps /health -> /pair -> ready using the canonical HTTP onboarding states` | ✅ COMPLIANT |
| Onboarding | CLI completion includes optional dashboard continuation | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_accept_path_always_contains_resume_commands`; `clients/agent-runtime/src/onboard/wizard.rs > dashboard_decline_branch_keeps_cli_only_output_by_default` | ✅ COMPLIANT |
| Onboarding | Chat surface completion requires session entry | `clients/web/apps/chat/src/App.spec.ts > pairs, gates on session start, and sends chat turns with bearer and session headers`; `clients/web/apps/chat/src/composables/useChat.spec.ts > enters session_pending only after gateway readiness and creates a UUID session`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should validate ready and session outcomes through the approved cli bridge transport` | ✅ COMPLIANT |
| Onboarding | HTTP pairing failure maps to a normalized recovery state | `clients/web/apps/chat/src/composables/useGateway.spec.ts > maps expired pairing input to the normalized trust recovery state`; `clients/web/apps/dashboard/src/composables/useConfig.spec.ts > maps expired pairing input to the normalized trust recovery state` | ✅ COMPLIANT |
| Onboarding | Mobile bridge failure maps to a normalized recovery state | `modules/agent-core-kmp/src/jvmTest/kotlin/com/profiletailors/agent/core/RustCliBridgeTest.kt > should classify linked bridge without session capability as blocked recovery` | ✅ COMPLIANT |
| Onboarding | Dashboard retry guidance preserves secure HTTP pairing | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_status_fallback_commands_match_secure_mapping`; `clients/agent-runtime/src/onboard/wizard.rs > dashboard_render_output_never_contains_sensitive_headers_or_token_values` | ✅ COMPLIANT |
| Onboarding | Mobile retry guidance preserves bridge-only transport | `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should keep mobile recovery guidance on the approved bridge path` | ✅ COMPLIANT |
| Onboarding | Product onboarding defers transport authority to client-surfaces | `clients/web/apps/chat/src/onboardingContract.spec.ts > defers transport and dashboard activation authority to the governing specs` | ✅ COMPLIANT |
| Onboarding | Product onboarding defers operator activation details to dashboard | `clients/web/apps/chat/src/onboardingContract.spec.ts > defers transport and dashboard activation authority to the governing specs` | ✅ COMPLIANT |
| Client Surfaces | Web dashboard aligns onboarding without changing transport | `clients/web/apps/dashboard/src/composables/useConfig.spec.ts > maps initial fetch response and options`; `clients/web/apps/dashboard/src/App.spec.ts > renders operator-ready completion copy when the dashboard is ready` | ✅ COMPLIANT |
| Client Surfaces | Mobile aligns onboarding without adopting HTTP pairing language | `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should keep mobile trust copy on linking instead of HTTP pairing`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should validate ready and session outcomes through the approved cli bridge transport` | ✅ COMPLIANT |
| Client Surfaces | Web and mobile expose comparable recovery states | `clients/web/apps/chat/src/onboardingContract.spec.ts > keeps web and mobile recovery labels comparable through normalized product taxonomy`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should expose normalized recovery labels comparable to web chat` | ✅ COMPLIANT |
| Client Surfaces | Operator surfaces expose operator-relevant recovery states | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_statuses_map_to_shared_onboarding_states`; `clients/web/apps/dashboard/src/composables/useConfig.spec.ts > preserves trust progress when the dashboard is paired but the gateway cannot complete auth fetches` | ✅ COMPLIANT |
| Client Surfaces | Onboarding validates readiness through the approved transport | `clients/web/apps/chat/src/composables/useGateway.spec.ts > maps /health -> /pair -> ready using the canonical HTTP onboarding states`; `clients/web/apps/dashboard/src/composables/useConfig.spec.ts > auto-connects after pairing with same-origin proxied api endpoints by default`; `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt > should validate ready and session outcomes through the approved cli bridge transport` | ✅ COMPLIANT |
| Dashboard | Dashboard activation remains an operator slice of shared onboarding | `clients/web/apps/chat/src/onboardingContract.spec.ts > defers transport and dashboard activation authority to the governing specs`; `clients/agent-runtime/src/onboard/wizard.rs > dashboard_accept_path_guide_uses_canonical_sequence_and_terms` | ✅ COMPLIANT |
| Dashboard | Dashboard recovery language matches shared taxonomy | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_statuses_map_to_shared_onboarding_states`; `clients/agent-runtime/src/onboard/wizard.rs > blocked_dashboard_output_prints_normalized_recovery_kind` | ✅ COMPLIANT |
| Dashboard | Accepted activation uses canonical terminology and sequence | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_accept_path_guide_uses_canonical_sequence_and_terms` | ✅ COMPLIANT |
| Dashboard | Dashboard diagnosis maps to shared recovery states | `clients/agent-runtime/src/onboard/wizard.rs > dashboard_statuses_map_to_shared_onboarding_states`; `clients/agent-runtime/src/onboard/wizard.rs > dashboard_status_fallback_commands_match_secure_mapping` | ✅ COMPLIANT |

**Compliance summary**: 25/25 scenarios compliant

---

### Correctness (Static - Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| Canonical onboarding contract exists | ✅ Implemented | Shared `SurfaceId`, `TrustMode`, `TransportMode`, `RecoveryKind`, and `OnboardingState` primitives remain present in `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CoreContracts.kt`. |
| HTTP onboarding normalization exists | ✅ Implemented | `/health`, `/pair`, and authenticated follow-up outcomes still normalize into canonical trust/readiness/recovery states in `clients/agent-runtime/src/gateway/mod.rs` and `clients/agent-runtime/src/gateway/utils.rs`. |
| CLI/runtime intent and canonical sequencing are explicit | ✅ Implemented | `clients/agent-runtime/src/onboard/wizard.rs` now exposes explicit operator intent selection, host-trust ready state, canonical dashboard sequence copy, and normalized recovery mapping. |
| Dashboard onboarding adapter aligns | ✅ Implemented | `clients/web/apps/dashboard/src/composables/useConfig.ts` preserves pairing-code ephemerality, bearer-token-only persistence, canonical HTTP recovery mapping, and operator-ready state. |
| Web chat onboarding/session split aligns | ✅ Implemented | `clients/web/apps/chat/src/composables/useGateway.ts` and `clients/web/apps/chat/src/composables/useChat.ts` separate HTTP trust/readiness from UUID session lifecycle and keep chat entry gated on readiness. |
| ComposeApp linking model aligns | ✅ Implemented | `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt`, `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`, and related shared/mobile files keep linking/bridge-only onboarding and session-first readiness. |
| Latest parity and intent-proof remediation landed | ✅ Implemented | Explicit intent-selection and parity evidence now exists in `clients/web/apps/chat/src/onboardingContract.spec.ts`, `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt`, and `clients/agent-runtime/src/onboard/wizard.rs`. |
| Docs and migration artifact updated | ✅ Implemented | Surface-ordered follow-up slices remain documented in `openspec/specs/client-surfaces/migrations.md`. |

---

### Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Canonical onboarding uses a shared state machine | ✅ Yes | Shared contract primitives and normalized state labels are implemented across Rust, KMP, dashboard, web chat, and composeApp. |
| Trust establishment remains adapter-specific | ✅ Yes | CLI stays host-trusted, dashboard/web chat stay HTTP-paired, and mobile stays bridge-linked without HTTP fallback language. |
| Ready state stays separate from trust state | ✅ Yes | Dashboard and web chat still distinguish trust/readiness from session entry; mobile still distinguishes linking from session readiness. |
| Recovery taxonomy is normalized | ✅ Yes | Rust, dashboard, web chat, and mobile all emit the same normalized recovery labels and transition labels. |
| File changes match design scope | ✅ Yes | Implemented files match the design's expected scope, including the added parity/verification artifacts. |

---

### Issues Found

**CRITICAL** (must fix before archive):
- None.

**WARNING** (should fix):
- `openspec/config.yaml` still points verify testing at `make test`, which does not cover the Rust cargo suite or web Vitest suites touched by this change; full verification still requires supplemental `make rust-test` and `make web-test-all` runs.
- Verification commands currently dirty the working tree by emitting coverage `.profraw` files under `clients/agent-runtime/`, and `make build` runs `agentsyncApply`, which can update managed MCP config files and `.gitignore` during validation.

**SUGGESTION** (nice to have):
- Update the verify command stack to a non-mutating repo-wide target (for example `make test-all` plus an explicit build target) so future verify passes do not need supplemental commands.
- Route LLVM coverage scratch artifacts to an ignored temp directory to keep verification evidence from polluting the worktree.

---

### Verdict

PASS WITH WARNINGS

The latest micro-remediation blockers are cleared: the configured build is green again, explicit intent-selection proofs now exist across CLI/chat/mobile, and cross-surface parity evidence is materially stronger. The change matches the approved specs/design/tasks, with only non-blocking verification-stack hygiene warnings remaining.
