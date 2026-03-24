# Tasks: Unify onboarding and pairing flow across Corvus clients

## Phase 1: Shared foundation

- [x] 1.1 RED: add failing canonical onboarding contract coverage in `modules/agent-core-kmp/src/commonTest/kotlin/com/profiletailors/agent/core/CoreContractsTest.kt` for the shared states, trust modes, recovery kinds, and ready-vs-session distinctions required by `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/specs/onboarding/spec.md`.
- [x] 1.2 GREEN: extend `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CoreContracts.kt` with the shared `SurfaceId`, `TrustMode`, `TransportMode`, `RecoveryKind`, and `OnboardingState` primitives that surface adapters will consume.
- [x] 1.3 RED: add failing HTTP adapter mapping coverage in `clients/agent-runtime/src/gateway/mod.rs` for `/health`, `/pair`, and authenticated follow-up outcomes that must normalize into trust, transport, and blocked recovery states without changing protocol semantics.
- [x] 1.4 GREEN: implement normalized HTTP trust/readiness/recovery mapping helpers in `clients/agent-runtime/src/gateway/mod.rs` and `clients/agent-runtime/src/gateway/utils.rs`, keeping pairing codes ephemeral and bearer tokens as the only persisted HTTP credential.

## Phase 2: CLI/runtime alignment

- [x] 2.1 RED: add failing onboarding flow tests in `clients/agent-runtime/src/onboard/wizard.rs` for canonical step order, operator-only completion, normalized recovery mapping, and the terminology scenarios defined in `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/specs/onboarding/spec.md` and `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/specs/dashboard/spec.md`.
- [x] 2.2 GREEN: update `clients/agent-runtime/src/onboard/wizard.rs` to present the canonical sequence (`runtime available` -> `trust this surface` -> `connect to gateway/runtime` -> `ready`) while preserving CLI host trust and optional dashboard continuation.
- [x] 2.3 GREEN: update user-facing dashboard activation and pairing guidance in `clients/agent-runtime/src/gateway/mod.rs` so printed instructions consistently use `pairing`, `pairing code`, `bearer token`, and `connect to gateway` without weakening current auth or origin protections.
- [x] 2.4 REFACTOR: keep `DASH-*` labels stable in `clients/agent-runtime/src/onboard/wizard.rs` while adding an explicit mapping from each dashboard activation diagnosis to the shared recovery taxonomy used by other surfaces.

## Phase 3: Dashboard HTTP onboarding alignment

- [x] 3.1 RED: extend `clients/web/apps/dashboard/src/composables/useConfig.spec.ts` with failing cases for canonical HTTP onboarding states, invalid vs expired pairing input, revoked/missing bearer token handling, and operator-ready completion.
- [x] 3.2 GREEN: update `clients/web/apps/dashboard/src/composables/useConfig.ts` to emit canonical trust/readiness/recovery states, preserve successful progress between retries, and clear stale bearer-token state on credential failures while never persisting pairing codes.
- [x] 3.3 RED/GREEN: update `clients/web/apps/dashboard/src/App.spec.ts` and `clients/web/apps/dashboard/src/App.vue` so the dashboard UI renders the shared onboarding sequence, operator-scoped ready state, and retry guidance mapped to the normalized recovery taxonomy.
- [x] 3.4 REFACTOR: align dashboard copy in `clients/web/apps/dashboard/src/i18n.ts` and any strings consumed by `clients/web/apps/dashboard/src/App.vue` / `clients/web/apps/dashboard/src/composables/useConfig.ts` to the approved terminology without reintroducing surface-specific drift.

## Phase 4: Web chat HTTP onboarding and session gating

- [x] 4.1 RED: create `clients/web/apps/chat/src/composables/useGateway.spec.ts` with failing scenarios for `/health` -> `/pair` -> authenticated-ready flow, invalid/expired pairing code, missing/revoked bearer token, and `paired but not connected` recovery.
- [x] 4.2 GREEN: implement `clients/web/apps/chat/src/composables/useGateway.ts` as the web-chat HTTP onboarding adapter with safe local URL validation, canonical trust/readiness/recovery state output, and no persistence of pairing codes.
- [x] 4.3 RED: create `clients/web/apps/chat/src/composables/useChat.spec.ts` for session create/resume behavior, `session unavailable` recovery, and the separation between onboarding readiness and chat-session lifecycle.
- [x] 4.4 GREEN: implement `clients/web/apps/chat/src/composables/useChat.ts` so web chat enters `session_pending` only after HTTP trust and transport readiness are satisfied, then creates or resumes UUID-based sessions.
- [x] 4.5 RED/GREEN: replace the local stub flow in `clients/web/apps/chat/src/App.vue`, `clients/web/apps/chat/src/components/ConfigPanel.vue`, and `clients/web/apps/chat/src/App.spec.ts` with onboarding-state rendering, ready gating, and normalized retry guidance shared with the dashboard HTTP model.

## Phase 5: ComposeApp mobile linking alignment

- [x] 5.1 RED: extend `modules/agent-core-kmp/src/jvmTest/kotlin/com/profiletailors/agent/core/RustCliBridgeTest.kt` with failing cases for bridge discovery, link establishment, session-capable readiness, unsupported-environment handling, and `linked but not session ready` recovery.
- [x] 5.2 GREEN: extend `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt` and `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CoreContracts.kt` with bridge-link and session-capability APIs needed by the mobile adapter.
- [x] 5.3 RED/GREEN: update `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt` and `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt` so mobile onboarding uses linking/runtime-ready/session-ready steps and never presents HTTP pairing as the primary trust flow.
- [x] 5.4 RED/GREEN: replace `AgentGatewayConfig` in `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`, `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ConfigPanel.kt`, and `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatComponents.kt` with bridge-linked, ready, and session-state models.
- [x] 5.5 GREEN: update `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/App.kt` and platform entrypoints under `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/MainActivity.kt` / `clients/composeApp/src/iosMain/kotlin/com/profiletailors/corvus/MainViewController.kt` to surface relink, retry, and unsupported-environment guidance without suggesting HTTP gateway pairing as fallback.

## Phase 6: Cross-surface observability, documentation, and verification

- [x] 6.1 GREEN: add shared onboarding transition and recovery labels to `clients/agent-runtime/src/onboard/wizard.rs`, `clients/web/apps/dashboard/src/composables/useConfig.ts`, `clients/web/apps/chat/src/composables/useGateway.ts`, and `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt` so logs and analytics use the same canonical state vocabulary.
- [x] 6.2 RED/GREEN: expand verification coverage in `clients/agent-runtime/src/onboard/wizard.rs`, `clients/web/apps/dashboard/src/composables/useConfig.spec.ts`, `clients/web/apps/chat/src/composables/useGateway.spec.ts`, `clients/web/apps/chat/src/composables/useChat.spec.ts`, `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/OnboardingDefaultsTest.kt`, and `modules/agent-core-kmp/src/jvmTest/kotlin/com/profiletailors/agent/core/RustCliBridgeTest.kt` to prove equivalent recovery labels and sequencing across operator and chat surfaces.
- [x] 6.3 DOCUMENT: update `openspec/specs/client-surfaces/migrations.md` with issue-ready follow-up slices for shared foundation, CLI/runtime, dashboard, web chat, composeApp mobile, and observability so implementation can be tracked per surface without losing dependency order.
