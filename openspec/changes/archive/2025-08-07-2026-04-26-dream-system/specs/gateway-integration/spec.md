# Delta for Gateway

## ADDED Requirements

### Requirement: Gateway Dream Integration Is Trigger-Only

Gateway behavior for Dream MUST be limited to invoking the runtime-defined session completion and
Dream trigger integration points.

The gateway MUST NOT become the behavioral source-of-truth for Dream eligibility, consolidation
content, or persistence semantics.

#### Scenario: Gateway delegates Dream semantics to runtime

- GIVEN the gateway completes a request flow that reaches the runtime session-completion path
- WHEN the gateway invokes the completion and Dream trigger integration points
- THEN the gateway MUST rely on the runtime to determine Dream eligibility and consolidation behavior
- AND the gateway MUST NOT define independent Dream eligibility rules.

#### Scenario: Gateway acceptance does not require Dream-specific transport contract

- GIVEN the runtime exposes Dream only through existing completion-trigger integration points
- WHEN a gateway-served request completes successfully
- THEN the gateway MUST remain valid without adding a new Dream-specific public HTTP contract
- AND Dream behavior MUST remain an internal runtime concern unless another spec adds a public surface.

### Requirement: Gateway Completion Hooks MUST Preserve Runtime Ordering and Idempotency

When the gateway participates in a session flow that records completion and triggers Dream, it MUST
invoke those runtime integration points in the runtime-defined order.

The gateway MUST preserve the runtime contract that completion recording happens before Dream
trigger evaluation, and repeated gateway completion handling for the same session MUST NOT require
gateway-defined duplicate-consolidation logic.

#### Scenario: Gateway calls completion recording before Dream trigger

- GIVEN a gateway-served session `sess-123` reaches its completion path
- WHEN the gateway invokes runtime integration for that completion
- THEN the completion-recording hook MUST run before the Dream-trigger hook
- AND Dream evaluation MUST consume the runtime-recorded completion state.

#### Scenario: Replayed gateway completion path stays safe through runtime idempotency

- GIVEN the gateway re-enters the same completion path for session `sess-123`
- WHEN it invokes the runtime completion and Dream hooks again
- THEN the gateway MUST rely on runtime idempotency for the completed session
- AND the repeated gateway path MUST NOT require a second independent Dream result.
