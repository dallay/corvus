# Delta for runtime-image-pipeline

## MODIFIED Requirements

### Requirement: Canonical Runtime Representation

The runtime MUST preserve all staged images from a single admitted turn through the canonical
`ContentPart::Image` → `StagedImage` → `ChatRequest.images` chain. Provider dispatch MUST consume the
entire `&[StagedImage]` slice for that turn, MUST preserve image order, and MUST NOT collapse the
turn to only the first staged image.

Previously: the canonical chain used an images slice, but provider request construction and adjacent
runtime behavior could still serialize only the first image from a turn.

#### Scenario: Provider dispatch preserves every staged image in order

- GIVEN a user turn has 3 staged images with distinct metadata in channel order
- WHEN the runtime constructs the provider request for that turn
- THEN the provider request MUST include all 3 staged images
- AND the images MUST appear in the same order as the staged-image slice
- AND none of the staged images in the slice may be dropped silently

### Requirement: Normalization Pipeline

The runtime MUST apply gating and handoff semantics to the full set of images in a single turn.
During gate evaluation, the runtime MUST compare the attempted image count to the effective
`multimodal.max_images_per_turn` value. During handoff, the runtime MUST attach all admitted staged
images for the last user turn to the provider request while preserving order.

Previously: the pipeline described an image-count check against a fixed limit of 1 and did not
explicitly require full-slice provider serialization for multi-image turns.

#### Scenario: Multi-image turn reaches provider handoff intact

- GIVEN multimodal is enabled with an effective image-count limit of 4
- WHEN a user sends a turn containing 3 valid images and text
- THEN gate evaluation MUST admit the turn
- AND the runtime MUST hand off all 3 staged images to the provider layer for that same turn
- AND the provider request MUST associate the full image set with the user's turn

### Requirement: Size and Count Limits

The runtime MUST treat `multimodal.max_images_per_turn` as the authoritative per-turn image-count
limit. The field MUST default to 4 when omitted. The runtime MUST admit turns with image counts less
than or equal to the effective limit and MUST reject turns above that limit without partially
dispatching any subset of their staged images.

Previously: the runtime enforced `MAX_IMAGES_PER_TURN = 1`.

#### Scenario: Default count limit allows four images

- GIVEN `multimodal.max_images_per_turn` is omitted
- WHEN a user sends a turn containing 4 valid images
- THEN the effective count limit MUST be 4
- AND the runtime MUST admit the turn for provider dispatch

#### Scenario: Fifth image triggers whole-turn rejection

- GIVEN `multimodal.max_images_per_turn` is omitted
- WHEN a user sends a turn containing 5 valid images
- THEN the runtime MUST reject the turn with `TooManyImages`
- AND it MUST NOT dispatch only the first 4 images to the provider

### Requirement: Error Taxonomy

The runtime MUST preserve the existing `TooManyImages` rejection reason while making the user-visible
error deterministic for configurable limits. The `TooManyImages` message MUST report the attempted
count and effective configured limit for the rejected turn.

Previously: `TooManyImages` was specified against a fixed maximum of 1 per message.

#### Scenario: Deterministic over-limit error reflects effective limit

- GIVEN `multimodal.max_images_per_turn` is set to `3`
- WHEN a user sends a turn containing 5 images
- THEN the runtime MUST reject with reason `TooManyImages`
- AND the user-facing error MUST state "Too many images (5). Maximum 3 per message."

### Requirement: Configuration Contract

The `[multimodal]` config section MUST support `max_images_per_turn` as an optional integer field.
When omitted, the effective value MUST default to 4. When set, startup validation MUST require a
positive integer value. Invalid values MUST produce a startup validation error before the runtime
accepts traffic.

Previously: the config contract supported image-ingress settings such as `max_image_bytes` and the
staged-image reaper threshold, but not a configurable per-turn image-count limit.

#### Scenario: Default max-images value is applied

- GIVEN a config file omits `multimodal.max_images_per_turn`
- WHEN the runtime starts
- THEN the effective max-images-per-turn value MUST be 4

#### Scenario: Invalid max-images value fails startup validation

- GIVEN a config file sets `multimodal.max_images_per_turn=0`
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error
- AND the error MUST indicate that the max-images-per-turn value must be greater than 0

## ADDED Requirements

### Requirement: Observability for Multi-Image Runtime Dispatch

The runtime MUST represent multi-image turns in observability using turn-level metadata for the full
set of staged images carried into provider dispatch. Observability for admitted, rejected,
provider-sent, and provider-error outcomes MUST remain metadata-only and MUST distinguish multi-image
turns from single-image turns without dropping later-image context.

#### Scenario: Provider-bound event represents the full dispatched turn

- GIVEN the runtime dispatches a turn containing 4 staged images to a provider
- WHEN the provider-bound observability event is emitted
- THEN the event MUST represent all 4 images for that turn
- AND it MUST preserve their turn order in the metadata representation
- AND it MUST NOT include raw bytes or base64 payloads

#### Scenario: Provider error still reports the full multi-image turn

- GIVEN the runtime attempts to dispatch a turn containing 2 staged images
- AND the provider returns an error
- WHEN the provider-error observability event is emitted
- THEN the event MUST report outcome `ProviderError`
- AND it MUST describe the same 2-image turn that was dispatched
- AND it MUST NOT degrade to first-image-only metadata

### Requirement: Regression Coverage for Multi-Image Runtime Behavior

The system MUST include regression coverage for runtime multi-image behavior. Coverage MUST verify
config validation for `max_images_per_turn`, whole-turn over-limit rejection, provider dispatch of
all staged images in order, and observability semantics for multi-image turns.

#### Scenario: Regression suite covers provider slice preservation

- GIVEN the multi-image runtime change is implemented
- WHEN the runtime regression suite is executed
- THEN it MUST include a case that verifies all staged images in a turn are serialized to the provider request in order

#### Scenario: Regression suite covers deterministic over-limit failures

- GIVEN the multi-image runtime change is implemented
- WHEN the runtime regression suite is executed
- THEN it MUST include a case for an over-limit turn rejected with `TooManyImages`
- AND that case MUST assert the deterministic user-facing message with attempted count and effective limit
