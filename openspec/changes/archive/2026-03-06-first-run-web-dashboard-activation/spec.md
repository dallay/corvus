# Spec: First-Run Web Dashboard Activation

## Status

Proposed

## Context

Interactive onboarding currently ends without a guided web dashboard activation step. Users must
discover and compose gateway, dashboard UI, and pairing steps manually. This spec defines a secure,
deterministic first-run activation experience that preserves existing CLI-only behavior when users
decline.

## Scope

### In Scope

- Add a final optional prompt in interactive onboarding to activate the web dashboard now.
- Define accept/decline behavior, including unchanged CLI-only flow on decline.
- Define one-screen activation guidance (3-5 clear steps) for accepted flow.
- Define deterministic failure diagnosis categories and exact fallback commands.
- Define a quick resume-later path users can run verbatim.
- Define testable acceptance criteria and requirement traceability.

### Out of Scope

- Any change to pairing protocol/token model/storage/hashing semantics.
- Any relaxation of origin/referer protections on admin endpoints.
- Broad dashboard frontend redesign unrelated to onboarding activation guidance.
- New long-running process manager/orchestration service.

## Functional Requirements

### RF1 - Final Activation Prompt

The system shall present a final interactive prompt during `corvus onboard --interactive` asking
whether to activate the web dashboard now.

Constraints:

- The prompt appears after onboarding summary is complete.
- The prompt wording clearly states this step is optional.

### RF2 - Accepted-Path Activation Guidance

If the user accepts activation, the system shall provide a compact activation guide that includes:

1. Dashboard URL to open.
2. Gateway status expectation (running/not running) and pairing expectation.
3. Pairing instructions that use existing secure flow.
4. Optional browser-open attempt when supported, with non-fatal fallback messaging.

Constraints:

- Guidance is 3-5 actionable steps.
- Canonical local defaults are used consistently (`http://corvus.localhost` entrypoint with
  proxied gateway health at `/api/health`).

### RF3 - Decline Preserves CLI-Only Experience

If the user declines activation, onboarding shall preserve current CLI-only behavior and next-step
messaging.

Constraints:

- No new required steps are introduced.
- Existing onboarding order and post-summary behavior remain functionally equivalent.

### RF4 - Deterministic Diagnosis and Fallback Commands

For accepted activation flow, the system shall classify activation readiness/failure into
deterministic local states and provide exact manual fallback commands for each state.

Minimum diagnosis states:

- Gateway not running.
- Gateway running and pairing required.
- Gateway running and already paired.
- Dashboard UI not available from current environment.

Constraints:

- Diagnosis logic uses bounded checks with explicit timeout limits.
- Fallback commands are copy-paste ready and avoid insecure direct admin API calls.

### RF5 - Resume Later Path

The system shall provide a quick resume path for users who decline now or cannot complete
activation.

Constraints:

- Resume block includes explicit commands for gateway status/start and dashboard launch.
- Resume instructions can be executed independently of the onboarding run.

## Non-Functional Requirements

### NFR-S1 Security

- Pairing remains required by default (`gateway.require_pairing = true` behavior unchanged).
- No new output path may expose secrets or bearer tokens.
- Guidance must keep users on existing secure pairing flow and local-origin constraints.

### NFR-U1 Clarity and UX

- Accepted activation instructions must be readable in one screen and limited to 3-5 steps.
- Wording must avoid ambiguous diagnostics and provide exact next action.
- URL/port examples must match runtime defaults.

### NFR-R1 Robustness

- Activation checks must be deterministic and bounded; no unbounded waits or hangs.
- Optional browser-open behavior must never fail onboarding if unsupported.
- Failure messaging must always include a successful manual path.

### NFR-C1 Compatibility

- Existing onboarding flow remains backward compatible for non-web users.
- Works on supported local development/runtime environments without requiring external network
  access.
- Does not require changes to existing pairing/auth model.

## Scenarios

### Scenario A - Accept and Web Path Available

Given interactive onboarding reaches final step,
When user chooses to activate dashboard now and required local services are available,
Then system shows 3-5 activation steps, shows canonical URLs, and user can complete pairing via
standard flow.

### Scenario B - Decline Activation

Given interactive onboarding reaches final step,
When user declines dashboard activation,
Then system exits through unchanged CLI-only path with existing next-step behavior preserved.

### Scenario C - Accept but Web Path Unavailable

Given user accepts activation,
When deterministic checks detect unavailable prerequisites (for example gateway not running or
dashboard UI unavailable),
Then system reports the exact diagnosed state and prints exact manual fallback commands for
recovery.

### Scenario D - Resume Later

Given user declined earlier or stopped during activation,
When user later follows resume instructions,
Then user can start/verify gateway and launch dashboard with explicit commands and complete pairing
through the same secure path.

## Acceptance Criteria

- AC1: Interactive onboarding always includes an optional final dashboard activation prompt. (RF1)
- AC2: Accept path always renders one-screen, 3-5 step activation guidance with URL, gateway/pairing
  status, and optional browser-open behavior. (RF2, NFR-U1, NFR-R1)
- AC3: Decline path remains functionally equivalent to existing CLI-only flow. (RF3, NFR-C1)
- AC4: Accepted flow emits deterministic diagnosis output and exact fallback commands per diagnosed
  state. (RF4, NFR-R1)
- AC5: Resume-later command block is always present when relevant and executable verbatim. (RF5)
- AC6: Security invariants hold: no secret/token leakage and pairing-required default unchanged. (
  NFR-S1)
- AC7: Messaging uses canonical local defaults and avoids insecure direct admin API fallback
  guidance. (RF2, RF4, NFR-S1, NFR-U1)

## Traceability Matrix

| Requirement | Covered By Scenarios | Verified By Acceptance Criteria |
|-------------|----------------------|---------------------------------|
| RF1         | A, B                 | AC1                             |
| RF2         | A, C                 | AC2, AC7                        |
| RF3         | B                    | AC3                             |
| RF4         | C                    | AC4, AC7                        |
| RF5         | D                    | AC5                             |
| NFR-S1      | A, C, D              | AC6, AC7                        |
| NFR-U1      | A, C                 | AC2, AC7                        |
| NFR-R1      | C                    | AC2, AC4                        |
| NFR-C1      | B, D                 | AC3, AC5                        |

## Open Decisions

1. Should resume guidance include a dedicated future command alias (for example
   `corvus dashboard resume`) or only existing commands in this change?
2. Resolved in implementation: optional browser-open targets the local proxied entrypoint
   (`http://corvus.localhost`) only.
3. Resolved in implementation Phase 4.1: bounded diagnosis uses 500 ms request timeout, one retry,
   and <= 1.5 s total budget.
4. Should deterministic diagnosis be exposed only in onboarding output, or also reusable by a future
   standalone command?
