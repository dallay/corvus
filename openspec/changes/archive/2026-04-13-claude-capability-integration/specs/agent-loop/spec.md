# Delta for agent-loop

## ADDED Requirements

### Requirement: Explicit Plan Mode Activation and Capability Gating

The canonical dispatcher-backed agent loop MUST support an explicit Plan Mode for this first slice.
Plan Mode MUST be an opt-in execution mode rather than an implicit heuristic.

When Plan Mode is active, the system MUST allow only analysis-only capability classes needed for
inspection, retrieval, and search. These allowed classes MAY include read-only file inspection,
read-only memory recall, code search, image inspection, and web search style capabilities.

When Plan Mode is active, the system MUST block capability classes that can mutate state, execute
commands, write files, change external systems, or otherwise cross the analysis-only boundary. Any
capability that is not explicitly classified as plan-safe for this slice MUST be treated as blocked.

#### Scenario: CLI turn explicitly enters Plan Mode

- GIVEN a canonical CLI turn requests Plan Mode explicitly
- WHEN the dispatcher evaluates tool access for that turn
- THEN the system MUST apply Plan Mode capability gating for the entire turn
- AND the system MUST allow only the analysis-only capability classes defined for this slice.

#### Scenario: Gateway webhook explicitly enters Plan Mode

- GIVEN a canonical gateway `/webhook` request explicitly selects Plan Mode
- WHEN the dispatcher evaluates tool access for that request
- THEN the system MUST apply the same Plan Mode capability gating used by the CLI path
- AND the gateway path MUST NOT introduce a broader or narrower allowed capability set.

#### Scenario: Unclassified capability is blocked in Plan Mode

- GIVEN a Plan Mode turn requests a capability that is mutating, execution-heavy, or not explicitly
  classified as plan-safe
- WHEN the dispatcher evaluates that capability
- THEN the system MUST block the capability
- AND the system MUST fail closed instead of inferring that the capability is safe.

### Requirement: Plan Mode Blocked Outcome Semantics

When Plan Mode blocks a capability, the canonical dispatcher MUST return a distinct blocked outcome
that identifies the restriction as a Plan Mode policy decision rather than a generic failure or a
standard approval-required result.

The blocked outcome MUST be machine-readable and MUST preserve enough information for callers and
operators to distinguish:

- that Plan Mode was active,
- which capability or requested action was blocked, and
- why the request crossed the analysis-only boundary.

Outside Plan Mode, the existing approval and denial semantics MUST remain unchanged for the same
capability request.

#### Scenario: Mutating capability returns a distinct Plan Mode blocked outcome

- GIVEN a Plan Mode turn requests a mutating capability such as write, shell, or external-action
  execution
- WHEN the dispatcher evaluates that request
- THEN the system MUST return a distinct Plan Mode blocked outcome
- AND the blocked outcome MUST be machine-readable
- AND the system MUST NOT execute the blocked capability.

#### Scenario: Standard-mode semantics remain unchanged

- GIVEN the same capability request is evaluated outside Plan Mode
- WHEN canonical policy determines that the request is allowed, denied, or approval-required under
  normal semantics
- THEN the system MUST preserve that standard outcome
- AND the system MUST NOT relabel the result as a Plan Mode block.

## MODIFIED Requirements

### Requirement: Gateway Webhook Response and Streaming Contract

Gateway `/webhook` MUST return a synchronous final turn result that preserves the canonical
dispatcher outcome for the request. The gateway MAY include a transport-specific projection of
canonical loop events in the same response, but any such projection MUST be explicitly treated as a
compatibility shim rather than a distinct runtime behavior. The gateway MUST NOT require a separate
streaming protocol to preserve parity for this change.

For this slice, when the canonical dispatcher returns a Plan Mode blocked outcome, the webhook
response MUST preserve that outcome as a distinct machine-readable terminal result. The gateway MUST
allow callers to distinguish successful completion, approval-required blocking, Plan Mode blocking,
other denials, and failures.

(Previously: The webhook response was required to preserve the canonical final turn result and let
callers distinguish successful completion from blocked, denied, and failed outcomes, but it did not
explicitly require a distinct Plan Mode blocked classification.)

#### Scenario: Webhook response preserves distinct Plan Mode blocked semantics

- GIVEN a gateway `/webhook` request runs in Plan Mode
- AND the canonical dispatcher blocks a requested capability because it is outside the plan-safe
  boundary
- WHEN the gateway returns the final HTTP response
- THEN the response MUST preserve a distinct machine-readable Plan Mode blocked outcome
- AND the response MUST NOT collapse that outcome into a generic error or approval-required result.
