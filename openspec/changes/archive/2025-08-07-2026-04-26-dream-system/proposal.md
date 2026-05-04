# Proposal: Dream System: Long-Term Memory Consolidation

## Intent

Issue #526 introduces a runtime-level "dream system" that consolidates completed session history into durable long-term memory so Corvus can preserve important knowledge without carrying full conversational transcripts forward indefinitely.

Exploration confirms that Dream already has seed implementation in `clients/agent-runtime/src/memory/dream.rs`, is exported from `clients/agent-runtime/src/memory/mod.rs`, and is already hooked from gateway session-completion flows via `record_session_completion` and `run_dream_if_triggered`. The proposal therefore focuses on promoting Dream from partial plumbing to a defined runtime memory capability with clear consolidation semantics, durable backend behavior, and explicit integration boundaries for sessions and gateway.

## Summary

This change will define and complete Dream as a memory/runtime-oriented feature that:

- detects eligible completed sessions,
- consolidates high-value information from session history into durable long-term memory,
- persists Dream artifacts across supported memory backends,
- updates session/runtime integration so consolidation is deterministic and safe,
- keeps gateway changes limited to integration points that trigger or expose runtime behavior.

## Motivation

Corvus currently has the primitives needed for long-term memory consolidation, but Dream is not yet specified as a first-class capability. Without a source-of-truth contract:

- session completion may record closure without guaranteed long-term consolidation behavior,
- different memory backends may diverge in how Dream artifacts persist or hydrate,
- runtime and gateway responsibilities can blur,
- future changes risk treating orchestration or HTTP entrypoints as the primary domain instead of memory/runtime.

A dedicated proposal is needed to align implementation around the runtime memory model and to make issue #526 deliverable in phased, testable slices.

## Scope

### In Scope
- Define Dream as a runtime memory-consolidation capability centered in `clients/agent-runtime/src/memory/`.
- Specify how completed sessions become Dream candidates and when consolidation is triggered.
- Specify the durable outputs Dream writes into long-term memory and/or Dream-specific persisted state.
- Align SQLite, markdown, and snapshot hydration/export behavior so Dream artifacts survive restart, export, and reload flows.
- Clarify session lifecycle integration points required to make consolidation deterministic after session completion.
- Clarify gateway integration points that trigger runtime Dream flows without making gateway the behavioral source-of-truth.
- Establish phased delivery intent for issue #526, including MVP consolidation first and optional follow-on refinements later.

### Out of Scope / Non-Goals
- Re-architecting multi-agent orchestration around Dream.
- Making orchestration reuse the primary spec domain for this change.
- Large new admin/operator UX surfaces for inspecting Dream artifacts unless required for existing runtime or gateway verification.
- General memory ranking, retrieval, or recommendation redesign outside Dream consolidation needs.
- Broad changes to unrelated gateway HTTP contracts except where needed to preserve integration correctness.
- Implementing speculative autonomous background schedulers beyond the approved Dream trigger model.

## Approach

The change should be specified primarily under a memory/runtime-oriented domain, with gateway and sessions treated as affected integration domains rather than the source of truth.

At a high level, Dream should:

1. observe session completion or another explicit runtime-eligible trigger,
2. identify whether the finished session satisfies Dream eligibility criteria,
3. read the relevant session transcript/memory context,
4. synthesize durable long-term memory artifacts from that session,
5. persist those artifacts through supported backends and snapshot/export paths,
6. record enough Dream state to avoid duplicate or ambiguous consolidation for the same session.

The proposal intentionally biases toward additive behavior that reuses the existing seed implementation and hooks already present in the runtime and gateway.

## Affected Spec Domains

| Domain | Role in Change | Why |
|---|---|---|
| `memory` / new Dream-focused runtime memory domain | Primary | Dream is fundamentally a long-term memory consolidation feature and should be specified where memory behavior lives. |
| `sessions` | Modified | Session completion, eligibility, and deduplication rules need explicit integration with Dream. |
| `gateway` | Modified (integration only) | Gateway already invokes Dream-related hooks and may need contract updates for runtime-triggered completion behavior. |
| `memory-visibility` | Possible follow-up | Only if existing admin visibility needs to expose Dream outputs for verification or operations. |
| `multi-agent-orchestration` | No primary change | Reuse is optional and explicitly not the main domain for this slice. |

## Affected Modules / Packages

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/memory/dream.rs` | Modified | Promote seed Dream logic into the canonical runtime consolidation path. |
| `clients/agent-runtime/src/memory/mod.rs` | Modified | Maintain Dream exports and memory module integration. |
| `clients/agent-runtime/src/memory/` backend implementations | Modified | Ensure SQLite, markdown, and snapshot-related backends persist and reload Dream artifacts consistently. |
| `clients/agent-runtime/src/session/` or equivalent session lifecycle modules | Modified | Ensure completion state, idempotency, and Dream eligibility are coordinated with session closure. |
| `clients/agent-runtime` snapshot hydration/export paths | Modified | Preserve Dream outputs across hydration, export, and restore flows. |
| Gateway integration points calling `record_session_completion` and `run_dream_if_triggered` | Modified | Keep trigger wiring aligned with runtime-defined Dream behavior. |

## Phased Intent

### Phase 1: Dream Contract and Runtime MVP
- Define Dream eligibility, trigger timing, and persistence expectations.
- Ensure one completed session can be consolidated deterministically into long-term memory.
- Make duplicate-trigger behavior explicit and safe.

### Phase 2: Backend and Snapshot Parity
- Guarantee Dream artifacts survive SQLite persistence, markdown storage where supported, and snapshot hydration/export.
- Ensure restored runtimes do not lose Dream state or replay ambiguously.

### Phase 3: Integration Hardening
- Align session lifecycle and gateway integration behavior with the Dream contract.
- Add any minimal observability or verification hooks required to validate consolidation end-to-end.

### Deferred / Future Considerations
- Rich operator inspection workflows for Dream outputs.
- More advanced consolidation heuristics, scoring, or background scheduling.
- Optional orchestration-driven reuse once the memory/runtime contract is stable.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Dream behavior duplicates or re-consolidates the same session multiple times | Medium | Require explicit idempotency markers or persisted Dream state tied to session completion. |
| Backends diverge in Dream persistence semantics | Medium | Specify minimum persistence and hydration/export guarantees across supported backends. |
| Gateway becomes the accidental source-of-truth for Dream behavior | Medium | Keep behavioral requirements in the memory/runtime domain and limit gateway spec changes to trigger integration. |
| Session closure semantics conflict with consolidation timing | Medium | Define exact ordering between session completion recording and Dream triggering in the sessions integration spec. |
| Consolidated memories are too lossy or too verbose | Medium | Scope MVP to stable, high-value summaries/facts and defer advanced heuristics to later phases. |

## Rollback Plan

If Dream consolidation causes incorrect memory persistence, duplicate outputs, or destabilizes session completion:

1. disable Dream triggering at the runtime integration points,
2. preserve ordinary session completion behavior without consolidation,
3. retain additive schema/backend changes where safe, but stop producing new Dream artifacts,
4. remove or ignore Dream-specific reads from restore/hydration paths until corrected.

Because the intended change is additive and runtime-centered, rollback should be able to preserve existing memory/session behavior while isolating Dream-specific execution.

## Dependencies

- Existing session completion hooks already calling `record_session_completion` and `run_dream_if_triggered`.
- Existing Dream seed implementation in `clients/agent-runtime/src/memory/dream.rs`.
- Existing memory backend support for SQLite, markdown, and snapshot hydration/export.
- Follow-on spec work to create delta specs in the primary Dream/memory domain plus sessions and gateway integration domains.

## Success Criteria

- [ ] OpenSpec delta specs define Dream primarily as a memory/runtime capability rather than a gateway feature.
- [ ] The change specifies deterministic trigger timing and idempotent consolidation for completed sessions.
- [ ] Supported backends have explicit persistence and hydration/export expectations for Dream artifacts.
- [ ] Session lifecycle integration clearly defines how completion and Dream triggering interact.
- [ ] Gateway impact is limited to integration requirements and does not become the behavioral source-of-truth.
- [ ] The phased plan provides a clear MVP path for issue #526 with advanced heuristics and orchestration reuse deferred.
