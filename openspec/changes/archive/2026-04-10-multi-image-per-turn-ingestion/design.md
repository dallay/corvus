# Design: Multi-Image Per-Turn Ingestion

## Technical Approach

This change keeps the existing channel → staging → provider slice contract and removes the remaining
single-image assumptions at the edges. The runtime already hands providers `&[StagedImage]`; the
design therefore focuses on four local changes instead of a broader multimodal refactor:

1. add a bounded `multimodal.max_images_per_turn` config with an effective default of 4,
2. use that effective limit in gate-time validation instead of the hardcoded `MAX_IMAGES_PER_TURN=1`,
3. preserve all staged images through provider dispatch and provider-outcome telemetry, and
4. evolve `ImageIngressEvent` from first-image metadata to turn-level image metadata without ever
   exposing raw payloads.

The implementation should stay inside existing Rust patterns in `config/schema.rs`,
`channels/media.rs`, `channels/mod.rs`, and the current observer backends. No new runtime layer,
provider trait, or history format is needed. Current main specs and the archived
`channel-image-ingestion-strategy` design provide the baseline because the active change-local delta
specs have not been written yet.

## Architecture Decisions

### Decision: Add a bounded config override instead of replacing the shared default constant

**Choice**: Add `multimodal.max_images_per_turn: Option<usize>` to `MultimodalConfig`, plus an
`effective_max_images_per_turn()` helper that defaults to `channels::media::DEFAULT_MAX_IMAGES_PER_TURN`
(4) and validates a conservative ceiling (recommended: 8).

**Alternatives considered**:
- Keep the limit hardcoded in `channels/media.rs`
- Make the field required instead of optional
- Allow any positive integer with no ceiling

**Rationale**: This follows the existing `max_image_bytes` pattern, preserves backward compatibility
for existing configs, and keeps operator control bounded. A ceiling is important because image count
multiplies provider payload size even when per-image byte limits remain unchanged.

### Decision: Keep turn-scoped slice handoff and do not introduce provider-specific image batching abstractions

**Choice**: Continue using `Vec<StagedImage>` in the channel pipeline and `ChatRequest.images: &[StagedImage]`
at the provider boundary. Fix gate/staging behavior around that existing contract rather than adding
new provider interfaces.

**Alternatives considered**:
- Introduce a new `TurnImages` wrapper type throughout the provider layer
- Split a multi-image turn into multiple provider calls
- Persist raw image bytes in history for replay

**Rationale**: The provider contract is already slice-based in `providers/traits.rs`, and the main
provider builders already iterate images in-order. A new abstraction would add churn without solving
the real issue, which is edge validation and telemetry. Splitting one user turn into multiple provider
requests would break turn semantics and complicate response handling.

### Decision: Emit one image-ingress event per turn with additive per-image summaries

**Choice**: Keep the current lifecycle model (`Admitted`, `Rejected`, `ProviderSent`, `ProviderError`)
as one event per turn, but extend `ImageIngressEvent` with ordered per-image summaries. Recommended
shape:

```rust
pub struct ImageIngressImageMeta {
    pub mime_type: String,
    pub byte_len: u64,
}

pub struct ImageIngressEvent {
    pub channel: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub outcome: ImageIngressOutcome,
    pub reason: Option<ImageIngressReason>,
    pub image_count: usize,
    pub images: Vec<ImageIngressImageMeta>,
    pub total_byte_len: Option<u64>,
    pub mime_type: Option<String>, // compatibility shim; populated only for single-image turns
    pub byte_len: Option<u64>,      // compatibility shim; populated only for single-image turns
}
```

**Alternatives considered**:
- Emit one event per image and keep the current schema unchanged
- Replace `mime_type`/`byte_len` immediately with only a vector field
- Store hashes or channel handles in telemetry

**Rationale**: One event per turn preserves the current lifecycle counting model and avoids metric/log
cardinality growth. The additive `images` field removes the misleading “first image wins” behavior.
Keeping `mime_type` and `byte_len` only for single-image turns is the safest compatibility path:
existing consumers do not break for the old case, and multi-image turns are no longer misreported.
Raw bytes, hashes, URLs, and handles stay out of telemetry.

### Decision: Clean up partially staged images inside `stage_channel_images()` on mid-turn failure

**Choice**: Add a local best-effort cleanup path inside `stage_channel_images()` so that if image N
fails after images 1..N-1 were already staged, those temp files are removed before returning the
rejection.

**Alternatives considered**:
- Accept partial-file leaks and rely only on startup reaping
- Construct `StagedImageGuard` only after the full staging loop and ignore mid-loop cleanup
- Stage all images in memory before writing any temp files

**Rationale**: With a single image, the leak surface was small; with multi-image turns it becomes a
normal failure mode. Local cleanup keeps request-path behavior aligned with the existing RAII cleanup
contract and avoids unnecessary temp-file growth without changing the overall architecture.

## Data Flow

### Runtime flow

```text
ChannelMessage.parts
  └─ image_parts().len()
        │
        ▼
gate_and_stage_images()
  ├─ effective_max_images_per_turn()
  ├─ validate_image_count(count, effective_limit)
  └─ stage_channel_images(config, msg)
        ├─ fetch_and_stage_image(...) for each image part in-order
        ├─ cleanup already-staged files on mid-loop failure
        └─ Vec<StagedImage>
               │
               ▼
Provider::chat(ChatRequest { images: &[StagedImage], ... })
  └─ attach all images to the last user message in-order
               │
               ▼
emit_image_provider_outcome(...)
  └─ ImageIngressEvent { image_count, images[], total_byte_len, ... }
               │
               ▼
StagedImageGuard::drop()
  └─ cleanup temp files after request completion
```

### Sequence diagram: admitted multi-image turn

```text
User/Channel        channels::mod        channel fetchers        provider         observer
     │                    │                    │                    │                │
     │ 2-4 image parts    │                    │                    │                │
     ├───────────────────▶│                    │                    │                │
     │                    │ validate count     │                    │                │
     │                    ├───────────────────▶│ stage img 1..N     │                │
     │                    │◀───────────────────┤ Vec<StagedImage>    │                │
     │                    │                    │                    │                │
     │                    ├─────────────────────────────────────────▶│ chat(images[]) │
     │                    │                    │                    │                │
     │                    ├─────────────────────────────────────────────────────────▶│
     │                    │      ImageIngressEvent(outcome=Admitted/ProviderSent)   │
     │                    │                    │                    │                │
     │◀───────────────────┤ response            │                    │                │
     │                    │ drop guard          │                    │                │
```

### Sequence diagram: partial staging failure

```text
channels::mod              channel fetchers                 observer
     │                           │                            │
     │ stage img 1               │                            │
     ├──────────────────────────▶│                            │
     │◀──────────────────────────┤ staged ok                  │
     │ stage img 2               │                            │
     ├──────────────────────────▶│                            │
     │◀──────────────────────────┤ FetchFailed / Oversize     │
     │ cleanup staged img 1      │                            │
     │ emit rejected turn event  ├───────────────────────────▶│
     │ return error              │                            │
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/config/schema.rs` | Modify | Add `multimodal.max_images_per_turn`, effective helper, startup validation, logging of effective limit, and focused config tests. |
| `clients/agent-runtime/src/channels/media.rs` | Modify | Replace the hardcoded count validator with a limit-aware helper and define the shared default/ceiling constants for image-count validation tests. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | Use the effective config limit in gate-time rejection text, preserve all staged images, emit turn-level multi-image telemetry, and clean up partially staged files on staging failure. |
| `clients/agent-runtime/src/observability/traits.rs` | Modify | Extend `ImageIngressEvent` with ordered per-image metadata and update observer trait tests. |
| `clients/agent-runtime/src/observability/log.rs` | Modify | Log multi-image event fields (`image_count`, `total_byte_len`, `images`) without logging sensitive payload data. |
| `clients/agent-runtime/src/providers/compatible.rs` | Modify | Add regression coverage that OpenAI-compatible payload construction preserves all images in order on the last user message. |
| `clients/agent-runtime/src/providers/anthropic.rs` | Modify | Add regression coverage that Anthropic content blocks append every staged image in order and still fail closed when the last non-system message is not a user message. |
| `clients/agent-runtime/src/providers/gemini.rs` | Modify | Add regression coverage that Gemini parts include every staged image in order on the last user content block. |

## Interfaces / Contracts

### Config shape

```toml
[multimodal]
enabled = true
allowed_channels = ["telegram", "discord"]
vision_model_hint = "vision"
max_image_bytes = 10485760
max_images_per_turn = 4
```

Recommended validation rules:

- absent or `null` → use default 4
- `0` → startup validation error
- `> 8` → startup validation error
- enabled/disabled state does not change bound validation behavior, matching `max_image_bytes`

### Validation helper contract

```rust
pub const DEFAULT_MAX_IMAGES_PER_TURN: usize = 4;
pub const MAX_IMAGES_PER_TURN_CEILING: usize = 8;

pub fn validate_image_count(
    count: usize,
    max_images_per_turn: usize,
) -> Result<(), ImageRejectionReason>;
```

This keeps the shared validator in `channels/media.rs` but removes the last hidden singleton limit.

### Observability contract

- `Rejected` before staging: `image_count` is set from message parts, `images=[]`, `total_byte_len=None`
- `Admitted` / `ProviderSent`: `images` contains one entry per staged image in original order,
  `total_byte_len` is the sum of staged byte sizes
- `ProviderError`: same image metadata as `Admitted`, because staging already succeeded
- `mime_type` / `byte_len` remain populated only when exactly one image exists; otherwise they are
  `None` to avoid false first-image reporting

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Config parsing and validation for `max_images_per_turn` default, zero rejection, ceiling rejection, and valid override | Extend inline tests in `config/schema.rs` alongside existing `max_image_bytes` coverage. |
| Unit | Count validation behavior for 1, 4, 5, and ceiling-bound values | Extend `channels/media.rs` tests to exercise explicit limit-aware validation. |
| Integration | Gate-time rejection text and behavior for turns above the effective limit | Add focused tests in `channels/mod.rs` around `gate_and_stage_images()` / channel handling helpers. |
| Integration | Partial staging cleanup when one image stages successfully and a later image fails | Add a regression test in `channels/mod.rs` that stages multiple temp files and asserts best-effort cleanup on error. |
| Integration | Provider outcome telemetry contains full ordered metadata instead of only the first image | Extend `channels/mod.rs` observer tests and `observability/traits.rs` tests to assert `images[]`, `total_byte_len`, and compatibility-shim behavior. |
| Provider | OpenAI-compatible, Anthropic, and Gemini request builders include every staged image in-order on the last user message | Add provider-local regression tests using staged temp files, without introducing new provider abstractions. |
| Compatibility | Single-image turns keep legacy fields populated and existing success paths unchanged | Add assertions in telemetry and provider tests for the unchanged 1-image case. |

## Migration / Rollout

No data migration is required.

Rollout should be compatibility-first:

1. ship `max_images_per_turn` as optional with default 4 so existing configs remain valid,
2. keep single-image behavior unchanged,
3. evolve telemetry additively by introducing `images` and `total_byte_len` while keeping legacy
   singular fields only for the single-image case,
4. update dashboards/log parsers that need multi-image fidelity to consume the new turn-level fields.

Operationally, this is a safe rollback: remove the config field, restore the default limit to 1, and
revert telemetry to the current single-image contract.

## Open Questions

- [ ] None blocking. The design recommends a ceiling of 8; if operators want stricter rollout control,
      that ceiling can be lowered without changing the overall architecture.
