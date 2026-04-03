## Exploration: Provider Vision Capability Gating and Multimodal Adapter Strategy

**Issue**: #268
**Date**: 2026-03-26
**Depends on**: #266 (channel-image-ingestion), #267 (runtime-image-normalization-pipeline)

### Current State

The multimodal image pipeline is already partially implemented across three layers:

1. **Channel ingestion** (spec: `channel-image-ingestion`): Telegram, WhatsApp, and Discord channels
   can parse, fetch, validate, and stage images as `StagedImage` structs. Gated by
   `multimodal.enabled` + `multimodal.allowed_channels` config.

2. **Runtime pipeline** (spec: `runtime-image-pipeline`): The `ChatRequest` struct carries a
   `&[StagedImage]` slice. The default `Provider::chat()` implementation is **fail-closed** — it
   returns an error if `images` is non-empty and the provider hasn't overridden `chat()`
   (`traits.rs:348-349`).

3. **Provider adapters**: Two providers currently implement multimodal `chat()` overrides:
    - `OpenAiCompatibleProvider` — `chat_multimodal()` method (`compatible.rs:494-609`)
    - `GeminiProvider` — inline `chat()` override (`gemini.rs:342-464`)

#### Provider Capability Declaration (Current)

`ProviderCapabilities` (`traits.rs:222-243`) has three fields:

- `native_tool_calling: bool`
- `image_input: bool`
- `image_transport_forms: Vec<ImageTransportForm>`

The `supports_image_input()` method (`traits.rs:248-250`) requires **both** `image_input == true`
AND at least one transport form.

**Current declarations per provider:**

| Provider                   | `image_input`     | `image_transport_forms` | `native_tool_calling` | Overrides `chat()`?          |
|----------------------------|-------------------|-------------------------|-----------------------|------------------------------|
| `OpenAiCompatibleProvider` | `true`            | `[InlineBytes]`         | `true`                | Yes — `chat_multimodal()`    |
| `GeminiProvider`           | `true`            | `[InlineBytes]`         | `false`               | Yes — inline in `chat()`     |
| `AnthropicProvider`        | `false` (default) | `[]` (default)          | `true`                | Yes — but no image handling  |
| `OllamaProvider`           | `false` (default) | `[]` (default)          | `false`               | No — uses default trait impl |

#### Provider Routing & Gating (Current)

- **`RouterProvider`** (`router.rs:153-158`): Fail-closed — rejects image turns if the resolved
  provider's `capabilities().supports_image_input()` is false.
- **`ReliableProvider`** (`reliable.rs:397-450`): For image turns, skips text-only providers in the
  fallback chain and only tries image-capable ones. If none are available, fails with
  `"No image-capable provider available for image turn"`.
- **`ModelRouteConfig`** (`schema.rs:1634-1648`): Has `allow_image_input: bool` field for explicit
  opt-in per route. The `vision_model_hint` in `MultimodalConfig` references a route hint.

#### Image Encoding Per Provider

| Provider          | API Format                                                              | Encoding                                                                                |
|-------------------|-------------------------------------------------------------------------|-----------------------------------------------------------------------------------------|
| OpenAI-compatible | `content: [{type:"image_url", image_url:{url:"data:mime;base64,..."}}]` | base64 data URL in content blocks array                                                 |
| Gemini            | `parts: [{inline_data: {mime_type:"...", data:"..."}}]`                 | base64 in `InlineData` struct                                                           |
| Anthropic         | Not implemented                                                         | N/A — would need `{type:"image", source:{type:"base64", media_type:"...", data:"..."}}` |
| Ollama            | Not implemented                                                         | N/A — would need `images: ["base64..."]` field in message                               |

### Answers to Questions

#### Q1: Which providers are in scope for multimodal image input in v1?

**Answer**: Based on current code, **three providers** already declare or can declare image support:

| Provider          | v1 Status               | Rationale                                                                                                                          |
|-------------------|-------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| OpenAI-compatible | **Done**                | Fully implemented (`chat_multimodal`). Covers OpenAI, Groq, Mistral, xAI, Venice, etc.                                             |
| Gemini            | **Done**                | Fully implemented (inline `chat()` override with `InlineData` parts).                                                              |
| Anthropic         | **Not implemented**     | Has native tool calling but no `chat()` image override. Anthropic's Messages API supports `image` content blocks — adapter needed. |
| Ollama            | **Out of scope for v1** | Ollama's `/api/chat` supports `images` field (base64 array), but local model vision support varies wildly. Defer to Wave 2.        |

**Recommendation**: v1 scope = OpenAI-compatible (done) + Gemini (done) + Anthropic (new adapter
needed).

#### Q2: How should provider vision capability be declared?

**Answer**: The mechanism **already exists** and is well-designed:

- Each provider overrides `fn capabilities() -> ProviderCapabilities` to declare `image_input: true`
  and list supported `image_transport_forms`.
- `supports_image_input()` is the canonical gate — requires both the flag and at least one transport
  form.
- The `ModelRouteConfig.allow_image_input` field provides operator-level opt-in per route.
- The `MultimodalConfig.vision_model_hint` points to the route used for image turns.

**No changes needed** to the declaration mechanism. What's missing is:

1. Anthropic provider declaring `image_input: true` + `[InlineBytes]`.
2. Documentation of the provider capability matrix in a spec.

#### Q3: What should happen when a user sends image input to a non-vision provider?

**Answer**: The behavior is **already implemented** and is fail-closed at multiple levels:

1. **Trait default** (`traits.rs:348`): `Provider::chat()` bails with `"Provider does not support
   image input"` if `images` is non-empty and the provider hasn't overridden `chat()`.
2. **Router** (`router.rs:153-158`): Checks `capabilities().supports_image_input()` before dispatch;
   rejects with a clear error if false.
3. **Reliable wrapper** (`reliable.rs:406-446`): Skips text-only providers in fallback chain for
   image turns. Fails with `"No image-capable provider available"` if none found.

**No changes needed** to gating behavior. It is already fail-closed at every layer.

#### Q4: Which provider-specific adapters are required for the initial rollout?

**Answer**: Only **one new adapter** is needed:

| Adapter                                              | Status      | Work Required                                                                                                      |
|------------------------------------------------------|-------------|--------------------------------------------------------------------------------------------------------------------|
| OpenAI-compatible (`chat_multimodal`)                | Done        | None                                                                                                               |
| Gemini (`chat()` with `InlineData`)                  | Done        | None                                                                                                               |
| **Anthropic** (`chat()` with `image` content blocks) | **Missing** | New adapter: build `{type:"image", source:{type:"base64", media_type, data}}` content blocks in `NativeContentOut` |

The Anthropic adapter needs:

- A new `NativeContentOut::Image` variant with `source_type`, `media_type`, `data` fields.
- `capabilities()` override returning `image_input: true, image_transport_forms: [InlineBytes]`.
- Image block injection in the `chat()` method (similar pattern to `compatible.rs:528-538`).
- Reading bytes from `StagedImage.temp_path` and base64-encoding them.

#### Q5: What compatibility or fallback behavior is expected for providers without multimodal support?

**Answer**: The current behavior is correct and should be preserved:

- **No silent fallback**: Image turns are never silently downgraded to text-only. The
  `ReliableProvider`
  explicitly skips text-only providers for image turns (`reliable.rs:413-417`).
- **No image stripping**: Images are never stripped from requests to make them fit a text-only
  provider. The user gets a clear error instead.
- **Fallback chain respects capabilities**: If the primary vision provider fails, only other
  image-capable providers in the fallback chain are tried.
- **Operator control**: The `vision_model_hint` config explicitly selects which route handles image
  turns. If not configured, image turns fail at the gate level (`MissingVisionRoute`).

### Affected Areas

- `clients/agent-runtime/src/providers/anthropic.rs` — needs image adapter implementation
- `clients/agent-runtime/src/providers/traits.rs` — no changes needed (already complete)
- `clients/agent-runtime/src/providers/router.rs` — no changes needed (gating works)
- `clients/agent-runtime/src/providers/reliable.rs` — no changes needed (fallback works)
- `clients/agent-runtime/src/providers/compatible.rs` — no changes needed (adapter complete)
- `clients/agent-runtime/src/providers/gemini.rs` — no changes needed (adapter complete)
- `clients/agent-runtime/src/config/schema.rs` — no changes needed (config fields exist)

### Provider Capability Matrix (v1)

| Provider          | Supports Images | Transport Form | API Format                               | Status                 |
|-------------------|-----------------|----------------|------------------------------------------|------------------------|
| OpenAI-compatible | Yes             | InlineBytes    | `image_url` content block with data URL  | **Complete**           |
| Gemini            | Yes             | InlineBytes    | `inline_data` part with mime + base64    | **Complete**           |
| Anthropic         | Yes (planned)   | InlineBytes    | `image` content block with base64 source | **Needs adapter**      |
| Ollama            | No (v1)         | N/A            | `images` array (base64)                  | **Deferred to Wave 2** |

### Approaches

1. **Anthropic-only adapter addition** — Add image support to `AnthropicProvider` only
    - Pros: Minimal scope, covers the three major cloud providers for v1, low risk
    - Cons: Ollama users with vision models (llava, etc.) can't use images yet
    - Effort: Low

2. **Anthropic + Ollama adapters** — Add both in one change
    - Pros: Broader coverage, Ollama vision models (llava, bakllava) are popular
    - Cons: Ollama vision model detection is hard (no standardized capability query), increases
      scope
    - Effort: Medium

3. **Spec-only (no code)** — Document the capability matrix and adapter contracts without
   implementing
    - Pros: Enables parallel implementation issues, zero risk
    - Cons: No new functionality delivered
    - Effort: Low

### Recommendation

**Approach 1: Anthropic-only adapter addition.** The infrastructure is already solid — gating,
routing, fallback, and two adapters are complete. The only gap for a credible v1 multimodal story is
Anthropic support. Ollama can follow in a focused Wave 2 change with proper vision model detection.

### Risks

- **Anthropic API format changes**: Anthropic's image content block format is stable but should be
  validated against current API version (`2023-06-01`). The `anthropic-version` header is already
  sent.
- **History replay with images**: The `ImageHistoryMeta` struct exists in `ChatMessage` but
  Anthropic's adapter doesn't yet reconstruct image blocks from history metadata. This needs design
  for multi-turn image conversations.
- **Ollama vision model detection gap**: There's no standard way to query whether an Ollama model
  supports vision. This is the main blocker for Wave 2 and may require a user-configured allowlist.
- **Transport form extensibility**: MVP uses only `InlineBytes`. Some providers (Anthropic with
  Files API, Gemini with GCS URIs) support URL-based transport that avoids base64 overhead. This is
  a future optimization, not a v1 blocker.
- **Max token overhead**: Base64 encoding adds ~33% overhead. Large images on models with small
  context windows could crowd out conversation history. No per-provider token budget exists yet.

### Ready for Proposal

Yes — the codebase exploration is complete. The capability infrastructure is mature, gating is
fail-closed at every layer, and the only implementation gap is the Anthropic adapter. The proposal
should scope:

1. Anthropic image adapter implementation
2. Provider capability matrix documentation (as a spec)
3. Explicit deferral of Ollama to a follow-up issue
