# Delta for Agent Runtime Providers

## ADDED Requirements

### Requirement: Image Input Capability Declaration

The system MUST declare image-input capability explicitly in provider capability metadata rather
than inferring it from provider family or model name alone. Capability metadata MUST state whether a
provider/account supports canonical inbound image parts and which image transport forms that
provider/account accepts. Supported transport forms for this MVP MUST be limited to provider-safe
remote references and runtime-managed inline image payloads. When image-input capability is absent,
unknown, or disabled, the system MUST treat that provider/account as text-only for routing
purposes.

#### Scenario: Provider declares accepted image transport forms

- GIVEN a Gemini or OpenAI-compatible provider/account is configured for MVP image input
- WHEN the runtime loads provider capability metadata
- THEN the system MUST expose that the provider/account supports image input
- AND the system MUST expose which of the MVP transport forms are accepted for that
  provider/account.

#### Scenario: Undeclared provider remains text-only

- GIVEN a provider/account configuration does not declare image-input capability
- WHEN the runtime evaluates that provider/account for an image turn
- THEN the system MUST treat the provider/account as not image-capable
- AND the provider/account MUST NOT be selected for canonical image routing.

### Requirement: MVP Provider Scope

The system MUST support canonical inbound image parts for OpenAI-compatible providers and Gemini in
this MVP. The system MUST NOT promise canonical inbound image support for Anthropic,
OpenRouter-specific routing, or any other provider family as part of this change.

#### Scenario: In-scope providers are eligible for MVP image routing

- GIVEN the runtime evaluates configured providers for a canonical image turn
- WHEN a configured provider belongs to the OpenAI-compatible family or Gemini and declares image
  capability
- THEN the system MUST consider that provider eligible for image routing
- AND eligibility MUST still depend on capability and transport-form compatibility.

#### Scenario: Out-of-scope provider is excluded from MVP promise

- GIVEN Anthropic or another non-MVP provider is configured in the runtime
- WHEN the runtime evaluates the provider for a canonical image turn
- THEN the system MUST NOT treat that provider as covered by this MVP image contract
- AND any future image support for that provider MUST be defined in a follow-up change.

### Requirement: Capability-Gated Image Routing and Fail-Closed Fallback

The system MUST route any turn containing canonical image parts only to a provider/account whose
declared capabilities include image input and an accepted transport form compatible with the
runtime-held image representation. The system MUST NOT send canonical image turns to text-only,
undeclared, disabled, or transport-incompatible providers. The system MUST NOT silently strip image
parts to force a text-only provider call. If no eligible provider/account is available, the system
MUST return a structured unsupported or unavailable outcome and SHOULD surface a channel-safe
explanation.

#### Scenario: Eligible provider receives the canonical image turn

- GIVEN a canonical image turn is ready for provider routing
- WHEN the selected provider/account declares image support and accepts the available normalized
  image transport form
- THEN the system MUST send the image turn to that provider/account
- AND the request MUST preserve the canonical image content needed for provider reasoning.

#### Scenario: No capable provider is available

- GIVEN a canonical image turn is ready for routing
- AND all configured providers are text-only, disabled for image input, or incompatible with the
  available image transport form
- WHEN the runtime selects a provider
- THEN the system MUST return a structured unsupported or unavailable outcome
- AND the system MUST NOT drop the image part and continue with a text-only provider request.

### Requirement: Provider Adaptation Data Minimization

The system SHOULD prefer Corvus-managed media retrieval and normalization over raw provider-side
fetching when provider-side fetching would weaken validation, determinism, or auditability. Provider
adapters MUST pass only the minimum normalized image data required by the chosen transport form,
MUST redact raw image payloads from logs and diagnostics, and MUST NOT retain raw image bytes longer
than required to complete the provider request.

#### Scenario: Runtime-managed inline payload is used for a validated image

- GIVEN a provider/account accepts runtime-managed inline image payloads
- WHEN the runtime has already validated and normalized the inbound image
- THEN the provider adapter MUST send only the minimum normalized payload needed for the request
- AND the adapter MUST NOT emit the raw image bytes into logs or diagnostics.

#### Scenario: Remote reference is rejected when it would weaken safety controls

- GIVEN a provider/account can fetch remote media references
- WHEN using that remote reference would bypass required validation, determinism, or audit controls
- THEN the system SHOULD choose Corvus-managed retrieval and normalization instead
- AND the adapter MUST NOT delegate the fetch in a way that weakens the MVP safety posture.
