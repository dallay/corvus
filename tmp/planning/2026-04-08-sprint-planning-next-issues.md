# Sprint Planning — Next Issue Selection

Date: 2026-04-08
Repository: `dallay/corvus`

## Goal

Capture the current issue selection analysis so the next implementation work can be chosen with
clear dependency awareness and without losing context.

## Executive Summary

Recommended next issue to implement:

1. **#365 — Add missing i18n keys for session sidebar and memory components**

Why this is the best next pick:

- small and explicit scope
- bug fix rather than open-ended exploration
- low dependency risk
- likely closes quickly and safely
- improves the session-memory follow-up line from `#277`

## Recommended Priority Order

### 1. #365 — Add missing i18n keys for session sidebar and memory components

**Recommendation:** pick this first.

**Why:**

- clearest scope
- low implementation risk
- minimal dependency surface
- likely no hidden backend prerequisite
- good cleanup/polish value for the session-memory work

**Tradeoff:**

- lower product visibility than a larger feature
- but highest execution efficiency right now

### 2. #330 — feat: startup temp file reaper for staged images

**Recommendation:** pick this second if we want a small-to-medium technical/runtime task.

**Why:**

- concrete and self-contained
- improves operational hygiene
- likely lower UX coordination cost
- advances the `#266` image-ingestion line safely

**Tradeoff:**

- less visible to users than a feature issue

### 3. #328 — feat: Discord image ingestion

**Recommendation:** pick this third if we want visible feature progress.

**Why:**

- concrete feature issue
- likely simpler than Slack ingestion
- strong leverage inside umbrella `#266`
- Discord path appears lower-friction than Slack

**Tradeoff:**

- broader integration surface than `#365` or `#330`

## Issues Grouped by Readiness

### Ready Now

- **#365** — Add missing i18n keys for session sidebar and memory components
    - Labels: `type|bug`, `module|memory`
    - Follow-up from `#277`

- **#330** — startup temp file reaper for staged images
    - Labels: `feature`
    - Child of `#266`

- **#328** — Discord image ingestion
    - Labels: `feature`
    - Child of `#266`

- **#329** — Slack image ingestion
    - Labels: `feature`
    - Child of `#266`
    - More integration friction than Discord

- **#331** — multi-image per turn support
    - Labels: `feature`
    - Child of `#266`
    - Broader scope than `#328` / `#330`

- **#364** — Chat context indicators — show when agent recalls memory
    - Labels: `type|enhancement`
    - Follow-up from `#277`
    - Possibly depends on memory recall metadata already being available

- **#361** — Cerebro memory enhancement layer for dashboard and gateway
    - Labels: `type|enhancement`, `gateway`, `module|memory`, `cerebro`
    - Follow-up from `#277`
    - Larger integration-heavy scope

### Blocked or Risky Due to Dependencies

- **#362** — Wire session history into KMP/mobile bridge
    - Explicitly blocked by Rust CLI bridge completion

- **#370** — Run Android and iOS mobile parity smoke validation
    - Depends on device/runtime readiness and evidence collection

- **#371** — Implement concrete iOS runtime companion client for mobile parity
    - Depends on transport/client direction and is a larger prerequisite issue

### Large / Strategic / Not Best Next Pick

- **#270** — Define next-stage routing capabilities: embedding routes and managed route updates
- **#271** — Define operator UX and documentation for model routing and query classification
- **#363** — Memory graph visualization for dashboard
- **#430 / #431** — PRD: Capability-Based Architecture for Composable AI Agents (v3)
- **#432 / #433** — Recurring: Monthly Code Quality Review (SonarQube)
- **#3** — Dependency Dashboard

## Dependency Notes

### Umbrella `#266`

Children identified:

- `#328` Discord image ingestion
- `#329` Slack image ingestion
- `#330` startup temp file reaper
- `#331` multi-image per turn support

Suggested order inside this line:

1. `#330`
2. `#328`
3. `#329`
4. `#331`

### Follow-up line from `#277`

Related issues:

- `#361` Cerebro memory enhancement layer
- `#362` session history into KMP/mobile bridge
- `#363` memory graph visualization
- `#364` chat context indicators
- `#365` missing i18n keys

Suggested order inside this line:

1. `#365`
2. `#364`
3. `#361`
4. `#363`
5. `#362` (only after blocker is resolved)

### Routing Definition Issues

- `#271` is already covered by the archived `productize-model-routing` work
- `#270` is a defer/decision issue, not the best next implementation target

These should not be treated as the next feature implementation candidates.

## Duplicates / Cleanup Signals

The issue export suggested likely duplicate pairs:

- `#430` / `#431`
- `#432` / `#433`

These should be verified directly in GitHub before planning implementation around them.

## Final Recommendation

### Best next issue:

**#365 — Add missing i18n keys for session sidebar and memory components**

### Why this is the strongest next move

- fastest likely completion
- low-risk execution
- low dependency uncertainty
- improves quality in an already active product area
- keeps sprint flow healthy before tackling broader feature work

## Proposed Sprint Sequence

If the goal is steady delivery with good risk control:

1. `#365`
2. `#330`
3. `#328`

That is the recommended sequence from this analysis.

## Risks in This Analysis

- Parent issue context for `#266` and `#277` should still be rechecked before implementation starts.
- Some medium-size issues may hide technical prerequisites not obvious from the issue text alone.
- Duplicate issue pairs should be confirmed and cleaned up in GitHub.

## Next Action

Before implementation, do a focused readiness review of `#365` to confirm:

- exact files likely affected
- whether translations already exist partially
- whether there is any hidden UI dependency

## Readiness Review — #365

Status: **ready to implement**

### Exact files likely affected

- `clients/web/packages/locales/src/en.json`
- `clients/web/packages/locales/src/es.json`

Reference usage locations:

- `clients/web/apps/chat/src/components/SessionSidebar.vue`
- `clients/web/apps/dashboard/src/components/memory/MemoryFilters.vue`
- `clients/web/apps/dashboard/src/components/memory/MemoryList.vue`
- `clients/web/apps/dashboard/src/components/memory/MemoryStats.vue`

Likely validation targets:

- `clients/web/packages/locales/src/parity.spec.ts`
- `clients/web/apps/chat/src/components/SessionSidebar.spec.ts`
- `clients/web/apps/dashboard/src/components/memory/MemoryList.spec.ts`
- `clients/web/apps/dashboard/src/components/memory/MemoryStats.spec.ts`

### Translation status

- `chat.newChat` already exists in both English and Spanish locales.
- Session sidebar keys used by `SessionSidebar.vue` are **not** present yet in `@corvus/locales`:
  - `session.history`
  - `session.justNow`
  - `session.minutesAgo`
  - `session.hoursAgo`
  - `session.daysAgo`
  - `session.messageCount`
  - `session.sidebarLabel`
  - `session.expand`
  - `session.collapse`
  - `session.noHistory`
- Dashboard memory UI already has config-related `memory.*` keys, but the memory screen keys used by
  `MemoryFilters.vue`, `MemoryList.vue`, and `MemoryStats.vue` are **missing** from locales.

### Hidden dependency check

- No backend or API dependency found.
- No transport or gateway prerequisite found.
- Dependency surface is limited to the shared locales package consumed by both chat and dashboard via
  Vue i18n.
- Main implementation risk is locale parity only; `parity.spec.ts` should catch key mismatch or
  placeholder mismatch.

### Notes

- There is one small polish gap outside the explicit issue scope: category option labels in
  `MemoryFilters.vue` (`Core`, `Daily`, `Conversation`, `Custom`) are still hardcoded in English.
  They are not part of the current acceptance criteria unless we decide to expand scope.

## Sprint Tasks Checklist

Use this checklist to mark tasks as they are completed during the sprint. Fill in Owner and Done
date as you go.

- [x] #365 — Add missing i18n keys for session sidebar and memory components — Owner: AI —
  Done: 2026-04-08
- [x] Readiness review for #365 — confirm files and existing translations — Owner: AI —
  Done: 2026-04-08
- [x] #330 — Startup temp file reaper for staged images — Owner: AI — Done: 2026-04-08
- [ ] #328 — Discord image ingestion — Owner: ______ — Done: ______
- [ ] #329 — Slack image ingestion — Owner: ______ — Done: ______
- [ ] #331 — Multi-image per turn support — Owner: ______ — Done: ______
- [ ] #364 — Chat context indicators — show when agent recalls memory — Owner: ______ — Done: ______
- [ ] #361 — Cerebro memory enhancement layer for dashboard and gateway — Owner: ______ —
  Done: ______
- [ ] Verify duplicates: #430/#431 and #432/#433 — Owner: ______ — Done: ______
- [ ] Re-check umbrella #266 dependencies before starting child work — Owner: ______ — Done: ______
- [ ] Update this sprint plan with outcomes and dates after each task completes — Owner: ______ —
  Done: ______

Latest outcome:

- 2026-04-08 — Readiness review completed for `#365`; issue is ready to implement with locale-only
  scope and no backend blocker identified.
- 2026-04-08 — `#365` implemented. Added missing `session.*`, `memory.*`, and supporting
  `pagination.*` locale keys in English and Spanish; updated session sidebar pluralized labels; and
  validated with locales, chat, and dashboard test suites.
- 2026-04-08 — `#330` implemented and archived. Added a startup-only staged image temp-file
  reaper with strict current/legacy filename matching, configurable threshold support, command-level
  startup wiring, and focused Rust test/lint/format validation.
- 2026-04-08 — Delivery branch pushed for completed work on `#365` and `#330`:
  `feat/330-365-runtime-i18n-followups`.

Notes:

- Mark the box when the task is complete and replace the blanks with the responsible person and
  completion date.
- If you want, I can also add a column for priority, estimated effort, or assignees automatically.
