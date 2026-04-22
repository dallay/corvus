# Proposal: Add Chat Completions Streaming Transport for OpenAI-Compatible SSE

## Intent

Rook issue #591 still needs one narrow transport follow-up after the archived inbound auth,
transport middleware baseline, global surface rate limits, and chat-completions idempotency slices.
The remaining gap for the gateway surface is streaming delivery for `POST /v1/chat/completions`
when the request body sets `stream: true`.

This slice exists to define OpenAI-compatible server-sent events (SSE) transport for streaming chat
completions without widening scope into `/api/*`, `GET /v1/models`, fairness/rate limiting, TLS,
RBAC, or unrelated provider-auth work. Here is the thing: the value is transport compatibility for
one gateway route and one request mode, not a broad rewrite of Rook’s HTTP behavior.

## Scope

### In Scope

- Define a dedicated #591 follow-up slice for `POST /v1/chat/completions` only when `stream: true`.
- Define the gateway response contract as OpenAI-compatible SSE for streaming chat completions.
- Define the minimum SSE event framing, media type, and stream termination expectations required for
  OpenAI client compatibility.
- Define how streaming transport composes with the archived inbound-auth boundary and transport
  middleware baseline without changing their responsibilities.
- Define route-local expectations for upstream streaming proxying or adaptation needed to preserve
  OpenAI-compatible SSE on the Rook gateway response.
- Define failure behavior for streaming setup or mid-stream transport failure at the gateway
  boundary.
- Identify the minimal server, gateway, transport, and spec areas that follow-on spec/design work
  must cover.

### Out of Scope

- Any streaming behavior for `/api/*` admin routes.
- Any streaming behavior for `GET /v1/models`.
- Any streaming behavior for routes outside `POST /v1/chat/completions`.
- Non-streaming `POST /v1/chat/completions` behavior, except where explicitly needed to preserve
  route separation from this slice.
- Idempotency, replay protection, duplicate suppression, or request-key semantics for streaming.
- New fairness controls, quotas, rate limiting, abuse prevention, or scheduling behavior.
- TLS, mTLS, certificate handling, or network-edge proxy policy.
- RBAC, route scopes, tenancy policy, or authorization-model expansion.
- Outbound provider authentication/header changes beyond what is already required for gateway
  proxying.
- Streaming semantics for dashboard, docs, marketing, or any other non-Rook HTTP surface.

## Approach

Treat this slice as a gateway transport contract for one route and one mode: when
`POST /v1/chat/completions` receives `stream: true`, Rook should return an OpenAI-compatible SSE
response instead of the standard buffered JSON completion payload. The follow-on spec/design should
define the wire contract at the Rook boundary first, then identify the smallest internal transport
changes needed to proxy or adapt upstream provider streaming into that contract.

The core contract is simple:

- Only `POST /v1/chat/completions` with `stream: true` participates in this slice.
- The response MUST be emitted as OpenAI-compatible SSE.
- `/api/*` remains unaffected.
- `GET /v1/models` remains unaffected.
- This slice MUST NOT introduce new fairness/rate-limit responsibilities.
- This slice MUST remain transport-focused and SHOULD avoid changing archived auth, middleware,
  rate-limit, or idempotency responsibilities.

This proposal intentionally avoids promising broader streaming support across every surface. The
design work should stay disciplined: preserve current route boundaries, keep rollback easy, and make
the OpenAI-shaped SSE contract explicit enough that clients can consume streamed chat-completion
deltas without relying on route-specific custom behavior.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/server/mod.rs` | Modified | Compose streaming handling only for `POST /v1/chat/completions` when `stream: true`, without widening behavior to `/api/*` or `GET /v1/models`. |
| `clients/rook/src/gateway/` | Modified | Add or refine chat-completions streaming response handling so the gateway emits OpenAI-compatible SSE. |
| `clients/rook/src/transport/` or gateway streaming helper modules | New or Modified | Add route-local streaming transport helpers, SSE framing utilities, and stream-failure shaping if needed. |
| `clients/rook/src/gateway/vendor.rs` | Modified only if required for existing proxying | Preserve current outbound proxy responsibilities while enabling upstream streaming passthrough/adaptation; no broader auth-scope expansion. |
| `openspec/specs/gateway/spec.md` | Modified (follow-on) | Add delta requirements for `POST /v1/chat/completions` streaming transport, OpenAI-compatible SSE framing, termination, and excluded surfaces. |
| `openspec/changes/rook-591-chat-completions-streaming-transport/` | New | Proposal and follow-on spec/design/tasks artifacts for this slice. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Streaming behavior leaks onto `/api/*` or `GET /v1/models` because transport code is shared | High | State explicit route boundaries in spec/design and require route-level composition for `POST /v1/chat/completions` only. |
| SSE emitted by Rook is not OpenAI-compatible enough for existing clients | High | Define explicit framing, media type, chunk/event shape, and termination requirements in follow-on spec/design. |
| Mid-stream upstream failure produces ambiguous client-visible behavior | Medium | Define gateway failure handling for setup-time and in-flight failures, including what can and cannot be represented once streaming has started. |
| Provider-specific streaming formats bleed through directly and break the gateway contract | Medium | Normalize or adapt upstream streaming to the OpenAI SSE shape at the gateway boundary rather than exposing provider-native framing. |
| This slice accidentally reopens idempotency, rate-limit, or auth-scope debates | Medium | Keep proposal bounded to streaming transport only and explicitly defer adjacent concerns already handled by separate #591 slices. |
| Long-lived streams interact awkwardly with existing transport middleware/tracing behavior | Medium | Reuse archived middleware baseline as-is where possible and let follow-on design document any stream-safe completion/tracing expectations without changing ownership. |

## Rollback Plan

If this slice causes regressions or interoperability problems, revert the route-local streaming
transport changes for `POST /v1/chat/completions` with `stream: true`, remove any SSE helper wiring
or gateway adaptation introduced for this feature, and restore the prior non-streaming behavior for
that request mode. Because this slice is intentionally narrow, rollback should not require changes to
`/api/*`, `GET /v1/models`, archived inbound auth behavior, transport middleware baseline,
global rate limits, chat-completions idempotency, TLS policy, RBAC, or broader provider-auth logic.

## Dependencies

- The archived `rook-591-inbound-auth-boundary` slice remains a completed prerequisite and must stay
  separate from this streaming transport slice.
- The archived `rook-591-transport-middleware-baseline` slice remains the transport composition
  baseline and should continue to own cross-cutting middleware concerns instead of pushing them into
  streaming-specific business logic.
- The archived `rook-591-global-surface-rate-limits` slice remains separate; this change must not
  absorb fairness, quota, or admission-control responsibilities.
- The archived `rook-591-chat-completions-idempotency` slice remains separate; streaming transport in
  this slice must not redefine replay behavior or request-key semantics.
- Existing gateway routing in `clients/rook/src/server/mod.rs` must continue to keep `/api/*`,
  `GET /v1/models`, and `POST /v1/chat/completions` clearly separated by responsibility.
- The gateway spec in `openspec/specs/gateway/spec.md` will need a follow-on delta for streaming
  `POST /v1/chat/completions` semantics and OpenAI-compatible SSE behavior.
- Follow-on design work must confirm whether upstream providers can be proxied directly as SSE or
  require gateway-side adaptation to preserve the OpenAI wire contract.

## Success Criteria

- [ ] The change remains narrowly scoped to `POST /v1/chat/completions` when `stream: true`.
- [ ] The proposal explicitly requires OpenAI-compatible SSE for the response transport.
- [ ] The proposal explicitly excludes `/api/*` and `GET /v1/models` from this slice.
- [ ] The proposal stays transport-focused and does not absorb fairness/rate limiting, TLS, RBAC, or
      unrelated outbound provider-auth work.
- [ ] The proposal makes setup-time and mid-stream failure behavior important follow-on design/spec
      concerns.
- [ ] The rollback plan is route-local and does not require undoing archived #591 slices.
- [ ] The change is ready for `sdd-spec` and `sdd-design` to define concrete streaming requirements
      and technical decisions.
