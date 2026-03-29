## Exploration: Define real runtime parity requirements for mobile clients

**Change**: 2026-03-29-mobile-runtime-parity-requirements
**Issue**: GitHub #274 / Linear DALLAY-179
**Date**: 2026-03-29

### Current State
- `clients/composeApp` is still a scaffolded mobile chat surface. `App.kt` mutates an in-memory `MobileBridgeSnapshot` and generates fake session IDs locally instead of talking to a runtime-backed bridge.
- `clients/composeApp/src/commonMain/.../ChatWorkspace.kt` still uses `buildLocalAssistantReply(...)`, which returns a stub assistant message rather than invoking Corvus. Messages are held in memory only, with no session history, no persistence, and no runtime/tool events.
- Mobile onboarding and recovery copy were recently aligned to the bridge-only transport contract (`App.kt`, `OnboardingScreen.kt`, `ChatComponents.kt`, `MobileOnboardingModels.kt`), but that work only defines readiness states and terminology. It does not deliver runtime communication.
- `modules/agent-core-kmp` provides a minimal contract layer (`CoreInvocation`, `CoreResult`, onboarding taxonomy) plus a JVM-only `RustCliBridge`. That bridge only runs a one-shot CLI command and parses simple probe output; it does not yet provide a session-aware, approval-aware, streaming mobile runtime contract.
- Android currently hosts the shared Compose app directly (`MainActivity.kt`), but iOS only wraps the same UI (`MainViewController.kt`) and cannot use the Android/JVM subprocess path. The canonical spec already says iOS needs a companion daemon or embedded Rust path rather than HTTP gateway fallback.
- The web chat surface is materially closer to real product behavior. `useGateway.ts` implements HTTP health/pairing/bearer-token handling, `useChat.ts` implements session state, SSE streaming, session list retrieval, and request headers, and `App.vue` renders onboarding gating, session sidebar, health indicator, and tool approval cards.
- Runtime support already exists for much of the end-user contract on the gateway side: pairing (`POST /pair`), health (`GET /health`), end-user scoped session list (`GET /session/list`), sync chat (`POST /webhook`), SSE chat (`POST /web/chat/stream`), and pairing-token auth (`security/pairing.rs`, `gateway/mod.rs`, `gateway/sessions.rs`).
- Archived OpenSpec work explicitly deferred mobile parity several times:
  - `2026-03-24-2026-03-23-unify-onboarding-pairing-flow` completed onboarding-language alignment only.
  - `2026-03-28-session-memory-visibility` explicitly deferred KMP/mobile because the bridge is not wired yet.
  - `2026-03-28-web-operational-parity` added runtime-backed web chat/dashboard behavior without including mobile.

### Affected Areas
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/App.kt` — current mobile state machine is UI-simulated, not runtime-backed.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt` — fake assistant replies, no persistence, no runtime event handling.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatComponents.kt` — bridge diagnostics and copy already encode the intended mobile trust model; this becomes the user-facing shell for real behavior.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt` — mobile onboarding already models runtime/link/session steps and should remain the entry gate.
- `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CoreContracts.kt` — current shared contract is too small for parity features like streaming events, approvals, session list, and bridge persistence.
- `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt` — usable as Android/JVM seed, but currently one-shot and not sufficient for parity milestone behavior.
- `clients/web/apps/chat/src/composables/useGateway.ts` — best available reference for real onboarding/auth/readiness states.
- `clients/web/apps/chat/src/composables/useChat.ts` and `src/App.vue` — best available reference for end-user chat parity: session gating, streaming, health, session list, and approval UI.
- `clients/agent-runtime/src/gateway/mod.rs`, `src/gateway/sessions.rs`, `src/security/pairing.rs` — existing runtime-backed chat/session/auth semantics that parity requirements should mirror conceptually, even though mobile transport is bridge-based rather than HTTP.
- `openspec/specs/client-surfaces/spec.md`, `surface-contracts/composeapp-mobile.md`, `surface-contracts/web-chat.md`, `openspec/specs/onboarding/spec.md` — current source-of-truth constraints and terminology.

### Approaches
1. **UI parity first** — keep mobile mostly local/demo, only polishing visuals and copy.
   - Pros: Lowest short-term effort.
   - Cons: Does not answer the issue; still not credible product parity; follow-up work remains ambiguous.
   - Effort: Low

2. **End-user runtime parity milestone** — define mobile v1 as the smallest real runtime-backed chat slice that matches the web chat/user journey, while excluding operator/admin capabilities.
   - Pros: Best fit for existing matrix; cleanly aligns with mobile surface role; implementation can be split into bridge, onboarding/linking, session/chat, and approval slices.
   - Cons: Requires explicit transport abstraction for Android vs iOS and a stronger KMP bridge contract.
   - Effort: Medium

3. **Full product parity** — require mobile to match chat + dashboard + runtime operator surfaces immediately.
   - Pros: Maximally ambitious, one spec for all client behavior.
   - Cons: Conflicts with the surface matrix; mixes end-user and admin roles; far too large for a first parity milestone.
   - Effort: High

### Recommendation
Use **Approach 2: End-user runtime parity milestone**.

The first real parity milestone for mobile should be defined as:

**“Android and iOS can complete mobile-specific onboarding, connect to a real Corvus runtime through the approved mobile transport, create/resume/end their own sessions, exchange real chat turns, and complete human tool approvals — without depending on demo replies or another surface to finish the user journey.”**

That milestone should be explicitly scoped to **end-user chat parity**, not operator/admin parity.

#### Mandatory runtime-backed capabilities for mobile v1
1. **Real bridge-backed runtime connectivity**
   - Android MUST stop using local stub replies and invoke a real bridge/runtime path.
   - iOS MUST provide an approved equivalent transport (companion daemon IPC or embedded Rust), with the same product behavior and recovery states.
   - Mobile MUST NOT depend on HTTP gateway as its primary or only runtime path.

2. **Mobile trust/linking flow as the auth entrypoint**
   - Mobile v1 MUST keep mobile-specific trust establishment on the linking/bridge path defined by the onboarding spec.
   - The app MUST validate runtime reachability, trust/link establishment, and transport readiness before chat is enabled.
   - The app MUST provide retry/relink flows for runtime unavailable, transport unavailable, linked-but-not-session-ready, and environment unsupported states.

3. **Session lifecycle backed by the runtime**
   - Users MUST be able to create a new session, resume an existing/resumable session, and end/clear a session from mobile.
   - Session IDs MUST remain UUID-based for cross-surface consistency.
   - Mobile MUST persist the active session ID locally so background/resume behavior survives app restarts.
   - A mobile session list/history view for the current user SHOULD be in scope for v1 because session resumption is already mandatory in the capability matrix and is part of the credible web-chat baseline.

4. **Real chat exchange**
   - Message submission MUST go to the runtime, not `buildLocalAssistantReply`.
   - Mobile MUST render real assistant responses.
   - Mobile SHOULD support incremental/streaming updates when the bridge contract can provide them, but MUST at least support a correct synchronous fallback so real runtime behavior is available on both platforms.

5. **Human approval loop for tool-gated actions**
   - Mobile v1 MUST support approval-required interactions that let the user approve or deny a pending tool action.
   - Approval UI MUST be runtime-backed, not local-only.
   - Approval results MUST continue or stop the underlying runtime action consistently with the existing web/gateway semantics.

6. **Minimal end-user operational visibility**
   - Mobile MUST show runtime/link readiness and current session state.
   - Mobile SHOULD expose lightweight diagnostics appropriate for end users (connected, linked, session active, retry guidance).
   - Mobile MUST NOT expose admin configuration, raw memory inspection, provider pool management, or dashboard-only operational controls.

#### Settings and auth/linking flows that belong in mobile v1
Mobile v1 settings should stay minimal and strictly tied to making the mobile transport work:
- Bridge/runtime target configuration needed to locate the local CLI or approved companion path.
- Connection timeout / retry behavior appropriate to bridge startup.
- Secure local persistence of bridge-linked state and resumable session identity as applicable.
- Clear relink/disconnect/reset actions.
- Health/readiness checks surfaced in a user-safe way.

Mobile v1 should **not** require or foreground web-style HTTP pairing code entry, bearer-token management, webhook secrets, or gateway URL management as the primary flow. Those belong to HTTP clients.

#### Intentional non-goals for early mobile parity
The following should be explicitly out of scope for the first parity milestone:
- Dashboard/operator/admin capabilities (`/web/admin/*`, config editing, memory browser, all-session monitoring).
- Raw memory visibility or long-term memory administration.
- Provider/model configuration editing beyond minimal display of the active runtime/model if available.
- File upload, image input, multimodal flows, push notifications, offline mode, and background automation beyond basic resume/persistence.
- Full observability dashboards, metrics views, cost tracking, and scheduler/channel management.
- Perfect transport symmetry between Android and iOS internals. The requirement is product-behavior parity, not identical implementation strategy.

### Concrete Split For Follow-up Work
A proposal/spec/design can cleanly split implementation into at least these slices:
1. **KMP bridge contract expansion** — sessions, structured output/events, approvals, diagnostics.
2. **Android runtime bridge** — replace stub behavior with real bridge-backed chat/session operations.
3. **iOS transport path** — companion/embedded transport that satisfies the same contract.
4. **Mobile chat/session UX** — session history, persistence, start/resume/end, runtime-backed messaging.
5. **Mobile approval UX** — approve/deny flows and runtime round-trip semantics.
6. **Mobile settings/linking UX** — relink, diagnostics, secure persistence, recovery guidance.

### Risks
- **iOS feasibility risk**: iOS cannot use the Android/JVM subprocess path, so parity requirements must avoid assuming `RustCliBridge` alone solves both platforms.
- **Contract gap risk**: current KMP contracts are too narrow for streaming, approvals, and session history; proposal work must define the shared bridge surface before design/tasks can be clean.
- **Parity target ambiguity**: web chat still contains one notable stub (`handleApprove` / `handleReject` in `App.vue`), so proposal work must define parity against intended runtime behavior, not every accidental current limitation.
- **Scope creep risk**: it is easy to pull in dashboard/admin or multimodal work; the milestone should stay limited to end-user runtime-backed chat.
- **Persistence/security risk**: mobile will need platform-safe storage rules for linked state/session state without drifting into HTTP token semantics that do not belong on this surface.

### Ready for Proposal
Yes — provided the proposal keeps the milestone explicitly framed as **mobile end-user runtime parity** and settles the shared bridge contract needed across Android and iOS.