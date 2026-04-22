# Proposal: Harden Rook Inbound Auth Boundary for `/api` and `/v1`

## Intent

Rook issue #591 starts from a verified security gap: `clients/rook` currently exposes both the
operator-facing admin surface under `/api/*` and the client-facing gateway surface under `/v1/*`
without any inbound authentication boundary. That is acceptable only under the current
loopback-first M1 posture, but it is not sufficient for a transport surface that may later be
reached by broader clients, automation, or proxied environments.

This first slice is intentionally narrow and security-first. It establishes a dedicated inbound
authentication boundary for requests entering Rook, while preserving the already-existing outbound
provider authentication logic in `clients/rook/src/gateway/vendor.rs`. Inbound client trust and
outbound provider credentials are different concerns and MUST remain separate.

## Scope

### In Scope

- Define the first implementation slice of #591 as inbound authentication/authorization boundary
  work for Rook HTTP entrypoints only.
- Add an explicit auth boundary in front of `/api/*` admin routes.
- Add an explicit auth boundary in front of `/v1/*` gateway routes.
- Define a Rook-specific bearer-token request contract for inbound clients.
- Define failure behavior for missing, malformed, or invalid inbound credentials.
- Define how auth enforcement composes with the existing loopback-first default bind posture,
  without relying on loopback alone as the only protection.
- Reuse proven ideas such as bearer extraction and browser-origin guarding where appropriate, but
  only after adapting them to Rook’s contracts and route model.
- Preserve the separation between inbound client auth and outbound vendor auth/header generation.
- Identify the minimal server, gateway, admin, and configuration areas that will need follow-on spec
  and design work for this slice.

### Out of Scope

- Upstream provider authentication changes in `clients/rook/src/gateway/vendor.rs`.
- Collapsing inbound and outbound auth into one shared abstraction.
- Pairing-code flows, runtime onboarding, webhook secret hashing, or other `agent-runtime`
  trust-state assumptions that do not already belong to Rook.
- TLS termination, reverse-proxy deployment policy, or certificate management.
- Rate limiting, quotas, abuse controls, IP allowlists, or WAF-style protections.
- Role-based access control, per-route scopes, multi-tenant authorization, or fine-grained policy.
- Dashboard UI changes, operator UX, or client SDK ergonomics.
- Secret storage redesign, encryption-at-rest, or full credential lifecycle management beyond what
  the inbound boundary minimally needs.

## Approach

Treat inbound access control as a dedicated transport concern in `clients/rook`, enforced at router
composition and/or request middleware boundaries before admin and gateway handlers execute.

The proposal assumes a simple bearer-token gate is the right first slice because it is narrower,
testable, and easier to reason about than introducing pairing or broader trust orchestration. The
existing runtime helpers in `clients/agent-runtime/src/gateway/utils.rs` provide evidence for two
useful patterns: robust bearer extraction and a defensive admin origin guard. However, this change
must not copy runtime-specific assumptions about pairing state, webhook secrets, or onboarding
recovery. Instead, Rook should define its own transport contract, error shape expectations, and
configuration inputs for validating inbound client credentials.

The auth boundary should be applied consistently to both `/api/*` and `/v1/*`, while still allowing
the repository to keep explicitly public/non-sensitive endpoints separate if later slices require
that distinction. For this first slice, the safe default is deny-by-default for protected inbound
routes unless valid credentials are present.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/server/mod.rs` | Modified | Compose inbound auth enforcement with the existing `/api` and `/v1` routers without disturbing dashboard routing. |
| `clients/rook/src/admin/mod.rs` | Modified | Ensure admin routes sit behind the new inbound auth boundary and preserve route intent. |
| `clients/rook/src/gateway/mod.rs` | Modified | Ensure gateway routes sit behind the new inbound auth boundary while keeping handler contracts stable. |
| `clients/rook/src/gateway/vendor.rs` | No functional change expected | Explicitly preserve outbound provider auth/header generation as a separate concern. |
| `clients/rook/src/config/` | Modified | Define minimal configuration inputs for Rook inbound auth enforcement. |
| `clients/rook/src/` auth/support module(s) | New | Add Rook-specific bearer parsing/validation and optional origin-guard helpers if the design chooses dedicated modules. |
| `openspec/specs/gateway/spec.md` | Modified (follow-on) | Update the M1 “no auth” posture language to reflect the new #591 inbound boundary slice. |
| `openspec/changes/rook-591-inbound-auth-boundary/` | New | Proposal, and follow-on spec/design/tasks artifacts for this slice. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Inbound auth is coupled to outbound vendor auth by mistake | Medium | Keep route-entry auth logic separate from `gateway/vendor.rs`; document the boundary explicitly in spec/design. |
| Borrowing `agent-runtime` helpers imports wrong assumptions (pairing, webhook, onboarding) | High | Reuse only small patterns, not full runtime auth flows; require Rook-specific contracts and tests. |
| Over-scoping the first slice delays delivery | Medium | Keep this change limited to request-entry authentication and rejection behavior only. |
| Misconfigured auth could lock out valid local clients | Medium | Specify deterministic config requirements, clear unauthorized responses, and a reversible rollback path. |
| Browser-origin checks alone are mistaken for authentication | Medium | Treat origin checks only as an optional defense-in-depth measure, never as the primary auth control. |
| Router-level auth changes accidentally break existing dashboard or health behavior | Low | Keep protection targeted to `/api/*` and `/v1/*`; add composition tests in the follow-on spec/tasks phases. |

## Rollback Plan

If this slice causes regressions or operational lockout, revert the inbound auth enforcement layer
for `/api/*` and `/v1/*`, remove any Rook-specific inbound auth module/config additions introduced
by #591, and restore the prior loopback-first unauthenticated behavior documented in the current
gateway spec. Because this slice is intentionally transport-boundary-only, rollback should not
require changes to outbound vendor auth, routing decisions, or admin business logic.

## Dependencies

- Existing Rook server composition in `clients/rook/src/server/mod.rs` must continue to host `/api`,
  `/v1`, and dashboard routes together.
- Existing outbound provider auth separation in `clients/rook/src/gateway/vendor.rs` must be
  preserved.
- Existing gateway spec language in `openspec/specs/gateway/spec.md` currently states that auth is
  deferred to #591 and will need a follow-on delta spec.
- A concrete source for inbound bearer credentials/configuration will be needed in follow-on spec and
  design work.

## Success Criteria

- [ ] The change remains narrowly scoped to inbound auth enforcement for Rook HTTP entrypoints.
- [ ] The proposal clearly separates inbound client auth from outbound provider auth.
- [ ] `/api/*` and `/v1/*` are identified as protected surfaces for this slice, with clear rejection
      behavior for unauthorized access to be defined in follow-on spec work.
- [ ] Runtime helper reuse is constrained to adaptable patterns, not copied trust assumptions.
- [ ] The proposal leaves TLS, RBAC, rate limiting, and other hardening work explicitly deferred.
- [ ] The change is ready for `sdd-spec` and `sdd-design` to define precise contract and
      implementation details.
