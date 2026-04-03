# Proposal: Code Agent Specialist

## Intent

Corvus already has the core runtime primitives for coding work, including the `code` capability
profile, iterative agent loop, secure tool dispatch, MCP integration, and a one-shot `delegate`
tool. What is missing is a first-class product flow for code work: an explicit code-mode entry,
an opinionated coding workflow, and a delegated sub-agent contract that can execute bounded,
auditable coding sessions instead of returning a single provider response.

This change formalizes a code specialist on top of the existing runtime rather than introducing a
parallel runtime. The goal is to turn current building blocks into a visible, safe, and reusable
coding experience for both direct user sessions and delegated implementation tasks.

## Problem

- Corvus exposes coding tools but not an official code-specialist workflow.
- The current delegated-agent path is one-shot and cannot perform iterative inspect/edit/verify
  work.
- Entry-point UX does not clearly signal when the user is running in a dedicated code mode.
- Audit, validation, and final-result structure are too generic for reliable coding sessions.

## Goals

- Establish `code` as a first-class runtime mode using the existing bootstrap and loop.
- Define a coding-specific prompt and structured final result contract for implementation tasks.
- Extend delegation so a parent agent can launch a bounded code-specialist session.
- Preserve current security, approval, workspace-only, and MCP invariants.
- Make MVP changes incremental and compatible with current config/profile architecture.

## Scope

### In Scope

- Add an explicit CLI/runtime entry into code mode using existing agent/bootstrap paths.
- Productize the existing `code` profile with specialist defaults for prompt, output, limits, and
  validation hooks.
- Define a structured code-session result envelope for human and machine consumers.
- Evolve `delegate` to support bounded delegated code sessions backed by the canonical agent loop.
- Add code-session observability for files changed, commands run, validations attempted, and final
  status.
- Keep configuration declarative through existing runtime config surfaces.

### Out of Scope

- A separate code runtime, binary, or execution architecture.
- Full gateway/webhook parity while that path still uses `Provider::simple_chat()`.
- IDE/dashboard-first UX, multi-user collaboration, or remote worker execution.
- Broad new tool categories beyond the current coding surface and approved MCP integrations.
- Stack-specific validation automation beyond an MVP-compatible contract and defaults.

## Approach

Implement the change in two incremental layers on top of the current architecture.

First, expose a first-class code-mode entry that still instantiates the normal runtime stack:
bootstrap, profile filtering, prompt assembly, dispatcher loop, approvals, and MCP tooling. This
should make `Agent::code_from_config()` and the existing `code` profile visible as an official
user-facing mode, not just an internal seam.

Second, extend `delegate` from one-shot model prompting into a delegated code-session runner. The
delegated runner should instantiate a bounded specialized `Agent` session with explicit profile,
tool-iteration budget, timeout, and structured result envelope. The delegated path MUST reuse the
same policy, approval, and audit boundaries as direct code mode so it cannot bypass workspace or
risk controls.

Validation and reporting should be modeled declaratively through existing config/schema surfaces,
not embedded as prompt-only behavior. The MVP should support a minimal contract for post-change
checks and structured output, while leaving richer stack-specific automation for later phases.

## MVP Sequencing

1. Expose explicit code-mode entry in CLI/runtime and align it with the existing `code` profile.
2. Define code-specialist prompt/output contract and structured final result envelope.
3. Add config/schema support for code-session defaults, budgets, and validation/reporting options.
4. Implement delegated code sessions in `delegate` using the canonical agent loop.
5. Add observability/audit fields for code-session outputs and verify approval parity.

## Affected Areas

| Area                                           | Impact     | Description                                                                            |
|------------------------------------------------|------------|----------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/main.rs`            | Modified   | Add explicit code-mode invocation and user-facing runtime entry.                       |
| `clients/agent-runtime/src/agent/agent.rs`     | Modified   | Reuse and formalize specialized agent creation and bounded delegated sessions.         |
| `clients/agent-runtime/src/agent/prompt.rs`    | Modified   | Add code-specialist prompt and structured final-output guidance.                       |
| `clients/agent-runtime/src/bootstrap/mod.rs`   | Modified   | Formalize `code` profile defaults without duplicating runtime assembly.                |
| `clients/agent-runtime/src/config/schema.rs`   | Modified   | Add declarative configuration for code-session behavior, budgets, and output contract. |
| `clients/agent-runtime/src/tools/delegate.rs`  | Modified   | Replace one-shot delegated coding with iterative specialized sessions.                 |
| `clients/agent-runtime/src/security/policy.rs` | Modified   | Preserve risk classification and approval behavior for code-session actions.           |
| `clients/agent-runtime/src/approval/mod.rs`    | Modified   | Ensure delegated code sessions respect existing approval semantics.                    |
| `clients/agent-runtime/src/observability/*`    | Modified   | Add dedicated code-session result and audit metadata.                                  |
| `openspec/specs/agent-loop/spec.md`            | Referenced | Proposal/spec work must align to canonical loop behavior.                              |
| `openspec/specs/mcp-runtime/spec.md`           | Referenced | Proposal/spec work must preserve current MCP fail-closed behavior.                     |

## Impact Areas

- `agent-runtime` CLI/runtime UX and profile selection.
- Specialized prompt construction and final-result formatting.
- Delegate execution model and recursion/session budgeting.
- Config schema and validation for code-session settings.
- Security, approval, and observability for coding actions.

## Non-Goals

- Replacing the general agent with a code-only runtime.
- Solving every entry-point inconsistency across CLI, channels, and gateway in this change.
- Introducing unrestricted filesystem or shell access outside current workspace protections.
- Shipping advanced task presets, skills orchestration, or remote CI execution in MVP.

## Dependencies

- Existing `code` profile and `Agent::code_from_config()` specialization seam.
- Canonical dispatcher loop and current approval/security pipeline.
- Existing MCP runtime integration and namespaced tool registration.
- Current tool contracts for file, shell, and git operations.

## Risks

| Risk                                                             | Likelihood | Mitigation                                                                                      |
|------------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------|
| Code mode becomes cosmetic instead of operationally distinct     | Medium     | Require explicit workflow/output contract and validation reporting in MVP.                      |
| Delegated sessions bypass or weaken approval semantics           | Medium     | Reuse the canonical dispatcher, policy, and approval flow rather than custom execution.         |
| Delegate changes become too broad for MVP                        | Medium     | Sequence direct code mode first, then bounded delegated sessions with a narrow result envelope. |
| Validation behavior becomes brittle or prompt-dependent          | Medium     | Model checks declaratively in config/schema and keep MVP defaults minimal.                      |
| Entry-point expectations drift beyond current gateway capability | High       | Scope MVP to CLI/runtime first and document gateway parity as follow-up work.                   |

## Rollback Plan

If implementation proves unstable, revert to the current generic CLI entry and one-shot delegate
behavior while retaining any non-invasive config/schema additions behind disabled defaults. Because
this proposal builds on existing runtime seams instead of a new runtime path, rollback is limited
to removing the explicit code-mode entry, delegated session branch, and related observability fields
without undoing core agent/bootstrap behavior.

## Success Criteria

- [ ] Corvus exposes an official code-mode entry that runs through the existing runtime stack.
- [ ] Code mode produces a structured final result that reports status, changed files, commands,
  validations, and blockers.
- [ ] Delegated coding work can execute as a bounded specialized session rather than a one-shot
  provider call.
- [ ] Code sessions preserve current workspace-only, approval, and MCP fail-closed behavior.
- [ ] MVP changes remain incremental and do not introduce a parallel runtime architecture.
