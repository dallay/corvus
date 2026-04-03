# Client Surfaces Migrations

Migration status legend: `not-started` | `in-progress` | `blocked` | `complete`

This tracker now mirrors the approved dependency order used by
`openspec/changes/2026-03-23-unify-onboarding-pairing-flow/` so follow-up issues can be opened per
surface without losing the shared onboarding model.

---

## O1: Shared Foundation And Canonical Contracts

**Status**: complete

**Depends on**: None

**Delivers**:

- Canonical onboarding state, trust mode, transport mode, and recovery taxonomy.
- Shared HTTP normalization for pairing, bearer-token, and readiness outcomes.
- Product-level terminology boundaries for pairing, bearer token, linking, and runtime connection.

**Issue-ready follow-ups**:

- [ ] Keep future transport additions mapped back to the canonical onboarding contract first.
- [ ] Add any new recovery kind to all surface adapters before UI copy diverges.

**Related files**:

- `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CoreContracts.kt`
- `clients/agent-runtime/src/gateway/mod.rs`
- `clients/agent-runtime/src/gateway/utils.rs`
- `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/specs/onboarding/spec.md`

---

## O2: CLI Runtime Onboarding Alignment

**Status**: complete

**Depends on**: O1

**Delivers**:

- Canonical operator-first onboarding sequence in the CLI/runtime wizard.
- Stable `DASH-*` diagnostics mapped to normalized recovery kinds.
- Dashboard activation guidance that uses approved HTTP pairing and bearer-token terminology.

**Issue-ready follow-ups**:

- [ ] Add any future operator onboarding metrics to the canonical observability label set.
- [ ] Keep dashboard handoff copy aligned with the shared onboarding spec when local entrypoints
  change.

**Related files**:

- `clients/agent-runtime/src/onboard/wizard.rs`
- `openspec/specs/dashboard/spec.md`

---

## O3: Web Dashboard HTTP Onboarding Alignment

**Status**: complete

**Depends on**: O1, O2

**Delivers**:

- Canonical HTTP onboarding states for operator web setup.
- Secure pairing-code exchange with bearer-token persistence only.
- Retry and blocked-state guidance mapped to the shared recovery taxonomy.

**Issue-ready follow-ups**:

- [ ] Reuse the same observability labels if dashboard emits analytics events.
- [ ] Keep quick-pair entry flows aligned with the approved local-origin safety rules.

**Related files**:

- `clients/web/apps/dashboard/src/composables/useConfig.ts`
- `clients/web/apps/dashboard/src/App.vue`
- `clients/web/apps/dashboard/src/i18n.ts`

---

## O4: Web Chat HTTP Onboarding And Session Gating

**Status**: complete

**Depends on**: O1, O3

**Delivers**:

- Canonical HTTP onboarding adapter for browser chat.
- Separation between onboarding readiness and chat-session lifecycle.
- UUID session creation or resume only after trust and transport readiness.

**Issue-ready follow-ups**:

- [ ] Reuse canonical labels for any future streaming, approvals, or chat analytics.
- [ ] Keep session recovery distinct from onboarding recovery when adding richer chat transport
  features.

**Related files**:

- `clients/web/apps/chat/src/composables/useGateway.ts`
- `clients/web/apps/chat/src/composables/useChat.ts`
- `clients/web/apps/chat/src/App.vue`

---

## O5: ComposeApp Mobile Linking Alignment

**Status**: complete

**Depends on**: O1

**Delivers**:

- Mobile-first linking language for bridge trust establishment.
- Ready and blocked states that preserve bridge-only transport rules.
- Session-capable mobile readiness without suggesting HTTP pairing as fallback.

**Issue-ready follow-ups**:

- [ ] Reuse canonical labels when bridge telemetry or diagnostics become persistent.
- [ ] Keep iOS and companion-path guidance aligned with the bridge-only surface contract.

**Related files**:

- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/App.kt`
-

`clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt`

- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`

---

## O6: Cross-Surface Observability And Copy Parity

**Status**: complete

**Depends on**: O2, O3, O4, O5

**Delivers**:

- Canonical onboarding transition labels and recovery labels for operator, web chat, dashboard, and
  mobile surfaces.
- Final documentation slice that preserves dependency order across all onboarding surfaces.
- Verification-focused coverage for equivalent sequencing and recovery wording.

**Issue-ready follow-ups**:

- [ ] Add product analytics sinks only after they consume the canonical label vocabulary unchanged.
- [ ] Extend parity tests when a new onboarding-capable surface is added.

**Related files**:

- `clients/agent-runtime/src/onboard/wizard.rs`
- `clients/web/apps/dashboard/src/composables/useConfig.ts`
- `clients/web/apps/chat/src/composables/useGateway.ts`
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`
- `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/tasks.md`

---

## Dependency Order

```text
O1 shared foundation
  -> O2 CLI/runtime
  -> O3 web dashboard
  -> O4 web chat
  -> O5 composeApp mobile
  -> O6 observability and docs parity
```

## Tracking Guidance

- Open future issues against the smallest affected slice above.
- Preserve the dependency order when a change spans multiple surfaces.
- Treat O6 as the final parity pass after any new surface-specific onboarding work lands.

## Status

Last updated: 2026-03-23
