# Delta for client-surfaces

## ADDED Requirements

### Requirement: Plan Mode Surface Scope and Activation Parity

For this first slice, only canonical runtime entry surfaces that already project dispatcher-backed
turn semantics MUST expose Plan Mode as a supported surface capability. This slice MUST include CLI
and gateway `/webhook` parity only.

Those surfaces MUST let callers activate Plan Mode explicitly through their surface contract rather
than through hidden defaults or inferred intent. Surfaces outside this slice MUST NOT claim Plan
Mode support unless a separate change defines that support.

This change MUST NOT be interpreted as requiring coordinator, memory-browser, worktree, dashboard,
mobile, or other surface parity beyond avoiding false support claims.

#### Scenario: CLI exposes explicit Plan Mode activation

- GIVEN an operator uses the canonical CLI surface
- WHEN the operator requests a Plan Mode turn or session explicitly
- THEN the CLI surface MUST project that request into the canonical runtime execution mode
- AND the CLI surface MUST NOT imply that Plan Mode was activated implicitly.

#### Scenario: Gateway webhook exposes explicit Plan Mode activation

- GIVEN a caller uses the canonical gateway `/webhook` surface
- WHEN the caller explicitly requests Plan Mode through the webhook contract
- THEN the gateway surface MUST project that request into the same canonical execution mode used by
  the CLI path
- AND the gateway surface MUST NOT invent surface-specific Plan Mode semantics.

#### Scenario: Out-of-scope surfaces do not claim Plan Mode support

- GIVEN a surface outside this slice, such as dashboard, mobile, worktree, or coordinator-oriented
  UX
- WHEN its capability contract is evaluated
- THEN the surface MUST NOT claim Plan Mode parity or support from this change alone
- AND any such support MUST be defined by a separate scoped change.

### Requirement: Plan Mode Transparency for Users and Audit Consumers

When a canonical client surface returns a Plan Mode blocked result, the surface MUST present that
result transparently as an analysis-only restriction rather than as a generic failure or standard
approval request.

User-visible and machine-consumable projections for this slice MUST preserve:

- a distinct blocked classification,
- the blocked capability or requested action,
- a reason indicating that Plan Mode allows only analysis-only capabilities, and
- the active execution mode.

Audit and observability projections for canonical surfaces MUST preserve the same distinction so
operators can tell that the request was blocked by Plan Mode policy rather than by unrelated runtime
failure. This requirement does not mandate new dashboard UX, new audit products, or parity for
other surfaces beyond preserving truthful classification where those records already exist.

#### Scenario: Gateway returns a transparent Plan Mode blocked result

- GIVEN a gateway `/webhook` request runs in Plan Mode
- AND the runtime blocks a non-plan-safe capability
- WHEN the gateway returns the terminal result
- THEN the result MUST identify the request as a distinct Plan Mode block
- AND the result MUST include the blocked capability, the reason, and the active execution mode.

#### Scenario: CLI preserves transparent blocked messaging

- GIVEN a CLI turn runs in Plan Mode
- AND the runtime blocks a non-plan-safe capability
- WHEN the CLI presents the outcome to the user or operator
- THEN the CLI MUST present the outcome as a Plan Mode restriction
- AND it MUST NOT misrepresent that restriction as successful execution or as ordinary approval flow.
