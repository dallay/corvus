# Tasks: Mobile Runtime Parity Requirements

## Prior Plan Status

### Invalidated prior work (do not extend)

- [x] I1 Prior Phase 2 adapter work in
  `modules/agent-core-kmp/src/{jvmMain,androidMain,iosMain}/kotlin/com/profiletailors/agent/core/*Bridge.kt`
  proved local-host/session parity and is no longer a milestone acceptance target.
- [x] I2 Prior Phase 4 Android packaging and approval/chat settings work in
  `clients/androidApp/build.gradle.kts` and
  `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/{ChatComponents.kt,ConfigPanel.kt}`
  is partially invalid because this milestone no longer requires host-first runtime, chat, or
  approval parity.
- [x] I3 Prior Phase 5 smoke validation for link/session/chat/approval completion is invalid;
  acceptance must now prove client-first startup, supported connection setup, readiness, and
  recovery only.

### Reusable completed work

- [x] R1 Shared contract/persistence scaffolding in
  `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/{CoreContracts.kt,MobileRuntimeFacade.kt,MobileRuntimePersistence.kt}`
  can be reused after renaming semantics from local bridge readiness to client connection readiness.
- [x] R2 Shared coordinator/startup scaffolding in
  `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/{App.kt,runtime/MobileRuntimeCoordinator.kt}`
  can be reused after routing into onboarding-first entry.

## Phase 1: Correct shared client-readiness contract

- [x] 1.1 RED: Update
  `modules/agent-core-kmp/src/commonTest/kotlin/com/profiletailors/agent/core/CoreContractsTest.kt`
  and `clients/composeApp/src/commonTest/kotlin/com/profiletailors/corvus/ComposeAppCommonTest.kt`
  to fail unless startup gates chat behind connection target, trust/auth, reachability, and recovery
  states.
- [x] 1.2 GREEN: Rewrite
  `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/{CoreContracts.kt,MobileRuntimeFacade.kt,MobileRuntimePersistence.kt}`
  and
  `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/runtime/RuntimeContracts.kt`
  around endpoint/pairing/trusted-companion methods instead of CLI-bridge defaults.

## Phase 2: Route startup and platform wiring to client-first setup

- [x] 2.1 GREEN: Modify
  `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/{App.kt,runtime/MobileRuntimeCoordinator.kt}`
  so desktop, Android, and iOS enter onboarding/readiness/configuration first unless a saved target
  restores to ready state.
- [x] 2.2 GREEN: Replace default host wiring in
  `clients/composeApp/src/{jvmMain,androidMain,iosMain}/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.*.kt`
  and trim `AndroidRuntimeBridge.kt` / `IosRuntimeBridge.kt` to expose only platform-supported
  client connection paths.
- [x] 2.3 GREEN: Remove Android default-host packaging assumptions from
  `clients/androidApp/build.gradle.kts`.

## Phase 3: Re-scope onboarding and diagnostics UX

- [x] 3.1 RED: Add failing UI-state/copy tests for
  `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt`
  and `ui/chat/ConfigPanel.kt` covering endpoint editing, supported-path disclosure, blocked
  readiness, retry, and reset.
- [x] 3.2 GREEN: Update
  `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/{onboarding/OnboardingScreen.kt,chat/ConfigPanel.kt,chat/ChatComponents.kt}`
  to show client-first setup, current target identity, trust/auth state, and recovery actions
  without local-host guidance.

## Phase 4: Validate corrected milestone

- [x] 4.1 Verify targeted tests with `bash ./scripts/gradlew.sh :agent-core-kmp:jvmTest` and
  `bash ./scripts/gradlew.sh :composeApp:jvmTest`, confirming only client-first startup, supported
  connection setup, readiness gating, and recovery scenarios from the delta specs.
- [x] 4.2 Replace the smoke checklist in
  `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/verify-report.md` or follow-on
  validation notes so desktop, Android, and iOS prove onboarding-first entry and supported
  connection readiness, not chat/session/approval parity.
