# Delta for Client Surfaces

## ADDED Requirements

### Requirement: Onboarding Contract Alignment

The `client-surfaces` capability matrix MUST remain the transport and capability source of truth for
all surfaces, while onboarding behavior MUST align to the shared product onboarding specification.
Each onboarding-capable surface SHALL map its first-run flow to the canonical onboarding steps
without changing its approved transport.

#### Scenario: Web dashboard aligns onboarding without changing transport
- GIVEN `clients/web/apps/dashboard` participates in first-run onboarding
- WHEN its flow is evaluated against the canonical onboarding model
- THEN it MUST implement the shared onboarding outcomes using HTTP Gateway transport only
- AND it MUST NOT introduce process bridges or direct runtime access.

#### Scenario: Mobile aligns onboarding without adopting HTTP pairing language
- GIVEN `clients/composeApp` participates in first-run onboarding
- WHEN its flow is evaluated against the canonical onboarding model
- THEN it MUST implement the shared onboarding outcomes using the approved CLI bridge path
- AND it MUST NOT redefine mobile linking as HTTP gateway pairing.

### Requirement: Cross-Surface Recovery State Coverage

All onboarding-capable surfaces MUST expose the shared recovery taxonomy defined by the onboarding
specification and MUST map transport-specific failures into those normalized states.

#### Scenario: Web and mobile expose comparable recovery states
- GIVEN `clients/web/apps/chat` and `clients/composeApp` encounter different transport failures
- WHEN each surface renders recovery guidance
- THEN each surface MUST use the normalized product-level recovery state that matches the failure
- AND a user comparing surfaces MUST be able to recognize equivalent failure categories.

#### Scenario: Operator surfaces expose operator-relevant recovery states
- GIVEN `clients/agent-runtime` or `clients/web/apps/dashboard` encounters an onboarding blockage
- WHEN recovery guidance is rendered
- THEN the surface MUST use the normalized recovery taxonomy for applicable states
- AND it MAY omit chat-only states that cannot occur on that operator surface.

## MODIFIED Requirements

### Requirement: Transport Invariant

Each surface MUST use exactly one transport for all runtime communication, and its onboarding flow
MUST validate readiness only through that approved transport.

(Previously: Each surface MUST use exactly one transport for all runtime communication.)

#### Scenario: Onboarding validates readiness through the approved transport
- GIVEN any onboarding-capable surface is preparing to enter ready state
- WHEN it validates runtime connectivity
- THEN it MUST perform that validation through the transport assigned in the canonical matrix
- AND it MUST NOT instruct the user to complete onboarding through another surface's transport as a
  substitute.
