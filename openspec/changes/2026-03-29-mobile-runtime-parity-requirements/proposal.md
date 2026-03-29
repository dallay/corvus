# Proposal: Mobile Runtime Parity Requirements

## Intent

Corvus now has a materially real web chat path and a clearly documented bridge-only mobile transport contract, but Android and iOS mobile chat still stop at scaffold behavior, local stub replies, and incomplete session semantics. This change is needed now to define the first implementation-ready parity milestone for mobile before more web-only behavior, onboarding language, or KMP bridge work widens the gap and causes scope creep.

The proposal sets a strict first milestone: mobile reaches **end-user runtime parity for chat**, not full product parity. Android and iOS must each be able to complete mobile onboarding, establish runtime trust through the approved mobile transport, create or resume their own sessions, exchange real runtime-backed chat turns, and complete runtime-backed tool approvals without requiring another surface to finish the journey.

## Scope

### In Scope
- Define the first parity milestone boundary as **mobile end-user runtime-backed chat parity** for Android and iOS.
- Require a shared mobile runtime contract that supports real connectivity, session lifecycle, runtime responses, approval flows, and readiness diagnostics across Android and iOS.
- Require mobile chat v1 capabilities: new session, resumable session entry, session end/reset, real message submission, real assistant responses, and persistence of active session identity across restart/resume.
- Require mobile v1 approval handling for approve/deny decisions on runtime-gated tool actions.
- Require mobile onboarding/settings/auth behavior for v1: linking-based trust establishment, runtime reachability checks, transport readiness, minimal bridge/runtime settings, relink/reset actions, and secure persistence of linked/session state.
- Define explicit non-goals so later spec/design/tasks stay limited to end-user mobile parity rather than dashboard, admin, or multimodal expansion.

### Out of Scope
- Dashboard/operator/admin capabilities, including all-session monitoring, memory administration, runtime configuration editing, and `/web/admin/*` parity.
- HTTP gateway as the primary or required mobile runtime path, including web-style bearer-token, pairing-code, webhook-secret, or gateway-URL management as the main mobile flow.
- Raw memory visibility, memory browser features, or long-term memory administration for end users.
- Multimodal/file upload/image input, push notifications, offline mode, background automation, scheduler/channel controls, metrics/cost dashboards, or other advanced mobile platform features.
- Requiring Android and iOS to share the exact same internal bridge implementation; this milestone requires product-behavior parity, not identical transport internals.
- Solving every future parity gap beyond the first runtime-backed chat milestone.

## Approach

Adopt the exploration recommendation: define parity around the smallest credible mobile journey that is fully runtime-backed and self-contained for end users.

The proposal anchors implementation around these capability requirements:
1. **Real runtime transport**: Android MUST stop using local stub chat behavior; iOS MUST use an approved companion or embedded runtime path; mobile MUST NOT depend on HTTP gateway as its primary path.
2. **Linking-first onboarding/auth**: mobile MUST validate runtime availability, trust/link state, transport readiness, and session entry readiness before chat unlocks; retry and relink flows MUST map to normalized onboarding recovery states.
3. **Runtime-backed session lifecycle**: mobile MUST support create, resume/select, and end/reset flows backed by the runtime; session IDs MUST remain UUID-based; active session identity MUST persist locally for resume.
4. **Runtime-backed chat exchange**: message submission MUST go through the mobile runtime contract and return real assistant output; streaming SHOULD be supported when the contract allows it, but correct synchronous behavior is the v1 floor on both platforms.
5. **Runtime-backed approval loop**: mobile MUST render pending approval requests and MUST send approve/deny decisions back to the runtime with web-equivalent semantics.
6. **Minimal user-safe settings/diagnostics**: mobile MUST expose only the settings needed to make bridge/runtime transport work and to recover safely when it does not.

This proposal intentionally keeps v1 small enough that later OpenSpec work can split cleanly into shared bridge contract, Android transport, iOS transport, mobile chat/session UX, approval UX, and settings/linking UX.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/proposal.md` | New | Defines the milestone boundary, scope, and follow-on implementation guardrails. |
| `openspec/specs/client-surfaces/spec.md` | Modified | Future delta spec will tighten the mobile-web parity requirement from generic parity language to a concrete first runtime-backed milestone. |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` | Modified | Future delta spec will align the mobile surface contract to the approved v1 runtime-backed capability set and non-goals. |
| `openspec/specs/onboarding/spec.md` | Modified | Future delta spec will align mobile linking, readiness, retry, and session-entry requirements to the v1 parity milestone. |
| `modules/agent-core-kmp/` | Modified | Future design/spec work will expand the shared contract for sessions, events, approvals, and diagnostics across Android and iOS. |
| `clients/composeApp/` | Modified | Future work will replace local stub chat/session behavior with runtime-backed onboarding, session, chat, approval, and settings flows. |
| `clients/androidApp/` | Modified | Future work will host the approved Android runtime bridge path and mobile-specific persistence/recovery behavior. |
| `clients/iosApp/` | Modified | Future work will define and implement the approved iOS transport path needed for parity without HTTP fallback. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| iOS transport feasibility constrains milestone delivery | High | Keep this proposal at the requirement boundary and require product-behavior parity without assuming Android's subprocess implementation applies to iOS. |
| Shared KMP bridge contract is too narrow for approvals, sessions, and streaming | High | Make contract expansion an explicit first-class follow-on requirement before UI implementation details. |
| Scope expands into dashboard/admin or advanced platform features | High | Lock the milestone to end-user runtime-backed chat only and list explicit non-goals in proposal/spec/design artifacts. |
| Mobile auth/linking drifts into web HTTP pairing semantics | Medium | Reuse onboarding terminology and require linking-based trust establishment for mobile v1. |
| Web parity baseline includes incidental stubs or unfinished details | Medium | Define parity against intended runtime-backed end-user behavior, not accidental implementation gaps in a single surface. |

## Rollback Plan

If this milestone boundary proves incorrect during spec or design, rollback is documentation-only: replace or remove this proposal, keep the change unimplemented, and re-open exploration with a narrower or different parity target. No production code or user data is affected by reverting this proposal artifact.

## Dependencies

- Existing exploration artifact: `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/exploration.md`
- Source issue context: GitHub #274 / Linear DALLAY-179
- Current source-of-truth specs: `openspec/specs/client-surfaces/spec.md`, `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md`, `openspec/specs/onboarding/spec.md`
- Future follow-on dependency: a shared mobile runtime contract in `modules/agent-core-kmp` that can be satisfied by both Android and iOS transports

## Success Criteria

- [ ] The proposal unambiguously defines the first mobile parity milestone as **runtime-backed end-user chat parity**, not full product parity.
- [ ] The proposal names the mandatory mobile v1 capabilities: real transport, runtime-backed chat, session lifecycle, approval flow, and minimal readiness diagnostics.
- [ ] The proposal names the required v1 mobile settings/auth/linking behavior and explicitly excludes web-style HTTP-first auth management from the primary mobile flow.
- [ ] The proposal lists explicit non-goals and follow-on boundaries so later spec/design/tasks can be scoped without ambiguity.
- [ ] The proposal identifies the concrete repo areas and packages that follow-on spec/design work is expected to affect.
