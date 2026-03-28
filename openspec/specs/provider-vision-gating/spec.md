# Provider Vision Gating

**Change**: provider-vision-gating
**Issue**: #268
**Date**: 2026-03-26
**Depends on**: #266 (channel-image-ingestion), #267 (runtime-image-normalization-pipeline)
**Cross-references**: `runtime-image-pipeline` spec, `agent-runtime-providers` spec

## MODIFIED Requirements

### Requirement: MVP Provider Scope

The system MUST support canonical inbound image parts for OpenAI-compatible providers, Gemini, and
Anthropic in this MVP. The system MUST NOT promise canonical inbound image support for Ollama,
GLM, Copilot, OpenRouter-specific routing, or any other provider family as part of this change.

(Previously: MVP scope was limited to OpenAI-compatible and Gemini only. Anthropic was explicitly
excluded from the MVP image promise.)

#### Scenario: In-scope providers are eligible for MVP image routing

- GIVEN the runtime evaluates configured providers for a canonical image turn
- WHEN a configured provider belongs to the OpenAI-compatible family, Gemini, or Anthropic and
  declares image capability
- THEN the system MUST consider that provider eligible for image routing
- AND eligibility MUST still depend on capability and transport-form compatibility

#### Scenario: Anthropic provider is now in MVP scope

- GIVEN an Anthropic provider is configured in the runtime with image capability declared
- WHEN the runtime evaluates the provider for a canonical image turn
- THEN the system MUST treat that provider as covered by the MVP image contract
- AND the provider MUST be eligible for image routing on the same terms as OpenAI-compatible and
  Gemini providers

#### Scenario: Out-of-scope providers remain excluded from MVP promise

- GIVEN Ollama, GLM, Copilot, or another non-MVP provider is configured in the runtime
- WHEN the runtime evaluates the provider for a canonical image turn
- THEN the system MUST NOT treat that provider as covered by this MVP image contract
- AND any future image support for that provider MUST be defined in a follow-up change

## ADDED Requirements

### Requirement: Provider Vision Capability Matrix (REQ-1)

The system MUST maintain a provider vision capability matrix that defines, for each supported
provider family, whether image input is supported in the current release and which transport forms
are accepted. The matrix for v1 MUST be:

| Provider Family      | Image Input | Transport Form | Status   |
|----------------------|-------------|----------------|----------|
| OpenAI-compatible    | Yes         | InlineBytes    | Complete |
| Gemini               | Yes         | InlineBytes    | Complete |
| Anthropic            | Yes         | InlineBytes    | Complete |
| Ollama               | No          | N/A            | Deferred |
| GLM                  | No          | N/A            | Deferred |
| Copilot              | No          | N/A            | Deferred |

Providers marked "Deferred" MUST NOT declare `image_input: true` in their capability metadata.
Adding image support for a deferred provider MUST be tracked as a separate change.

#### Scenario: Vision capability matrix matches provider declarations

- GIVEN the runtime loads all configured providers at startup
- WHEN it inspects the capability metadata for each provider
- THEN OpenAI-compatible, Gemini, and Anthropic providers MUST report `image_input: true`
- AND OpenAI-compatible, Gemini, and Anthropic providers MUST report `InlineBytes` in their
  `image_transport_forms`
- AND Ollama, GLM, and Copilot providers MUST report `image_input: false`
- AND Ollama, GLM, and Copilot providers MUST report an empty `image_transport_forms` list

### Requirement: Capability Declaration Contract (REQ-2)

Each provider MUST declare image support explicitly via `ProviderCapabilities`. A provider declares
image support by setting `image_input: true` AND listing at least one supported entry in
`image_transport_forms`. The canonical gate `supports_image_input()` MUST require both conditions
to be true.

A provider that does not override `capabilities()` MUST inherit the trait default, which MUST
declare `image_input: false` and an empty `image_transport_forms` list. This ensures undeclared
providers are treated as text-only without any code change.

#### Scenario: Provider declares image support with transport forms

- GIVEN a provider overrides `capabilities()` with `image_input: true` and
  `image_transport_forms: [InlineBytes]`
- WHEN the runtime calls `supports_image_input()` on that provider
- THEN the method MUST return `true`
- AND the provider MUST be eligible for image routing

#### Scenario: Provider declares image_input but no transport forms

- GIVEN a provider overrides `capabilities()` with `image_input: true` but
  `image_transport_forms: []`
- WHEN the runtime calls `supports_image_input()` on that provider
- THEN the method MUST return `false`
- AND the provider MUST NOT be eligible for image routing

#### Scenario: Provider uses trait default capabilities

- GIVEN a provider does not override `capabilities()`
- WHEN the runtime calls `supports_image_input()` on that provider
- THEN the method MUST return `false`
- AND the provider MUST be treated as text-only for routing purposes

### Requirement: Fail-Closed Gating (REQ-3)

The system MUST enforce fail-closed image gating at three layers. All three layers MUST reject
image turns directed at non-vision providers. The system MUST NOT rely on any single layer alone.

**Layer 1 — Trait default**: The default `Provider::chat()` implementation MUST return an error if
the `images` parameter is non-empty and the provider has not overridden `chat()`. This is the
last-resort safety net.

**Layer 2 — Router**: The `RouterProvider` MUST check
`capabilities().supports_image_input()` on the resolved provider BEFORE dispatching an image turn.
If the check fails, the router MUST reject the request with `RouteNotImageCapable` and MUST NOT
invoke the provider's `chat()` method.

**Layer 3 — Reliable wrapper**: The `ReliableProvider` MUST skip text-only providers in the
fallback chain when processing an image turn. It MUST only attempt image-capable providers. If no
image-capable provider is available in the fallback chain, it MUST fail with a structured error
indicating no image-capable provider is available. The reliable wrapper MUST NOT silently strip
images to fit a text-only provider.

#### Scenario: Trait default rejects image turn on unoverridden provider

- GIVEN a provider has not overridden the default `chat()` implementation
- WHEN the provider receives a `ChatRequest` with a non-empty `images` slice
- THEN the provider MUST return an error indicating it does not support image input
- AND no API call MUST be made to the provider's backend

#### Scenario: Router rejects image turn before dispatch

- GIVEN the router resolves a provider whose `supports_image_input()` returns `false`
- WHEN the router receives an image turn for that provider
- THEN the router MUST reject the request with `RouteNotImageCapable`
- AND the provider's `chat()` method MUST NOT be called
- AND the rejection MUST occur before any network request to the provider

#### Scenario: Reliable wrapper skips text-only providers for image turns

- GIVEN a reliable provider wraps a fallback chain of [Provider A (text-only), Provider B
  (image-capable)]
- WHEN an image turn is processed
- THEN the reliable wrapper MUST skip Provider A
- AND the reliable wrapper MUST route the image turn to Provider B
- AND Provider A's `chat()` method MUST NOT be called

#### Scenario: Reliable wrapper fails when no image-capable provider exists

- GIVEN a reliable provider wraps a fallback chain where all providers are text-only
- WHEN an image turn is processed
- THEN the reliable wrapper MUST return a structured error
- AND the error MUST indicate that no image-capable provider is available
- AND no provider's `chat()` method MUST be called

### Requirement: Provider-Specific Image Format Contracts (REQ-4)

Each vision-capable provider MUST construct image content blocks in the format required by its
backend API. The runtime-normalized `StagedImage` (see `runtime-image-pipeline` spec REQ-1) is
the canonical input. All providers MUST read image bytes from `StagedImage.temp_path` and
base64-encode them for the `InlineBytes` transport form.

**OpenAI-compatible format**: The provider MUST construct an `image_url` content block within the
message's `content` array. The `image_url.url` field MUST be a data URL in the format
`data:{mime_type};base64,{base64_data}`. The content block type MUST be `"image_url"`.

**Anthropic format**: The provider MUST construct an `image` content block within the message's
`content` array. The content block MUST contain a `source` object with `type: "base64"`,
`media_type` set to the image's MIME type string (e.g., `"image/jpeg"`), and `data` set to the
raw base64-encoded bytes (no data URL prefix). The content block type MUST be `"image"`.

**Gemini format**: The provider MUST construct an `inline_data` part within the message's `parts`
array. The `inline_data` object MUST contain `mime_type` set to the image's MIME type string and
`data` set to the raw base64-encoded bytes. The part MUST appear alongside any text parts in the
same `parts` array.

All providers MUST attach image content blocks to the last user message in the provider request.
Providers MUST NOT emit raw image bytes into logs or diagnostics (see `agent-runtime-providers`
spec, Requirement: Provider Adaptation Data Minimization).

#### Scenario: OpenAI-compatible provider formats image as data URL content block

- GIVEN a `StagedImage` with `mime_type=image/jpeg` and bytes at `temp_path`
- WHEN the OpenAI-compatible provider constructs the API request
- THEN the last user message's `content` array MUST contain an object with
  `type: "image_url"` and `image_url.url` matching `data:image/jpeg;base64,{base64_data}`
- AND the base64 data MUST be read from `StagedImage.temp_path`

#### Scenario: Anthropic provider formats image as base64 source content block

- GIVEN a `StagedImage` with `mime_type=image/png` and bytes at `temp_path`
- WHEN the Anthropic provider constructs the API request
- THEN the last user message's `content` array MUST contain an object with `type: "image"`
- AND the object MUST contain `source.type: "base64"`
- AND the object MUST contain `source.media_type: "image/png"`
- AND the object MUST contain `source.data` set to the base64-encoded bytes from `temp_path`
- AND the `source.data` field MUST NOT include a `data:` URL prefix

#### Scenario: Gemini provider formats image as inline_data part

- GIVEN a `StagedImage` with `mime_type=image/webp` and bytes at `temp_path`
- WHEN the Gemini provider constructs the API request
- THEN the message's `parts` array MUST contain an object with `inline_data.mime_type: "image/webp"`
- AND `inline_data.data` MUST be set to the base64-encoded bytes from `temp_path`
- AND the `inline_data` part MUST coexist with any text parts in the same `parts` array

### Requirement: Error Behavior for Non-Vision Providers (REQ-5)

When a user sends an image turn and the resolved provider does not support image input, the system
MUST reject the request with `RouteNotImageCapable` BEFORE making any API call to the provider's
backend. The rejection MUST occur at the router layer (REQ-3, Layer 2).

The system MUST NOT:
- Silently strip image parts and send a text-only request
- Queue the image for later processing
- Attempt to transcoded the image into a text description as a fallback

The user MUST receive the error message defined in the `runtime-image-pipeline` spec REQ-7 error
taxonomy: "The configured vision route does not allow image input."

#### Scenario: Non-vision provider rejects image before API call

- GIVEN the router resolves a provider with `supports_image_input() == false`
- WHEN a user sends a message containing an image
- THEN the system MUST reject with `RouteNotImageCapable`
- AND zero HTTP requests MUST be made to the provider's API endpoint
- AND the user MUST receive "The configured vision route does not allow image input."

#### Scenario: Image parts are never silently stripped

- GIVEN a text-only provider is the only configured route
- WHEN a user sends an image turn
- THEN the system MUST NOT remove the image and forward only the text
- AND the system MUST return a rejection, not a partial text-only response

### Requirement: Config Integration for Vision Routing (REQ-6)

The `vision_model_hint` field in `[multimodal]` config MUST resolve to a model route where
`allow_image_input=true`. If the hint resolves to a route with `allow_image_input=false` or
unset, the system MUST reject image turns with `RouteNotImageCapable` at the config gate level
(see `runtime-image-pipeline` spec REQ-2 step 2).

When multiple providers are configured and the `vision_model_hint` resolves to a valid vision
route, the router MUST select that route for image turns regardless of the default text route
configuration.

#### Scenario: vision_model_hint resolves to image-capable route

- GIVEN `vision_model_hint = "gpt-4o"` and route `"gpt-4o"` has `allow_image_input=true`
- AND route `"gpt-4o"` is backed by an OpenAI-compatible provider declaring image capability
- WHEN a user sends an image turn
- THEN the router MUST select the `"gpt-4o"` route for the image turn
- AND the image MUST be dispatched to the OpenAI-compatible provider

#### Scenario: vision_model_hint resolves to non-image route

- GIVEN `vision_model_hint = "text-model"` and route `"text-model"` has
  `allow_image_input=false`
- WHEN a user sends an image turn
- THEN the system MUST reject with `RouteNotImageCapable`
- AND the user MUST receive "The configured vision route does not allow image input."
- AND no provider MUST receive the image turn

#### Scenario: Router selects vision route over default text route

- GIVEN `vision_model_hint = "claude-sonnet"` with `allow_image_input=true`
- AND the default text route is `"gpt-4o-mini"` with `allow_image_input=false`
- WHEN a user sends an image turn
- THEN the router MUST select the `"claude-sonnet"` vision route
- AND the router MUST NOT use the default `"gpt-4o-mini"` text route for the image turn

#### Scenario: Multiple vision-capable providers with explicit hint

- GIVEN routes `"gpt-4o"` (OpenAI, image-capable) and `"claude-sonnet"` (Anthropic, image-capable)
  are both configured
- AND `vision_model_hint = "claude-sonnet"`
- WHEN a user sends an image turn
- THEN the router MUST select the `"claude-sonnet"` route as specified by the hint
- AND the image MUST be formatted using the Anthropic content block format (REQ-4)
