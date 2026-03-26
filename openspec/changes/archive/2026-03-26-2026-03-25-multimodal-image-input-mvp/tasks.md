# Tasks: Multimodal Image Input MVP

## Execution Order

- Foundation first: runtime contracts, reusable ingress handle, config schema, and rejection taxonomy.
- Provider and channel streams can then proceed in parallel once the foundation lands.
- WhatsApp gateway convergence depends on both the runtime handle and WhatsApp channel parsing/fetch work.
- Telemetry should land before end-to-end verification so rejected/admitted paths are observable in tests.
- Final verification closes with regression coverage for text-only paths and rollout/rollback controls.

## Parallel Streams

- Stream A: runtime foundation + config schema
- Stream B: provider capability + adapter work (after Stream A)
- Stream C: Telegram channel ingest (after Stream A)
- Stream D: WhatsApp channel ingest (after Stream A)
- Stream E: gateway convergence + verification (after Streams A, B, and D)

## Phase 1: Runtime Foundation

- [x] 1.1 Add canonical multimodal channel contracts in `clients/agent-runtime/src/channels/traits.rs` for ordered `text` and `image` parts while preserving derived `content` compatibility behavior. Depends on: none. Deliverable: canonical `ChannelMessage` part model and text-projection rules aligned with `openspec/changes/2026-03-25-multimodal-image-input-mvp/specs/agent-loop/spec.md`.
- [x] 1.2 Create `clients/agent-runtime/src/channels/media.rs` with shared image staging types, MIME/size admission helpers, normalized rejection reasons, temp-file lifecycle handling, and startup cleanup hooks. Depends on: 1.1. Deliverable: reusable staging/validation module for Telegram and WhatsApp image turns.
- [x] 1.3 Refactor `clients/agent-runtime/src/channels/mod.rs` to introduce a reusable channel runtime handle that can enqueue canonical messages from listener channels and gateway-owned ingress. Depends on: 1.1. Deliverable: shared runtime seam for `start_channels(...)` and gateway handoff.
- [x] 1.4 Update `clients/agent-runtime/src/channels/mod.rs` to stage admitted image turns through the shared validation pipeline, enforce fail-closed behavior, preserve text-only compatibility, and delete staged files on all exit paths. Depends on: 1.2, 1.3. Deliverable: canonical image-turn processing path with single-turn retention semantics.
- [x] 1.5 Write failing and then passing runtime unit/integration tests covering text projection, MIME admission, byte ceilings, too-many-images rejection, staging cleanup, and provider-blocked image turns in `clients/agent-runtime` test modules. Depends on: 1.2, 1.4. Deliverable: runtime safety regression suite for the new multimodal seam.

## Phase 2: Provider Work

- [x] 2.1 Extend `clients/agent-runtime/src/providers/traits.rs` with explicit image-input capability metadata, accepted transport forms, and multimodal request/message-part contracts that keep image support route-aware. Depends on: 1.1. Deliverable: provider contract for capability-gated image routing.
- [x] 2.2 Update `clients/agent-runtime/src/providers/router.rs`, `clients/agent-runtime/src/providers/pool.rs`, and `clients/agent-runtime/src/providers/reliable.rs` so image turns resolve only through image-capable routes/accounts and never silently fall back to text-only providers. Depends on: 2.1. Deliverable: routing wrappers that preserve fail-closed semantics for image turns.
- [x] 2.3 Add Gemini multimodal request formatting in `clients/agent-runtime/src/providers/gemini.rs` using staged inline image payloads and preserve existing auth-mode behavior. Depends on: 2.1, 2.2. Deliverable: Gemini adapter support for canonical `text` + `image` requests.
- [x] 2.4 Add OpenAI-compatible multimodal request formatting in `clients/agent-runtime/src/providers/compatible.rs` using inline data URL payloads and reject unsupported fallback request shapes for image turns. Depends on: 2.1, 2.2. Deliverable: compatible-provider adapter support for canonical `text` + `image` requests.
- [x] 2.5 Write failing and then passing provider tests for capability declaration, transport-form compatibility, no-fallback routing, Gemini serialization, and OpenAI-compatible serialization in the provider test suite. Depends on: 2.2, 2.3, 2.4. Deliverable: provider coverage for all MVP in-scope routing and request-shape scenarios.

## Phase 3: Channel Work

- [x] 3.1 Update `clients/agent-runtime/src/channels/telegram.rs` to normalize inbound text, captioned photo, and image-document events into canonical parts using the largest supported image variant. Depends on: 1.1. Deliverable: Telegram parser support for MVP image-turn admission.
- [x] 3.2 Add Telegram media fetch helpers in `clients/agent-runtime/src/channels/telegram.rs` that resolve `getFile`, stream bytes under configured limits, and hand validated payloads to the shared staging pipeline. Depends on: 1.2, 3.1. Deliverable: Telegram-specific image retrieval integrated with runtime validation.
- [x] 3.3 Update `clients/agent-runtime/src/channels/whatsapp.rs` to normalize text and image webhook payloads into canonical parts, including media id and optional caption handling, while leaving non-MVP message types out of scope. Depends on: 1.1. Deliverable: WhatsApp parser support for MVP image-turn admission.
- [x] 3.4 Add WhatsApp Graph media resolution/download helpers in `clients/agent-runtime/src/channels/whatsapp.rs` that exchange media ids for signed URLs, stream bounded bytes, and hand validated payloads to the shared staging pipeline. Depends on: 1.2, 3.3. Deliverable: WhatsApp-specific image retrieval integrated with runtime validation.
- [x] 3.5 Write failing and then passing channel tests for Telegram caption/photo normalization, Telegram image-document filtering, WhatsApp image parsing, WhatsApp unsupported-message behavior, and channel-specific fetch failure mapping. Depends on: 3.2, 3.4. Deliverable: parser and fetch coverage for both MVP channels. (Telegram portion complete: 11 tests covering text-only, photo, caption+photo, document image/non-image/webp/gif filtering, largest variant selection, and fetch URL format. WhatsApp portion complete: 7 tests covering text-part production, image-part parsing, caption+image ordering, no-mime handling, missing-id skip, unsupported-type skip, empty-caption filtering. Runtime regression coverage now also proves MIME and oversize image rejections skip provider dispatch.)

## Phase 4: Config and Telemetry

- [x] 4.1 Extend `clients/agent-runtime/src/config/schema.rs` with `multimodal.enabled`, `allowed_channels`, `vision_model_hint`, `max_image_bytes`, and route-level `allow_image_input` controls. Depends on: 1.3, 2.1. Deliverable: rollout and rollback configuration surface for MVP image ingress.
- [x] 4.2 Update configuration loading/validation paths in `clients/agent-runtime` so invalid multimodal combinations fail fast and image turns reject when the configured vision route or channel allowlist is missing. Depends on: 4.1, 2.2. Deliverable: validated config behavior for default-deny rollout.
- [x] 4.3 Extend `clients/agent-runtime/src/observability/traits.rs` and connected observer plumbing with image-ingress lifecycle events, normalized reason codes, byte/mime metadata, and redaction-safe logging fields. Depends on: 1.2, 1.4. Deliverable: telemetry and tracing contract for admitted, rejected, provider-sent, and provider-error image turns.
- [x] 4.4 Write failing and then passing tests for config gating, disabled-channel rejection, missing vision-route rejection, telemetry emission, and log/observer redaction expectations. Depends on: 4.2, 4.3. Deliverable: config and observability regression coverage for rollout safety.

## Phase 5: Gateway Convergence and Verification

- [x] 5.1 Update `clients/agent-runtime/src/gateway/mod.rs` so `/whatsapp` remains responsible for transport verification, idempotency, and rate-control checks but hands admitted canonical messages into the shared channel runtime handle instead of calling provider chat directly. Depends on: 1.3, 3.3, 3.4. Deliverable: WhatsApp gateway convergence onto the canonical runtime seam.
- [x] 5.2 Add integration tests in `clients/agent-runtime` covering verified WhatsApp image turns entering the canonical runtime path, rejected transport never reaching the runtime, admitted image turns producing one provider request and one channel reply, and rejected image turns skipping provider dispatch. Depends on: 5.1, 2.5, 3.5, 4.4. Deliverable: end-to-end verification for the MVP execution path.
- [x] 5.3 Add regression tests confirming text-only Telegram, text-only WhatsApp, generic `/webhook`, and non-MVP providers remain text-only and unaffected by image-input plumbing. Depends on: 5.1. Deliverable: backward-compatibility coverage for out-of-scope surfaces and providers.
- [x] 5.4 Update `openspec/changes/2026-03-25-multimodal-image-input-mvp/specs/agent-loop/spec.md` and `openspec/changes/2026-03-25-multimodal-image-input-mvp/specs/agent-runtime-providers/spec.md` only if implementation discoveries require clarifying acceptance wording during apply/verify; otherwise confirm no spec delta is needed. Depends on: 5.2, 5.3. Deliverable: spec/implementation alignment check recorded before archive.
- [x] 5.5 Execute the standard verification stack (`make test`, `make build`) plus targeted image-turn scenarios for Telegram and WhatsApp rollout controls, and record results in the change verification artifact during the verify phase. Depends on: 5.2, 5.3. Deliverable: final readiness evidence for `sdd-verify` and rollout review.
