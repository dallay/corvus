# Onboarding Specification

## Purpose

This specification defines the canonical Corvus onboarding and pairing journey across the
CLI/runtime,
web dashboard, web chat, and composeApp mobile surfaces. It establishes shared product terminology,
the required first-run sequence, surface-specific variants, and the recovery states that every
client
MUST expose without changing the existing transport contracts.

## Requirements

### Requirement: Canonical Onboarding Sequence

The system MUST define one product-level onboarding sequence for every new Corvus user or operator.
The canonical sequence MUST be:

1. Choose surface and intent.
2. Confirm runtime availability.
3. Establish trust with the runtime.
4. Connect the surface transport.
5. Confirm ready state.
6. Create or resume the first session when the surface supports chat.

Each surface MAY present these steps with different UI, but it MUST preserve the same user outcomes
and order dependencies.

#### Scenario: New operator follows the canonical sequence from the CLI

- GIVEN a first-run operator starts Corvus from the CLI/runtime surface
- WHEN the onboarding flow is presented
- THEN the flow MUST require intent selection, runtime availability confirmation, local trust
  confirmation, transport readiness confirmation, and ready-state confirmation in that order
- AND the flow MUST end with operator next steps instead of requiring chat session creation.

#### Scenario: New end-user follows the canonical sequence from a chat surface

- GIVEN a first-run user starts Corvus from web chat or composeApp mobile
- WHEN the onboarding flow is presented
- THEN the flow MUST require intent selection, runtime availability confirmation, trust
  establishment, transport connection, and ready-state confirmation before chat becomes available
- AND the flow MUST end with either first-session creation or resumable-session selection.

### Requirement: Shared Step Outcomes

Every surface MUST implement the canonical sequence using the same shared step outcomes.

- `Choose surface and intent` MUST identify whether the user is acting as an operator/admin or an
  end-user.
- `Confirm runtime availability` MUST verify that a usable Corvus runtime exists for the chosen
  transport.
- `Establish trust with the runtime` MUST complete the one-time trust step required for that
  surface.
- `Connect the surface transport` MUST verify the active transport path can reach the runtime with
  the trust state obtained in the previous step.
- `Confirm ready state` MUST state what the user can do next on that surface.
- `Create or resume the first session` MUST apply only to chat-capable surfaces.

#### Scenario: Shared steps map to the same user outcomes across surfaces

- GIVEN two different Corvus surfaces are compared for first-run onboarding
- WHEN the shared steps are evaluated
- THEN each step MUST describe the same user outcome regardless of transport
- AND surface-specific copy MUST NOT redefine the meaning of the shared steps.

#### Scenario: Operator surface stops before session creation

- GIVEN the chosen surface is CLI/runtime or web dashboard
- WHEN the user reaches ready state
- THEN the onboarding flow MUST describe the surface as ready for management tasks
- AND it MUST NOT require chat session creation as a completion condition.

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

### Requirement: Consistent Onboarding Terminology

The system MUST use the following terminology consistently across all onboarding artifacts and
surfaces:

- `Pairing` MUST mean the one-time HTTP code exchange that yields a bearer token.
- `Pairing code` MUST mean the short-lived code shown by the runtime for HTTP clients.
- `Bearer token` MUST mean the persisted HTTP credential used after successful pairing.
- `Linking` MUST mean mobile trust establishment to the CLI bridge or companion path.
- `Connect to gateway` MUST mean validating an HTTP client can reach and authenticate to the
  gateway.
- `Connect to runtime` MUST mean the product-level act of reaching a usable Corvus backend across
  any approved transport.

#### Scenario: Product copy distinguishes pairing from linking

- GIVEN onboarding copy is shown for web and mobile surfaces
- WHEN the trust step is described
- THEN web surfaces MUST use `pairing`, `pairing code`, and `bearer token` for HTTP trust
  establishment
- AND mobile surfaces MUST use `linking` for bridge trust establishment.

#### Scenario: Transport validation uses the correct connection term

- GIVEN a surface validates post-trust connectivity
- WHEN the connection state is shown to the user
- THEN HTTP surfaces MUST describe that state as connecting to the gateway
- AND all surfaces MAY additionally describe the outcome as connecting to runtime.

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

### Requirement: Retry Guidance Must Preserve Security Boundaries

Recovery guidance MUST stay within the existing transport and authentication boundaries for the
surface. Onboarding flows MUST NOT instruct users to bypass pairing, bearer token requirements,
origin protections, or mobile bridge requirements as a recovery shortcut.

#### Scenario: Dashboard retry guidance preserves secure HTTP pairing

- GIVEN the user is recovering from missing or revoked dashboard authentication
- WHEN retry guidance is shown
- THEN the system MUST direct the user back to the standard HTTP pairing and bearer-token flow
- AND it MUST NOT instruct the user to use direct runtime or insecure admin access as a workaround.

#### Scenario: Mobile retry guidance preserves bridge-only transport

- GIVEN the user is recovering on composeApp mobile
- WHEN retry guidance is shown
- THEN the system MUST direct the user to relink or restore the approved bridge path
- AND it MUST NOT present HTTP gateway pairing as the primary recovery path.

### Requirement: Source-Of-Truth Boundaries

This specification MUST be the product-level source of truth for onboarding sequence, terminology,
and recovery expectations. It MUST NOT replace the transport and capability authority of
`client-surfaces` or the operator activation authority of `dashboard`.

#### Scenario: Product onboarding defers transport authority to client-surfaces

- GIVEN a question arises about which transport a surface may use
- WHEN this onboarding specification is applied
- THEN the answer MUST be governed by the `client-surfaces` specification
- AND this specification MUST only define how onboarding maps to that approved transport.

#### Scenario: Product onboarding defers operator activation details to dashboard

- GIVEN a question arises about the operator-specific dashboard activation slice
- WHEN this onboarding specification is applied
- THEN the answer MUST be governed by the `dashboard` specification
- AND this specification MUST only require that the dashboard slice fits the shared onboarding
  model.
