# Exploration: multimodal-image-input-mvp

## Scope framing

This exploration uses the prompt goal and the referenced planning bundle (`#259`, `#266`, `#267`,
`#268`) as the product source of truth: Corvus needs a native MVP for inbound image understanding
from selected messaging channels across selected provider backends. No extra GitHub issue text was
fetched.

### Current State

Corvus already has the right top-level extension points for this feature, but every relevant runtime
contract is still text-first.

- Channel ingress is text-only. `ChannelMessage` carries only `id`, `sender`, `reply_target`,
  `content`, `channel`, and `timestamp`, with no attachment or media part field
  (`clients/agent-runtime/src/channels/traits.rs:5`).
- Provider input is text-only. `ChatMessage` is just `{ role, content: String }`, and
  `ConversationMessage` stores text/tool-call history but no multimodal parts
  (`clients/agent-runtime/src/providers/traits.rs:9`,
  `clients/agent-runtime/src/providers/traits.rs:90`).
- Capability modeling is too narrow for vision routing. `ProviderCapabilities` only declares
  `native_tool_calling`, so the runtime cannot express or negotiate image-input support
  (`clients/agent-runtime/src/providers/traits.rs:199`).
- The unified channel loop already exists and is the best current seam. Channel messages flow
  through memory enrichment, canonical blocking checks, provider chat/tool execution, and response
  delivery in `process_channel_message(...)` / `build_history(...)`
  (`clients/agent-runtime/src/channels/mod.rs:472`,
  `clients/agent-runtime/src/channels/mod.rs:612`).
- Telegram has outbound media delivery, but inbound parsing is still text-only. Incoming updates
  only
  read `message.text`, while outbound responses can already emit `[IMAGE:...]` / `[DOCUMENT:...]`
  markers and send actual media (`clients/agent-runtime/src/channels/telegram.rs:733`,
  `clients/agent-runtime/src/channels/telegram.rs:1673`).
- WhatsApp inbound is explicitly text-only today. `extract_whatsapp_text_content(...)` returns
  `None` for non-text messages, and webhook handling calls `provider.simple_chat(...)` directly
  instead of the canonical channel/dispatcher path (
  `clients/agent-runtime/src/channels/whatsapp.rs:28`,
  `clients/agent-runtime/src/channels/whatsapp.rs:40`,
  `clients/agent-runtime/src/gateway/mod.rs:1980`).
- There is already precedent for attachment filtering in other channels. Signal can ignore
  attachment-only messages, and email can surface text attachments as text, which shows channel
  normalization already varies by media type (`clients/agent-runtime/src/channels/signal.rs:219`,
  `clients/agent-runtime/src/channels/email_channel.rs:197`).
- There is explicit repo evidence that multimodal support is anticipated but not wired in yet.
  `ImageInfoTool` says providers are currently text-only and exposes base64 data for future
  multimodal provider support (`clients/agent-runtime/src/tools/image_info.rs:12`).
- Generic gateway transport is also text-only. `/webhook` accepts `{ "message": string }`, and the
  dispatcher request type also carries only `message: String`
  (`clients/agent-runtime/src/gateway/mod.rs:1488`,
  `clients/agent-runtime/src/gateway/webhook_dispatch.rs:23`).
- Provider implementations mostly serialize text strings even when their upstream APIs could support
  richer content blocks. Examples: OpenAI/OpenRouter chat wrappers use `content: String`, OpenAI-
  compatible chat uses `messages[].content: String`, and Gemini currently builds `parts` with only
  text fields (`clients/agent-runtime/src/providers/openai.rs:15`,
  `clients/agent-runtime/src/providers/openrouter.rs:15`,
  `clients/agent-runtime/src/providers/compatible.rs:162`,
  `clients/agent-runtime/src/providers/gemini.rs:83`).

### Affected Areas

- `clients/agent-runtime/src/channels/traits.rs` — canonical inbound channel message contract would
  need image/media representation.
- `clients/agent-runtime/src/channels/mod.rs` — the unified channel loop is where normalized inbound
  image parts should be added to history and routed to providers.
- `clients/agent-runtime/src/channels/telegram.rs` — inbound Telegram parsing currently drops photos
  and documents.
- `clients/agent-runtime/src/channels/whatsapp.rs` — inbound WhatsApp parsing currently drops image
  messages and only produces text content.
- `clients/agent-runtime/src/gateway/mod.rs` — WhatsApp currently bypasses the canonical dispatcher
  path; generic `/webhook` is also string-only.
- `clients/agent-runtime/src/providers/traits.rs` — needs a richer provider capability model and a
  multimodal request contract.
- `clients/agent-runtime/src/providers/compatible.rs`,
  `clients/agent-runtime/src/providers/gemini.rs`,
  `clients/agent-runtime/src/providers/anthropic.rs`, and likely
  `clients/agent-runtime/src/providers/openrouter.rs`
  — candidate adapters for first-wave image-capable backends.
- `openspec/specs/agent-runtime-providers/spec.md` — already owns provider capability/config shape,
  so it is the natural spec home for image-input capability modeling.
- `openspec/specs/client-surfaces/spec.md` — confirms web/mobile surface transport boundaries, which
  argues this MVP should stay in runtime/channel/provider scope first, not end-user surface scope
  (`openspec/specs/client-surfaces/spec.md:49`).

### Candidate Channels

1. **Telegram** — best MVP channel candidate.
    - Pros: already a first-class channel in the unified runtime, already has media send semantics,
      and has a mature allowlist/onboarding path.
    - Cons: inbound photo/document retrieval and security validation still need to be designed.
    - Effort: Medium.

2. **WhatsApp** — product-important candidate, but architecturally riskier.
    - Pros: already has webhook verification, signature validation, and allowlist controls.
    - Cons: current path is webhook-special-cased and bypasses the canonical dispatcher/tool path,
      so image MVP work here can sprawl into runtime-parity work.
    - Effort: Medium-High.

3. **Signal / Matrix / Email** — useful reference channels, not primary MVP targets.
    - Pros: existing attachment/media handling patterns can inform normalization rules.
    - Cons: adding them to MVP would widen scope without clear product pressure from the prompt.
    - Effort: High if included in the initial slice.

### Candidate Providers

1. **OpenAI-compatible provider family** (`compatible.rs`) — best primary backend seam.
    - Pros: many Corvus providers already funnel through this adapter; one multimodal contract here
      could unlock multiple compatible backends and custom endpoints.
    - Cons: current request models are still string-only, and image support varies significantly by
      vendor/model even behind an OpenAI-compatible facade.
    - Effort: Medium.

2. **Gemini** — strong first-class multimodal candidate.
    - Pros: API structure already uses `contents[].parts[]`, which is a natural fit for text + image
      inputs.
    - Cons: current implementation only serializes text parts, so image part handling and tests
      would
      still be new work.
    - Effort: Medium.

3. **Anthropic** — viable second-wave candidate.
    - Pros: content-block architecture is already richer than plain text and should extend cleanly.
    - Cons: current implementation only models text/tool blocks; adding image blocks is more work
      than the current capability model suggests.
    - Effort: Medium.

4. **OpenRouter-specific adapter** — useful later, but weaker as the design center.
    - Pros: broad model reach.
    - Cons: route-level vision capability is inconsistent, so it is a poor source of truth for the
      core Corvus multimodal contract.
    - Effort: Medium.

### Approaches

1. **Prompt-proxy MVP** — download inbound images, run local metadata/OCR/caption tooling, and
   inject
   text summaries into the current text-only provider flow.
    - Pros: smallest change to runtime contracts; works with text-only providers.
    - Cons: not truly Corvus-native multimodal understanding; quality depends on sidecar tooling;
      loses raw-image reasoning and provider-native vision strengths.
    - Effort: Medium.

2. **End-to-end multimodal contract** — add canonical media parts to channel ingress, conversation
   history, provider requests, and capability routing.
    - Pros: correct long-term architecture; cleanly supports future audio/document/video expansion.
    - Cons: too broad for an MVP unless tightly constrained to image-only + selected
      channels/providers.
    - Effort: High.

3. **Hybrid MVP contract** — add a minimal canonical image-input model end to end, but scope it to
   inbound image parts only, selected channels only, and selected provider backends only.
    - Pros: preserves native multimodal semantics while keeping change size controlled; creates the
      right seam for proposal/spec/design.
    - Cons: still requires contract changes across channels, provider traits, and at least one
      runtime
      path divergence (`/whatsapp`).
    - Effort: Medium-High.

### Recommendation

Use **Approach 3**.

Concrete recommendation for proposal/spec/design:

- Define a minimal canonical **inbound content-part contract** with at least `text` and `image`
  parts, plus image source metadata (channel source ID, MIME type if known, byte/url handle,
  and optional caption). Do not design generic file/document/video support yet.
- Extend `ProviderCapabilities` beyond `native_tool_calling` to include explicit image-input support
  and the accepted transport forms (for example: remote URL, inline base64, or fetched bytes).
- Keep MVP channel scope to **Telegram + WhatsApp** only. Treat Signal/Matrix/Email as follow-up or
  reference-only.
- Keep MVP provider scope to **OpenAI-compatible + Gemini** first. Make Anthropic a likely follow-up
  if the contract is designed cleanly.
- Explicitly decide whether WhatsApp image ingress is implemented by (a) moving WhatsApp onto the
  canonical channel runtime seam first, or (b) adding a temporary WhatsApp-specific multimodal
  adapter with a stated follow-up to converge on the shared seam. Option (a) is architecturally
  cleaner; option (b) is faster but carries debt.
- Keep generic `/webhook`, web chat, dashboard, and mobile bridge **out of scope** for MVP. Their
  current contracts are string-only, and the prompt is about selected messaging channels rather than
  end-user surfaces.
- Include security rules in proposal/spec/design for media fetch size limits, MIME allowlists,
  signed webhook validation, redaction, storage lifetime, and whether raw media is persisted to
  memory at all.

### Major Trade-offs

- **Speed vs architecture**: channel-specific image hacks ship faster, but a tiny canonical image
  part model avoids repeating the same work for every channel/provider.
- **Provider breadth vs certainty**: broad provider promises increase product appeal, but backend
  image support varies enough that MVP should target a small set of adapters with clear capability
  flags.
- **Remote URL pass-through vs Corvus-managed download**: remote URLs reduce local storage cost, but
  Corvus-managed download gives stronger security controls, deterministic retries, and provider
  independence.
- **WhatsApp inclusion vs scope control**: product pressure likely wants WhatsApp early, but its
  current webhook-special-case path is the biggest architecture risk in this MVP.

### Risks

- The current string-only message contracts (`ChannelMessage`, `ChatMessage`, `/webhook`) make this
  a
  cross-cutting contract change, not a leaf feature.
- WhatsApp currently bypasses the canonical channel/dispatcher flow, so parity and multimodal work
  can
  collapse into the same change if scope is not enforced.
- Provider capability routing is under-specified today; without explicit image capability flags,
  multimodal requests could be misrouted to text-only backends.
- Media handling introduces new security exposure: untrusted URLs, oversized payloads, MIME
  spoofing,
  malware-in-document edges, and accidental long-term storage of sensitive user images.
- Conversation history and memory semantics are not defined yet for raw images, captions, and
  derived
  summaries, which can create privacy and cost surprises if not specified upfront.

### Ready for Proposal

Yes — the repo has enough evidence to draft a proposal.

The proposal should lock these decisions first:

1. Canonical inbound image-part contract shape.
2. MVP channels: Telegram and WhatsApp only.
3. MVP providers: OpenAI-compatible and Gemini only.
4. Whether WhatsApp first converges onto the canonical channel runtime seam or ships behind a
   temporary gateway-specific adapter.
5. Security/storage policy for inbound media download, retention, and memory persistence.
