# Proposal: Mobile Runtime Parity Requirements

## Intent

The active change must be corrected to match the client-first product model for `composeApp` surfaces. Desktop, Android, and iOS are clients first, not standalone local agents. They MUST NOT assume a locally installed `corvus` binary or immediate local runtime execution as the default experience.

The previous proposal defined the wrong milestone boundary by centering parity around local runtime-backed mobile chat. That framing now causes spec, design, task, copy, and startup drift toward host behavior instead of client behavior. This proposal resets the milestone to the smallest correct product slice: client-first onboarding, readiness, and connection setup for desktop, Android, and iOS before session, chat, and approval UX are unlocked.

## Scope

### In Scope
- Redefine the first milestone boundary as **client-first onboarding/readiness parity** for `composeApp` surfaces across desktop, Android, and iOS.
- Require all three surfaces to start in onboarding/readiness/configuration UX instead of probing or launching a local runtime by default.
- Require onboarding to guide users through a supported client connection path: runtime URL/endpoint, authenticated pairing flow where supported, and/or trusted companion flow depending on platform/surface support.
- Require minimal settings, recovery, and readiness diagnostics to answer: what runtime is targeted, whether trust is established, whether the connection path is reachable, and whether chat entry can safely unlock.
- Explicitly invalidate the recent host-first assumptions already encoded in active change artifacts so follow-on spec/design/tasks can be corrected narrowly and consistently.

### Out of Scope
- Making desktop, Android, or iOS host a local runtime by default.
- Requiring a packaged or installed `corvus` binary on desktop or Android as the normal user path.
- Full runtime-backed chat/session/approval feature parity implementation; those remain follow-on work after the client connection model is corrected.
- Dashboard/operator/admin capabilities, runtime configuration editing, memory administration, multimodal features, notifications, offline mode, or other broader parity goals.
- Finalizing every platform-specific transport implementation detail in this proposal; later spec/design work will define the exact supported connection mechanisms per surface.

## Approach

Adopt the exploration recommendation: correct the change to a **client-first transport model with capability-based onboarding**.

The corrected milestone is:

**Desktop, Android, and iOS MUST launch into onboarding/readiness/configuration first, MUST help the user establish a supported connection to an existing Corvus runtime, and MUST NOT unlock normal chat startup until that client connection is ready.**

Implementation and follow-on spec/design work should anchor on these requirements:
1. **Client-first startup**: `composeApp` surfaces MUST route normal startup into onboarding/readiness/configuration UX rather than immediate local runtime execution.
2. **No default local binary assumption**: desktop and Android MUST NOT assume `corvus` is installed, packaged, or runnable locally; iOS MUST NOT be framed as a special exception to an otherwise host-first model.
3. **Supported connection paths**: each surface MUST support one or more approved client connection paths appropriate to the platform, including runtime URL/endpoint configuration and/or pairing/trusted companion flows where supported.
4. **Readiness before chat**: readiness UX MUST confirm target selection, trust/auth state, connection reachability, and safe entry into session/chat flows before chat becomes available.
5. **Tight correction scope**: this change corrects the product model and milestone boundary first; full session/chat/approval parity is intentionally deferred until the client-first contract is captured in downstream specs and design.

## Invalidated Recent Assumptions

The following recent assumptions are explicitly invalidated by this proposal update:
- Desktop/JVM should default to `RustCliBridge()` or any equivalent local runtime-host path on launch.
- Android should assume a packaged executable, `libcorvus.so`, or local `corvus` binary is available and runnable by default.
- iOS should be treated only as an exception to a host-first Android/desktop model instead of as a client-first surface.
- Mobile or composeApp onboarding should primarily guide users to install or launch a local CLI/runtime.
- Runtime URL/endpoint configuration is out of scope for the primary client journey.
- Startup readiness should immediately probe local process execution instead of routing users into onboarding/readiness/configuration.
- The first parity milestone should be defined as local runtime-backed chat parity rather than client-first connection/readiness parity.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/proposal.md` | Modified | Resets the milestone boundary, rationale, and scope to the corrected client-first model. |
| `openspec/specs/client-surfaces/spec.md` | Modified | Future delta spec must remove host-first mobile/desktop transport defaults and encode client-first connection rules. |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` | Modified | Future delta spec must stop requiring CLI installation/process spawning as the default composeApp journey. |
| `openspec/specs/onboarding/spec.md` | Modified | Future delta spec must define client-first onboarding, readiness, and connection outcomes for desktop, Android, and iOS. |
| `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/design.md` | Modified | Existing design assumptions about local runtime hosting, startup, and transport precedence must be corrected. |
| `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/tasks.md` | Modified | Existing and future tasks must be re-scoped away from proving local host behavior and toward client readiness behavior. |
| `clients/composeApp/` | Modified | Follow-on implementation will change startup routing, onboarding copy, readiness UX, and chat unlock conditions. |
| `clients/androidApp/` | Modified | Follow-on implementation will remove default local-runtime assumptions from Android packaging/startup expectations. |
| `clients/iosApp/` | Modified | Follow-on implementation will define supported client connection/readiness behavior without host-first framing. |
| `modules/agent-core-kmp/` | Modified | Follow-on contract work will need client-safe readiness/configuration/session-entry abstractions instead of default local bridge assumptions. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Recent design/tasks/validation work is partially mis-scoped around local hosting | High | Explicitly mark those assumptions invalid here so spec/design/task updates can narrow and correct before more implementation continues. |
| Surface-specific connection options remain ambiguous | Medium | Keep this proposal at the product-model level and require later spec/design artifacts to define supported connection paths per surface. |
| Scope creeps back into full chat parity or local-host infrastructure | High | Lock this proposal to startup, onboarding, readiness, and connection model correction only. |
| Shared `composeApp` artifacts keep desktop implicit and mobile explicit | Medium | Name desktop, Android, and iOS together in the milestone boundary and affected areas. |

## Rollback Plan

If the corrected client-first boundary proves wrong, rollback is documentation-only: restore the previous proposal or replace it with a narrower follow-up proposal before additional implementation proceeds. No production behavior or persisted user data depends on this proposal artifact alone.

## Dependencies

- Existing exploration artifact: `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/exploration.md`
- Source issue context: GitHub #274 / Linear DALLAY-179
- Current source-of-truth specs to be corrected next: `openspec/specs/client-surfaces/spec.md`, `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md`, `openspec/specs/onboarding/spec.md`
- Existing active change artifacts that now require follow-on correction: `design.md`, `tasks.md`, `verify-report.md`, `smoke-validation-report.md`

## Success Criteria

- [ ] The proposal defines the active milestone as **client-first onboarding/readiness parity** for desktop, Android, and iOS.
- [ ] The proposal explicitly states that desktop, Android, and iOS are clients first and MUST NOT assume a local `corvus` binary by default.
- [ ] The proposal requires startup to route users into onboarding/readiness/configuration UX before session/chat unlock.
- [ ] The proposal allows supported client connection paths to include URL/endpoint and/or pairing/trusted companion flows depending on surface/platform support.
- [ ] The proposal explicitly lists the recent host-first assumptions that are now invalidated.
- [ ] The proposal keeps scope tight by deferring full chat/session/approval parity and broader platform features to later spec/design work.
