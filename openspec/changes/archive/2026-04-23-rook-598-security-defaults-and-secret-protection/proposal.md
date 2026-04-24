# Proposal: Rook Security Defaults and Secret Protection

## Intent

Harden the first meaningful security baseline for Rook without redesigning its auth model. This
change codifies local-only serving defaults, makes safe exposure behavior explicit, reinforces the
existing separation between inbound admin authentication and outbound provider credentials, and
closes remaining secret-protection/documentation gaps while staying aligned with shared
pairing/onboarding terminology and constraints.

## Scope

### In Scope
- Codify Rook's local-only default bind behavior around the existing `127.0.0.1:4141` default and
  define the safe conditions and operator-facing behavior for intentional non-local exposure.
- Reinforce and document the existing separation between inbound Rook admin auth and outbound
  provider auth so implementation and operator guidance do not conflate the two trust boundaries.
- Harden secret-protection boundaries for Rook-facing account/admin surfaces, logs, and config or
  status outputs so secret material remains redacted and presence-only where contracts already imply
  that pattern.
- Align Rook behavior and wording with shared onboarding/pairing constraints where relevant,
  without inventing a new pairing flow or claiming full pairing reuse that is not yet evidenced in
  code.

### Out of Scope
- Building a new auth architecture, token issuer, or account/provider credential system.
- Claiming or implementing full pairing integration reuse beyond what the current Rook code already
  proves.
- Broad network-exposure redesign, remote deployment automation, or non-local onboarding flows.
- Expanding this slice into unrelated Rook feature work.

## Approach

Use the existing Rook defaults and auth split as the baseline, then tighten the contract around
them. The change should formalize `127.0.0.1:4141` as the safe default, make non-local binding an
explicit and intentionally handled choice, preserve the current separation between inbound admin
token validation and outbound provider credentials, and standardize redaction/presence-only secret
handling across the affected Rook surfaces. Shared onboarding/pairing terminology should be reused
only as a consistency guardrail, not as evidence of a complete shared pairing implementation.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/src/main.rs` | Modified | Codify and document secure default startup/bind behavior at the main entry point. |
| `clients/rook/src/server/mod.rs` | Modified | Formalize server-side local-only defaults and intentional non-local exposure handling. |
| `clients/rook` admin/auth/config surfaces | Modified | Clarify inbound admin auth versus outbound provider auth boundaries in code paths and operator semantics. |
| `clients/rook` logging/account presentation surfaces | Modified | Ensure secret values remain redacted, presence-only, or omitted in operator-visible outputs. |
| `openspec/specs/onboarding/spec.md` (reference only) | Referenced | Keep terminology and trust-boundary alignment without asserting unsupported pairing reuse. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Hardening local-only defaults could unintentionally disrupt existing remote or scripted operator setups that relied on permissive assumptions. | Medium | Keep explicit override paths intact, document exposure behavior clearly, and validate default-vs-override scenarios. |
| Auth-boundary clarification could accidentally conflate admin access checks with provider credential state if requirements are written too loosely. | Medium | State inbound and outbound auth responsibilities separately in proposal/spec language and test them independently. |
| Secret-protection tightening may miss a remaining display or logging edge case, creating false confidence. | Medium | Enumerate affected output surfaces and require regression coverage for redaction/presence-only behavior where practical. |

## Rollback Plan

Revert the Rook hardening changes in `clients/rook` to restore the prior default/exposure behavior
and prior operator messaging if the change blocks legitimate workflows. Because this proposal avoids
introducing a new auth system or transport contract, rollback is limited to restoring the previous
bind/default handling and any associated redaction or documentation adjustments.

## Dependencies

- Existing Rook bind defaults already present in `clients/rook/src/main.rs` and
  `clients/rook/src/server/mod.rs`.
- Existing inbound token validation, redacted account indicators such as `has_api_key`, and current
  structured-logging secret protections.
- Shared terminology and trust constraints from `openspec/specs/onboarding/spec.md`.

## Success Criteria

- [ ] The proposal/spec/design work explicitly defines `127.0.0.1:4141` local-only serving as the
  secure default and describes how intentional non-local exposure is handled safely.
- [ ] The change preserves and clarifies the separation between inbound Rook admin authentication
  and outbound provider credentials, without inventing a new auth model.
- [ ] Operator-visible Rook outputs, logs, and account/admin views do not expose raw secret
  material and use redacted or presence-only semantics consistently.
- [ ] The change aligns with shared onboarding/pairing constraints and terminology without claiming
  pairing integration that is not evidenced in current code.
