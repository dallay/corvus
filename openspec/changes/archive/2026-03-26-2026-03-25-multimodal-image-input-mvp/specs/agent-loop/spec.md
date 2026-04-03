# Delta for Agent Loop

## MODIFIED Requirements

### Requirement: Entry Points Alignment

The system MUST provide a unified loop contract across dispatcher-backed entry points: CLI,
channels, gateway `/webhook`, and admitted WhatsApp MVP image turns. Gateway `/webhook` MUST
execute through the canonical dispatcher boundary and MUST preserve the same session, policy,
approval, tool-dispatch, and result semantics as other canonical entry points unless an explicitly
documented transport compatibility shim applies. Gateway `/whatsapp` MUST preserve transport
verification, idempotency, and rate-control checks before canonical execution begins, and any
admitted WhatsApp turn that contains an MVP image part MUST execute through the same canonical
channel/runtime seam used by other dispatcher-backed turns. This change applies only to WhatsApp
MVP image turns and MUST NOT be interpreted as a broader parity promise for unrelated `/whatsapp`
behaviors.

Previously: WhatsApp was explicitly outside the parity contract and any `/whatsapp` behavior
changes were deferred to a separate follow-up.

#### Scenario: WhatsApp image turn enters the canonical runtime seam

- GIVEN a WhatsApp webhook event passes transport verification and contains an admitted MVP image
  turn
- WHEN the gateway hands the turn to the runtime
- THEN the system MUST execute that turn through the same canonical dispatcher-backed channel loop
  used for other admitted channel turns
- AND the turn MUST inherit the same session, policy, approval, tool, and result semantics.

#### Scenario: Rejected WhatsApp transport never reaches the runtime

- GIVEN a WhatsApp webhook event fails signature validation, idempotency, or another required
  transport check
- WHEN the gateway evaluates the event
- THEN the system MUST reject the event before canonical runtime execution begins
- AND the dispatcher-backed channel loop MUST NOT run for that rejected request.

## ADDED Requirements

### Requirement: MVP Inbound Image Turn Contract

The system MUST normalize admitted multimodal turns into an ordered canonical content-part contract
limited to `text` and `image` parts for this MVP. Each canonical image part MUST preserve the
originating channel identity, a channel media reference or runtime-managed media handle, MIME type
when known, and associated caption text when supplied by the channel. The system MUST preserve the
relative ordering of text and image parts that is required to reconstruct the user turn, and it
MUST NOT require generic document, audio, video, or arbitrary attachment semantics in this MVP.

#### Scenario: Telegram photo with caption is normalized into canonical parts

- GIVEN Telegram delivers a user message containing a photo and a caption
- WHEN the runtime admits the message as an MVP multimodal turn
- THEN the system MUST normalize the turn into canonical `text` and `image` parts
- AND the image part MUST retain the Telegram media reference and known metadata needed for later
  validation and provider adaptation.

#### Scenario: Non-image attachment is not coerced into an image turn

- GIVEN a channel event contains a document, audio clip, or video without an admitted MVP image
  part
- WHEN the runtime evaluates the event for this change
- THEN the system MUST NOT coerce that attachment into the canonical image-part contract
- AND the event MUST remain outside the multimodal image MVP scope.

### Requirement: Image Admission Safety and Retention Controls

The system MUST treat all inbound image media as untrusted. Before media fetch or provider handoff,
it MUST validate channel origin and enforce an image allowlist for MIME type, bounded retrieval,
and configured size ceilings. The system MUST redact raw image content from logs, traces, and
operator diagnostics, raw image bytes MUST be ephemeral, and raw image bytes MUST NOT be persisted
to long-term memory by default in this MVP. The system MUST emit rollout telemetry that can
distinguish admitted, rejected, filtered, and provider-routed image turns without exposing the
image contents themselves.

#### Scenario: Oversized or disallowed media is rejected before provider routing

- GIVEN an inbound Telegram or WhatsApp image exceeds the configured size ceiling or fails the
  allowed image MIME policy
- WHEN the runtime validates the admitted media
- THEN the system MUST reject the image turn before any provider request is made
- AND the rejection telemetry MUST identify the turn as filtered or rejected without logging the raw
  media payload.

#### Scenario: Admitted image bytes are handled ephemerally

- GIVEN an inbound image turn passes validation and completes provider processing
- WHEN the turn is recorded in runtime history and observability systems
- THEN the system MUST avoid persisting raw image bytes to long-term memory by default
- AND any stored audit or telemetry record MUST omit or redact the raw image payload.

### Requirement: MVP Channel Boundaries and Ingress Fallback

The system MUST support inbound image understanding in this MVP only for Telegram and WhatsApp. It
MUST NOT extend canonical image-turn admission to generic gateway `/webhook`, web chat, dashboard,
mobile bridge, Signal, Matrix, Email, or other channels as part of this change. When image ingress
is disabled for a supported channel, or a supported channel image turn is rejected by admission
policy, the system MUST fail closed for the image turn and MUST NOT silently drop the image while
continuing as if the request were text-only. The system SHOULD return a channel-safe explanation
that image input is unavailable or rejected.

#### Scenario: Supported channel image ingress is disabled by rollout control

- GIVEN Telegram or WhatsApp image ingress is disabled by configuration
- WHEN a user sends an image turn through that channel
- THEN the system MUST return an explicit unsupported or unavailable image outcome for that turn
- AND the system MUST NOT silently downgrade the turn into text-only processing.

#### Scenario: Out-of-scope surface remains text-only

- GIVEN a request reaches generic gateway `/webhook` or another out-of-scope surface with image-like
  input
- WHEN this MVP contract is evaluated
- THEN the system MUST NOT treat that surface as supporting canonical inbound image turns
- AND any broader multimodal surface support MUST be defined in a follow-up change.
