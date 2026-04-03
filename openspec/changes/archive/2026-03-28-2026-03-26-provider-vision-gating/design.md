# Design: Provider Vision Capability Gating

## Technical Approach

This change formalizes the existing three-layer fail-closed vision gating infrastructure as
architecture decisions, and delivers the one missing provider adapter — Anthropic — so that all
three major cloud providers support multimodal image input. The existing `ProviderCapabilities`
trait mechanism, `RouterProvider` gate, and `ReliableProvider` fallback chain are already correct
and complete; the implementation work is confined to `anthropic.rs` and a single new enum variant
in `NativeContentOut`.

Maps to proposal approach: spec + design + code. References spec scenarios for capability
declaration (REQ-CAP-*), gating behavior (REQ-GATE-*), and format translation (REQ-FMT-*).

## Architecture Overview

`StagedImage` flows from channel ingestion through three gating layers before reaching
provider-specific format translation:

```
Channel Layer          Runtime Core           Provider Layer
─────────────         ─────────────          ──────────────
                      ┌──────────────┐
Telegram ──┐          │  ChatRequest │       ┌─────────────────────┐
WhatsApp ──┼─ StagedImage ──┐       │       │  RouterProvider      │
Discord ──┘          │  messages[]   │──────▶│  Gate 2: capabilities│
                     │  images[]     │       │  check               │
                     │  tools[]      │       └────────┬────────────┘
                     └──────────────┘                 │
                            ▲                         ▼
                            │              ┌──────────────────────┐
                     Gate 1: trait default  │  ReliableProvider    │
                     rejects images if     │  Gate 3: skip text-  │
                     no chat() override    │  only in fallback    │
                                           └────────┬─────────────┘
                                                    │
                              ┌──────────────────────┼──────────────┐
                              ▼                      ▼              ▼
                     ┌─────────────┐      ┌──────────────┐  ┌───────────┐
                     │ OpenAI-compat│      │  Anthropic   │  │  Gemini   │
                     │ image_url    │      │  image block │  │ inline_   │
                     │ data URI     │      │  base64 src  │  │ data part │
                     └─────────────┘      └──────────────┘  └───────────┘
```

Each provider reads `StagedImage.temp_path`, base64-encodes the bytes, and wraps them in the
provider's native API format. The `StagedImage` struct is the boundary type — no provider
receives raw channel data.

## Architecture Decisions

### ADR-1: Trait-Level Capability Declaration

**Choice**: `ProviderCapabilities` with `image_input: bool` and
`image_transport_forms: Vec<ImageTransportForm>` is the canonical gate for vision support.
The `supports_image_input()` method requires both the flag AND at least one transport form.

**Alternatives considered**:

- Feature flag in config only — rejected because capability is intrinsic to the provider/model,
  not a deployment concern
- Separate `VisionProvider` trait — rejected because it would fragment the trait hierarchy and
  require downcasting; the existing `Provider` trait with capability queries is cleaner

**Rationale**: Already implemented (`traits.rs:222-250`) and proven correct across two adapters.
The dual-field gate (`image_input` + non-empty `image_transport_forms`) prevents accidental
enablement. Formalizing this as an ADR ensures future providers follow the same pattern.
The `ModelRouteConfig.allow_image_input` field provides operator-level override on top of
the trait declaration.

### ADR-2: Three-Layer Fail-Closed Gating

**Choice**: Image turns are rejected at three independent layers, each fail-closed by default:

1. **Trait default** (`traits.rs:348-349`): `Provider::chat()` bails if `images` is non-empty
   and the provider hasn't overridden `chat()`.
2. **Router** (`router.rs:153-158`): Checks `capabilities().supports_image_input()` before
   dispatch; rejects with a clear error naming the provider and model.
3. **Reliable wrapper** (`reliable.rs:406-446`): Skips text-only providers in the fallback
   chain for image turns. Fails with `"No image-capable provider available"` if none found.

**Alternatives considered**:

- Single gate at router only — rejected because providers loaded outside the router (tests,
  direct usage) would bypass the check
- Silent fallback to text-only — rejected because stripping images changes the user's intent
  without consent; explicit errors are safer

**Rationale**: Defense in depth. No single layer's failure can route an image turn to a text-only
provider. Each layer produces a distinct, actionable error message. This is already implemented
and requires no code changes — only formalization as a spec.

### ADR-3: Anthropic Image Adapter Design

**Choice**: Extend the existing `AnthropicProvider` with image support by:

1. Adding `NativeContentOut::Image` variant to the file-local enum
2. Overriding `capabilities()` to return `image_input: true`,
   `image_transport_forms: [InlineBytes]`
3. Injecting image content blocks in the existing `chat()` method when `request.images`
   is non-empty

**Variant definition** (added to `NativeContentOut` enum in `anthropic.rs:63-87`):

```rust
#[serde(rename = "image")]
Image {
    source: ImageSource,
},
```

With a supporting struct:

```rust
#[derive(Debug, Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,       // always "base64"
    media_type: String,        // from StagedImage.mime_type
    data: String,              // base64-encoded bytes
}
```

**`chat()` modification** (`anthropic.rs:469-511`): When `request.images` is non-empty:

- Find the last user message index (same pattern as `compatible.rs:509`)
- For that message, interleave text and image `NativeContentOut` blocks:
    1. `NativeContentOut::Text` with the user's text content
    2. For each `StagedImage`: read `temp_path`, base64-encode, emit `NativeContentOut::Image`
- Non-user messages pass through unchanged

**`capabilities()` override** (new method on `impl Provider for AnthropicProvider`):

```rust
fn capabilities(&self) -> ProviderCapabilities {
    ProviderCapabilities {
        native_tool_calling: true,
        image_input: true,
        image_transport_forms: vec![ImageTransportForm::InlineBytes],
    }
}
```

**`apply_cache_to_last_message` update** (`anthropic.rs:201-213`): Add an arm for the new
`NativeContentOut::Image` variant — image blocks do not carry `cache_control`, so the match
arm is a no-op (same as `ToolUse`).

**Alternatives considered**:

- Generic multimodal adapter trait — rejected; each provider's API format is different enough
  that a shared abstraction adds complexity without reducing code
- Separate `chat_multimodal()` method (OpenAI-compatible pattern) — rejected for Anthropic
  because the existing `chat()` already builds `NativeContentOut` content blocks via
  `convert_messages()`, so extending the same method is more natural

**Rationale**: Follows the established pattern. OpenAI-compatible uses a separate
`chat_multimodal()` because its non-image path uses simple string content. Anthropic already
uses `Vec<NativeContentOut>` content blocks in every request, so image blocks slot in
naturally. Keeps the change minimal and local to `anthropic.rs`.

### ADR-4: Provider Format Translation

**Choice**: `StagedImage` is the universal boundary type. Each provider translates independently
to its native API format. No shared "format adapter" abstraction.

| Provider          | API Format                                                                  | Translation                           |
|-------------------|-----------------------------------------------------------------------------|---------------------------------------|
| OpenAI-compatible | `{type:"image_url", image_url:{url:"data:{mime};base64,{data}"}}`           | Data URI in content blocks array      |
| Anthropic         | `{type:"image", source:{type:"base64", media_type:"{mime}", data:"{b64}"}}` | Source object in content blocks array |
| Gemini            | `{inline_data:{mime_type:"{mime}", data:"{b64}"}}`                          | InlineData struct in parts array      |

**Alternatives considered**:

- Shared base64-encoding utility — already exists implicitly (each provider calls
  `std::fs::read` + `base64_encode`); a shared helper could be extracted later but is not
  required for three call sites
- Provider-agnostic content block abstraction — rejected because the three formats differ
  in structure (data URI vs source object vs inline_data), nesting, and field names

**Rationale**: Three providers, three formats, zero shared structure beyond "read file,
base64-encode, wrap in JSON." A forced abstraction would be more complex than the duplication
it eliminates. Each adapter owns its format contract completely.

### ADR-5: Ollama Deferral

**Choice**: Defer Ollama vision support to Wave 2.

**Alternatives considered**:

- Implement now with a hardcoded model allowlist — rejected because the list would be
  immediately stale as new vision models are released
- Query Ollama `/api/show` for model capabilities — rejected because the response does not
  reliably indicate vision support; the `template` field hints at multimodal but is not
  standardized

**Rationale**: Ollama's `/api/chat` endpoint supports an `images` field (base64 array), so the
wire format is simple. The blocker is **capability detection**: there is no standard way to
query whether a pulled model supports vision. The correct solution is a user-configured
allowlist (`multimodal.ollama_vision_models: [llava, bakllava, ...]`) that the operator
maintains. This requires a config schema addition and documentation — scope that belongs in
a dedicated follow-up issue, not this change.

## Data Flow

### Sequence: Image Turn Through the Stack

```
User (via channel)
  │
  │  image + text message
  ▼
Channel (Telegram/WhatsApp/Discord)
  │  validate → normalize → stage as StagedImage
  │  StagedImage { sha256, mime_type, byte_len, temp_path, transport_form, channel_origin }
  ▼
Agent Loop
  │  builds ChatRequest { messages, images: &[StagedImage], tools }
  ▼
RouterProvider::chat()
  │  resolve(model) → (provider_idx, resolved_model)
  │  CHECK: provider.capabilities().supports_image_input()
  │  ✗ → bail!("Image turn cannot be routed to provider '{name}'")
  │  ✓ ↓
  ▼
ReliableProvider::chat()  [if wrapped]
  │  for each provider in fallback chain:
  │    SKIP if !capabilities().supports_image_input()
  │    TRY provider.chat(request, model, temperature)
  │    on success → return Ok
  │    on failure → accumulate, try next image-capable provider
  │  no capable provider left → bail!("No image-capable provider available")
  ▼
AnthropicProvider::chat()
  │  credential check
  │  convert_messages(request.messages) → (system_prompt, native_messages)
  │  find last_user_idx in native_messages
  │  for each StagedImage in request.images:
  │    bytes = std::fs::read(image.temp_path)
  │    b64 = base64_encode(&bytes)
  │    push NativeContentOut::Image { source: { type: "base64", media_type, data: b64 } }
  │  inject image blocks into last user message's content array
  │  build NativeChatRequest { model, max_tokens, system, messages, temperature, tools }
  │  POST /v1/messages with anthropic-version: 2023-06-01
  │  parse NativeChatResponse → ProviderChatResponse { text, tool_calls }
  ▼
Agent Loop
  │  process response, update conversation history
  ▼
Channel
  │  send reply to user
```

## File Changes

| File                                                       | Action | Description                                                                                                                                                                                                                         |
|------------------------------------------------------------|--------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/providers/anthropic.rs`         | Modify | Add `NativeContentOut::Image` variant + `ImageSource` struct; override `capabilities()` to declare image support; extend `chat()` to inject image content blocks from `StagedImage`; update `apply_cache_to_last_message` match arm |
| `clients/agent-runtime/src/providers/anthropic.rs` (tests) | Modify | Add unit tests for: capability declaration, image content block construction, interleaved text+image blocks, cache control with image variant                                                                                       |

## Interfaces / Contracts

### New: `NativeContentOut::Image` variant (anthropic.rs)

```rust
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum NativeContentOut {
    // ... existing variants ...

    #[serde(rename = "image")]
    Image {
        source: ImageSource,
    },
}

#[derive(Debug, Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,       // "base64"
    media_type: String,        // e.g. "image/jpeg", "image/png", "image/gif", "image/webp"
    data: String,              // base64-encoded image bytes
}
```

Serializes to:

```json
{
  "type": "image",
  "source": {
    "type": "base64",
    "media_type": "image/jpeg",
    "data": "/9j/4AAQ..."
  }
}
```

### Modified: `capabilities()` override

```rust
#[async_trait]
impl Provider for AnthropicProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            image_input: true,
            image_transport_forms: vec![ImageTransportForm::InlineBytes],
        }
    }
    // ...
}
```

## Testing Strategy

| Layer       | What to Test                                                          | Approach                                                                                                                                         |
|-------------|-----------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| Unit        | `NativeContentOut::Image` serializes to correct Anthropic JSON format | Construct variant, `serde_json::to_value`, assert structure matches `{type:"image", source:{type:"base64", media_type, data}}`                   |
| Unit        | `capabilities()` returns `image_input: true` + `[InlineBytes]`        | Call `AnthropicProvider::new(...).capabilities()`, assert fields                                                                                 |
| Unit        | `supports_image_input()` returns `true` for Anthropic capabilities    | Call `.capabilities().supports_image_input()`, assert true                                                                                       |
| Unit        | Image blocks interleaved with text in user message                    | Build a mock `ChatRequest` with text + images, verify `convert_messages` output contains both `Text` and `Image` blocks in the last user message |
| Unit        | Images attached only to last user message                             | Multi-turn request with two user messages; verify only the last one gets image blocks                                                            |
| Unit        | `apply_cache_to_last_message` handles `Image` variant without panic   | Message ending with `Image` block — verify no crash, cache not applied to image blocks                                                           |
| Unit        | Empty images slice produces no image blocks                           | `ChatRequest` with `images: &[]` — verify output matches existing text-only behavior exactly                                                     |
| Integration | Router rejects image turn to non-image provider                       | `RouterProvider` with an Anthropic (image-capable) and Ollama (text-only); route image turn to Ollama route, expect error                        |
| Integration | Reliable skips text-only, selects Anthropic for image turn            | `ReliableProvider` with mixed providers; send image turn, verify Anthropic is selected                                                           |

Tests use `cargo test --manifest-path clients/agent-runtime/Cargo.toml` and follow the
existing pattern in `anthropic.rs` tests (lines 532+).

## Migration / Rollout

No migration required. All changes are additive and gated by `capabilities()`:

- **Enable**: Deploy new code. Anthropic automatically declares `image_input: true`. Router and
  reliable wrapper immediately recognize it as image-capable.
- **Disable/rollback**: Revert commit. Anthropic reverts to default `image_input: false`. All
  three gating layers automatically exclude it from image routing. No config changes needed.
- **Operator control**: Existing `ModelRouteConfig.allow_image_input` and
  `MultimodalConfig.vision_model_hint` provide per-route opt-in without code changes.

## Open Questions

- [x] ~Which providers are in scope for v1?~ — Resolved: OpenAI-compatible + Gemini + Anthropic
- [x] ~Is the gating behavior correct?~ — Resolved: fail-closed at three layers, no changes needed
- [ ] Image history replay: `ImageHistoryMeta` exists but Anthropic adapter does not yet
  reconstruct image blocks from history metadata for multi-turn image conversations. Tracked
  separately — does not block this change.
