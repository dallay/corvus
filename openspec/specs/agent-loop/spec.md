# Agent Loop Specification

## Purpose

This specification defines the canonical Agent Loop behavior for the Corvus project, consolidating
the dual-loop paths (`loop_.rs` and `agent.rs` + `dispatcher.rs`) into a single explicit contract.
It covers the loop lifecycle, tool-dispatch semantics, session scoping, approval invariants, and
security requirements across all entry points (CLI, channels, and gateway).

## Requirements

### Requirement: Entry Points Alignment

The system MUST provide a unified loop contract across dispatcher-backed entry points: CLI,
channels, gateway `/webhook`, and admitted WhatsApp MVP image turns. Gateway `/webhook` MUST
execute through the canonical dispatcher boundary and MUST preserve the same session, policy,
approval, tool-dispatch, and result semantics as other canonical entry points unless an explicitly
documented transport compatibility shim applies. Gateway `/whatsapp` MUST preserve transport
verification, idempotency, and rate-control checks before canonical execution begins, and any
admitted WhatsApp turn that contains an MVP image part MUST execute through the same canonical
channel/runtime seam used by other dispatcher-backed turns. This change applies only to WhatsApp
MVP image turns and MUST NOT be interpreted as a broader parity promise for unrelated `/whatsapp`
behaviors.

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

#### Scenario: WhatsApp image turn enters the canonical runtime seam

- GIVEN a WhatsApp webhook event passes transport verification and contains an admitted MVP image
  turn
- WHEN the gateway hands the turn to the runtime
- THEN the system MUST execute that turn through the same canonical dispatcher-backed channel loop
  used for other admitted channel turns
- AND the turn MUST inherit the same session, policy, approval, tool, and result semantics.

#### Scenario: Rejected WhatsApp transport never reaches the runtime

- GIVEN a WhatsApp webhook event fails signature validation, idempotency, or another required
  transport check
- WHEN the gateway evaluates the event
- THEN the system MUST reject the event before canonical runtime execution begins
- AND the dispatcher-backed channel loop MUST NOT run for that rejected request.

### Requirement: MVP Inbound Image Turn Contract

The system MUST normalize admitted multimodal turns into an ordered canonical content-part contract
limited to `text` and `image` parts for this MVP. Each canonical image part MUST preserve the
originating channel identity, a channel media reference or runtime-managed media handle, MIME type
when known, and associated caption text when supplied by the channel. The system MUST preserve the
relative ordering of text and image parts that is required to reconstruct the user turn, and it
MUST NOT require generic document, audio, video, or arbitrary attachment semantics in this MVP.

#### Scenario: Telegram photo with caption is normalized into canonical parts

- GIVEN Telegram delivers a user message containing a photo and a caption
- WHEN the runtime admits the message as an MVP multimodal turn
- THEN the system MUST normalize the turn into canonical `text` and `image` parts
- AND the image part MUST retain the Telegram media reference and known metadata needed for later
  validation and provider adaptation.

#### Scenario: Non-image attachment is not coerced into an image turn

- GIVEN a channel event contains a document, audio clip, or video without an admitted MVP image
  part
- WHEN the runtime evaluates the event for this change
- THEN the system MUST NOT coerce that attachment into the canonical image-part contract
- AND the event MUST remain outside the multimodal image MVP scope.

### Requirement: Image Admission Safety and Retention Controls

The system MUST treat all inbound image media as untrusted. Before media fetch or provider handoff,
it MUST validate channel origin and enforce an image allowlist for MIME type, bounded retrieval,
and configured size ceilings. The system MUST redact raw image content from logs, traces, and
operator diagnostics, raw image bytes MUST be ephemeral, and raw image bytes MUST NOT be persisted
to long-term memory by default in this MVP. The system MUST emit rollout telemetry that can
distinguish admitted, rejected, filtered, and provider-routed image turns without exposing the
image contents themselves.

#### Scenario: Oversized or disallowed media is rejected before provider routing

- GIVEN an inbound Telegram or WhatsApp image exceeds the configured size ceiling or fails the
  allowed image MIME policy
- WHEN the runtime validates the admitted media
- THEN the system MUST reject the image turn before any provider request is made
- AND the rejection telemetry MUST identify the turn as filtered or rejected without logging the raw
  media payload.

#### Scenario: Admitted image bytes are handled ephemerally

- GIVEN an inbound image turn passes validation and completes provider processing
- WHEN the turn is recorded in runtime history and observability systems
- THEN the system MUST avoid persisting raw image bytes to long-term memory by default
- AND any stored audit or telemetry record MUST omit or redact the raw image payload.

### Requirement: MVP Channel Boundaries and Ingress Fallback

The system MUST support inbound image understanding in this MVP only for Telegram and WhatsApp. It
MUST NOT extend canonical image-turn admission to generic gateway `/webhook`, web chat, dashboard,
mobile bridge, Signal, Matrix, Email, or other channels as part of this change. When image ingress
is disabled for a supported channel, or a supported channel image turn is rejected by admission
policy, the system MUST fail closed for the image turn and MUST NOT silently drop the image while
continuing as if the request were text-only. The system SHOULD return a channel-safe explanation
that image input is unavailable or rejected.

#### Scenario: Supported channel image ingress is disabled by rollout control

- GIVEN Telegram or WhatsApp image ingress is disabled by configuration
- WHEN a user sends an image turn through that channel
- THEN the system MUST return an explicit unsupported or unavailable image outcome for that turn
- AND the system MUST NOT silently downgrade the turn into text-only processing.

#### Scenario: Out-of-scope surface remains text-only

- GIVEN a request reaches generic gateway `/webhook` or another out-of-scope surface with image-like
  input
- WHEN this MVP contract is evaluated
- THEN the system MUST NOT treat that surface as supporting canonical inbound image turns
- AND any broader multimodal surface support MUST be defined in a follow-up change.

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

For this slice, when the canonical dispatcher returns a Plan Mode blocked outcome, the webhook
response MUST preserve that outcome as a distinct machine-readable terminal result. The gateway MUST
allow callers to distinguish successful completion, approval-required blocking, Plan Mode blocking,
other denials, and failures.

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

#### Scenario: Webhook response preserves distinct Plan Mode blocked semantics

- GIVEN a gateway `/webhook` request runs in Plan Mode
- AND the canonical dispatcher blocks a requested capability because it is outside the plan-safe
  boundary
- WHEN the gateway returns the final HTTP response
- THEN the response MUST preserve a distinct machine-readable Plan Mode blocked outcome
- AND the response MUST NOT collapse that outcome into a generic error or approval-required result.

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

### Requirement: Stream Events Lifecycle

The canonical loop MUST emit predictable stream events during its lifecycle, ensuring callers can
accurately track prompt assembly, tool execution, and final response generation.

#### Scenario: Standard Iteration Events

- GIVEN an active agent loop
- WHEN a tool call is dispatched and completed
- THEN the system MUST emit start, progress, and completion events for the tool execution
- AND the system MUST append the results to the loop's context before the next iteration.

### Requirement: Context Compaction

The system MUST enforce context compaction to protect memory limits and runtime stability when the
loop iteration history grows beyond the configured threshold.

#### Scenario: Triggering Compaction

- GIVEN an agent loop iterating over multiple tool calls
- WHEN the cumulative context size exceeds the predefined safety threshold
- THEN the system MUST trigger a compaction routine to summarize or truncate older history
- AND the system MUST preserve the current `session_id` and essential context required for the
  ongoing task without interruption.

### Requirement: Timeout Aborts

The loop MUST respect per-turn latency and total iteration budgets to prevent runaway execution or
unresponsive loops.

#### Scenario: Runaway Loop Abortion

- GIVEN an active agent loop with a configured iteration budget or timeout limit
- WHEN the loop exceeds the maximum allowed iterations or processing time
- THEN the system MUST forcefully abort the loop
- AND the system MUST emit a timeout error event to the caller
- AND the system MUST safely release associated session resources.

### Requirement: Error Handling and Fallbacks

The system MUST gracefully handle tool execution failures, network timeouts, and model errors
without crashing the agent loop, utilizing retry and backoff discipline.

#### Scenario: Recoverable Tool Failure

- GIVEN a tool call dispatched during an active loop iteration
- WHEN the tool execution fails due to a transient error (e.g., network timeout)
- THEN the system SHOULD attempt to retry the tool call based on configured backoff policies
- AND if the failure persists, the system MUST return a structured error to the model to allow for
  an alternative strategy or graceful degradation.

#### Scenario: Unrecoverable Error

- GIVEN an active agent loop
- WHEN an unrecoverable error occurs (e.g., severe parsing failure or auth rejection)
- THEN the system MUST terminate the loop immediately
- AND the system MUST scrub sensitive values before logging or returning the error to the user.

### Requirement: Security Profiling and Invariants

The loop MUST enforce strict approval, risk classification, and authorization boundaries at every
iteration and tool dispatch phase.

#### Scenario: Tool Dispatch with High-Risk Classification

- GIVEN a tool dispatched by the model that requires elevated privileges
- WHEN the dispatcher intercepts the tool call request
- THEN the system MUST evaluate the action against the current session's risk classification and
  approval policy
- AND the system MUST block the execution and request explicit user approval if the action exceeds
  the permitted risk threshold
- AND the system MUST NOT proceed until explicit authorization is granted or the request is aborted.

### Requirement: Specialized Session Reuse

The canonical agent loop MUST support specialized runtime sessions that reuse the same bootstrap,
dispatcher, approval, and security boundaries as generic sessions. A specialized session MUST add
mode-specific behavior without creating a parallel loop contract.

#### Scenario: Code-specialist session uses canonical loop

- GIVEN a caller starts a code-specialist session from a canonical runtime entry point
- WHEN the session enters execution
- THEN the system MUST run that session through the same canonical loop lifecycle used by other
  dispatcher-backed sessions
- AND any specialized prompt or output behavior MUST remain inside the canonical loop contract.

### Requirement: Delegated Specialized Sessions

The canonical agent loop MUST permit a parent session to launch a bounded delegated specialized
session when configuration allows it. Delegated specialized sessions MUST inherit the same policy
and approval semantics as direct canonical sessions and MUST terminate within their configured
bounds.

#### Scenario: Delegated code session inherits canonical protections

- GIVEN a parent canonical session delegates work to a code-specialist session
- WHEN the delegated session executes tool calls
- THEN the system MUST apply the same dispatcher policy, approval checks, and security invariants
  used for direct canonical sessions
- AND the delegated session MUST return a structured completion result to the parent session.

#### Scenario: Delegated specialized session hits configured limit

- GIVEN a delegated specialized session with explicit iteration or timeout limits
- WHEN execution reaches a configured limit before task completion
- THEN the system MUST stop the delegated session within the same safety model used by the
  canonical loop
- AND the session MUST return a structured non-success result that identifies the enforced limit.

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

### Requirement: Slash Session Command Ingress Classification

The system MUST classify `/resume`, `/suspend`, `/tldr`, and `/compact` as slash session commands at
runtime ingress before autosave, memory enrichment, normal pre-execution evaluation, tool planning,
and model/provider execution.

Recognized slash session commands MUST take precedence over normal prompt handling across the
canonical agent-runtime entry points covered by this change.

#### Scenario: Recognized slash command bypasses normal prompt side effects

- GIVEN a canonical runtime entry point receives the exact user input `/tldr`
- WHEN ingress classification runs
- THEN the system MUST classify the input as a slash session command before autosave, memory enrichment, and normal pre-execution handling
- AND the system MUST route the request to the dedicated slash command handler instead of the normal agent loop.

#### Scenario: Unknown slash-like input falls through to normal prompt handling

- GIVEN a canonical runtime entry point receives the user input `/resume-later`
- WHEN ingress classification runs
- THEN the system MUST NOT classify the input as one of the supported slash session commands
- AND the system MUST preserve existing prompt handling semantics for the request.

#### Scenario: Leading supported slash command wins over conversational interpretation

- GIVEN a canonical runtime entry point receives the user input `/compact please help me later`
- WHEN ingress classification runs
- THEN the system MUST classify the request as `/compact`
- AND any remaining command text MUST be interpreted by the slash command handler rather than by the model.

### Requirement: Deterministic Slash Session Command Handling Path

The system MUST handle the supported slash session commands through a deterministic non-LLM path.

For this slice, classification, validation, persistence, session-state mutation, snapshot lookup,
and user-visible result generation for `/resume`, `/suspend`, `/tldr`, and `/compact` MUST complete
without invoking model inference, tool execution, or generic conversational memory as the source of
truth.

#### Scenario: Supported slash command does not invoke model execution

- GIVEN the runtime receives `/suspend` for a valid active session
- WHEN the command handler processes the request
- THEN the system MUST complete the command without invoking model/provider inference or tool dispatch
- AND the returned result MUST come from deterministic command logic backed by persisted session state.

#### Scenario: Slash command failure remains deterministic

- GIVEN the runtime receives `/resume missing-session`
- WHEN the target session cannot be resolved as a resumable suspended session
- THEN the system MUST return a deterministic command error result
- AND the system MUST NOT fall back to model execution to interpret or repair the request.

### Requirement: Coordinator-Backed Delegation Boundary

The canonical agent loop MUST permit delegated specialized work to run through the Track 4
in-process coordinator foundation without creating a second runtime loop contract. When the
coordinator path is used, the parent canonical session MUST remain the authoritative owner of child
lifecycle, orchestration status, and final delegated outcome.

Coordinator-backed delegation MUST preserve the same dispatcher, policy, approval, and security
boundaries already required for canonical and delegated specialized sessions. This slice MUST NOT be
interpreted as enabling remote child transport, disk-backed mailbox delivery, worktree isolation, or
full delegated permission escalation inside the agent loop.

#### Scenario: Parent session delegates through coordinator foundations

- GIVEN a parent canonical session delegates bounded specialized work through the Track 4 Slice 1
  coordinator path
- WHEN the delegated child session executes inside that orchestration run
- THEN the child session MUST remain inside the canonical loop's existing policy and approval
  boundaries
- AND the parent session MUST receive the final delegated outcome through the coordinator-owned
  orchestration result.

#### Scenario: Coordinator-backed delegation remains in-process for this slice

- GIVEN a delegated specialized session is launched through the coordinator foundations
- WHEN the runtime evaluates how child communication or isolation should be handled
- THEN the system MUST keep the delegated execution in-process for this slice
- AND the agent loop MUST NOT claim that remote bridge transport, mailbox-on-disk, or worktree
  isolation are already part of the delivered delegated path.
