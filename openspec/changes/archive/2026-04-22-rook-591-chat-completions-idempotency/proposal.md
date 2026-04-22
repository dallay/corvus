# Proposal: Add Chat Completions Idempotency for Meaningful Replay Protection

## Intent

Rook issue #591 still needs a narrow replay-protection slice after the archived inbound auth,
transport middleware baseline, and global surface rate-limit changes. The next follow-up should add
idempotency only where duplicate submission is materially harmful in the current gateway surface:
`POST /v1/chat/completions`.

The goal is meaningful replay protection for non-streaming chat-completion creation requests without
expanding scope into admin APIs, model-list reads, fairness/rate limiting, TLS, RBAC, or outbound
provider authentication. This keeps the slice tight: protect the one mutation-style public route
where a retried request can otherwise create duplicate upstream work and ambiguous client outcomes.

## Scope

### In Scope

- Define a dedicated #591 follow-up slice for idempotency on `POST /v1/chat/completions` only.
- Define the request/response contract for meaningful replay protection on repeated
  `POST /v1/chat/completions` submissions.
- Define how the server recognizes a duplicate chat-completions create attempt for this slice.
- Define replay behavior for equivalent repeated chat-completions requests so clients receive a
  deterministic result instead of duplicate execution when the original request has already been
  accepted.
- Define mismatch behavior when the same idempotency identity is reused with materially different
  `POST /v1/chat/completions` inputs.
- Define the minimum retention/window and lifecycle expectations needed so replay protection is
  meaningful for practical client retries.
- Define how idempotency composes with the archived inbound-auth boundary and transport middleware
  baseline without changing their responsibilities.
- Identify the minimal server, gateway, persistence/cache, config, and spec areas that follow-on
  spec/design work must cover.

### Out of Scope

- Any idempotency behavior for `/api/*` admin routes.
- Any idempotency behavior for `GET /v1/models`.
- Any idempotency behavior for routes outside `POST /v1/chat/completions`.
- Per-client fairness, rate limiting, quotas, abuse controls, or request scheduling.
- Streaming chat completions or partial-response replay behavior.
- TLS, mTLS, certificate handling, or network-edge deployment policy.
- RBAC, route scopes, tenancy policy, or authorization-model expansion.
- Outbound provider authentication/header changes in `clients/rook/src/gateway/vendor.rs`.
- Broader deduplication across unrelated endpoints, cross-route request correlation, or general
  exactly-once guarantees.

## Approach

Treat this slice as a transport/gateway boundary contract applied only to non-streaming
`POST /v1/chat/completions`. The design should define an explicit idempotency identity that the
request must present, a normalization or equivalence rule for determining whether a replay is the
same logical request, and a bounded retention model so the server can return a deterministic
duplicate outcome instead of re-executing the upstream call.

The core contract is simple:

- Only `POST /v1/chat/completions` participates in this slice.
- `GET /v1/models` and all `/api/*` routes remain unaffected.
- Replays that match the original idempotent chat-completions request within the supported replay
  window MUST resolve deterministically without causing duplicate meaningful execution.
- Reuse of the same idempotency identity with materially different chat-completions inputs MUST be
  rejected explicitly rather than silently treated as equivalent.
- The slice MUST remain non-streaming; acceptance MUST NOT depend on stream replay semantics.
- The slice SHOULD be implemented with reversible server-side storage/cache and startup/config
  wiring rather than hard-coded, non-expiring behavior.

This proposal intentionally avoids promising universal exactly-once processing. Here is the thing:
for a networked gateway, the practical value is bounded replay protection for meaningful client
retries on the one mutation-like endpoint that exists today, not an overly broad guarantee that
would drag in distributed transaction semantics.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/server/mod.rs` | Modified | Compose chat-completions-only idempotency handling in front of the existing `POST /v1/chat/completions` route without affecting `/api/*` or `GET /v1/models`. |
| `clients/rook/src/gateway/` | Modified | Enforce idempotency requirements and replay semantics for `POST /v1/chat/completions` while preserving current OpenAI-shaped gateway behavior. |
| `clients/rook/src/config/` | Modified | Define startup/config inputs for replay-window, storage/cache behavior, and validation rules if this slice exposes operator-tunable idempotency settings. |
| `clients/rook/src/` transport/middleware or replay-protection module(s) | New or Modified | Add helpers for idempotency identity lookup, request equivalence checks, stored-result lifecycle, and duplicate/mismatch response shaping. |
| `clients/rook/src/gateway/vendor.rs` | Unchanged by scope | Outbound provider authentication/header behavior remains explicitly out of scope for this slice. |
| `openspec/specs/gateway/spec.md` | Modified (follow-on) | Add delta requirements for `POST /v1/chat/completions` idempotency, replay acceptance, mismatch rejection, and excluded surfaces. |
| `openspec/changes/rook-591-chat-completions-idempotency/` | New | Proposal and follow-on spec/design/tasks artifacts for this slice. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Idempotency scope creeps onto `/api/*` or `GET /v1/models` because the transport layer is shared | High | State explicit route boundaries in spec/design and require route-level composition that only wraps `POST /v1/chat/completions`. |
| The system accepts the same idempotency identity for materially different request bodies | High | Define canonical equivalence rules and explicit conflict behavior for mismatched replays. |
| Retention is too short to protect realistic client retries or too long for safe operational cost | Medium | Specify a bounded replay window with operator-visible configuration semantics and clear cleanup expectations. |
| Replay storage/cache failures create ambiguous behavior under retry | Medium | Design fail-closed vs fail-open behavior explicitly in the follow-on spec/design and keep rollback easy by isolating the feature behind targeted composition/config. |
| Clients mistake replay protection for general exactly-once guarantees across providers | Medium | Document that this slice provides meaningful bounded replay protection for one route, not universal exactly-once semantics. |
| Non-streaming assumptions are accidentally violated by future stream support | Medium | Keep `stream: true` and all streaming semantics explicitly out of scope for this slice and require separate future work. |

## Rollback Plan

If this slice causes regressions or operational confusion, revert the chat-completions-only
idempotency composition for `POST /v1/chat/completions`, remove any startup/config entries and
replay storage/cache wiring introduced for this feature, and restore the current behavior in which
chat-completion retries are processed without replay protection. Because this slice is intentionally
limited to one route, rollback should not require changes to `/api/*`, `GET /v1/models`, inbound
auth semantics, rate-limit behavior, TLS policy, RBAC, or outbound provider auth.

## Dependencies

- The archived `rook-591-inbound-auth-boundary` slice remains a completed prerequisite and must stay
  separate from this idempotency slice.
- The archived `rook-591-transport-middleware-baseline` slice remains the transport composition
  baseline and should host this route-specific replay protection cleanly rather than widening
  middleware scope.
- The archived `rook-591-global-surface-rate-limits` slice remains separate; this change must not
  absorb fairness or quota responsibilities.
- Existing server composition in `clients/rook/src/server/mod.rs` must continue to host `/api/*`,
  `GET /v1/models`, and `POST /v1/chat/completions` without broadening replay protection to the
  excluded surfaces.
- The gateway spec in `openspec/specs/gateway/spec.md` will need a follow-on delta for chat
  completions idempotency, duplicate replay behavior, mismatch handling, and route exclusions.
- Follow-on design work must confirm where replay state lives, how request equivalence is computed,
  and what retention/config model is operationally safe in `clients/rook`.

## Success Criteria

- [ ] The change remains narrowly scoped to idempotency for `POST /v1/chat/completions` only.
- [ ] The proposal explicitly excludes `/api/*` admin routes and `GET /v1/models` from idempotency
      coverage.
- [ ] The proposal defines meaningful replay protection as deterministic duplicate handling for
      equivalent chat-completions retries within a bounded replay window.
- [ ] The proposal explicitly requires mismatch handling when the same idempotency identity is reused
      with materially different chat-completions inputs.
- [ ] The proposal keeps fairness/rate limiting, streaming, TLS, RBAC, and outbound provider auth
      work explicitly deferred.
- [ ] The rollback plan is route-local and does not require undoing archived auth, transport, or
      rate-limit slices.
- [ ] The change is ready for `sdd-spec` and `sdd-design` to define concrete requirements and
      technical decisions.
