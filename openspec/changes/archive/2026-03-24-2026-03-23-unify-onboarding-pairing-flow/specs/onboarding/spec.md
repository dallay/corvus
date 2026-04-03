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
- ComposeApp mobile MUST use linking terminology for bridge or companion-path trust establishment
  and MUST NOT describe that step as HTTP pairing.

#### Scenario: HTTP surface completes trust by pairing

- GIVEN the user is onboarding through web dashboard or web chat
- WHEN the trust-establishment step begins
- THEN the system MUST request a valid pairing code from the local runtime or gateway
- AND it MUST exchange that pairing code once for a bearer token before the surface is considered
  trusted.

#### Scenario: Mobile surface completes trust by linking

- GIVEN the user is onboarding through composeApp mobile
- WHEN the trust-establishment step begins
- THEN the system MUST guide the user through linking to the CLI bridge or companion path
- AND it MUST NOT describe the mobile trust step as pairing unless HTTP gateway transport becomes an
  explicitly approved exception.

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
- ComposeApp mobile MUST finish with bridge linking, runtime reachability, and readiness to create
  or resume a chat session.

#### Scenario: CLI completion includes optional dashboard continuation

- GIVEN a new operator completes CLI/runtime onboarding
- WHEN the flow reaches completion
- THEN the system MUST confirm local runtime readiness
- AND it MAY offer a separate dashboard activation continuation without redefining onboarding
  completion.

#### Scenario: Chat surface completion requires session entry

- GIVEN a user completes onboarding on web chat or composeApp mobile
- WHEN the surface enters ready state
- THEN the system MUST offer first-session creation or resumable-session entry
- AND the user MUST NOT be considered fully onboarded while chat entry remains blocked.

### Requirement: Recovery And Retry Taxonomy

Every onboarding-capable surface MUST expose recovery or retry states using the same product-level
taxonomy. The minimum taxonomy MUST include:

- Runtime unavailable.
- Surface transport unavailable.
- Pairing code invalid or expired.
- Bearer token missing, invalid, or revoked.
- Gateway reachable but not paired or authenticated.
- Bridge linked but session start or resume unavailable.
- Session expired or no resumable session exists.
- Local environment unsupported for the chosen surface.

Each surface MUST map its transport-specific failures into one of these states before presenting
user
guidance.

#### Scenario: HTTP pairing failure maps to a normalized recovery state

- GIVEN a web dashboard or web chat user submits an expired pairing code
- WHEN the trust step fails
- THEN the system MUST classify the failure as `pairing code invalid or expired`
- AND it MUST offer a retry path that does not require the user to infer a lower-level transport
  error.

#### Scenario: Mobile bridge failure maps to a normalized recovery state

- GIVEN a mobile user has completed linking but the bridge cannot start or resume a session
- WHEN the chat entry step fails
- THEN the system MUST classify the failure as `bridge linked but session start or resume
  unavailable`
- AND it MUST offer a retry or fallback path appropriate to the bridge environment.

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
