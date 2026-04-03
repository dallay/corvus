# Tasks: Runtime Image Normalization Pipeline

## Phase 1: Documentation (spec + design)

All documentation artifacts for this change are complete.

- [x] 1.1 Explore existing codebase and identify gaps (exploration phase)
- [x] 1.2 Write proposal.md with intent, scope, approach, and rollback plan
- [x] 1.3 Write runtime-image-pipeline spec (
  `openspec/changes/.../specs/runtime-image-pipeline/spec.md`) covering REQ-1 through REQ-8
- [x] 1.4 Write design.md with ADR-1 through ADR-5, sequence diagrams, and interface contracts
- [x] 1.5 Write tasks.md (this file)

## Phase 2: Code Implementation (targeted code changes)

### 2A: Foundation — New types and modified signatures

- [x] 2.1 Add `ImageHistoryMeta` struct to `clients/agent-runtime/src/channels/media.rs`
    - Define struct with fields: `mime`, `sha256`, `byte_len`, `channel_origin`, `caption`,
      `description` (per design ADR-2)
    - Derive `Debug, Clone, Serialize, Deserialize`
    - Implement
      `ImageHistoryMeta::from_staged(staged: &StagedImage, caption: Option<String>) -> Self`
    - Implement `ImageHistoryMeta::to_context_string() -> String` for history injection
    - **Acceptance**: Struct compiles, `from_staged` maps all fields correctly, `to_context_string`
      produces `[Prior image: {mime}, {byte_len} bytes, sha256:{prefix}. Description: {desc}]`
      format
    - **Spec ref**: REQ-6 (conversation history image representation)

- [x] 2.2 Add `image_metadata` field to `ChatMessage` in
  `clients/agent-runtime/src/providers/traits.rs`
    - Add `pub image_metadata: Option<Vec<ImageHistoryMeta>>` with
      `#[serde(default, skip_serializing_if = "Option::is_none")]`
    - Update existing `ChatMessage::user()` constructor to set `image_metadata: None`
    - Add `ChatMessage::user_with_images(content, metadata)` constructor
    - Update any other `ChatMessage` constructors (`system()`, `assistant()`, etc.) to set
      `image_metadata: None`
    - **Acceptance**: All existing code compiles without changes to call sites (field defaults to
      `None`); new constructor available
    - **Spec ref**: REQ-6

- [x] 2.3 Add `max_bytes: u64` parameter to `stream_validate_and_stage()` in
  `clients/agent-runtime/src/channels/media.rs`
    - Change signature to accept `max_bytes: u64` instead of hardcoding `MAX_IMAGE_BYTES`
    - Replace hardcoded `MAX_IMAGE_BYTES` references at lines ~192 and ~205 with `max_bytes`
    - Update all callers to pass `config.multimodal.max_image_bytes.unwrap_or(MAX_IMAGE_BYTES)`
    - **Acceptance**: Function accepts custom limit; callers pass resolved config value
    - **Spec ref**: REQ-4 (`max_image_bytes` override), design ADR-3

### 2B: Core Logic — Config validation and history injection

- [x] 2.4 Add startup validation for `max_image_bytes` bounds in config loading
    - Add `MAX_IMAGE_BYTES_CEILING` constant (50 MiB = `52_428_800`) to
      `clients/agent-runtime/src/channels/media.rs`
    - In config validation (likely `clients/agent-runtime/src/config/schema.rs` or startup path):
      reject `max_image_bytes` values ≤ 0 or > `MAX_IMAGE_BYTES_CEILING`
    - Log effective `max_image_bytes` at startup when multimodal is enabled, noting source (config
      override vs default)
    - **Acceptance**: Config with `max_image_bytes=0` fails startup; config with
      `max_image_bytes=104857600` (100 MiB) fails startup; valid values pass
    - **Spec ref**: REQ-8 (configuration contract), REQ-4

- [x] 2.5 Store image metadata in conversation history in `handle_successful_response()` in
  `clients/agent-runtime/src/channels/mod.rs`
    - At ~line 1210, after successful provider response: build `ImageHistoryMeta::from_staged()` for
      each staged image
    - Store user turn with `ChatMessage::user_with_images()` instead of `ChatMessage::user()` when
      images are present
    - Store assistant turn normally (no image metadata on assistant messages)
    - **Acceptance**: After an image turn, the conversation history entry for the user message
      contains `image_metadata: Some([...])`
    - **Spec ref**: REQ-6 scenario "Follow-up question about a previous image"

- [x] 2.6 Inject image history context into provider prompts on follow-up turns
    - When building `ChatRequest` messages for a turn, scan history for `ChatMessage` entries with
      `image_metadata: Some(...)`
    - For each such entry, prepend `ImageHistoryMeta::to_context_string()` to the message content in
      the provider request
    - Do NOT modify the stored history — inject context only in the outbound `ChatRequest`
    - **Acceptance**: On turn N+1 (text-only), the provider receives history with synthetic context
      block `[Prior image: ...]` for turn N's image
    - **Spec ref**: REQ-6 scenario "Follow-up question about a previous image", design data flow
      diagram

### 2C: Testing

- [x] 2.7 Add unit tests for `ImageHistoryMeta` in `clients/agent-runtime/src/channels/media.rs`
    - RED: Test `from_staged()` maps all `StagedImage` fields correctly
    - RED: Test `to_context_string()` output format with description present
    - RED: Test `to_context_string()` output format with description `None`
    - RED: Test `to_context_string()` with short SHA-256 (edge case: hash < 16 chars)
    - GREEN: Make all tests pass
    - **Spec ref**: REQ-6, design testing strategy row 1-2

- [x] 2.8 Add unit tests for `ChatMessage::user_with_images()` in
  `clients/agent-runtime/src/providers/traits.rs`
    - RED: Test that non-empty metadata vec produces `image_metadata: Some(...)`
    - RED: Test that empty metadata vec produces `image_metadata: None`
    - RED: Test serde roundtrip: serialize → deserialize preserves `image_metadata`
    - RED: Test backward compat: deserializing JSON without `image_metadata` field yields `None`
    - GREEN: Make all tests pass
    - **Spec ref**: REQ-6, design testing strategy row 3

- [x] 2.9 Add unit tests for `stream_validate_and_stage()` custom `max_bytes` in
  `clients/agent-runtime/src/channels/media.rs`
    - RED: Test with body size between default `MAX_IMAGE_BYTES` and custom higher limit → passes
      with custom limit
    - RED: Test with body size above custom limit → rejected with `Oversize`
    - RED: Test with custom limit lower than default → rejects images between custom and default
    - GREEN: Make all tests pass
    - **Spec ref**: REQ-4 scenarios "Config override reduces limit" and "Config override increases
      limit"

- [x] 2.10 Add unit tests for `max_image_bytes` config startup validation
    - RED: Test `max_image_bytes = 0` → startup validation error
    - RED: Test `max_image_bytes = 104_857_600` (100 MiB) → startup validation error (exceeds 50 MiB
      ceiling)
    - RED: Test `max_image_bytes = 5_242_880` (5 MiB) → validation passes
    - RED: Test `max_image_bytes = None` → validation passes (uses default)
    - GREEN: Make all tests pass
    - **Spec ref**: REQ-8 scenarios "Invalid config — max_image_bytes too large" and "
      max_image_bytes is zero"

- [ ] 2.11 Add integration tests for history image context injection
    - RED: Test image turn stores `ImageHistoryMeta` in conversation history
    - RED: Test follow-up text turn includes synthetic `[Prior image: ...]` context in outbound
      messages
    - RED: Test two image turns produce distinct context entries (different SHA-256)
    - RED: Test history does NOT contain base64 image bytes
    - GREEN: Make all tests pass
    - **Spec ref**: REQ-6 all scenarios

- [x] 2.12 Verify existing tests still pass — run `make rust-test` and `make rust-clippy`
    - Confirm no regressions in existing `channel-image-ingestion` behavior
    - Fix any compilation errors or clippy warnings from the new code
    - **Spec ref**: Proposal success criteria "All existing channel-image-ingestion spec scenarios
      continue to pass"

## Phase 3: Follow-up Issues (deferred — not part of this change)

- [ ] 3.1 Provider-specific adapters for Anthropic, Gemini content block formats (#268)
- [ ] 3.2 Model-generated image descriptions — extract description from assistant response and
  attach to `ImageHistoryMeta.description` (enhancement to ADR-2)
- [ ] 3.3 Multi-image per turn support (DALLAY-195) — remove `MAX_IMAGES_PER_TURN = 1` constraint

## Acceptance Criteria Mapping

| Proposal Success Criterion                                  | Task(s)                   |
|-------------------------------------------------------------|---------------------------|
| Runtime-layer spec formalizes canonical pipeline            | 1.3 ✅                     |
| Spec covers conversation-history image representation       | 1.3 (REQ-6) ✅             |
| Spec formalizes error taxonomy with stable identifiers      | 1.3 (REQ-7) ✅             |
| `max_image_bytes` config override is wired and effective    | 2.3, 2.4, 2.9, 2.10       |
| Multi-turn image conversations retain image context         | 2.1, 2.2, 2.5, 2.6, 2.11  |
| Existing channel-image-ingestion scenarios still pass       | 2.12                      |
| Design document exists with ADRs for history representation | 1.4 ✅                     |
| Targeted tests cover new behavior                           | 2.7, 2.8, 2.9, 2.10, 2.11 |
