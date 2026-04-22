# Proposal: Establish Rook Transport Middleware Baseline

## Intent

Rook issue #591 still has a narrow but important follow-up slice after the inbound auth boundary was
completed: the HTTP transport layer needs a baseline middleware contract before later hardening work
can be added safely. Today, transport concerns such as request correlation, tracing/logging hooks,
header sanitation, and proxy-header trust boundaries are not yet defined as a cohesive baseline for
`/api/*` and `/v1/*`.

This slice defines that baseline and keeps it explicitly separate from the archived inbound auth
boundary work. The goal is to make request handling more observable and safer by default, while
preserving the chosen security posture: Rook MUST default to strict handling and MUST NOT trust
`X-Forwarded-*` headers unless an explicit trusted-proxy policy is configured.

## Scope

### In Scope

- Define a dedicated #591 follow-up slice for transport middleware baseline concerns on Rook HTTP
  entrypoints only.
- Define request ID generation and propagation expectations for inbound `/api/*` and `/v1/*`
  requests.
- Define tracing/logging hook requirements at the transport boundary so requests can be correlated
  without changing downstream business contracts.
- Define inbound header sanitation requirements for transport-layer and proxy-related headers before
  handlers rely on them.
- Define a strict-by-default forwarded-header trust policy.
- Define the explicit opt-in conditions under which `X-Forwarded-*` or similar forwarded metadata MAY
  be honored.
- Identify the minimal server, middleware, configuration, and spec areas that follow-on spec/design
  work must cover for this slice.

### Out of Scope

- Any changes already covered by `rook-591-inbound-auth-boundary`.
- Inbound bearer auth semantics, token parsing, or authorization policy changes.
- Rate limiting, quotas, abuse controls, or IP throttling.
- Idempotency keys or replay-protection behavior.
- Streaming request/response transport behavior.
- TLS termination, certificate handling, or mTLS policy.
- RBAC, route scopes, multi-tenant authorization, or permission models.
- Outbound provider authentication/header changes in `clients/rook/src/gateway/vendor.rs`.
- Response-shaping work unrelated to request correlation or middleware observability.

## Approach

Treat this slice as transport-boundary middleware policy for `clients/rook`, applied before admin or
gateway handlers execute. The baseline should establish deterministic request metadata handling,
sanitized header views, and observability hooks without changing Rook’s existing route intent.

The key design constraint is trust: forwarded headers are attacker-controlled unless a trusted proxy
boundary is explicitly configured. For that reason, the proposal sets a strict default posture where
Rook ignores untrusted `X-Forwarded-*` inputs for security-sensitive interpretation and relies on the
direct connection context unless configuration says otherwise. Any future proxy-aware behavior must be
 opt-in, narrowly scoped, and testable.

Request IDs and tracing hooks should be defined as middleware-level concerns so that later slices can
reuse the same correlation model across auth, diagnostics, and operational analysis. Header
sanitation should likewise be defined at entry so downstream code does not need to re-implement
transport hygiene inconsistently.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/server/mod.rs` | Modified | Compose a transport middleware baseline in front of `/api/*` and `/v1/*` routing. |
| `clients/rook/src/admin/` | Modified | Consume sanitized transport context and request-correlation behavior without changing admin business semantics. |
| `clients/rook/src/gateway/` | Modified | Consume sanitized transport context and request-correlation behavior without changing gateway business semantics. |
| `clients/rook/src/config/` | Modified | Define minimal configuration for request ID behavior, transport logging hooks, and trusted forwarded-header policy. |
| `clients/rook/src/` transport/middleware module(s) | New | Add middleware or helpers for request IDs, tracing/logging hooks, header sanitation, and forwarded-header policy enforcement. |
| `openspec/specs/gateway/spec.md` | Modified (follow-on) | Add delta requirements for the transport middleware baseline and strict forwarded-header trust posture. |
| `openspec/changes/rook-591-transport-middleware-baseline/` | New | Proposal and follow-on spec/design/tasks artifacts for this slice. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Forwarded headers are trusted implicitly through framework defaults or convenience APIs | High | Make strict-by-default trust policy explicit in spec/design and require opt-in trusted-proxy configuration before honoring forwarded metadata. |
| Header sanitation removes data needed by existing local integrations | Medium | Limit this slice to transport and proxy metadata hygiene, document exact sanitation rules, and preserve rollback to previous pass-through behavior if needed. |
| Request ID propagation becomes inconsistent across `/api/*` and `/v1/*` | Medium | Define one baseline correlation contract at shared middleware composition points rather than per-router custom behavior. |
| Logging/tracing hooks accidentally capture sensitive headers or credentials | Medium | Require redaction/sanitization boundaries in follow-on spec/design and keep hooks metadata-focused. |
| Slice expands into broader traffic-management or security work | Medium | Keep proposal bounded to correlation, sanitation, and forwarded-header trust policy only; explicitly defer rate limiting, idempotency, TLS, and RBAC. |

## Rollback Plan

If this slice causes regressions, revert the transport middleware layer introduced for request IDs,
tracing/logging hooks, header sanitation, and forwarded-header policy, remove any new trusted-proxy
configuration added by this slice, and restore the prior direct-request handling behavior for
`/api/*` and `/v1/*`. Because this slice is transport-boundary-only, rollback should not require
changes to auth boundary semantics, admin business logic, gateway routing, or outbound provider auth.

## Dependencies

- The archived `rook-591-inbound-auth-boundary` slice remains the completed prerequisite security
  baseline and must stay separate from this transport-middleware slice.
- Existing server composition in `clients/rook/src/server/mod.rs` must continue to host `/api/*`,
  `/v1/*`, and dashboard routes without widening scope into unrelated surfaces.
- The gateway spec in `openspec/specs/gateway/spec.md` will need a follow-on delta to capture request
  correlation, transport observability hooks, header sanitation, and forwarded-header trust policy.
- Any framework or library defaults around proxy-header parsing must be reviewed in follow-on spec and
  design work so strict-by-default behavior is not undermined accidentally.

## Success Criteria

- [ ] The change remains narrowly scoped to the transport middleware baseline for Rook HTTP entrypoints.
- [ ] The proposal explicitly states that `X-Forwarded-*` headers are NOT trusted unless explicitly configured.
- [ ] In-scope transport concerns are limited to request IDs, tracing/logging hooks, header sanitation, and forwarded-header policy.
- [ ] Auth boundary work already completed under `rook-591-inbound-auth-boundary` remains separate and unmodified by scope.
- [ ] Rate limiting, idempotency, streaming, TLS, RBAC, and outbound provider auth changes remain explicitly deferred.
- [ ] The change is ready for `sdd-spec` and `sdd-design` to define concrete contracts and implementation details.
