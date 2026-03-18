# Proposal: First-Run Web Dashboard Activation

## Problem

`corvus onboard --interactive` currently ends without offering to activate the web dashboard, so
first-run users must discover and compose gateway/dashboard/pairing commands on their own. This
creates avoidable setup friction and inconsistent troubleshooting while still requiring strict
security defaults.

## Goals

- Add an optional final onboarding prompt to activate the web dashboard immediately.
- If accepted, provide one-screen actionable guidance (3-5 steps) covering:
  - dashboard URL,
  - gateway status,
  - pairing instructions,
  - optional browser-open attempt when supported.
- Preserve `gateway.require_pairing = true` behavior and avoid secret leakage in onboarding
  logs/output.
- Provide deterministic activation diagnostics and exact manual fallback commands.
- Preserve unchanged CLI-only flow when the user declines.
- Provide a quick resume path for later dashboard activation.
- Include testing and documentation updates in rollout.

## Scope

### In Scope

- Extend onboarding wizard output with a final "activate dashboard now" prompt.
- Add accept/decline branch handling in interactive onboarding completion flow.
- Add a bounded, deterministic diagnosis matrix for common activation states (gateway running/not
  running, pairing required/already paired, dashboard dev UI availability guidance).
- Add explicit manual fallback command sequence for each diagnosed failure path.
- Add resume-later hints (`corvus gateway`, `corvus status`, dashboard launch command) when declined
  or when activation cannot complete.
- Update tests and docs for the onboarding activation flow and troubleshooting guidance.

### Out of Scope

- Pairing protocol changes (token model, storage, hashing, or auth semantics).
- Relaxing origin/referer protections for `/web/admin/*`.
- Large dashboard frontend UX refactors outside first-run activation guidance.
- New daemon/process-manager orchestration for long-running dashboard services.

## High-Level Approach

1. Insert a final interactive prompt in onboarding after existing summary output and before exit.
2. Keep decline path behavior stable by reusing current messaging and command flow.
3. For accept path, render a compact activation panel with:
  - canonical local URLs,
  - current gateway status and pairing expectation,
  - stepwise instructions to pair via dashboard,
  - best-effort browser-open attempt when platform/runtime support exists (with explicit non-fatal
    fallback).
4. Run bounded checks (no unbounded waits) to classify activation outcome into deterministic
   categories and print exact commands for manual recovery.
5. Add a "resume later" command hint block that users can run verbatim.
6. Add/adjust tests and docs to codify stable behavior and security constraints.

## Acceptance Criteria

- [ ] `corvus onboard --interactive` presents a final optional prompt to activate dashboard now.
- [ ] Declining preserves existing CLI-only onboarding flow and next-step behavior.
- [ ] Accepting prints one-screen actionable guidance (3-5 steps) including URL, gateway status,
  pairing flow, and optional browser-open attempt.
- [ ] Pairing/security defaults remain unchanged (`require_pairing` preserved); no secrets/tokens
  are printed beyond existing controlled pairing flow.
- [ ] Failure handling is deterministic and includes exact manual fallback commands per diagnosed
  condition.
- [ ] Output includes a clear quick-resume path for later activation.
- [ ] Automated tests cover accept/decline branches, diagnostics mapping, and security-safe output
  expectations.
- [ ] Docs update onboarding + dashboard activation/resume troubleshooting.

## Risks and Mitigations

| Risk                                                                   | Impact | Mitigation                                                                                                                                                       |
|------------------------------------------------------------------------|--------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Regression in onboarding UX ordering (especially post-summary prompts) | Medium | Add snapshot/behavior tests around prompt order and decline path parity with current flow.                                                                       |
| Accidental exposure of secrets in activation output                    | High   | Enforce output guardrails: never print bearer tokens; only emit pairing instructions already compatible with current secure flow. Add redaction/assertion tests. |
| Diagnostics become flaky due to timing/network assumptions             | Medium | Use bounded local checks with explicit timeouts and deterministic mapping; avoid dependence on external network.                                                 |
| Browser-open behavior differs across platforms                         | Low    | Treat browser open as optional best-effort; always print copy-paste URLs and commands as primary path.                                                           |
| Confusion from stale/default port messaging                            | Medium | Standardize onboarding messaging to canonical proxied local defaults (`http://corvus.localhost` + `/api`) and reuse shared constants where possible.             |

## Proposed Implementation Phases

1. **Onboarding Prompt + Branch Control**
  - Add final "activate now" prompt.
  - Preserve decline branch output/flow parity.

2. **Activation Guidance + Optional Browser Open**
  - Implement one-screen 3-5 step guidance block.
  - Add non-fatal optional browser-open attempt with clear fallback text.

3. **Deterministic Diagnostics + Manual Fallbacks**
  - Add bounded status checks and deterministic condition mapping.
  - Attach exact manual fallback commands per condition.

4. **Resume-Later Path + Docs**
  - Add explicit resume command hints in onboarding output.
  - Update user docs for activation, troubleshooting, and resume.

5. **Testing + Rollout Hardening**
  - Add/expand unit/integration tests for accept/decline, diagnostics, and secret-safe output.
  - Validate command examples and docs consistency before spec/design handoff.
