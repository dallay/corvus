# Design: Rook Security Defaults and Secret Protection

## Technical Approach

This change hardens the existing Rook security baseline without redesigning its auth model.

The implementation should stay narrow and focus on four concrete areas already partially present in
 code and specs:

- preserve and codify `127.0.0.1:4141` as the safe default bind target for protected Rook HTTP
  surfaces;
- make non-loopback exposure an explicit operator choice rather than an implicit default;
- reinforce the separation between inbound Rook auth and outbound provider credentials;
- tighten operator-visible secret protection across admin/status/config/logging surfaces.

This slice does **not** implement a new pairing flow. Instead, it aligns terminology and trust-boundary
 wording with shared onboarding specs while leaving Rook’s real runtime model grounded in its current
 inbound bearer-token configuration.

## Architecture Decisions

### Decision: Treat `127.0.0.1:4141` as a contract, not just an implementation accident

**Choice**: keep the current loopback defaults in `clients/rook/src/main.rs` and
 `clients/rook/src/server/mod.rs`, and add tests/spec language that make those defaults a deliberate
 safe posture.

**Rationale**:

- the defaults already exist in code;
- M3 should convert those defaults into explicit security guarantees;
- this reduces accidental drift toward permissive exposure while preserving existing operator
  overrides.

### Decision: Preserve explicit non-loopback override paths

**Choice**: do not remove support for `--host 0.0.0.0` or other non-loopback values, but ensure the
 system treats them as intentional operator overrides and does not describe them as inherently safe
 because the product is local-first.

**Rationale**:

- remote or scripted setups may legitimately depend on explicit host overrides;
- the hardening goal is “safe by default,” not “loopback only forever.”

### Decision: Keep inbound and outbound auth responsibilities separate in code and tests

**Choice**: use the existing separation as the implementation baseline:

- inbound client authentication is validated in `clients/rook/src/auth/middleware.rs`
- inbound auth config is validated in `clients/rook/src/config/mod.rs`
- outbound provider auth is derived from provider account credentials in
  `clients/rook/src/gateway/upstream.rs`

The slice should add or tighten tests proving the accepted inbound token is never reused as outbound
 vendor auth and never becomes a fallback for missing provider credentials.

**Rationale**:

- the separation is already conceptually correct;
- M3 should harden the boundary against regression and documentation drift.

### Decision: Expand secret-protection review beyond account CRUD responses

**Choice**: audit and harden operator-visible outputs that may disclose auth/config state, including:

- admin response models and config/status-style outputs
- structured logs and debug output
- future CLI/config export paths if currently stubbed or newly activated by this slice

Where the surface only needs presence information, it should use existing presence-only semantics
 such as `has_api_key` or an enabled/configured state rather than raw secret values.

**Rationale**:

- admin account redaction already exists and should be treated as the pattern to generalize;
- secret handling often regresses through status/config/logging paths rather than core CRUD paths.

### Decision: Align wording with shared onboarding/pairing constraints without claiming integration

**Choice**: if product copy, spec text, or operator guidance refers to inbound auth, describe it as
 a Rook inbound bearer-token boundary unless and until real pairing integration exists in Rook code.

**Rationale**:

- shared onboarding specs already define pairing terminology;
- mislabeling Rook’s current auth as pairing would be a security and product-trust bug.

## Expected Implementation Areas

Likely files and surfaces for this slice:

- `clients/rook/src/main.rs`
  - retain and test secure defaults
  - possibly improve operator-facing startup guidance wording
- `clients/rook/src/server/mod.rs`
  - reinforce default-vs-override semantics in tests and any operator-visible reporting
- `clients/rook/src/config/mod.rs`
  - preserve fail-closed inbound auth validation
  - add redaction-safe config/state reporting if any such surface is implemented here
- `clients/rook/src/auth/middleware.rs`
  - preserve strict inbound boundary behavior
- `clients/rook/src/gateway/upstream.rs`
  - reinforce provider-credential-only outbound auth behavior
- `clients/rook/src/admin/types.rs`
  - preserve presence-only/redacted account semantics
- `clients/rook/src/transport/middleware.rs`
  - extend structured logging regression checks if needed

## Testing Strategy

### 1. Local-default and override behavior

Add or tighten tests proving:

- default `rook serve` configuration binds to `127.0.0.1:4141`
- explicit non-loopback overrides remain honored
- effective bind reporting does not imply that local placement is itself an auth mechanism

### 2. Auth-boundary regression tests

Add or tighten tests proving:

- accepted inbound bearer tokens are not forwarded upstream as provider auth
- missing provider `api_key` does not cause fallback to inbound auth token
- inbound auth remains independent from unrelated trust states or terminology assumptions

### 3. Secret-protection regression tests

Add or tighten tests proving:

- operator-visible account/config/status outputs remain presence-only or redacted
- structured logs do not serialize raw secret-bearing values
- any operator-visible auth-state reporting shows enabled/configured state only

### 4. Documentation/spec consistency checks

Validation should confirm Rook copy/spec text uses “inbound bearer token” or equivalent precise
 terminology rather than unsupported pairing claims.

## Risks and Controls

### Risk: over-hardening breaks intentional remote use

**Control**: preserve explicit host overrides and test them directly.

### Risk: auth terminology drifts into unsupported pairing claims

**Control**: keep wording tied to evidenced code paths only.

### Risk: a secret leaks through a less obvious operator surface

**Control**: enumerate and test operator-visible outputs beyond basic CRUD response bodies.
