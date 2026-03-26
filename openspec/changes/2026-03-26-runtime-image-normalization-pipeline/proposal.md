# Proposal: Runtime Image Normalization Pipeline

## Intent

The Corvus runtime has a working multimodal image ingestion pipeline (`ContentPart::Image` →
`StagedImage` → `ChatRequest.images`), but it lacks a formal runtime-layer specification. The
existing `channel-image-ingestion` spec (#266) covers the channel layer only — it stops at the
`StagedImage` handoff boundary.

This creates three concrete problems:

1. **Multi-turn image conversations are broken.** After the first turn, image context is lost from
   conversation history. Users asking follow-up questions about an image get confused responses
   because the model has no record that an image was part of a prior exchange.

2. **Operator config override is silently ignored.** The `multimodal.max_image_bytes` config field
   exists in the schema but is never wired to the validation functions. Operators who set it expect
   it to work.

3. **The runtime contract is undocumented.** Provider dispatch format, error taxonomy, and the
   normalization pipeline have no spec. This blocks safe evolution of the provider layer (#268) and
   new channel implementations (#266 follow-ups).

This proposal formalizes the existing patterns as the canonical runtime contract and addresses the
gaps identified in the exploration phase.

**Reference**: GitHub issue [#267](https://github.com/dallay/corvus/issues/267), channel ingestion
spec (#266, `openspec/specs/channel-image-ingestion/spec.md`).

## Scope

### In Scope

- **Formalize the runtime normalization contract** — Specify `ContentPart::Image` → `StagedImage` →
  `ChatRequest.images` → provider content blocks as the canonical pipeline. Document invariants,
  ordering guarantees, and transport-form extensibility.
- **Design conversation-history image representation** — Define how image turns are stored in
  `ConversationHistory` so that subsequent turns retain image context. This is the most significant
  gap (exploration §Gap 2).
- **Wire `max_image_bytes` config override** — Connect `MultimodalConfig.max_image_bytes` to
  `validate_size()` and `stream_validate_and_stage()`, falling back to the `MAX_IMAGE_BYTES`
  constant when unset.
- **Formalize error taxonomy as stable contract** — Codify `ImageRejectionReason` variants, their
  user-facing messages, and observability event mappings as a versioned contract for API consumers
  and operator dashboards.

### Out of Scope

- **New channel implementations** — Slack, Matrix, and other channels are covered by #266 follow-up
  work. This change does not add channel-layer code.
- **Provider-specific adapters** — Anthropic, Gemini, and other non-OpenAI-compatible content block
  formats are tracked in #268. The runtime spec will acknowledge transport-form extensibility but
  will not implement new adapters.
- **GIF/animated image support** — Animated formats require additional validation (frame count,
  duration). Deferred.
- **Multi-image per turn** — `MAX_IMAGES_PER_TURN = 1` remains hardcoded. Multi-image support
  (DALLAY-195) requires provider capability negotiation and UX work beyond this scope.
- **`NormalizedImageMessage` intermediate type** — Exploration Approach 2 introduced this concept.
  It may be evaluated during design if the history gap requires a new type, but it is not a
  committed deliverable.

## Approach

This is primarily a **spec + design change with targeted code changes**:

1. **Spec**: Write a runtime-layer spec (`runtime-image-normalization`) that formalizes the existing
   pipeline as the canonical contract. This spec layers on top of (and cross-references) the
   channel-ingestion spec. It covers: normalization invariants, provider dispatch format,
   conversation history representation, config overrides, and error taxonomy.

2. **Design**: Produce a design document that details:
   - The conversation-history image representation (the biggest code change)
   - The `max_image_bytes` config wiring (small, mechanical)
   - Error taxonomy stabilization (mostly documentation, possibly adding stable error codes)

3. **Code**: Implement the designed changes:
   - Modify `handle_successful_response()` to store image metadata in conversation history
   - Wire `MultimodalConfig.max_image_bytes` through to validation functions
   - Add/update tests for both changes

4. **Verify**: Confirm all spec scenarios pass and the channel-ingestion spec remains unbroken.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/runtime-image-normalization/` | New | Runtime-layer spec formalizing the normalization pipeline |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | History storage in `handle_successful_response()` (~line 1210) |
| `clients/agent-runtime/src/channels/media.rs` | Modified | Wire `max_image_bytes` config to `validate_size()` and `stream_validate_and_stage()` |
| `clients/agent-runtime/src/config/schema.rs` | Modified | `max_image_bytes` startup validation (bounds checking with 50 MiB ceiling) |
| `clients/agent-runtime/src/providers/traits.rs` | Modified | `ChatMessage.image_metadata` field added; `user_with_images()` constructor |
| `clients/agent-runtime/src/providers/compatible.rs` | Unchanged | Base64/data-URL encoding documented but not modified |
| `openspec/specs/channel-image-ingestion/spec.md` | Unchanged | Cross-referenced; no modifications needed |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| History representation increases memory usage for long conversations | Medium | Design a compact representation (metadata + hash, not raw bytes). Set a TTL or sliding window for image metadata in history. |
| Changing history format breaks existing conversation replay | Low | The current format already loses image data. Any representation is strictly additive — no existing behavior degrades. |
| `max_image_bytes` override introduces operator misconfiguration surface | Low | Validate at config load time: reject values ≤ 0 or > hardcoded ceiling. Log effective limit at startup. |
| Spec drift between channel-ingestion and runtime-normalization specs | Medium | Explicit cross-references between specs. Both specs share the `StagedImage` boundary type as the interface contract. |
| Provider compatibility — non-OpenAI providers may reject the normalized format | Low | Out of scope for this change. The spec will document `ImageTransportForm` as extensible. Provider adapters (#268) own format translation. |

## Rollback Plan

All changes are additive and backward-compatible:

1. **Spec artifacts**: Delete `openspec/specs/runtime-image-normalization/` and the change folder.
   No runtime behavior changes.
2. **History representation**: Revert the `handle_successful_response()` change. Conversations
   return to the current behavior (image context lost after first turn). This is a degradation, not
   a breakage — it restores the status quo.
3. **Config wiring**: Revert the `validate_size()` / `stream_validate_and_stage()` changes.
   `max_image_bytes` returns to being a no-op config field. The hardcoded `MAX_IMAGE_BYTES`
   constant resumes as the sole limit.
4. **Git**: All code changes will be on a feature branch. `git revert` of the merge commit cleanly
   undoes everything.

## Dependencies

- **Channel ingestion spec** (`openspec/specs/channel-image-ingestion/spec.md`, #266) — This
  proposal layers on top of the channel-ingestion contract. The `StagedImage` type is the shared
  boundary.
- **No external dependencies** — All changes are within the `clients/agent-runtime` crate and
  `openspec/` artifacts.

## Success Criteria

- [ ] A runtime-layer spec exists that formalizes the `ContentPart::Image` → `StagedImage` →
  provider pipeline as the canonical contract
- [ ] The spec covers conversation-history image representation with concrete requirements and
  scenarios
- [ ] The spec formalizes the error taxonomy with stable identifiers
- [ ] `multimodal.max_image_bytes` config override is wired and effective in validation
- [ ] Multi-turn image conversations retain image context in history (model sees prior image turns)
- [ ] All existing `channel-image-ingestion` spec scenarios continue to pass
- [ ] Design document exists with architecture decisions for history representation
- [ ] Targeted tests cover the new behavior (history storage, config override)
