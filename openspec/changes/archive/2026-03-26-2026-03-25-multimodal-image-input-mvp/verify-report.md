# Verification Report

**Change**: `2026-03-25-multimodal-image-input-mvp`
**Verification date**: 2026-03-26
**Artifact mode**: openspec

---

## Completeness

| Metric           | Value |
|------------------|-------|
| Tasks total      | 24    |
| Tasks complete   | 24    |
| Tasks incomplete | 0     |

All checklist items in `openspec/changes/2026-03-25-multimodal-image-input-mvp/tasks.md` are now
marked complete.

---

## Build, Test, and Coverage Execution

Commands executed during this verification refresh:

- `make test` -> PASS
- `make build` -> PASS
- `make rust-test` -> PASS
- `make rust-build` -> PASS
- `make rust-coverage` -> PASS

Execution evidence:

- `make rust-test`: `2838 passed, 0 failed`
- `make rust-coverage`: `2838 passed, 0 failed`
- Rust line coverage: `78.09%` (`57052 / 73058`) vs threshold `60%` -> PASS

Selected changed-file Rust coverage:

- `clients/agent-runtime/src/channels/media.rs`: `98.38%`
- `clients/agent-runtime/src/channels/mod.rs`: `74.97%`
- `clients/agent-runtime/src/channels/telegram.rs`: `67.10%`
- `clients/agent-runtime/src/channels/whatsapp.rs`: `89.84%`
- `clients/agent-runtime/src/config/schema.rs`: `94.70%`
- `clients/agent-runtime/src/gateway/mod.rs`: `87.07%`
- `clients/agent-runtime/src/observability/traits.rs`: `98.88%`
- `clients/agent-runtime/src/providers/compatible.rs`: `68.63%`
- `clients/agent-runtime/src/providers/gemini.rs`: `86.33%`
- `clients/agent-runtime/src/providers/reliable.rs`: `89.22%`
- `clients/agent-runtime/src/providers/router.rs`: `96.61%`
- `clients/agent-runtime/src/providers/traits.rs`: `86.53%`

Non-blocking warnings observed during `make build`:

- `clients/web/apps/chat/src/App.spec.ts` still reports four Biome `noNonNullAssertion` warnings.
- `clients/web/apps/chat/src/App.vue` still reports one Biome `noUnusedVariables` warning.

---

## Spec Compliance Matrix

| Requirement                                             | Scenario                                                          | Evidence                                                                                                                                                                                                                                                                                                                                                                                                            | Result      |
|---------------------------------------------------------|-------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------|
| Entry Points Alignment                                  | WhatsApp image turn enters the canonical runtime seam             | `clients/agent-runtime/src/gateway/mod.rs` -> `whatsapp_verified_image_turn_enqueues_canonical_message_when_runtime_handle_present`                                                                                                                                                                                                                                                                                 | ✅ COMPLIANT |
| Entry Points Alignment                                  | Rejected WhatsApp transport never reaches the runtime             | `clients/agent-runtime/src/gateway/mod.rs` -> `whatsapp_rejected_transport_never_reaches_runtime`                                                                                                                                                                                                                                                                                                                   | ✅ COMPLIANT |
| MVP Inbound Image Turn Contract                         | Telegram photo with caption is normalized into canonical parts    | `clients/agent-runtime/src/channels/telegram.rs` -> `parse_update_photo_with_caption_produces_text_then_image`                                                                                                                                                                                                                                                                                                      | ✅ COMPLIANT |
| MVP Inbound Image Turn Contract                         | Non-image attachment is not coerced into an image turn            | `clients/agent-runtime/src/channels/telegram.rs` -> `parse_update_document_non_image_mime_no_image_part`; `clients/agent-runtime/src/channels/whatsapp.rs` -> `whatsapp_unsupported_types_still_skipped`                                                                                                                                                                                                            | ✅ COMPLIANT |
| Image Admission Safety and Retention Controls           | Oversized or disallowed media is rejected before provider routing | `clients/agent-runtime/src/channels/mod.rs` -> `mime_rejection_skips_provider_dispatch`; `clients/agent-runtime/src/channels/mod.rs` -> `oversize_rejection_skips_provider_dispatch`; rejection telemetry structure is implemented, but these exact rejection paths are not directly asserted against observer payloads                                                                                             | ⚠️ PARTIAL  |
| Image Admission Safety and Retention Controls           | Admitted image bytes are handled ephemerally                      | `clients/agent-runtime/src/channels/media.rs` -> `staged_image_cleanup_removes_temp_file`; `clients/agent-runtime/src/channels/mod.rs` -> `staged_image_guard_cleanup_called_on_drop`; `clients/agent-runtime/src/channels/mod.rs:602` / `clients/agent-runtime/src/channels/mod.rs:618` persist only text projection, but no end-to-end test proves history/memory/telemetry omit raw bytes after an admitted turn | ⚠️ PARTIAL  |
| MVP Channel Boundaries and Ingress Fallback             | Supported channel image ingress is disabled by rollout control    | `clients/agent-runtime/src/channels/mod.rs` -> `process_rejects_when_multimodal_disabled`                                                                                                                                                                                                                                                                                                                           | ✅ COMPLIANT |
| MVP Channel Boundaries and Ingress Fallback             | Out-of-scope surface remains text-only                            | `clients/agent-runtime/src/gateway/mod.rs` -> `generic_webhook_regression_remains_text_only`; `clients/agent-runtime/src/channels/whatsapp.rs` -> `text_only_whatsapp_regression_remains_text_only`; `clients/agent-runtime/src/channels/telegram.rs` -> `text_only_telegram_regression_remains_text_only`                                                                                                          | ✅ COMPLIANT |
| Image Input Capability Declaration                      | Provider declares accepted image transport forms                  | `clients/agent-runtime/src/providers/compatible.rs` -> `compatible_capabilities_declare_inline_image_support`; `clients/agent-runtime/src/providers/gemini.rs` capability tests                                                                                                                                                                                                                                     | ✅ COMPLIANT |
| Image Input Capability Declaration                      | Undeclared provider remains text-only                             | `clients/agent-runtime/src/providers/traits.rs` -> `providers_without_capability_overrides_remain_text_only`                                                                                                                                                                                                                                                                                                        | ✅ COMPLIANT |
| MVP Provider Scope                                      | In-scope providers are eligible for MVP image routing             | `clients/agent-runtime/src/providers/compatible.rs` -> `compatible_capabilities_declare_inline_image_support`; `clients/agent-runtime/src/channels/mod.rs` -> `run_unified_channel_tool_loop_uses_hint_model_for_image_execution`; `clients/agent-runtime/src/providers/gemini.rs` capability and serialization tests                                                                                               | ✅ COMPLIANT |
| MVP Provider Scope                                      | Out-of-scope provider is excluded from MVP promise                | Static capability defaults and fail-closed routing keep non-MVP providers text-only, but no provider-family-specific runtime test proves Anthropic/OpenRouter exclusion                                                                                                                                                                                                                                             | ⚠️ PARTIAL  |
| Capability-Gated Image Routing and Fail-Closed Fallback | Eligible provider receives the canonical image turn               | `clients/agent-runtime/src/channels/mod.rs` -> `image_turn_prefers_vision_route_selector_for_provider_execution`; `clients/agent-runtime/src/channels/mod.rs` -> `run_unified_channel_tool_loop_forwards_staged_images_to_provider`; `clients/agent-runtime/src/channels/mod.rs` -> `run_unified_channel_tool_loop_uses_hint_model_for_image_execution`                                                             | ✅ COMPLIANT |
| Capability-Gated Image Routing and Fail-Closed Fallback | No capable provider is available                                  | `clients/agent-runtime/src/providers/reliable.rs` -> `image_turn_fails_when_no_image_capable_provider`                                                                                                                                                                                                                                                                                                              | ✅ COMPLIANT |
| Provider Adaptation Data Minimization                   | Runtime-managed inline payload is used for a validated image      | `clients/agent-runtime/src/providers/compatible.rs` -> `multimodal_content_blocks_serialization`; `clients/agent-runtime/src/providers/gemini.rs` multimodal serialization tests                                                                                                                                                                                                                                    | ✅ COMPLIANT |
| Provider Adaptation Data Minimization                   | Remote reference is rejected when it would weaken safety controls | Adapters read staged local files (`std::fs::read(&image.temp_path)`) and format inline payloads only, but there is no dedicated runtime test that proves remote-reference delegation is rejected                                                                                                                                                                                                                    | ⚠️ PARTIAL  |

**Compliance summary**: `12 / 16` scenarios compliant, `4` partial, `0` failing.

---

## Correctness (Static Structural Evidence)

| Area                                                                 | Status        | Notes                                                                                                                                                                                              |
|----------------------------------------------------------------------|---------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Canonical channel parts and text projection                          | ✅ Implemented | `clients/agent-runtime/src/channels/traits.rs` defines `ContentPart::{Text, Image}`, ordered `parts`, and `text_projection()` with caption-as-text semantics.                                      |
| Shared staging, MIME, byte ceilings, rejection taxonomy, and cleanup | ✅ Implemented | `clients/agent-runtime/src/channels/media.rs` defines MIME allowlist, byte limit, rejection reasons, single-image ceiling, and staged temp-file cleanup.                                           |
| Explicit vision-route routing for image turns                        | ✅ Implemented | `clients/agent-runtime/src/channels/mod.rs:166` resolves `vision_model_hint`, and `clients/agent-runtime/src/channels/mod.rs:885` now switches image turns to the resolved `hint:<name>` selector. |
| OpenAI-compatible provider capability declaration                    | ✅ Implemented | `clients/agent-runtime/src/providers/compatible.rs:629` advertises `image_input = true` with `InlineBytes` transport support.                                                                      |
| Config fail-fast validation for multimodal rollout                   | ✅ Implemented | `clients/agent-runtime/src/config/schema.rs:3134` now rejects enabled multimodal configs that omit `vision_model_hint`, omit `allowed_channels`, or include unsupported channels.                  |
| WhatsApp gateway convergence                                         | ✅ Implemented | `clients/agent-runtime/src/gateway/mod.rs` enqueues verified WhatsApp messages into `ChannelRuntimeHandle` instead of using direct provider chat.                                                  |
| Telemetry/redaction contract                                         | ✅ Implemented | `clients/agent-runtime/src/observability/traits.rs` defines metadata-only `ImageIngressEvent` fields and shared redaction helpers without raw media payloads.                                      |

---

## Coherence (Design)

| Design decision                                     | Followed?  | Notes                                                                                                                                           |
|-----------------------------------------------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| Keep `content` as derived compatibility field       | ✅ Yes      | `content` remains the compatibility projection while `parts` is canonical.                                                                      |
| Corvus-managed staging over provider-side fetch     | ✅ Yes      | Runtime stages validated files locally and adapters read `temp_path` instead of delegating remote fetch.                                        |
| Explicit vision routing through `vision_model_hint` | ✅ Yes      | Image turns now select `hint:<vision_model_hint>` while text-only turns keep the default model route.                                           |
| Converge WhatsApp onto canonical runtime seam       | ✅ Yes      | `/whatsapp` remains a transport boundary and hands admitted turns to the shared runtime handle.                                                 |
| Single-turn image retention                         | ⚠️ Partial | Cleanup and text-only persistence are implemented, but archive-grade behavioral proof for non-persistence of raw image bytes is still indirect. |

---

## Issues Found

**CRITICAL**

- None.

**WARNING**

- The previous blocking defects are resolved, but four spec scenarios still have indirect or partial
  runtime proof rather than direct end-to-end evidence.
- `clients/web/apps/chat/src/App.spec.ts` and `clients/web/apps/chat/src/App.vue` still emit five
  pre-existing Biome warnings during `make build`.
- There is still no provider-family-specific runtime regression proving Anthropic/OpenRouter remain
  out of MVP scope for image turns.
- There is still no dedicated end-to-end test proving an admitted image turn persists only text
  projection while memory/history/telemetry omit raw bytes.
- There is still no dedicated runtime test proving remote-reference delegation is rejected when it
  would weaken the staging safety boundary.

**SUGGESTION**

- Add one observer-backed runtime test for MIME/oversize rejection telemetry.
- Add one admitted-image persistence test that asserts memory/history store only text projection.
- Add one provider-boundary regression covering Anthropic or OpenRouter image-turn exclusion
  explicitly.

---

## Verdict

**PASS WITH WARNINGS**

The corrective apply batch resolved the prior blocking defects: image turns now execute through the
configured vision route, OpenAI-compatible providers now declare inline image capability, multimodal
config validation now fails fast, and the new regression tests pass. The remaining gaps are
verification-depth warnings rather than functional blockers, so the change is ready for archive once
the team accepts those residual evidence gaps.
