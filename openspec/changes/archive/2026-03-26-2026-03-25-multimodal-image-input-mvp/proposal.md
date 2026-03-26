# Proposal: Multimodal Image Input MVP

## Intent

Deliver a native MVP for inbound image understanding in Corvus so selected messaging channels can
send user images into selected vision-capable providers without collapsing the experience into a
text-only proxy. The MVP should introduce the smallest canonical image-input seam that preserves
future extensibility while keeping scope tightly limited to the channels, providers, and safety
controls required by the planning bundle.

## Scope

### In Scope
- Add a minimal canonical inbound content model for `text` and `image` parts only.
- Support inbound image understanding for `Telegram` and `WhatsApp` only.
- Support provider routing for `OpenAI-compatible` backends and `Gemini` only.
- Define provider capability signaling for image-input support and accepted image transport forms.
- Route inbound image turns through the canonical runtime/channel/provider seam, including
  WhatsApp.
- Define MVP safety rules for media fetch, validation, retention, redaction, and observability.

### Out of Scope
- Generic attachment support beyond images, including document, audio, and video reasoning.
- Any multimodal changes for generic gateway `/webhook`, web chat, dashboard, or mobile bridge.
- Adding `Signal`, `Matrix`, `Email`, or other channels to the MVP.
- Adding `Anthropic`, `OpenRouter-specific`, or other providers to the MVP promise.
- Product work for outbound image generation/rendering changes beyond existing channel behavior.
- Long-term memory/search semantics for persisting raw user images.

## Key Decisions

### MVP Shape

Use a hybrid MVP contract: one canonical inbound image-part seam end-to-end, but constrained to
image input only, selected messaging channels only, and selected provider adapters only.

### Channel and Provider Boundaries

- Channels in MVP: `Telegram`, `WhatsApp`
- Providers in MVP: `OpenAI-compatible`, `Gemini`
- All other channels and providers remain explicitly deferred.

### WhatsApp Runtime Recommendation

WhatsApp SHOULD converge onto the canonical channel runtime seam in this MVP rather than using a
temporary gateway-specific multimodal adapter.

Rationale:
- WhatsApp is already the major architecture exception; extending the exception for images would
  compound debt instead of containing it.
- The canonical seam is where Corvus already applies memory enrichment, blocking, provider routing,
  tool execution, and response semantics; image turns should inherit those same controls.
- Safety posture is stronger when webhook verification remains a transport boundary and all
  multimodal reasoning happens inside one canonical runtime path.
- This recommendation keeps the convergence narrowly scoped to the WhatsApp image-turn path, not a
  broader redesign of generic gateway `/webhook` or unrelated channel behavior.

## Safety Posture

- Treat all inbound media as untrusted.
- Require channel-origin validation before any fetch or provider handoff.
- Enforce strict size ceilings, MIME allowlists, and bounded download behavior.
- Prefer Corvus-managed media retrieval and normalization over raw provider-side fetching when
  safety or determinism would otherwise degrade.
- Minimize retention: raw image bytes SHOULD be ephemeral, MUST be redacted from logs, and MUST NOT
  be added to long-term memory by default in the MVP.
- Emit rollout telemetry that distinguishes admitted, rejected, filtered, and provider-routed image
  turns without exposing sensitive media contents.

## Rollout Philosophy

Ship the smallest production-credible slice with explicit capability gating. Roll out behind
configuration so operators can enable the MVP only for selected channels and image-capable
providers, observe behavior, and fall back by disabling image ingress or provider capability usage
without changing the rest of the runtime contract.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/channels/traits.rs` | Modified | Canonical inbound channel contract gains minimal image-part semantics |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Unified channel loop admits and routes inbound image parts |
| `clients/agent-runtime/src/channels/telegram.rs` | Modified | Telegram parses inbound image messages into canonical parts |
| `clients/agent-runtime/src/channels/whatsapp.rs` | Modified | WhatsApp ingress converges on canonical channel runtime path for MVP image turns |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | WhatsApp transport boundary preserves verification while handing off to canonical runtime seam |
| `clients/agent-runtime/src/providers/traits.rs` | Modified | Provider capabilities and request contract declare image-input support |
| `clients/agent-runtime/src/providers/compatible.rs` | Modified | OpenAI-compatible adapter accepts canonical image input |
| `clients/agent-runtime/src/providers/gemini.rs` | Modified | Gemini adapter accepts canonical image input |
| `openspec/specs/agent-runtime-providers/spec.md` | Modified | Provider capability and multimodal request requirements |
| `openspec/specs/agent-loop/spec.md` | Modified | WhatsApp convergence and canonical-runtime expectations for this change |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Cross-cutting text-to-multimodal contract change expands beyond MVP | Medium | Keep the canonical model image-only and limit adoption to two channels and two provider seams |
| WhatsApp convergence increases delivery scope | Medium | Constrain the change to preserving transport checks while moving only runtime execution onto the shared seam |
| Provider capability mismatch causes image turns to hit text-only backends | High | Require explicit image-input capability flags and gated routing |
| Media handling introduces security/privacy exposure | High | Use strict validation, bounded fetches, redaction, and default non-persistence of raw images |

## Rollback Plan

Disable multimodal image ingress and provider image capability routing via rollout controls, causing
channels to return to text-only handling while preserving existing webhook verification and channel
transport behavior. If needed, Telegram and WhatsApp image admission can be turned off
independently without undoing the broader runtime contract work.

## Dependencies

- Planning bundle intent and acceptance for selected channels/providers (`#259`, `#266`, `#267`,
  `#268`)
- Existing agent-loop and provider specs as the canonical contract baseline

## Success Criteria

- [ ] Proposal locks the MVP to inbound `image` + `text` parts only.
- [ ] Proposal explicitly limits channel scope to `Telegram` and `WhatsApp`.
- [ ] Proposal explicitly limits provider scope to `OpenAI-compatible` and `Gemini`.
- [ ] Proposal resolves the WhatsApp architecture decision in favor of canonical runtime convergence.
- [ ] Proposal defines non-goals, safety posture, and rollout philosophy clearly enough for spec and
      design work to proceed without reopening MVP scope.
