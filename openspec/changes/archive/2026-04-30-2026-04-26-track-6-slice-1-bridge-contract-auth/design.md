# Design: Track 6 Slice 1 Bridge Contract, Auth, and Admission

## Technical Approach

Track 6 Slice 1 establishes the first delivered remote-session boundary without pretending that full remote execution already exists. The codebase already contains the essential seam in `clients/agent-runtime/src/bridge/mod.rs`:

- a versioned bridge protocol enum (`BridgeProtocolVersion::V1`)
- transport kinds (`sse`, `websocket`)
- a minimal admission request shape (`RemoteBridgeRequest`)
- explicit fail-closed availability states (`Deferred`, `Rejected`)
- a transport-agnostic envelope (`BridgeEnvelope`)

At the same time, `clients/agent-runtime/src/tools/delegate_launch.rs` still rejects `remote_bridge` under the local Track 4 orchestration tool path with a stable deferred reason code. This slice preserves that local fail-closed behavior while creating a dedicated delivered contract in a new `bridge-remote-sessions` domain and corresponding runtime boundary for authenticated bridge session admission.

The technical strategy is therefore:

1. keep `multi-agent-orchestration` responsible for the **local** seam and continued rejection of `remote_bridge` through `delegate_launch`;
2. make `clients/agent-runtime/src/bridge/mod.rs` the shared contract/types layer for remote bridge session establishment;
3. introduce a **bridge admission/authentication boundary** that validates protocol version, requested transport, session scope, and JWT-backed caller identity before a remote session is accepted;
4. keep the slice deliberately narrow: no streaming tool execution semantics, no reconnect/resume, no authority recovery, and no child-run lifecycle beyond initial admission.

This design maps directly to the change spec in `openspec/changes/2026-04-26-track-6-slice-1-bridge-contract-auth/specs/bridge-remote-sessions/spec.md`:

- the bridge session contract is a dedicated domain;
- admission is versioned and transport-specific;
- authentication is JWT-based and fail-closed;
- session binding is explicit to a session scope;
- only SSE/WebSocket negotiation is delivered in this slice.

## Architecture Decisions

### Decision: Separate Track 6 bridge session admission from Track 4 local orchestration launch

**Choice**: Leave `delegate_launch` unchanged as the local orchestration entry point that continues to reject `remote_bridge`, and introduce Track 6 behavior as a separate bridge-session boundary using `bridge/mod.rs` types.

**Alternatives considered**:
- Teach `delegate_launch` to admit remote bridge children directly.
- Move bridge contract ownership into `multi-agent-orchestration`.
- Treat remote session admission as a gateway-only HTTP concern.

**Rationale**:
- The proposal explicitly creates a new `bridge-remote-sessions` source-of-truth domain.
- `delegate_launch` is scoped to local orchestration and currently fail-closes `remote_bridge` with a stable reason code; keeping that boundary avoids widening Track 4.
- Remote bridge sessions have distinct concerns: protocol negotiation, authentication, session binding, and transport establishment.

**Tradeoffs**:
- Two adjacent but separate surfaces now exist: local orchestration launch and remote bridge session admission.
- This separation is intentional and keeps scope precise.

### Decision: Use `bridge/mod.rs` as the canonical shared contract layer for Slice 1

**Choice**: Evolve the existing `BridgeProtocolVersion`, `BridgeTransportKind`, `RemoteBridgeRequest`, `RemoteBridgeAvailability`, and `BridgeEnvelope` types into the authoritative runtime contract for this slice.

**Alternatives considered**:
- Recreate separate admission structs in a different module.
- Keep the spec detached from current code and defer runtime type alignment.
- Put all admission logic directly inside transport handlers.

**Rationale**:
- These types already embody the correct seam and are the narrowest path to delivery.
- Reusing them minimizes drift between spec and implementation.
- Shared contract types let SSE and WebSocket handlers remain thin adapters over one admission core.

**Tradeoffs**:
- `bridge/mod.rs` may need moderate expansion to represent authenticated admission results and failure reasons.
- The module becomes more central, which is appropriate for a dedicated spec domain.

### Decision: Fail closed on unsupported version, invalid JWT, unsupported transport, or unsafe session binding

**Choice**: Introduce explicit admission outcomes for protocol mismatch, authentication failure, unsupported transport, and session-scope binding failure. No fallback or silent downgrade is allowed.

**Alternatives considered**:
- Implicitly default to V1 when version metadata is missing.
- Accept JWT parsing/auth issues and mark the client unauthenticated-but-connected.
- Silently switch between SSE and WebSocket based on server preference.

**Rationale**:
- The spec requires required admission metadata, authenticated client binding, and fail-closed transport negotiation.
- A bridge session is a trust boundary; partial admission would create ambiguous authority.
- Explicit rejection reasons are essential for operator review and future compatibility.

**Tradeoffs**:
- Slice 1 may reject more cases than a looser prototype would.
- This is desirable because the change is about contract correctness, not permissive rollout.

### Decision: Bind admission to session scope and authenticated principal before any bridge messaging is established

**Choice**: A remote bridge session is not considered admitted until the runtime has validated the JWT and bound the connection to the requested `session_scope`.

**Alternatives considered**:
- Let transport establishment happen before authentication.
- Authenticate the socket/stream but defer session binding until later messages.
- Use anonymous session scopes and layer auth later.

**Rationale**:
- The slice is specifically about contract, auth, and admission—not only transport syntax.
- Binding early guarantees that any subsequent envelopes are scoped to the right logical session.
- This avoids future ambiguity about whether a connection is authenticated-but-unbound.

**Tradeoffs**:
- Admission logic becomes a precondition for all downstream bridge activity.
- Later reconnection/resume slices must build on an already bound session model.

### Decision: Keep SSE and WebSocket as equivalent admission targets under one logical negotiation flow

**Choice**: Support both `sse` and `websocket` as requested transport kinds, but route both through the same protocol/auth/session admission core.

**Alternatives considered**:
- Deliver only SSE in Slice 1 and defer WebSocket.
- Give each transport a separate admission contract.
- Make one transport primary and the other best-effort.

**Rationale**:
- The spec and existing contract types already name both transport kinds.
- Slice 1 is about negotiation and contract symmetry, not transport-specific streaming behavior.
- A shared admission core reduces drift and keeps transport adapters narrow.

**Tradeoffs**:
- Implementers must define a common admitted-session representation early.
- Transport-specific runtime behavior beyond admission remains explicitly deferred.

## Existing Codebase Anchors

### `clients/agent-runtime/src/bridge/mod.rs`

Primary shared contract module.

Existing relevant types:
- `BridgeProtocolVersion`
- `BridgeTransportKind`
- `RemoteBridgeRequest`
- `RemoteBridgeAvailability`
- `BridgeEnvelope`

Slice 1 should build on these rather than replace them.

### `clients/agent-runtime/src/tools/delegate_launch.rs`

Current local seam behavior:
- rejects `CoordinatorTransport::RemoteBridge`
- emits stable structured reason code `remote_bridge_deferred`
- documents that remote transport remains out of scope for local orchestration

This behavior should remain intact in Slice 1 to preserve Track 4 boundaries.

### `openspec/specs/multi-agent-orchestration/spec.md`

Relevant ownership constraint:
- local orchestration remains the parent domain for in-process and mailbox-backed lifecycle behavior;
- Track 6 bridge delivery must not collapse those semantics into the local orchestration contract.

## Proposed Runtime Structure

### Shared contract layer: `bridge/mod.rs`

Responsibilities:
- canonical request/response/admission enums and structs;
- bridge envelope metadata shared by both transports;
- serialization compatibility tests.

Expected additions in Slice 1:
- authenticated admission result type;
- explicit admission rejection reasons/codes;
- JWT-derived principal/session binding metadata;
- optional transport-negotiation response shape if current types are too narrow.

### Admission service layer: new bridge admission/auth module

Introduce a narrow internal service responsible for:
- validating `protocol_version`;
- validating requested `transport` against delivered support;
- validating JWT and deriving authenticated client identity;
- binding authenticated client to `session_scope`;
- returning accepted or rejected admission outcomes.

This should be transport-agnostic and callable from future SSE and WebSocket entry points.

Possible locations:
- `clients/agent-runtime/src/bridge/admission.rs`
- `clients/agent-runtime/src/bridge/auth.rs`

The exact file split may vary, but the design requires a single logical admission core.

### Transport adapter layer

Transport adapters for SSE and WebSocket should remain thin:
- parse incoming admission request;
- extract auth material (JWT);
- call shared admission service;
- on success, establish admitted bridge session context;
- on failure, return explicit rejection outcome and do not open an admitted session.

No streaming command execution semantics are added here in Slice 1.

## Data Model Changes

### Extend remote bridge request/admission shapes

The current `RemoteBridgeRequest` already includes:
- `protocol_version`
- `transport`
- `session_scope`

Slice 1 likely needs additional admission-facing structures such as:
- `BridgeAdmissionRequest` or equivalent wrapper carrying JWT/auth material separately from the transport-neutral request metadata;
- `BridgeAdmissionOutcome` with accepted and rejected variants;
- accepted-session metadata including authenticated principal and bound session scope.

The design does **not** require JWT itself to live inside `BridgeEnvelope`; JWT is part of connection admission, not ongoing message payload metadata.

### Rejection taxonomy

Define explicit rejection reasons aligned with the spec, for example:
- unsupported protocol version
- unsupported transport kind
- invalid or missing JWT
- unauthorized principal for requested session scope
- unsafe or unknown session scope binding

These should be stable enough for tests and future operator-facing diagnostics.

### Admitted session context

Introduce a transport-neutral admitted context that contains at least:
- protocol version
- negotiated transport
- bound session scope
- authenticated principal identity
- initial sequence state for bridge envelopes

This context becomes the prerequisite for any later streaming or lifecycle work in future slices.

## Admission Flow

### Logical steps

1. Client presents bridge admission request with:
   - `protocol_version`
   - `transport`
   - `session_scope`
   - JWT auth material
2. Runtime validates protocol version.
3. Runtime validates requested transport is currently supported for Slice 1.
4. Runtime validates JWT and derives the authenticated principal.
5. Runtime authorizes and binds the principal to the requested `session_scope`.
6. Runtime returns accepted admission context or explicit rejection.
7. Only after acceptance may the connection participate as a bridge session.

## Sequence Diagrams

### Successful bridge session admission

```text
Remote client -> Bridge transport endpoint: admission request(protocol_version, transport, session_scope, JWT)
Bridge transport endpoint -> Bridge admission service: validate request
Bridge admission service -> Protocol validator: validate version
Bridge admission service -> Auth validator: validate JWT, derive principal
Bridge admission service -> Session binder: authorize principal for session_scope
Session binder --> Bridge admission service: bound session context
Bridge admission service --> Bridge transport endpoint: accepted admission outcome
Bridge transport endpoint --> Remote client: session admitted for requested transport and scope
```

### Fail-closed rejection on invalid JWT or unsafe scope

```text
Remote client -> Bridge transport endpoint: admission request(..., JWT)
Bridge transport endpoint -> Bridge admission service: validate request
Bridge admission service -> Auth validator: validate JWT
Auth validator --> Bridge admission service: invalid token
Bridge admission service --> Bridge transport endpoint: rejected(reason=auth_invalid)
Bridge transport endpoint --> Remote client: admission rejected; no bridge session established
```

### Local orchestration still rejects `remote_bridge`

```text
Parent -> delegate_launch: launch(child transport=remote_bridge)
delegate_launch -> local validation: inspect execution.transport
local validation --> delegate_launch: remote_bridge_deferred
DelegateLaunchTool --> Parent: validation error(remote_bridge_deferred)
```

## Implementation Plan

### Phase 1: Domain and contract alignment

1. Keep the dedicated `bridge-remote-sessions` spec domain as the source of truth.
2. Align `bridge/mod.rs` names and serialization behavior with the spec’s required admission contract.
3. Preserve `delegate_launch` local rejection semantics and ensure comments/tests reference the dedicated bridge domain rather than implying local support.

### Phase 2: Admission/auth core

1. Introduce a shared bridge admission service/module.
2. Add validation for protocol version and transport negotiation.
3. Add JWT validation boundary and authenticated principal extraction.
4. Add session-scope authorization/binding logic.
5. Define explicit accepted/rejected admission outcomes.

### Phase 3: Transport integration

1. Add thin SSE and WebSocket admission adapters that call the shared admission core.
2. Ensure both transports return equivalent contract semantics for success and rejection.
3. Keep all post-admission streaming/lifecycle behavior out of scope.

### Phase 4: Tests and documentation

1. Add unit tests for request serialization, rejection reasons, and accepted admission context.
2. Add regression tests ensuring `delegate_launch` still rejects `remote_bridge` through the local tool surface.
3. Add transport-level tests verifying SSE and WebSocket both honor the same admission contract.

## Testing Strategy

### `bridge/mod.rs` contract tests

Validate:
- `BridgeProtocolVersion::V1` serializes as expected;
- `BridgeTransportKind::{Sse, Websocket}` serialize as `sse` / `websocket`;
- request and envelope shapes remain stable;
- new admission outcome/rejection types serialize deterministically.

### Admission/auth unit tests

Validate:
- unsupported protocol version is rejected;
- unsupported transport is rejected;
- missing/invalid JWT is rejected;
- unauthorized session-scope binding is rejected;
- valid JWT + valid scope yields accepted admission context.

### Transport adapter tests

Validate:
- SSE and WebSocket both route through the same admission core;
- neither transport creates an admitted session on rejection;
- transport-specific framing does not alter admission semantics.

### Local seam regression tests

Validate:
- `delegate_launch` continues to reject `remote_bridge` with the stable reason code `remote_bridge_deferred`;
- local orchestration docs/comments do not imply delivered remote execution in Track 4.

## Risks and Mitigations

### Risk: Auth validation details are underspecified relative to future deployment reality

**Mitigation**: Keep Slice 1 focused on the contract boundary—JWT required, validated, and bound—without overcommitting to deployment-specific issuer plumbing in this design.

### Risk: Transport handlers duplicate admission logic

**Mitigation**: Require a single transport-agnostic admission core and keep adapters thin.

### Risk: Track 4 and Track 6 boundaries blur in code comments or tool behavior

**Mitigation**: Preserve `delegate_launch` fail-closed behavior and explicitly document that delivered bridge admission lives in the dedicated bridge domain.

### Risk: Session-scope binding becomes too permissive

**Mitigation**: Make scope binding an explicit admission check with fail-closed rejection when authorization is uncertain.

## Rollback Plan

If the first bridge admission implementation introduces ambiguity or unsafe assumptions:
- revert bridge admission/auth service additions;
- preserve `bridge/mod.rs` as a metadata-only seam if needed;
- keep `delegate_launch` rejecting `remote_bridge` with the existing stable reason code;
- retain the dedicated `bridge-remote-sessions` spec domain so contract work is not lost even if runtime delivery is rolled back.

This rollback is low risk because Slice 1 is intentionally bounded to admission/auth/negotiation and does not yet alter streaming execution, recovery, or local orchestration lifecycle behavior.
