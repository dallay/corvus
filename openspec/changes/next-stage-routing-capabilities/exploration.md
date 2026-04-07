# Exploration: Next-Stage Routing Capabilities

**Change**: next-stage-routing-capabilities
**Issues**: DALLAY-174 (GitHub #270), DALLAY-175 (GitHub #271)
**Date**: 2026-04-07

---

## DALLAY-175 Coverage Verdict: FULLY COVERED

Issue #271 asks: "Define operator UX and documentation for model routing and query classification."

The recently completed `productize-model-routing` change (DALLAY-173, GitHub #269) delivered
**every acceptance criterion** from #271:

### Evidence

| #271 Question | Delivered Artifact | Coverage |
|---|---|---|
| Where should routing/classification be documented? | EN guide: `clients/web/apps/docs/src/content/docs/guides/model-routing.md` / ES guide: `es/guides/model-routing.md` | ✅ Full |
| What examples should be considered canonical? | 4 examples: fast/reasoning split, code-specialized, vision route, multi-provider with classification | ✅ Full |
| What diagnostics should operators rely on? | 4 `corvus doctor` warnings implemented with tests in `doctor/mod.rs` (orphaned hint, zero rules, zero routes, never-matching rule) | ✅ Full |
| How should fallback behavior be explained? | Hint flow diagram (text-based), troubleshooting table (6 symptoms), formal spec in `openspec/specs/model-routing/spec.md` | ✅ Full |
| What guardrails reduce misconfiguration risk? | Doctor warnings (non-blocking), unknown-hint WARN log, failed-provider route-impact WARN log, operator checklist | ✅ Full |

### Acceptance Criteria Check

- ✅ Operator-facing docs and UX expectations are defined (EN + ES guides)
- ✅ Minimum examples and diagnostics story are defined (4 examples, 4 doctor checks)
- ✅ Follow-up docs and UX work can be split cleanly (spec has clear contracts)

**Recommendation**: Close DALLAY-175 (#271) as completed. No remaining work.

---

## DALLAY-174 Current State and Gap Analysis

Issue #270 asks five explicit questions about extending routing beyond current capabilities.

### Current State

**What exists today:**

1. **Request-time routing** — `[[model_routes]]` maps `hint:<name>` to provider+model. Fully
   productized with formal spec, docs, and doctor checks.

2. **Query classification** — `[query_classification]` with keyword/pattern rules, priority
   ordering, length constraints. Fully productized.

3. **Embedding provider** — `memory.embedding_provider` and `memory.embedding_model` in
   `config/schema.rs`. Single provider, no routing table. Used by `memory/embeddings.rs`
   via `create_embedding_provider()`. Supports: `none`, `openai`, `custom:URL`.

4. **Cerebro MCP** — `memory.cerebro` config with endpoint + auth. External memory service,
   no routing relevance — it's a memory backend, not a routing consumer.

**What does NOT exist:**

- No `embedding_routes` config field or table
- No routing dispatch for embedding workloads
- No managed route update API (admin or agent surface)
- No audit trail for routing config changes
- No openspec work for this change

### Answers to Issue Questions

#### 1. Does Corvus need embedding route support as a first-class product feature?

**Not yet.** Evidence:

- Embedding workloads today are memory-internal (RAG retrieval, document chunking). They
  use a single provider configured in `memory.embedding_provider`.
- There is no user-facing routing decision for embeddings — the memory system picks the
  provider at init time and uses it for all embedding calls.
- Unlike request-time routing (where different prompts benefit from different models),
  embedding consistency matters MORE than variety — switching embedding models mid-corpus
  creates vector space mismatches.
- The competitive gap mentioned in #270 is real but low-urgency: most users run one
  embedding model per workspace.

**When it would matter:** If Corvus supports multiple memory backends with different vector
dimensions, or if operators want to use different embedding models for different document
types (code vs prose), then embedding routes become useful. This is a Phase 2+ concern.

#### 2. What workloads would embedding routes govern?

If implemented, embedding routes would govern:

- **Memory store**: embedding text before writing to vector DB
- **Memory recall**: embedding queries for similarity search
- **Document ingestion**: chunking and embedding workspace files for RAG
- **Cerebro sync**: embedding content for external memory service

The key constraint: store and recall MUST use the same embedding model to maintain vector
space consistency. This means embedding routes are NOT analogous to request-time model
routes — they're more like "embedding profile" configurations.

#### 3. Should routing changes remain config-file-driven or support managed updates?

**Config-file-driven for now.** Rationale:

- Corvus's security model is file-based: config.toml is the source of truth, and
  `corvus doctor` validates it.
- A managed update flow (API/admin surface) introduces new attack surface: who can
  change routes? What approval is needed? How do you roll back?
- The current operator workflow (edit TOML → run doctor → restart) is simple, auditable
  (git diff), and safe (restart to apply).
- Managed updates would make sense when Corvus has a multi-tenant admin dashboard, which
  is not in scope for v1.0.0.

#### 4. What auditability and approval model would managed updates require?

If managed updates are added later:

- **Audit log**: every route change must be logged with timestamp, actor, old value, new
  value
- **Approval**: changes to routing should require the same autonomy level as config changes
  (currently `AutonomyLevel::Supervised` blocks most mutations)
- **Rollback**: must support reverting to previous route config atomically
- **Validation**: must run the same doctor checks before applying changes

This is significant engineering effort with security implications. Not justified for v1.0.0.

#### 5. What should be in scope for the first next-stage routing release vs later?

**Recommended scope for first release (Phase 1 — minimal, decision-only):**

- Close the planning issue (#270) with explicit decisions documented
- No code changes needed

**Recommended scope for Phase 2 (if demand emerges):**

- `[[embedding_routes]]` config with `profile` name and provider/model/dimensions
- Doctor validation for embedding route consistency (same model for store/recall)
- Documentation in the model-routing guide

**Deferred to Phase 3+:**

- Managed route updates via admin API
- Audit trail for route changes
- Multi-tenant route policies

### Approaches

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **A. Close as decided** | Document decisions in #270, close issue. No code. | Zero risk, unblocks roadmap | Doesn't future-proof anything | Low |
| **B. Add embedding route schema only** | Add `[[embedding_routes]]` to config schema + doctor checks, but no runtime behavior change yet | Reserves config surface, validates early | Premature if no demand | Medium |
| **C. Full embedding routing + managed updates** | Implement embedding routes AND admin API for route management | Closes all gaps at once | High risk, large scope, security surface expansion | High |

---

## Recommendation

1. **Close DALLAY-175 (#271)** as completed — all acceptance criteria are met by
   `productize-model-routing`.

2. **Close DALLAY-174 (#270)** with a decision comment documenting:
   - Embedding routes: **not needed for v1.0.0**. Revisit when multi-embedding-model
     workloads emerge.
   - Managed route updates: **not needed for v1.0.0**. Config-file-driven routing is
     sufficient and safer.
   - Both capabilities are explicitly deferred, not rejected. Follow-up issues can be
     created when demand materializes.

3. **No proposal phase needed** — this is a planning/decision issue, not an implementation
   change.

---

## Risks

- **Low risk**: If DALLAY-175 is closed without review of the delivered artifacts, someone
  might reopen it. Mitigate by linking to the specific commits/PRs from
  `productize-model-routing`.
- **Low risk**: Deferring embedding routes could slow adoption for operators with complex
  RAG setups. Mitigate by documenting the `memory.embedding_provider` config clearly in
  existing docs.

## Ready for Proposal

**No** — This exploration concludes that no proposal phase is needed. The recommended action
is to close both issues with documented decisions. If the orchestrator/user disagrees and
wants to pursue embedding routes or managed updates, THEN a proposal would be warranted.
