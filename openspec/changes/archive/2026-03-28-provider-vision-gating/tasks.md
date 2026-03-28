# Tasks: Provider Vision Capability Gating

## Phase 1: Documentation (Complete)

- [x] 1.1 Explore existing provider vision infrastructure and identify gaps (#268)
- [x] 1.2 Write proposal.md with intent, scope, approach, and rollback plan
- [x] 1.3 Write delta spec against `agent-runtime-providers` (REQ-1 through REQ-6)
- [x] 1.4 Write design.md with ADRs, data flow, interfaces, and testing strategy
- [x] 1.5 Write tasks.md (this file)

## Phase 2: Code Implementation

### Foundation

- [x] 2.1 Add `NativeContentOut::Image` variant and `ImageSource` struct to `anthropic.rs`
  - **File**: `clients/agent-runtime/src/providers/anthropic.rs` (near line 63-87, existing enum)
  - **Action**: Add `Image { source: ImageSource }` variant with `#[serde(rename = "image")]`;
    add `ImageSource` struct with `source_type` (`"base64"`), `media_type`, `data` fields and
    appropriate `#[serde(rename)]` annotations
  - **Verify**: `cargo check --manifest-path clients/agent-runtime/Cargo.toml` passes
    (exhaustive match errors expected until 2.4)
  - **Maps to**: design ADR-3 variant definition, spec REQ-4 Anthropic format

- [x] 2.2 Override `capabilities()` in `AnthropicProvider` to declare image support
  - **File**: `clients/agent-runtime/src/providers/anthropic.rs` (Provider impl block)
  - **Action**: Add `capabilities()` method returning `ProviderCapabilities { native_tool_calling: true, image_input: true, image_transport_forms: vec![ImageTransportForm::InlineBytes] }`
  - **Verify**: `cargo check` passes
  - **Maps to**: design ADR-1, spec REQ-1 (capability matrix), REQ-2 (declaration contract)

### Core Implementation

- [x] 2.3 Implement image block injection in `AnthropicProvider::chat()`
  - **File**: `clients/agent-runtime/src/providers/anthropic.rs` (chat method, near line 469-511)
  - **Action**: When `request.images` is non-empty: find last user message index, read each
    `StagedImage.temp_path`, base64-encode bytes, build `NativeContentOut::Image` blocks,
    interleave with `NativeContentOut::Text` in the last user message's content array.
    Follow the pattern established in `compatible.rs:494-609`
  - **Verify**: `cargo check` passes
  - **Maps to**: design ADR-3 chat() modification, spec REQ-4 (Anthropic format scenario)

- [x] 2.4 Update `apply_cache_to_last_message` match arm for `Image` variant
  - **File**: `clients/agent-runtime/src/providers/anthropic.rs` (near line 201-213)
  - **Action**: Add match arm for `NativeContentOut::Image { .. }` — no-op (image blocks do not
    carry `cache_control`), same pattern as `ToolUse` variant
  - **Verify**: `cargo check --manifest-path clients/agent-runtime/Cargo.toml` passes with no
    exhaustive match warnings
  - **Maps to**: design ADR-3, testing strategy (cache control with image variant)

### Testing

- [x] 2.5 Add unit tests for Anthropic image adapter
  - **File**: `clients/agent-runtime/src/providers/anthropic.rs` (tests module, line 532+)
  - **Tests**:
    - `capabilities()` returns `image_input: true` + `[InlineBytes]` (spec REQ-2 scenario 1)
    - `supports_image_input()` returns `true` (spec REQ-2 scenario 1)
    - `NativeContentOut::Image` serializes to `{type:"image", source:{type:"base64", media_type, data}}` (spec REQ-4 Anthropic scenario)
    - Empty images slice produces no image blocks — output matches text-only (spec REQ-4, design testing strategy)
    - `apply_cache_to_last_message` handles `Image` variant without panic (design testing strategy)
  - **Verify**: `cargo test --manifest-path clients/agent-runtime/Cargo.toml -- anthropic` passes
  - **Maps to**: spec REQ-2 scenarios 1-3, REQ-4 Anthropic scenario, design testing table

- [x] 2.6 Add integration test for image turn through Anthropic adapter
  - **File**: `clients/agent-runtime/src/providers/anthropic.rs` (tests module)
  - **Tests**:
    - Build mock `ChatRequest` with text + `StagedImage` (temp file with known bytes), invoke
      image block construction, verify JSON structure matches
      `{type:"image", source:{type:"base64", media_type:"image/jpeg", data:"..."}}` exactly
    - Verify image blocks are attached only to the last user message (spec REQ-4 "last user message" requirement)
  - **Verify**: `cargo test --manifest-path clients/agent-runtime/Cargo.toml -- anthropic` passes
  - **Maps to**: spec REQ-4 Anthropic scenario, design testing table (integration row)

### Validation

- [x] 2.7 Run full validation: `cargo fmt`, `clippy`, `test`
  - **Commands**:
    - `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --check`
    - `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`
    - `cargo test --manifest-path clients/agent-runtime/Cargo.toml`
  - **Verify**: All three pass with zero warnings, zero failures
  - **Maps to**: proposal success criteria (cargo test + clippy pass with no warnings)

## Phase 3: Follow-Up Issues (Deferred — Not Part of This Change)

- [ ] 3.1 Ollama vision support — needs vision model detection strategy; no standard capability
  query exists. Requires user-configured allowlist (`multimodal.ollama_vision_models`). Track as
  separate issue. (design ADR-5)
- [ ] 3.2 URL-based transport optimization — Anthropic Files API, Gemini GCS URIs. Future
  optimization for large images. (proposal out-of-scope)
- [ ] 3.3 Token budget management for base64 overhead (~33% size increase). No token budget system
  yet. (proposal out-of-scope)
- [ ] 3.4 Image history replay for Anthropic multi-turn — `ImageHistoryMeta` exists but Anthropic
  adapter does not yet reconstruct image blocks from history metadata. (design open question)

## Acceptance Criteria Mapping

| Proposal Success Criterion | Task(s) |
|---|---|
| Anthropic declares `image_input: true` + `[InlineBytes]` | 2.2, 2.5 |
| Anthropic `chat()` builds correct image content blocks from `StagedImage` | 2.3, 2.6 |
| All existing provider tests pass (no regressions) | 2.7 |
| New unit tests cover image block construction + capability declaration | 2.5 |
| Router/reliable correctly route image turns to Anthropic | Existing behavior — verified by spec REQ-3 scenarios (no code change) |
| Provider capability matrix documented as formal spec | 1.3 (complete) |
| `cargo test` and `cargo clippy` pass with no warnings | 2.7 |
