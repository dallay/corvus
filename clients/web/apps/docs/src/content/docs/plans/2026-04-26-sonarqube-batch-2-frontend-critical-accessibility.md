# SonarQube Batch 2 Frontend Critical and Accessibility Implementation Plan

> **For agentic workers:** Execute this batch on branch `maintenance/sonarqube-remediation` after the Batch 1 backend refactor commit. Keep changes localized, test first where practical, and validate each app independently before final combined verification.

**Goal:** Resolve the current frontend-critical and accessibility Sonar issues in the dashboard and rook-dashboard apps by reducing control-flow complexity, improving semantics, and correcting likely contrast problems without changing the existing admin, pairing, session, or embedded-operator UX.

**Architecture:** Prefer local component and composable fixes over shared design churn. Replace nested inline decision logic with named derived state/helpers, upgrade weak ARIA patterns to semantic structure when safe, and apply the smallest CSS/token changes that improve readability and contrast. Avoid broad design-system edits.

**Tech Stack:** Vue 3, TypeScript, Vitest, Biome, Vite, app-local CSS, existing unit/a11y tests in `clients/web/apps/dashboard` and `clients/web/apps/rook-dashboard`.

---

## File Structure

### Files likely to modify

#### Dashboard app
- `clients/web/apps/dashboard/src/components/sessions/CerebroSessionActions.vue`
  - Replace inline conditional action-label/state rendering with explicit derived helpers/computed labels.
  - Preserve button disabled behavior and current admin action contracts.
- `clients/web/apps/dashboard/src/components/chat/ChatWorkspace.vue`
  - Review live-region usage, message rendering semantics, and any weak role assignment patterns.
  - Preserve keyboard flow, screen-reader announcements, and existing chat/session behavior.
- `clients/web/apps/dashboard/src/composables/useAdmin.ts`
  - Extract duplicated pagination/offset logic if Sonar is flagging repeated complexity.
- `clients/web/apps/dashboard/src/composables/useChat.ts`
  - Replace terse nested/inline payload branching with explicit parsing helpers where Sonar complexity is triggered.
- `clients/web/apps/dashboard/src/App.vue` or related auth/session shell components if Sonar/a11y points there.
  - Only touch if issues are confirmed during targeted inspection.

#### Dashboard tests to extend
- `clients/web/apps/dashboard/src/components/sessions/CerebroSessionActions.spec.ts`
- `clients/web/apps/dashboard/src/components/chat/ChatWorkspace.spec.ts`
- `clients/web/apps/dashboard/src/App.spec.ts`
- `clients/web/apps/dashboard/src/composables/useAdmin.spec.ts`
- `clients/web/apps/dashboard/src/test/runAxe.ts` helpers if existing a11y tests need extension

#### Rook dashboard app
- `clients/web/apps/rook-dashboard/src/style.css`
  - Adjust local contrast-sensitive colors only where needed.
  - Preserve dark visual identity and avoid re-theming the app.
- `clients/web/apps/rook-dashboard/src/features/**/` page components
  - Replace nested presentation branching or weak semantic wrappers if Sonar flags them.
- `clients/web/apps/rook-dashboard/src/lib/api/client.spec.ts`
  - Minor cleanup already identified (`?:` simplification / explicit readability) if tied to Sonar maintainability.

#### Rook dashboard tests to extend
- `clients/web/apps/rook-dashboard/src/features/**/*.spec.ts`
- `clients/web/apps/rook-dashboard/src/lib/api/client.spec.ts`
- Add focused tests near touched components rather than creating broad new suites.

### Files unlikely to modify
- Shared design-system packages
- Backend gateway/runtime code
- Unrelated docs beyond plan updates unless necessary

---

## Implementation Strategy

### Phase 1 — Confirm concrete issue surfaces
1. Inspect Sonar-targeted dashboard and rook-dashboard files for:
   - nested ternaries and inline branching in templates/composables
   - duplicated conditional parameter construction
   - weak role usage where semantic HTML can replace it
   - low-contrast foreground/background combinations in rook-dashboard CSS
2. Map each issue to the smallest safe file-local fix.
3. Do not start with CSS churn; first confirm markup/control-flow hotspots.

### Phase 2 — Dashboard remediation
1. Add or extend tests first for any behavior-sensitive component:
   - button labels/states in `CerebroSessionActions.vue`
   - live region and message-list behavior in `ChatWorkspace.vue`
   - any extracted pagination helper behavior in `useAdmin.ts`
2. Refactor inline conditional rendering into named helpers/computed state.
3. Where semantic HTML can replace ARIA/role duplication without changing behavior, prefer that swap.
4. Re-run dashboard checks before moving on.

### Phase 3 — Rook dashboard remediation
1. Add/extend tests around any feature/page logic changed.
2. Fix local maintainability issues in components/specs first.
3. Adjust contrast-sensitive CSS values conservatively:
   - improve text/background separation
   - preserve dark theme hierarchy
   - avoid broad hue changes unless required
4. Re-run rook-dashboard checks before final combined validation.

### Phase 4 — Combined validation
Run formatting, lint/check, tests, and builds for both apps. If available and fast enough, run app-local accessibility-focused tests for dashboard as an extra guardrail.

---

## Task Breakdown

### Task 1: Inspect and pin down Batch 2 files
**Files:** read-only inspection across both app src trees

**Actions:**
- Search for nested ternaries and inline branching in Vue templates and composables.
- Search for role/ARIA patterns that may be replaceable with semantic structure.
- Inspect rook-dashboard CSS for likely contrast offenders.
- Keep a short working list of exact files to touch.

**Success criteria:**
- A bounded set of candidate files is identified before implementation begins.

### Task 2: Remediate dashboard control-flow complexity
**Files:** likely `CerebroSessionActions.vue`, `useAdmin.ts`, `useChat.ts`, possibly `App.vue` or session/chat supporting components

**Actions:**
- Add/extend tests before refactoring behavior-sensitive logic.
- Extract derived labels, status lookup, or payload parsing helpers.
- Replace nested inline decision logic with explicit branches.
- Preserve all current text, disabled states, request parameters, and emitted events.

**Success criteria:**
- Sonar-style complexity hotspots are reduced.
- Dashboard tests remain green.
- No UX copy or state regressions.

### Task 3: Remediate dashboard accessibility semantics
**Files:** likely `ChatWorkspace.vue` and any touched auth/session shell components

**Actions:**
- Review live region usage, role duplication, labels, and semantic wrappers.
- Replace weak role usage with semantic elements only when behavior remains equivalent.
- Keep screen-reader announcements and existing tests stable; extend tests if semantics change.

**Success criteria:**
- Accessibility posture improves without introducing noisy announcements or broken focus flow.

### Task 4: Remediate rook-dashboard maintainability and contrast
**Files:** `src/style.css`, plus feature/page components/specs as confirmed during inspection

**Actions:**
- Apply minimal CSS changes to improve contrast where needed.
- Simplify small maintainability issues in specs/components flagged by Sonar.
- Keep the embedded dashboard visually consistent and operationally unchanged.

**Success criteria:**
- Build/test/check remain green.
- Styling remains recognizably the same, only clearer/more accessible.

### Task 5: Final verification
**Run from `clients/web/apps/dashboard`:**
```bash
pnpm check
pnpm test
pnpm test:a11y
pnpm build
```

**Run from `clients/web/apps/rook-dashboard`:**
```bash
pnpm check
pnpm test
pnpm build
```

**Optional if UI-sensitive changes warrant it:**
- run app preview/dev server and perform a quick visual sanity pass

**Expected result:**
- Both apps pass formatting/lint-style checks, tests, and builds.
- No obvious semantic/styling regressions.

---

## Implementation Notes

- Prefer additive helper functions/computed state over aggressive component decomposition unless complexity clearly warrants it.
- Do not rewrite shared CSS foundations imported from `@corvus/shared` unless a local fix is impossible.
- When changing semantics, preserve existing selectors/test hooks where feasible to avoid unnecessary test churn.
- Favor tests that prove user-visible behavior over snapshot-style assertions.
- Keep commits focused if the batch needs to be split mid-flight, but implementation can still land on the same maintenance branch.
