# Delta for channel-image-ingestion

## MODIFIED Requirements

### Requirement: Canonical Ingestion Pipeline

The system MUST continue to follow the 5-step channel image ingestion pipeline, but the validation and
staging path MUST admit every image in a single user turn up to the effective
`multimodal.max_images_per_turn` value instead of stopping after the first image. The effective
default MUST be 4 when the config field is omitted. Admitted images MUST remain associated with the
same turn and MUST be staged in the same order they appeared in `ChannelMessage.parts`.

Previously: validation enforced a fixed `MAX_IMAGES_PER_TURN = 1`, which caused later images in the
same turn to be rejected before staging.

#### Scenario: Multiple images in one admitted turn are staged in order

- GIVEN multimodal is enabled for an allowed channel
- AND `multimodal.max_images_per_turn` is omitted from config
- WHEN a user sends a turn containing 3 valid images in channel order
- THEN the ingestion pipeline MUST admit all 3 images in that same turn
- AND it MUST stage all 3 images successfully
- AND the staged image sequence MUST preserve the original channel order

#### Scenario: Limit applies to the full turn, not only the first image

- GIVEN multimodal is enabled for an allowed channel
- AND `multimodal.max_images_per_turn` is set to `4`
- WHEN a user sends a turn containing 4 valid images
- THEN the ingestion pipeline MUST admit the full turn
- AND it MUST NOT reject images 2 through 4 solely because an earlier image was already admitted

### Requirement: Size and Count Limits

The system MUST enforce `multimodal.max_images_per_turn` as the effective per-turn image-count limit.
When the field is omitted, the effective default MUST be 4. When the field is provided, startup
validation MUST reject invalid values deterministically before runtime admission begins. A turn whose
image count exceeds the effective limit MUST be rejected as a whole with `ImageRejectionReason::TooManyImages`.

Previously: the image-count limit was a fixed constant of 1 and was not configurable.

#### Scenario: Default count limit admits up to four images

- GIVEN multimodal is enabled and `multimodal.max_images_per_turn` is not set
- WHEN a user sends a turn containing 4 valid images
- THEN the effective image-count limit MUST be 4
- AND the turn MUST be admitted

#### Scenario: Configured lower count limit is enforced

- GIVEN multimodal is enabled and `multimodal.max_images_per_turn` is set to `2`
- WHEN a user sends a turn containing 3 valid images
- THEN the turn MUST be rejected with `ImageRejectionReason::TooManyImages`
- AND the rejection MUST use the configured limit of 2

#### Scenario: Over-limit turn is rejected deterministically

- GIVEN multimodal is enabled and `multimodal.max_images_per_turn` is set to `4`
- WHEN a user sends a turn containing 5 images
- THEN the system MUST reject the turn with `ImageRejectionReason::TooManyImages`
- AND the user-visible error MUST report count `5` and limit `4`
- AND no image in that turn MUST be staged or partially admitted

### Requirement: Observability

The system MUST emit `ImageIngressEvent` semantics that represent the full image turn rather than
collapsing metadata to the first image. For admitted, rejected, and provider-bound multi-image turns,
observability MUST report the turn-level image count and metadata that accurately describe every
image admitted for that turn without including sensitive payload bytes.

Previously: observability captured only first-image metadata even when later images were present in
the turn.

#### Scenario: Admitted multi-image turn reports turn-level metadata

- GIVEN a user turn is admitted with 3 staged images
- WHEN the ingestion pipeline emits its observability event
- THEN the event MUST report an image count of 3
- AND the event metadata MUST describe the admitted turn rather than only the first image
- AND the event MUST NOT include raw image bytes

#### Scenario: Rejected over-limit turn reports full attempted count

- GIVEN `multimodal.max_images_per_turn` is `4`
- WHEN a user sends a turn containing 6 images
- THEN the rejection event MUST report outcome `Rejected`
- AND the reason MUST be `TooManyImages`
- AND the event metadata MUST report the attempted count of 6 and effective limit of 4

## ADDED Requirements

### Requirement: Regression Coverage for Multi-Image Channel Ingestion

The system MUST include regression coverage for multi-image channel ingestion behavior. Coverage
MUST verify the effective default limit, configured-limit admission, deterministic over-limit
rejection, preservation of staged-image ordering, and observability semantics for multi-image turns.

#### Scenario: Regression suite covers the default and configured count limits

- GIVEN the multi-image ingestion change is implemented
- WHEN the channel-ingestion regression suite is executed
- THEN it MUST include at least one case proving the default limit is 4
- AND it MUST include at least one case proving a configured limit below 4 is enforced

#### Scenario: Regression suite covers observability for multi-image turns

- GIVEN the multi-image ingestion change is implemented
- WHEN the channel-ingestion regression suite is executed
- THEN it MUST include a case for an admitted multi-image turn
- AND it MUST include a case for an over-limit rejected turn
- AND both cases MUST assert turn-level observability semantics rather than first-image-only metadata
