# Proposal: Multi-Image Per-Turn Ingestion

## Intent

Corvus currently hard-rejects image turns after the first image even though the provider handoff is already mostly slice-oriented. This change enables a bounded multi-image turn path so users can send up to four images in one turn, and the runtime can validate, stage, observe, and dispatch those images consistently.

## Scope

### In Scope
- Raise the default image-per-turn limit from 1 to 4 and expose it as `multimodal.max_images_per_turn` with startup validation and an effective default.
- Update channel/runtime handoff so image turns preserve all admitted images through staging and provider request construction.
- Evolve `ImageIngressEvent` and related logging/observer flows so telemetry reports multi-image turns without collapsing metadata to the first image.
- Add or update focused tests for config validation, gating, provider payload construction, and observability behavior for multi-image turns.
- Update OpenSpec deltas for `channel-image-ingestion` and `runtime-image-pipeline`.

### Out of Scope
- Increasing per-image byte limits or changing supported MIME formats.
- Deduplication, image batching across turns, or persistence of raw image bytes in history.
- New channel integrations beyond the channels already covered by the current ingestion pipeline.

## Approach

Introduce a configurable effective `max_images_per_turn` alongside the existing byte-limit config, then replace the remaining hardcoded single-image checks in the channel gate and shared media validation with that effective value. Preserve the existing `Vec<StagedImage>`/slice-based runtime contract, but update provider request builders that still assume one image per user turn so they serialize all staged images in-order beside the user text/caption. Expand observability from first-image metadata to turn-level multi-image metadata while keeping the payload metadata-only and non-sensitive.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/channel-image-ingestion/spec.md` | Modified | Main spec must define configurable multi-image turn limits and multi-image telemetry semantics. |
| `openspec/specs/runtime-image-pipeline/spec.md` | Modified | Main spec must define provider handoff and validation behavior for up to four images per turn. |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Add `multimodal.max_images_per_turn`, validation, and effective default handling. |
| `clients/agent-runtime/src/channels/media.rs` | Modified | Replace hardcoded count limit with config-aware validation helpers and defaults. |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Gate multi-image turns using effective config and preserve all staged images through dispatch. |
| `clients/agent-runtime/src/observability/traits.rs` | Modified | Evolve `ImageIngressEvent` so multi-image turns do not collapse metadata to a single image. |
| `clients/agent-runtime/src/observability/log.rs` | Modified | Keep log output aligned with the new multi-image event contract. |
| `clients/agent-runtime/src/providers/*.rs` | Modified | Update provider-specific request builders that currently serialize only one image payload. |
| `clients/agent-runtime/src/channels/**/*.rs` and `clients/agent-runtime/src/providers/**/*.rs` tests | Modified | Add regression coverage for multi-image admission, rejection, payload ordering, and telemetry. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Some provider adapters still serialize only the first image, causing silent drops. | Med | Add provider-focused regression tests that assert full image arrays are emitted in request payloads. |
| Observability schema changes could break existing log/metric consumers. | Med | Keep the event metadata-only, document the contract delta, and prefer additive fields or explicit turn-level structures. |
| Configurability could allow unexpectedly large provider payloads when operators raise the image count. | Low | Keep the default at 4, validate configured bounds at startup, and retain existing per-image byte limits. |

## Rollback Plan

Revert the change by removing `multimodal.max_images_per_turn`, restoring the default limit to 1, and returning provider serialization and observability to the current single-image contract. Because this change is bounded to ingress validation, provider payload shaping, and telemetry metadata, rollback is a code/config revert with no persisted data migration.

## Dependencies

- Existing image-capable provider integrations in `clients/agent-runtime/src/providers/` must continue to accept the shared `&[StagedImage]` handoff contract.
- Follow-up `sdd-spec` work is required to capture the delta requirements for `channel-image-ingestion` and `runtime-image-pipeline`.

## Success Criteria

- [ ] A turn with up to 4 valid images is admitted, staged, and dispatched without dropping later images.
- [ ] A turn exceeding the effective configured limit is rejected with the correct user-visible error and observability reason.
- [ ] Provider payload builders preserve image ordering and include every staged image for supported providers.
- [ ] Observability events for multi-image turns expose non-sensitive metadata for the full turn rather than only the first image.
- [ ] Regression tests cover config validation, gating, provider payloads, and telemetry for multi-image turns.
