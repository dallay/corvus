# Delta for Agent Loop

## MODIFIED Requirements

### Requirement: Entry Points Alignment

The system MUST provide a unified loop contract across dispatcher-backed entry points: CLI,
channels, and gateway `/webhook`. Gateway `/webhook` MUST execute through the canonical
dispatcher boundary and MUST preserve the same session, policy, approval, tool-dispatch, and
result semantics as other canonical entry points unless an explicitly documented transport
compatibility shim applies. Transport shims MUST narrow only how canonical events or results are
projected onto HTTP responses; they MUST NOT weaken runtime policy or bypass dispatcher decisions.

(Previously: The system MUST provide a unified loop contract across entry points that execute the
canonical dispatcher (CLI and channels). The gateway webhook path is currently a scoped exception
that applies canonical pre-checks and then responds via `Provider::simple_chat()`. Any semantic
differences MUST be explicitly justified and narrow in scope.)

#### Scenario: Gateway webhook uses canonical dispatcher semantics

- GIVEN a user prompt arrives through gateway `/webhook`
- WHEN the request is admitted past gateway transport checks
- THEN the system MUST execute the turn through the same canonical dispatcher-backed loop used by
  other canonical entry points
- AND the system MUST apply the same policy, approval, tool, and result semantics as CLI and
  channels for an equivalent turn.

#### Scenario: Transport shim does not change runtime semantics

- GIVEN gateway `/webhook` returns an HTTP-specific projection of canonical loop output
- WHEN the gateway shapes that response for transport compatibility
- THEN the system MUST preserve the canonical dispatcher decision and final turn outcome
- AND the projection MUST NOT bypass, suppress, or reinterpret a blocked, denied, failed, or
  completed runtime outcome.

#### Scenario: WhatsApp remains outside this parity contract

- GIVEN a request arrives through gateway `/whatsapp`
- WHEN this change's parity contract is evaluated
- THEN the system MUST NOT treat `/whatsapp` as covered by the `/webhook` dispatcher parity
  requirements
- AND any `/whatsapp` behavior changes MUST be defined in a separate follow-up change.

## ADDED Requirements

### Requirement: Gateway Transport Boundary Preservation

Gateway-specific transport controls, including authentication, webhook validation, rate limiting,
and idempotency, MUST remain enforced before canonical runtime execution begins. These transport
controls MUST complement dispatcher protections and MUST NOT replace or dilute canonical policy,
approval, or tool-dispatch enforcement.

#### Scenario: Transport checks gate runtime entry

- GIVEN a gateway `/webhook` request fails authentication, webhook validation, rate limiting, or
  idempotency checks
- WHEN the gateway evaluates the inbound request
- THEN the system MUST reject the request before canonical loop execution starts
- AND the dispatcher-backed runtime MUST NOT run for that rejected request.

### Requirement: Gateway Webhook Approval Outcome Parity

Gateway `/webhook` MUST enforce the same approval and risk decisions as the canonical dispatcher
for an equivalent turn. When a canonical dispatcher outcome requires approval that cannot be
completed synchronously within the webhook request, the system MUST return a structured non-success
turn result that identifies the action as blocked or needing approval, and MUST NOT execute the
gated action.

#### Scenario: Approved action proceeds normally

- GIVEN a gateway `/webhook` turn produces a tool action allowed by canonical policy and approval
  rules
- WHEN the dispatcher evaluates the action
- THEN the system MUST execute the action under the same policy outcome that would apply for CLI or
  channels
- AND the final webhook result MUST reflect canonical turn completion semantics.

#### Scenario: Approval-required action is returned as blocked

- GIVEN a gateway `/webhook` turn produces a tool action that canonical policy classifies as
  approval-required
- WHEN the gateway cannot complete that approval within the request lifecycle
- THEN the system MUST return a structured blocked or needs-approval result
- AND the system MUST NOT execute the gated action.

### Requirement: Gateway Webhook Session Scoping

Gateway `/webhook` MUST treat `X-Session-Id` as the canonical session continuity key for
conversation history, memory association, and audit/event correlation. When `X-Session-Id` is
present, the system MUST scope the turn to that session and continue the associated canonical
history. When `X-Session-Id` is absent, the system MUST process the request as a standalone turn
with no implicit reuse of prior session state.

#### Scenario: Explicit session id reuses canonical state

- GIVEN a gateway `/webhook` request includes `X-Session-Id`
- WHEN the system executes the turn
- THEN the system MUST attach the turn to that canonical session scope
- AND conversation history, memory association, and audit continuity MUST use that same session
  identity.

#### Scenario: Missing session id is isolated

- GIVEN a gateway `/webhook` request omits `X-Session-Id`
- WHEN the system executes the turn
- THEN the system MUST treat the request as a standalone turn
- AND the system MUST NOT implicitly attach the turn to an existing session.

### Requirement: Gateway Webhook Response and Streaming Contract

Gateway `/webhook` MUST return a synchronous final turn result that preserves the canonical
dispatcher outcome for the request. The gateway MAY include a transport-specific projection of
canonical loop events in the same response, but any such projection MUST be explicitly treated as a
compatibility shim rather than a distinct runtime behavior. The gateway MUST NOT require a separate
streaming protocol to preserve parity for this change.

#### Scenario: Synchronous final result mirrors canonical outcome

- GIVEN a gateway `/webhook` request completes through the canonical dispatcher-backed loop
- WHEN the gateway returns the HTTP response
- THEN the response MUST include the final turn outcome corresponding to the canonical loop result
- AND callers MUST be able to distinguish successful completion from blocked, denied, and failed
  outcomes.

#### Scenario: Event projection remains informational

- GIVEN the gateway includes projected loop events or preview frames in a webhook response
- WHEN those events are presented to the caller
- THEN they MUST reflect the canonical dispatcher turn that already executed or was blocked
- AND they MUST NOT introduce a second source of truth that conflicts with the final turn result.

### Requirement: Gateway Compatibility Fallback and Rollout Safety

The system MUST support a gateway-scoped compatibility fallback that can route `/webhook` back to
the legacy `Provider::simple_chat()` path during rollout validation. When this fallback is active,
the system MUST preserve gateway transport controls, MUST clearly mark the request as using legacy
compatibility behavior in observability signals, and MUST NOT claim dispatcher parity for that
request.

#### Scenario: Fallback disables parity claims for a request

- GIVEN the gateway compatibility fallback is enabled for `/webhook`
- WHEN a webhook request is processed through the legacy path
- THEN the system MUST mark the request as legacy compatibility behavior in telemetry or audit
  signals
- AND the request MUST NOT be reported as dispatcher-parity execution.

#### Scenario: Comparative observability supports rollout

- GIVEN `/webhook` dispatcher parity is being rolled out behind a compatibility control
- WHEN requests are processed across dispatcher-backed and fallback paths
- THEN the system MUST emit enough structured observability data to distinguish which runtime path
  handled each request
- AND operators MUST be able to compare parity-path and fallback-path outcomes without exposing
  sensitive values.
