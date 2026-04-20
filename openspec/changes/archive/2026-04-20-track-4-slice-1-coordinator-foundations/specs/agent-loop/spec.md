# Delta for Agent Loop

## ADDED Requirements

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
