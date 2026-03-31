## Exploration: Re-evaluate mobile runtime parity under client-first product direction

**Change**: 2026-03-29-mobile-runtime-parity-requirements
**Issue**: GitHub #274 / Linear DALLAY-179
**Date**: 2026-03-30

### Current State
- The active change no longer reflects only “replace stubs with real runtime behavior.” It now encodes a stronger assumption: Android and desktop/JVM should behave like local runtime hosts, while iOS should reach parity through a companion/embedded exception.
- That assumption is visible in the source-of-truth artifacts:
  - `openspec/specs/client-surfaces/spec.md:54-61,118-129` defines mobile transport as `RustCliBridge (process bridge)` and says Android/desktop use process spawning.
  - `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md:40-45,86-92,97-106` requires process spawning, `corvus agent`, and “onboarding for corvus CLI installation.”
  - `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/proposal.md:21-24,32-37` excludes gateway URL management from the main mobile flow and frames mobile around non-HTTP bridge transport.
  - `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/design.md:13,29-37,191-215` explicitly chooses Android local CLI/process transport, iOS companion/embedded transport, and only treats desktop/JVM as a preview/testing driver.
- The implementation follows those host-first assumptions:
  - `clients/composeApp/src/jvmMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.jvm.kt:11-43` defaults desktop/JVM to `RustCliBridge()` with no connection/onboarding choice.
  - `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt:7-83` and `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/AndroidRuntimeBridge.kt:7-129` default to executable `corvus`, construct `ProcessBuilder`, and probe immediately.
  - `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.android.kt:17-22` resolves a packaged executable or falls back to `corvus`.
  - `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/App.kt:35-40,49-85` refreshes runtime readiness on startup, so normal launch immediately tries to reach the local runtime path rather than entering client onboarding/configuration first.
  - `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatComponents.kt:560-596` and `ConfigPanel.kt:81-116,219-229` use copy such as “Link this app to local Corvus,” “Messages now flow through the local CLI bridge,” and `Transport: local Corvus CLI bridge`.
  - `clients/androidApp/build.gradle.kts:6-19,43,55` packages `libcorvus.so` payloads as if Android will host/launch local runtime artifacts.
- Existing validation already shows the host-first path is not just incomplete, but structurally wrong for the new product direction:
  - `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/smoke-validation-report.md` shows Android still packages fake placeholder runtime payloads and iOS has no installed companion client.
  - `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/verify-report.md:124-156` marks transport/parity as only partial or missing even after the recent implementation work.
- New product direction supersedes that assumption: desktop, Android, and iOS are **clients first**. They should default to onboarding/configuration/readiness UX that helps users connect to an existing runtime via URL/endpoint and/or pairing/trusted companion flow, not default to spawning a local `corvus` executable.

### Affected Areas
- `openspec/specs/client-surfaces/spec.md` — current canonical transport rules are too local-host specific for desktop/Android and need a client-first transport model.
- `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` — currently hardcodes process spawning, `corvus agent`, and CLI installation onboarding; these are now invalid defaults.
- `openspec/specs/onboarding/spec.md` — needs explicit client-first completion criteria for desktop, Android, and iOS, including runtime endpoint/companion configuration before ready state.
- `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/proposal.md` — scope/approach currently reject URL management too broadly and assume local bridge transport as the normal mobile path.
- `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/design.md` — Android local process execution, iOS-only exception handling, and desktop de-prioritization all need correction.
- `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/tasks.md` — completed tasks 2.2, 2.3, 4.1, and pending 5.2 are framed around proving local-runtime-host behavior rather than client readiness behavior.
- `clients/composeApp/src/jvmMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.jvm.kt` — desktop currently assumes local `RustCliBridge` by default.
- `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt` and `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/AndroidRuntimeBridge.kt` — hardwired local executable/process model.
- `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.android.kt` and `clients/androidApp/build.gradle.kts` — Android packaging/startup still assume shipped local runtime artifacts.
- `clients/composeApp/src/iosMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.ios.kt` and `IosRuntimeBridge.kt` — iOS is treated only as a special-case transport exception rather than a client surface that may need endpoint and/or trusted companion onboarding.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/App.kt`, `ui/onboarding/OnboardingScreen.kt`, `ui/chat/ChatComponents.kt`, and `ui/chat/ConfigPanel.kt` — onboarding and diagnostics copy need to shift from “local CLI bridge” to client connection/readiness language.

### Approaches
1. **Keep local-runtime-first and patch platform gaps** — preserve current desktop/Android local binary assumptions and finish Android packaging plus iOS companion work.
   - Pros: Reuses most of the current design/tasks as written.
   - Cons: Conflicts directly with the clarified product direction; keeps startup behavior wrong; keeps desktop/Android behaving like hosts instead of clients.
   - Effort: Medium

2. **Client-first transport model with capability-based onboarding** — make desktop, Android, and iOS all start in connection/onboarding/readiness UX, then connect through whichever client-safe path each surface supports: URL/endpoint, pairing/auth, and/or trusted companion flow.
   - Pros: Matches the new product direction; removes false dependency on local binaries; unifies onboarding semantics across desktop/mobile; makes iOS no longer an awkward exception.
   - Cons: Requires spec/design/task corrections and some already-completed implementation work becomes invalid or needs rollback/rework.
   - Effort: Medium/High

3. **Split the change: desktop/web-style client direction later, keep this change mobile-only** — narrow the current change and defer desktop correction.
   - Pros: Smaller immediate rewrite.
   - Cons: Still leaves `composeApp` desktop startup behavior wrong; keeps shared client architecture inconsistent; likely causes another corrective change immediately.
   - Effort: Medium

### Recommendation
Use **Approach 2: Client-first transport model with capability-based onboarding**.

The corrected milestone should be:

**“Desktop, Android, and iOS must behave as clients that enter onboarding/configuration/readiness first, then connect to a Corvus runtime through a client-appropriate path (runtime URL/endpoint, authenticated/pairing flow where applicable, and/or trusted companion flow), and only unlock session/chat/approval UX after that connection is ready.”**

Concrete client expectations:

1. **Desktop (Compose/JVM)**
   - MUST stop defaulting to `RustCliBridge()`/local `corvus` process execution on launch.
   - MUST open into client onboarding that lets the user configure a reachable runtime endpoint and required trust/auth state.
   - MAY support an advanced local companion/runtime path later, but MUST NOT assume one is installed by default.
   - SHOULD behave closest to web chat onboarding: endpoint selection, readiness check, auth/pairing if needed, then session entry.

2. **Android**
   - MUST stop assuming a packaged executable or local `corvus` binary is present and runnable.
   - MUST launch into onboarding/configuration/readiness instead of probing `ProcessBuilder` immediately.
   - MUST let the user connect through an approved Android-capable client path: runtime URL/endpoint and/or trusted companion flow.
   - MUST treat local-hosted runtime execution as optional future infrastructure, not as the default parity path.

3. **iOS**
   - MUST remain client-first and MUST NOT be blocked on “embedded runtime parity” as the default story.
   - MUST guide the user through whichever supported client path exists first: trusted companion flow and/or runtime endpoint flow.
   - SHOULD fail closed when no supported path is configured, but that blocked state must still be presented as onboarding/readiness UX, not as “missing local runtime host infrastructure.”

Onboarding implications:
- Startup should land in connection setup, trust establishment, readiness diagnostics, and recovery actions.
- “Linking” should no longer imply “link to local Corvus executable”; it should mean establish trust with the chosen client connection path.
- Minimal settings must now include runtime target selection/configuration for client surfaces. The current blanket exclusion of gateway/runtime URL management is too strong.
- Readiness must answer: what runtime am I targeting, is trust established, is transport reachable, can I create/resume a session, and can I complete approvals here?

Recent implementation assumptions that are now invalid:
- Desktop defaulting to `RustCliBridge()` as the normal runtime path.
- Android resolving packaged `libcorvus.so`/`corvus` payloads and launching them via `ProcessBuilder`.
- Change artifacts calling Android the normal local-CLI host path and iOS only an exception.
- Mobile copy saying “local CLI bridge” as the expected transport.
- Config/settings excluding runtime URL/endpoint management from the primary client flow.
- Smoke-validation criteria centered on proving local executable packaging rather than proving client onboarding/readiness/session/chat/approval through the chosen connection path.

### Risks
- Already-completed work in tasks 2.2, 2.3, 4.1, and related verification evidence is now partly misaligned and may need rollback or re-scope rather than incremental patching.
- The canonical `client-surfaces` and `composeapp-mobile` specs currently encode the wrong transport default, so downstream work will keep drifting unless those are corrected first.
- Desktop is not explicitly covered by the active change artifacts today; if left implicit, the repo may continue shipping one shared `composeApp` client with contradictory startup behavior across JVM vs mobile.
- Client-first direction introduces a product decision that must be nailed down per surface: when to use URL/endpoint, when to use pairing/auth, and when trusted companion is required.
- If the team keeps local-runtime execution as an “optional advanced mode,” the UX must clearly separate that from the default path or the same host/client confusion will return.

### Ready for Proposal
No — not without correcting the existing proposal/spec/design/task assumptions first.

The next proposal/design update should explicitly redefine `composeApp` desktop, Android, and iOS as client surfaces with onboarding-first startup and capability-based connection flows, then re-scope transport, settings, validation, and smoke criteria around that client behavior.