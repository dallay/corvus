# Delta for onboarding

## MODIFIED Requirements

### Requirement: Surface-Specific Trust Establishment

The system MUST preserve transport-specific trust establishment while keeping the shared product
sequence intact.

- CLI/runtime trust MUST be treated as local-host trust because the CLI is the host surface.
- Web dashboard and web chat MUST use HTTP pairing to exchange a pairing code for a bearer token.
- Desktop composeApp MUST treat connection to an existing runtime by URL or endpoint as a supported
  default onboarding path for this milestone.
- Android composeApp MUST treat connection to an existing runtime by URL or endpoint as a supported
  default onboarding path for this milestone.
- Desktop, Android, and iOS composeApp MUST guide pairing or trusted companion flows only on the
  platforms where those flows are approved and supported in this milestone.
- iOS composeApp MUST guide the approved iOS client connection path for this milestone and MUST NOT
  imply that local `corvus` execution is the default trust model.
- Desktop, Android, and iOS composeApp MUST NOT frame local binary installation, local process
  spawning, or immediate local runtime execution as the normal onboarding step.

(Previously: The requirement defined composeApp onboarding around mobile linking for a bridge or
companion-path model and excluded endpoint-led client onboarding as the corrected default contract.)

#### Scenario: Desktop onboarding defaults to existing-runtime connection setup

- GIVEN a desktop user begins composeApp onboarding
- WHEN the trust and connection setup steps are shown
- THEN the flow MUST guide the user to connect to an existing runtime through supported client
  configuration such as runtime URL or endpoint entry
- AND it MUST NOT start by instructing the user to install or launch a local `corvus` process.

#### Scenario: Android onboarding defaults to existing-runtime connection setup

- GIVEN an Android user begins composeApp onboarding
- WHEN the trust and connection setup steps are shown
- THEN the flow MUST guide the user to connect to an existing runtime through supported client
  configuration such as runtime URL or endpoint entry
- AND it MUST NOT assume a packaged executable or local runtime process is available by default.

#### Scenario: iOS onboarding guides only the approved iOS trust path

- GIVEN an iOS user begins composeApp onboarding
- WHEN the trust step is shown
- THEN the flow MUST guide only the approved iOS client connection path or paths for this milestone
- AND it MUST NOT tell the user that local runtime execution is the default iOS onboarding path.

### Requirement: Surface-Specific Completion Criteria

Each surface MUST finish onboarding according to its role and capability tier.

- CLI/runtime MUST finish with local runtime readiness and MAY offer optional dashboard activation
  guidance.
- Web dashboard MUST finish with authenticated gateway access and readiness for operator tasks.
- Web chat MUST finish with authenticated gateway access and readiness to create or resume a chat
  session.
- Desktop, Android, and iOS composeApp clients MUST finish this milestone with:
    - a selected or confirmed supported connection path,
    - a known target runtime or endpoint,
    - known trust, auth, link, or pairing state for that path,
    - known reachability and readiness status,
    - an actionable next step of either enter the unlocked client flow or recover from a blocked
      state.
- Desktop, Android, and iOS composeApp onboarding MUST NOT require completion of a runtime-backed
  chat, session, or approval flow for milestone acceptance.

(Previously: The requirement treated composeApp completion as readiness to create or resume a
runtime-
backed mobile chat session on the surface itself.)

#### Scenario: Client onboarding completes at actionable readiness

- GIVEN a desktop, Android, or iOS user has finished the supported connection setup steps
- WHEN readiness evaluation succeeds
- THEN the surface MUST show the chosen target runtime, current connection state, and that client
  entry is now allowed
- AND onboarding for this milestone MUST be considered complete without requiring a full chat turn.

#### Scenario: Client onboarding remains incomplete while readiness is blocked

- GIVEN a desktop, Android, or iOS user has not yet satisfied trust, auth, link, pairing,
  reachability, or configuration requirements
- WHEN readiness is evaluated
- THEN onboarding MUST remain incomplete
- AND the surface MUST show the blocked state and the recovery action needed before client entry can
  unlock.

### Requirement: Recovery And Retry Taxonomy

Every onboarding-capable surface MUST expose recovery or retry states using the same product-level
taxonomy. The minimum taxonomy MUST include:

- Runtime unavailable.
- Runtime URL or endpoint missing or invalid.
- Surface transport unavailable.
- Pairing code invalid or expired.
- Bearer token missing, invalid, or revoked.
- Gateway reachable but not paired or authenticated.
- Trusted companion unavailable or not trusted.
- Connection configured but readiness check failed.
- Local environment unsupported for the chosen surface.

For this milestone, desktop, Android, and iOS composeApp MUST expose the subset of those states that
can occur on their approved connection paths and MUST provide retry, edit configuration, re-pair,
disconnect, reset, or companion recovery guidance appropriate to the active failure.

(Previously: The requirement centered mobile recovery on bridge-linked session start or resume
failure
for a runtime-backed parity milestone.)

#### Scenario: Endpoint configuration failure maps to a normalized recovery state

- GIVEN a desktop or Android user enters an invalid runtime URL or endpoint during onboarding
- WHEN readiness is evaluated
- THEN the system MUST classify the failure as `runtime URL or endpoint missing or invalid`
- AND it MUST offer a direct way to edit the configuration and retry.

#### Scenario: Trusted companion failure maps to a normalized recovery state

- GIVEN a client surface uses a trusted companion path in this milestone
- WHEN the companion is unavailable, untrusted, or unreachable during onboarding
- THEN the system MUST classify the failure as `trusted companion unavailable or not trusted`
- AND it MUST offer a recovery action appropriate to restoring that companion path.

## ADDED Requirements

### Requirement: Client Startup Entry Point

Desktop, Android, and iOS composeApp clients MUST enter onboarding, readiness, or configuration UX
on
startup whenever a ready client connection is not already available.

Those surfaces MUST NOT attempt to satisfy startup by spawning a local runtime as the normal first
step.

#### Scenario: First startup enters onboarding state

- GIVEN a desktop, Android, or iOS user opens composeApp with no previously validated connection
- WHEN startup completes initial state selection
- THEN the user MUST land in onboarding, readiness, or configuration UX
- AND normal chat or session entry MUST remain unavailable until readiness succeeds.

#### Scenario: Existing but broken configuration enters recovery state

- GIVEN a desktop, Android, or iOS user opens composeApp with a previously saved but no longer valid
  client configuration
- WHEN startup checks the saved configuration
- THEN the user MUST land in a readiness recovery state instead of a normal chat workspace
- AND the surface MUST identify the recovery action needed to restore client readiness.

### Requirement: Minimal Client Configuration Surface

Desktop, Android, and iOS composeApp clients MUST provide a minimal configuration surface sufficient
to establish, inspect, and recover the approved client connection path for that platform.

- Desktop MUST allow the user to inspect and change the configured runtime URL or endpoint.
- Android MUST allow the user to inspect and change the configured runtime URL or endpoint.
- iOS MUST allow the user to inspect and perform the approved iOS connection setup action or actions
  for this milestone.
- All three clients MUST allow retry and reset or disconnect actions appropriate to the active
  connection path.
- All three clients MUST show the currently targeted runtime or endpoint identity when known.

#### Scenario: Desktop and Android can correct the target endpoint

- GIVEN a desktop or Android user is in onboarding or recovery
- WHEN the configured runtime URL or endpoint needs correction
- THEN the surface MUST let the user edit that target configuration
- AND the updated value MUST be used for the next readiness check.

#### Scenario: iOS exposes only approved configuration actions

- GIVEN an iOS user opens onboarding or recovery settings in this milestone
- WHEN the available configuration actions are shown
- THEN the surface MUST expose only the approved iOS connection actions for this milestone
- AND it MUST NOT expose unsupported local-host configuration steps as required actions.

### Requirement: Corrected Milestone Exclusions

This onboarding milestone MUST stay limited to client connection setup, readiness validation, and
client-safe recovery for desktop, Android, and iOS composeApp surfaces.

The system MUST NOT require the following for onboarding acceptance in this change:

- local `corvus` execution as the default client path,
- full runtime-backed chat, session, or approval completion,
- operator or admin configuration flows,
- memory administration,
- notifications,
- offline mode,
- multimodal input.

#### Scenario: Missing deferred features does not block onboarding acceptance

- GIVEN a composeApp client satisfies startup routing, supported connection setup, readiness
  validation, and recovery requirements for this milestone
- WHEN onboarding acceptance is evaluated
- THEN missing chat-turn parity, approvals, admin controls, notifications, offline mode, or
  multimodal features MUST NOT fail onboarding acceptance
- AND those features MUST remain out of scope until a later change adds them.
