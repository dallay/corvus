# Bridge Remote Sessions Specification

## Purpose

This specification defines the Track 6 source-of-truth contract for Corvus bridge and remote
sessions. Slice 1 covers the first delivered remote-session boundary only: protocol contract,
authentication, admission, and transport negotiation.

This specification is intentionally narrower than full remote execution. It defines how a remote
bridge client identifies its requested protocol and transport, how the runtime authenticates and
admits that client, and how the runtime fails closed when a bridge session cannot be safely bound.

## Requirements

### Requirement: Dedicated Bridge Remote Sessions Domain

The system MUST treat bridge and remote sessions as a dedicated source-of-truth domain distinct from
local multi-agent orchestration and distinct from the general gateway HTTP bind posture.

The `multi-agent-orchestration` specification MAY continue to describe `remote_bridge` as the local
fail-closed seam that existed before delivery, but delivered Track 6 behavior for bridge session
contract, authentication, admission, and transport negotiation MUST be specified here.

#### Scenario: Remote bridge behavior is specified outside Track 4 local orchestration

- GIVEN Corvus already exposes `remote_bridge` as a transport value in the local orchestration seam
- WHEN Track 6 Slice 1 is defined as a real delivered source-of-truth slice
- THEN the bridge session contract MUST be authored in a dedicated bridge/remote-sessions domain
- AND `multi-agent-orchestration` MUST remain responsible only for the local fail-closed seam and
  compatibility expectations.

### Requirement: Versioned Bridge Session Contract

The system MUST define a versioned bridge session contract for remote session establishment.

At minimum, the admission request contract MUST include:

- a protocol version
- a requested transport kind
- a session scope identifier

The system MUST treat these fields as required admission metadata rather than optional hints. A
request that omits one of these fields, uses an unknown version, or uses an unknown transport kind
MUST be rejected before a remote session is admitted.

For this slice, the versioned contract MUST align with the existing bridge seam represented by a V1
protocol version and distinct `sse` and `websocket` transport kinds.

#### Scenario: Valid V1 bridge request is accepted for admission evaluation

- GIVEN a remote bridge client sends a request with protocol version `v1`, a recognized transport,
  and a non-empty session scope
- WHEN the runtime evaluates the request for admission
- THEN the runtime MUST treat the request as structurally valid for further auth and admission checks
- AND the runtime MUST keep the requested version and transport as part of the bridge session
  contract.

#### Scenario: Unknown protocol version is rejected before session admission

- GIVEN a remote bridge client sends a bridge request with an unknown protocol version
- WHEN the runtime evaluates the request
- THEN the runtime MUST reject the request
- AND the runtime MUST NOT admit a bridge session under a best-effort or downgraded version.

#### Scenario: Unknown transport kind is rejected before session admission

- GIVEN a remote bridge client sends a bridge request with an unrecognized transport kind
- WHEN the runtime evaluates the request
- THEN the runtime MUST reject the request
- AND the runtime MUST NOT silently reinterpret the request as another transport.

### Requirement: Transport-Agnostic Bridge Envelope Metadata

The system MUST preserve one transport-agnostic bridge envelope metadata model across supported
remote bridge transports.

At minimum, each bridge envelope for an admitted session MUST carry:

- protocol version
- bound session scope
- sequence identifier
- transport kind
- event or message kind
- payload body

The envelope metadata MUST be sufficient to correlate an event to one admitted bridge session scope
without relying on transport-specific framing alone. SSE or WebSocket framing MAY differ, but the
logical envelope contract MUST remain shared.

#### Scenario: Same logical bridge event can be represented across supported transports

- GIVEN an admitted bridge session emits a bridge event with a defined kind and payload
- WHEN that event is delivered over SSE or WebSocket
- THEN the event MUST preserve the same logical envelope metadata
- AND the transport-specific framing MUST NOT change the event's session scope, version, or
  sequencing meaning.

#### Scenario: Envelope with mismatched session scope is rejected

- GIVEN a bridge envelope claims a session scope different from the admitted bridge session binding
- WHEN the runtime evaluates that envelope
- THEN the runtime MUST reject the envelope as invalid for that session
- AND the envelope MUST NOT be processed as if it belonged to the admitted scope.

### Requirement: Authenticated Bridge Admission

The system MUST require authenticated admission before a remote bridge session becomes active.

For this slice, authenticated admission MUST be JWT-based. A remote bridge client MUST present a
JWT that can be validated before the runtime upgrades the connection or treats the session as
admitted.

If the JWT is missing, malformed, expired, fails signature verification, or does not satisfy the
bridge session policy, the runtime MUST reject the session. The runtime MUST fail closed and MUST
NOT create an admitted remote session on unauthenticated or unverifiable input.

#### Scenario: Valid JWT allows bridge admission to continue

- GIVEN a remote bridge client presents a bridge request with a valid JWT
- WHEN the runtime validates the token successfully
- THEN the runtime MUST allow admission evaluation to continue
- AND the bridge session MUST remain unauthoritative until the remaining admission checks complete.

#### Scenario: Missing JWT is rejected

- GIVEN a remote bridge client requests a bridge session without presenting a JWT
- WHEN the runtime evaluates the request
- THEN the runtime MUST reject the request as unauthenticated
- AND the runtime MUST NOT admit the remote bridge session.

#### Scenario: Invalid or expired JWT is rejected

- GIVEN a remote bridge client presents a malformed, unverifiable, or expired JWT
- WHEN the runtime validates the token
- THEN the runtime MUST reject the request as unauthenticated
- AND the runtime MUST NOT upgrade or admit the session.

### Requirement: Session-Scope Binding During Admission

The system MUST bind an admitted remote bridge session to one session scope during admission.

The bound session scope MUST become part of the admitted session identity for all later bridge
traffic in that session. A bridge client MUST NOT be able to switch session scope mid-session by
sending later envelopes for a different scope.

If the requested session scope is empty, malformed, unauthorized for the authenticated client, or
conflicts with admission policy, the runtime MUST reject the request.

#### Scenario: Admitted bridge session is bound to one session scope

- GIVEN a remote bridge client is authenticated and passes admission checks for session scope
  `scope-123`
- WHEN the runtime admits the bridge session
- THEN the admitted session MUST be bound to `scope-123`
- AND later envelopes for that session MUST be evaluated against that same bound scope.

#### Scenario: Mid-session scope switch is rejected

- GIVEN a remote bridge session was admitted for session scope `scope-123`
- WHEN the client later sends an envelope claiming session scope `scope-456`
- THEN the runtime MUST reject that envelope
- AND the admitted session binding for `scope-123` MUST remain authoritative.

### Requirement: Fail-Closed Admission Outcomes

The system MUST expose fail-closed bridge admission outcomes.

For this slice, admission outcomes MUST distinguish at minimum:

- a deferred or unavailable outcome when the transport or capability is not yet delivered in the
  current runtime context
- an explicit rejection outcome with a machine-usable reason when the request is denied

The system MUST NOT silently fall back from a requested remote bridge session to `in_process`,
`mailbox`, or another local transport during bridge admission.

#### Scenario: Unsupported bridge capability is reported as deferred or unavailable

- GIVEN a bridge request is structurally valid but targets a capability not yet delivered in the
  current runtime context
- WHEN the runtime evaluates the request
- THEN the system MUST return a fail-closed non-admitted outcome
- AND the outcome MUST indicate that the capability is deferred or unavailable rather than pretending
  the session is active.

#### Scenario: Rejected bridge admission includes a reason

- GIVEN a bridge request fails authentication, policy, or session binding checks
- WHEN the runtime rejects the request
- THEN the admission outcome MUST record an explicit rejection reason
- AND the session MUST remain non-admitted.

#### Scenario: Remote bridge request is not downgraded to local transport

- GIVEN a caller requested remote bridge behavior
- WHEN bridge admission fails or the runtime cannot deliver the remote contract
- THEN the system MUST reject or defer that remote bridge request
- AND the system MUST NOT silently execute the request through `in_process` or `mailbox` transport.

### Requirement: Explicit Transport Negotiation

The system MUST perform explicit transport negotiation for remote bridge session admission.

For this slice, the recognized transport choices are `sse` and `websocket`. The runtime MUST
evaluate the client's requested transport as part of admission and MUST return a fail-closed outcome
when that requested transport cannot be served safely in the current runtime context.

The runtime MUST NOT claim that both transports are always interchangeable. Transport negotiation
MUST preserve which transport was requested and which transport, if any, was admitted.

#### Scenario: Requested SSE transport is admitted explicitly

- GIVEN a bridge client requests transport `sse`
- AND the runtime can safely serve an authenticated admitted bridge session over SSE in the current
  context
- WHEN admission succeeds
- THEN the admitted bridge session MUST record `sse` as its negotiated transport
- AND the runtime MUST NOT report that a different transport was admitted.

#### Scenario: Requested WebSocket transport is admitted explicitly

- GIVEN a bridge client requests transport `websocket`
- AND the runtime can safely serve an authenticated admitted bridge session over WebSocket in the
  current context
- WHEN admission succeeds
- THEN the admitted bridge session MUST record `websocket` as its negotiated transport
- AND the runtime MUST NOT report that a different transport was admitted.

#### Scenario: Requested transport is unavailable in the current runtime context

- GIVEN a bridge client requests either `sse` or `websocket`
- AND the current runtime context cannot safely serve that transport
- WHEN the runtime evaluates the request
- THEN the runtime MUST return a fail-closed non-admitted outcome
- AND the runtime MUST NOT silently switch the client to the other transport.

### Requirement: Slice Boundaries for Track 6 Slice 1

This slice MUST cover only the bridge session contract, JWT authentication, admission, session-scope
binding, and transport negotiation.

This slice MUST NOT be interpreted as delivering full remote child execution, tool/result streaming,
reconnect/resume, reattach after parent loss, historical replay as authority, or delegated approval
completion by remote clients.

The existence of a bridge envelope contract and negotiated transport in this slice MUST NOT be used
as proof that later Track 6 execution-streaming or recovery behavior already exists.

#### Scenario: Streaming execution remains out of scope

- GIVEN a caller expects this slice to provide full remote execution streaming semantics
- WHEN the Track 6 Slice 1 contract is evaluated
- THEN the system MUST treat streaming execution behavior as not yet delivered
- AND the slice MUST remain limited to contract, auth, admission, and transport negotiation.

#### Scenario: Reconnect and reattach remain out of scope

- GIVEN a remote bridge client disconnects after admission
- WHEN a caller expects reconnect, resume, or reattach behavior from this slice alone
- THEN the system MUST treat those capabilities as deferred
- AND the slice MUST NOT claim that admission alone delivers recovery semantics.
