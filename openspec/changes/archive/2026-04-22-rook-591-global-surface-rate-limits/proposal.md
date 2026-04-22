# Proposal: Add Global Surface Rate Limits for Rook Transport Entry Points

## Intent

Rook issue #591 still needs a narrow traffic-protection slice after the archived inbound auth
boundary and transport middleware baseline work. The next follow-up should define global transport
rate limiting by surface so Rook can shed excess request volume deterministically before admin or
gateway handlers are overloaded.

This slice is intentionally coarse-grained. It applies limits to shared HTTP surfaces rather than to
individual clients, identities, API keys, or source IPs. The goal is to establish a simple,
configurable startup-time policy for the most important Rook entry points now, without pulling in
identity-aware throttling, idempotency, streaming, TLS, RBAC, or outbound provider-auth work.

## Scope

### In Scope

- Define a dedicated #591 follow-up slice for global rate limiting on Rook HTTP entry points.
- Define separate global policies for these three surfaces:
  - `/api/*`
  - `/v1/models`
  - `/v1/chat/completions`
- Define rate-limit exceed behavior as `429 Too Many Requests`.
- Define `Retry-After` response-header requirements when a request is rejected for exceeding a
  configured limit.
- Define startup/config-driven limit inputs so operators can configure the per-surface policies for
  this slice.
- Define how rate limiting composes with the already-archived auth-boundary and transport-middleware
  baseline slices without changing their responsibilities.
- Identify the minimal server, transport/middleware, config, and spec areas that follow-on
  spec/design work must cover.

### Out of Scope

- Any per-client, per-user, per-token, per-account, per-session, or per-IP rate limiting behavior.
- Identity-aware quotas, fairness policies, abuse scoring, reputation systems, or WAF-style
  controls.
- Idempotency keys, replay protection, or duplicate-request suppression.
- Streaming-specific rate limiting or partial-response behavior.
- TLS, mTLS, reverse-proxy certificate policy, or network-edge deployment controls.
- RBAC, route scopes, or any authorization-policy expansion.
- Outbound provider authentication/header changes in `clients/rook/src/gateway/vendor.rs`.
- Broad traffic-management work beyond the three named surfaces.

## Approach

Treat this slice as transport-boundary middleware or router-layer policy in `clients/rook`, applied
before the target admin or gateway handlers execute. The design should define one global budget per
named surface, not a keyed budget per caller identity.

The core contract is simple:

- `/api/*` has its own global limit policy.
- `/v1/models` has its own global limit policy.
- `/v1/chat/completions` has its own global limit policy.
- Requests over the applicable budget MUST be rejected with HTTP `429 Too Many Requests`.
- Rejected responses MUST include `Retry-After` so callers know when to retry.
- Limits MUST be sourced from startup/config inputs in this slice rather than being hard-coded as the
  only runtime behavior.

This proposal intentionally avoids designing identity-aware throttling because that would expand the
 slice into a different security and fairness problem. For now, the value is a predictable global
back-pressure policy by surface that can be configured and rolled back easily.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/server/mod.rs` | Modified | Compose global per-surface rate-limiting policy in front of `/api/*`, `/v1/models`, and `/v1/chat/completions`. |
| `clients/rook/src/admin/` | Modified | Ensure `/api/*` requests consume the admin-surface global policy without changing admin business semantics. |
| `clients/rook/src/gateway/` | Modified | Ensure `/v1/models` and `/v1/chat/completions` consume distinct gateway-surface policies without changing OpenAI-shaped handler contracts. |
| `clients/rook/src/config/` | Modified | Define startup/config inputs for per-surface global rate-limit settings and validation rules. |
| `clients/rook/src/` transport/middleware module(s) | New or Modified | Add middleware/helpers for shared surface counters, limit evaluation, and `429` + `Retry-After` response shaping. |
| `openspec/specs/gateway/spec.md` | Modified (follow-on) | Add delta requirements for global per-surface rate limiting and rejection behavior. |
| `openspec/changes/rook-591-global-surface-rate-limits/` | New | Proposal and follow-on spec/design/tasks artifacts for this slice. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Global limits are mistaken for identity-aware fairness controls | High | State explicitly in spec/design that this slice is surface-wide only and does not distinguish callers. |
| A single noisy caller can consume the entire surface budget | High | Document this tradeoff clearly and defer per-client/per-IP limiting to a later slice instead of silently expanding scope now. |
| Misconfigured startup limits cause unnecessary `429` responses or operational lockout | Medium | Require explicit config validation, clear operator-facing configuration semantics, and a reversible rollback path. |
| `Retry-After` semantics become inconsistent across the three surfaces | Medium | Define one rejection contract at shared transport composition points rather than per-handler custom logic. |
| Rate limiting leaks into unrelated routes such as dashboard assets or other future endpoints | Medium | Bind enforcement only to `/api/*`, `/v1/models`, and `/v1/chat/completions` in this slice. |
| The slice grows into idempotency, streaming, proxy, or broader traffic-shaping work | Medium | Keep proposal bounded to global-by-surface admission control and explicitly defer adjacent concerns. |

## Rollback Plan

If this slice causes regressions or unacceptable operational throttling, revert the global
rate-limiting middleware/router composition for `/api/*`, `/v1/models`, and
`/v1/chat/completions`, remove the startup/config entries introduced for per-surface limits, and
restore the prior behavior from the archived inbound-auth and transport-middleware slices without
global request throttling. Because this slice is transport-boundary-only, rollback should not
require changes to auth semantics, business handlers, outbound vendor auth, or unrelated routes.

## Dependencies

- The archived `rook-591-inbound-auth-boundary` slice remains a completed prerequisite and must stay
  separate from this rate-limiting slice.
- The archived `rook-591-transport-middleware-baseline` slice remains the transport composition
  baseline and should host this new policy cleanly rather than reworking unrelated middleware scope.
- Existing server composition in `clients/rook/src/server/mod.rs` must continue to host `/api/*`,
  `/v1/*`, and dashboard routes without widening this slice.
- The gateway spec in `openspec/specs/gateway/spec.md` will need a follow-on delta for global
  per-surface limits, `429` behavior, and `Retry-After` requirements.
- Follow-on design work must confirm where startup/config values are parsed, validated, and exposed
  to transport composition in `clients/rook`.

## Success Criteria

- [ ] The change remains narrowly scoped to global-by-surface rate limiting for Rook HTTP entry
      points.
- [ ] The proposal explicitly defines separate policies for `/api/*`, `/v1/models`, and
      `/v1/chat/completions`.
- [ ] The proposal explicitly requires HTTP `429 Too Many Requests` with `Retry-After` on limit
      exceed.
- [ ] The proposal explicitly requires startup/config-driven per-surface limits for this slice.
- [ ] Per-client, per-IP, identity-aware, idempotency, streaming, TLS, RBAC, and outbound provider
      auth work remain explicitly deferred.
- [ ] The change is ready for `sdd-spec` and `sdd-design` to define concrete contracts and
      implementation details.
