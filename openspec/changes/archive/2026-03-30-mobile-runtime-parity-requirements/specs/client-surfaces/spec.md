# Delta for client-surfaces

## MODIFIED Requirements

### Requirement: Transport Invariant

Each surface MUST use exactly one approved transport for all runtime communication, and its
onboarding
flow MUST validate readiness only through that approved transport.

For the composeApp client surfaces in this milestone:

- Desktop MUST be treated as a client-first surface.
- Android MUST be treated as a client-first surface.
- iOS MUST be treated as a client-first surface.
- Desktop and Android MUST NOT assume a locally installed `corvus` binary, packaged executable, or
  immediate local process execution as the default path.
- iOS MUST NOT imply that local `corvus` execution is the expected default path.
- Desktop MUST support connecting to an existing runtime through runtime URL or endpoint
  configuration.
- Android MUST support connecting to an existing runtime through runtime URL or endpoint
  configuration.
- iOS MUST expose only the approved client connection path or paths supported on iOS for this
  milestone, which MAY include runtime URL or endpoint configuration and MAY include pairing or a
  trusted companion flow.

(Previously: The requirement treated composeApp mobile parity as a runtime-hosted bridge milestone,
centered Android and iOS on a mobile runtime bridge path, and excluded HTTP or endpoint-led client
setup as the primary product contract.)

#### Scenario: Desktop starts as a client instead of a local host

- GIVEN a desktop user opens composeApp with no saved ready connection
- WHEN startup is evaluated
- THEN the surface MUST enter onboarding, readiness, or configuration UX
- AND it MUST NOT immediately spawn, probe, or require a local `corvus` process as the default
  action.

#### Scenario: Android starts as a client instead of a packaged runtime host

- GIVEN an Android user opens composeApp with no saved ready connection
- WHEN startup is evaluated
- THEN the surface MUST enter onboarding, readiness, or configuration UX
- AND it MUST NOT assume a packaged executable, local binary, or immediate process launch is
  available.

#### Scenario: iOS shows only supported client connection paths

- GIVEN an iOS user opens composeApp for first-run setup
- WHEN the app presents connection options
- THEN the app MUST present only the iOS connection path or paths approved for this milestone
- AND it MUST NOT present local runtime execution as a default or required iOS path.

### Requirement: Capability Tier Enforcement

Each surface MUST implement only the capabilities assigned to it in the canonical matrix.

For this milestone, the required composeApp client capability set for desktop, Android, and iOS MUST
be limited to:

- startup routing into onboarding, readiness, and configuration UX,
- supported connection-path selection or configuration,
- display of the currently targeted runtime or endpoint,
- display of trust, auth, link, or pairing state as applicable to that platform,
- user-safe reachability and readiness checks,
- retry, edit, reset, disconnect, or re-pair actions appropriate to the active connection path,
- gating of chat or session entry until ready state is confirmed.

For this milestone, composeApp client surfaces MUST NOT be required to provide:

- runtime-backed chat-turn parity,
- session creation, resumption, or termination parity,
- tool approval handling,
- operator or admin capabilities,
- runtime configuration editing beyond client connection settings,
- memory browsing,
- multimodal input,
- notifications,
- offline mode,
- local runtime hosting as a milestone acceptance condition.

(Previously: The requirement defined a runtime-backed mobile parity capability set including
session,
chat, and approval behavior, and treated those end-user flows as mandatory for this milestone.)

#### Scenario: Client settings expose only readiness-critical controls in this milestone

- GIVEN a desktop, Android, or iOS user opens client settings during this milestone
- WHEN the available controls are inspected
- THEN the surface MUST provide only connection setup, readiness diagnostics, and recovery controls
- AND it MUST NOT fail milestone acceptance for omitting full chat, session, approval, admin, or
  memory features.

#### Scenario: Chat entry remains gated until readiness succeeds

- GIVEN a desktop, Android, or iOS user has not yet completed the required connection and readiness
  checks
- WHEN the user attempts to enter normal chat or session flow
- THEN the surface MUST keep that entry blocked
- AND it MUST direct the user to the unresolved onboarding, readiness, or configuration state.

## ADDED Requirements

### Requirement: Client-First Startup Routing

Desktop, Android, and iOS composeApp clients MUST route startup into onboarding, readiness, or
configuration UX before normal chat startup whenever a ready client connection is not already
available.

If a previously configured connection exists, startup MUST still land in a readiness-confirmed
client
state rather than silently starting a local runtime.

#### Scenario: First launch goes to onboarding instead of chat workspace

- GIVEN a user launches desktop, Android, or iOS composeApp for the first time
- WHEN no ready client connection has been established yet
- THEN startup MUST open onboarding, readiness, or configuration UX first
- AND the surface MUST NOT drop the user directly into a normal chat workspace backed by assumed
  local execution.

#### Scenario: Relaunch with saved configuration still stays client-first

- GIVEN a user relaunches desktop, Android, or iOS composeApp with a previously saved target runtime
  configuration
- WHEN startup validates the saved state
- THEN the surface MUST show readiness-confirmed client state or an actionable recovery state
- AND it MUST NOT silently spawn a local runtime as part of normal relaunch behavior.

### Requirement: Platform-Specific Connection Path Disclosure

Each composeApp client surface MUST disclose only the connection paths that are actually supported
on
that platform in this milestone.

- Desktop MUST disclose runtime URL or endpoint configuration as a supported path.
- Android MUST disclose runtime URL or endpoint configuration as a supported path.
- If desktop or Android also support pairing or a trusted companion flow in this milestone, the
  surface MUST guide that flow explicitly.
- iOS MUST disclose at least one approved client connection path for this milestone.
- If pairing or a trusted companion flow is the approved iOS path, iOS onboarding MUST guide that
  flow explicitly.
- A surface MUST NOT present unsupported connection paths as available, required, or coming from
  default local execution.

#### Scenario: Unsupported connection path is not shown as available

- GIVEN a platform does not support a pairing, trusted companion, or endpoint path in this milestone
- WHEN the connection setup UI is rendered on that platform
- THEN the unsupported path MUST be absent or clearly marked unavailable
- AND the user MUST NOT be told to complete setup through that unsupported path.

#### Scenario: Supported connection path includes platform-appropriate guidance

- GIVEN a platform supports runtime endpoint configuration or a pairing or trusted companion flow in
  this milestone
- WHEN the user starts connection setup
- THEN the surface MUST guide the user through that supported path
- AND the guidance MUST describe the path as connecting to an existing runtime rather than starting
  a
  local host by default.

### Requirement: Milestone Scope Exclusions

This milestone MUST stay limited to client-first onboarding, readiness, and connection configuration
for desktop, Android, and iOS.

The system MUST NOT treat the following as required for milestone completion:

- default local runtime execution on any composeApp client surface,
- mandatory local `corvus` installation guidance as the normal path,
- runtime-backed chat, session, or approval parity,
- dashboard or admin capabilities,
- raw memory visibility,
- multimodal features,
- notifications,
- offline mode,
- background automation beyond preserving client configuration needed to re-enter readiness UX.

#### Scenario: Milestone acceptance does not depend on full chat parity

- GIVEN desktop, Android, and iOS satisfy the client-first startup, connection setup, readiness, and
  recovery requirements
- WHEN the milestone is evaluated for acceptance
- THEN missing runtime-backed chat, session, approval, notification, offline, or admin features MUST
  NOT fail the milestone
- AND those capabilities MUST remain follow-on work until another change adds them.
