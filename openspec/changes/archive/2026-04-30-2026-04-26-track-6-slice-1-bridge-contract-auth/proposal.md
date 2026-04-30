# Track 6 Slice 1: Bridge Contract, Auth, and Admission

## Why

Track 4 intentionally stopped at a fail-closed `remote_bridge` seam so the runtime could preserve a
single orchestration contract without pretending that remote execution already existed. The seam is
now concrete enough in code and roadmap scope to justify a first real Track 6 source-of-truth slice.

`clients/agent-runtime/src/bridge/mod.rs` already defines shared bridge-facing metadata for:

- protocol version negotiation (`BridgeProtocolVersion::V1`)
- transport kinds (`sse`, `websocket`)
- a remote bridge request shape (`protocol_version`, `transport`, `session_scope`)
- bridge availability outcomes that currently stop at `deferred` or explicit rejection
- a transport-agnostic bridge envelope carrying version, session scope, sequence, transport, kind,
  and payload

`clients/agent-runtime/src/tools/delegate_launch.rs` also already exposes `remote_bridge` as an
explicit transport request under child `execution.transport`, while continuing to reject streaming
payloads and unsupported remote behavior for the local orchestration slice.

Those seams mean Track 6 no longer belongs only as deferred text inside
`multi-agent-orchestration`. It now needs its own source-of-truth domain so Corvus can specify the
remote session contract without overloading either Track 4 orchestration semantics or the `gateway`
domain.

## Decision

Create a new spec domain: `bridge-remote-sessions`.

Rationale:

- `gateway` is the right maturity precedent for spec structure, but it is not the correct ownership
  domain for bridge session semantics.
- `multi-agent-orchestration` should continue to own local orchestration lifecycle and the fact that
  `remote_bridge` existed first as a fail-closed seam.
- Track 6 introduces a distinct contract surface: remote session negotiation, bridge admission,
  authenticated client binding, and transport negotiation across SSE and WebSocket.
- A dedicated domain lets later slices add reconnect/resume, streaming execution, and remote child
  lifecycle behavior without muddying the Track 4 local contract.

## Scope

This first Track 6 slice covers only:

- bridge contract identity and protocol versioning
- bridge client authentication requirements
- bridge admission decisions and fail-closed outcomes
- transport negotiation for SSE and WebSocket
- session-scope binding and rejection semantics

This slice does not cover:

- full remote child execution
- tool or result streaming execution semantics
- reconnect/resume after disconnect
- reattach after parent/runtime loss
- historical replay as authority
- mailbox fallback for remote requests
- delegated approval completion by remote clients

## Spec Changes

- Add new domain spec `openspec/specs/bridge-remote-sessions/spec.md`
- Define the first delivered Track 6 source-of-truth slice for remote bridge session contract and
  admission
- Keep Track 4 as the fail-closed seam owner for local orchestration compatibility

## Roadmap Impact

Track 6 should now be described as having an explicit source-of-truth slice for bridge protocol
contract, authentication, admission, and transport negotiation, while remote execution streaming,
reconnect/resume, and full isolated remote child lifecycle remain pending.
