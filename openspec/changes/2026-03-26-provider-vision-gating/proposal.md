# Proposal: Provider Vision Capability Gating

## Intent

The multimodal image pipeline has channel ingestion (Telegram, WhatsApp, Discord) and runtime
normalization fully implemented, with working provider adapters for OpenAI-compatible and Gemini.
However, Anthropic — one of the three major cloud providers — lacks an image adapter, and the
existing provider capability matrix is undocumented as a formal spec.

This change closes both gaps: it formalizes the provider vision capability matrix as a spec
(including gating behavior that is already implemented but not formally specified), and delivers
the Anthropic image adapter so that all three major cloud providers support multimodal image input.

**Issue**: #268
**Depends on**: #266 (channel-image-ingestion), #267 (runtime-image-normalization-pipeline)

## Scope

### In Scope

- Formalize the provider vision capability matrix as a delta spec against
  `agent-runtime-providers`, documenting per-provider image support, transport forms, and API
  format contracts
- Implement Anthropic image adapter: `NativeContentOut::Image` variant, `capabilities()` override
  declaring `image_input: true` + `[InlineBytes]`, and `chat()` override building Anthropic
  `image` content blocks (`{type:"image", source:{type:"base64", media_type, data}}`)
- Formalize fail-closed gating behavior across trait default, router, and reliable provider layers
  as spec scenarios (documenting existing behavior, not changing it)
- Define provider-specific image format contracts: OpenAI data-URL content blocks, Anthropic base64
  source blocks, Gemini `inline_data` parts

### Out of Scope

- Ollama vision support (Wave 2 — needs vision model detection strategy; no standard capability
  query exists)
- URL-based transport (Anthropic Files API, Gemini GCS URIs) — future optimization
- Token budget management for base64 overhead (~33% size increase)
- Provider-side image resizing or compression
- Image history replay reconstruction in Anthropic adapter (tracked separately)

## Approach

**Type**: spec + design + code

The infrastructure for vision gating is already mature and fail-closed at every layer (trait
default, router, reliable provider). The work is primarily:

1. **Spec**: Write delta spec requirements and scenarios formalizing the provider capability matrix,
   per-provider image format contracts, and the Anthropic adapter contract. Update the existing
   `agent-runtime-providers` MVP provider scope requirement to include Anthropic.

2. **Design**: Document the Anthropic adapter architecture — `NativeContentOut::Image` variant
   structure, `chat()` override flow, base64 encoding from `StagedImage.temp_path`, and content
   block construction. Include a sequence diagram for the Anthropic image turn path.

3. **Code**: Implement the Anthropic adapter following the same pattern as
   `OpenAiCompatibleProvider::chat_multimodal()` (`compatible.rs:494-609`):
   - Add `NativeContentOut::Image` variant with `source_type`, `media_type`, `data` fields
   - Override `capabilities()` in `AnthropicProvider` to declare image support
   - Override `chat()` to inject image content blocks when `StagedImage` data is present
   - Read bytes from `StagedImage.temp_path` and base64-encode for the API payload

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/providers/anthropic.rs` | Modified | Add image adapter: `capabilities()` override + `chat()` image block injection + local `NativeContentOut::Image` variant |
| `openspec/specs/agent-runtime-providers/spec.md` | Modified | Delta spec: update MVP scope to include Anthropic, add image format contract requirements |
| `clients/agent-runtime/src/providers/router.rs` | None | Gating already correct — documented in spec only |
| `clients/agent-runtime/src/providers/reliable.rs` | None | Fallback already correct — documented in spec only |
| `clients/agent-runtime/src/providers/compatible.rs` | None | Reference pattern for the Anthropic adapter |
| `clients/agent-runtime/src/providers/gemini.rs` | None | Reference pattern for the Anthropic adapter |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Anthropic API image content block format changes | Low | Format is stable since `2023-06-01` API version; validate against current docs before implementation |
| Image history replay not handled in Anthropic adapter | Med | Out of scope for this change — `ImageHistoryMeta` exists but Anthropic reconstruction deferred; document as known gap |
| Base64 overhead crowds context window on small models | Low | No token budget system yet; defer to future optimization; large images are already size-gated at ingestion |
| `NativeContentOut::Image` variant breaks exhaustive matches | Low | Compiler enforces exhaustive matching — any missed arms will fail at build time |

## Rollback Plan

All changes are additive and behind capability declarations:

1. **Code rollback**: Revert the Anthropic adapter commit. The provider reverts to `image_input:
   false` (default), and the existing fail-closed gating at router/reliable layers will
   automatically exclude it from image routing. No data migration needed.
2. **Spec rollback**: Revert the delta spec. The `agent-runtime-providers` spec returns to its
   current MVP scope (OpenAI-compatible + Gemini only).
3. **No config changes required**: Anthropic image support is gated by the provider's own
   `capabilities()` declaration, not by external configuration. Removing the code removes the
   capability.

## Dependencies

- #266 channel-image-ingestion spec (merged)
- #267 runtime-image-normalization-pipeline (merged)
- Anthropic Messages API documentation for `image` content block format

## Success Criteria

- [ ] Anthropic provider declares `image_input: true` and `image_transport_forms: [InlineBytes]`
- [ ] Anthropic `chat()` override correctly builds `{type:"image", source:{type:"base64", media_type, data}}` content blocks from `StagedImage` data
- [ ] All existing provider tests pass (no regressions in OpenAI-compatible or Gemini adapters)
- [ ] New unit tests cover Anthropic image content block construction and capability declaration
- [ ] Router and reliable provider correctly route image turns to Anthropic when it is the selected provider
- [ ] Provider capability matrix is documented as a formal spec with Given/When/Then scenarios
- [ ] `cargo test` and `cargo clippy` pass with no warnings
