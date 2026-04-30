# Delta for Multi-Agent Orchestration

## ADDED Requirements

### Requirement: Enforceable Local Execution Isolation Contract

For any child execution request the runtime accepts under delivered local Track 4 transports, the
runtime MUST enforce a concrete local execution isolation contract rather than treating repository,
worktree, and access constraints as advisory metadata only.

At minimum, when those fields are part of an accepted child contract, the runtime MUST bind the
child to the accepted local repository identity, accepted local worktree identity if one is required
for that mode, and the accepted read-only versus writable project access posture. If the runtime
cannot enforce the accepted local contract for a requested child, it MUST reject the launch rather
than silently admitting the child with weaker guarantees.

#### Scenario: Accepted local child remains bound to enforced repository and worktree scope

- GIVEN a parent launches a child through a delivered local transport with an accepted repository and
  worktree contract
- WHEN the runtime admits that child into the orchestration run
- THEN the runtime MUST enforce that the child executes within that accepted local repository and
  worktree scope
- AND the system MUST NOT silently allow the child to execute against a different repository or
  worktree context.

#### Scenario: Launch is rejected when local isolation cannot be enforced

- GIVEN a child launch request asks for a local repository, worktree, or access posture that the
  runtime cannot actually enforce in the current live context
- WHEN the runtime evaluates the request
- THEN the runtime MUST reject the launch
- AND the system MUST NOT silently continue with weaker or unspecified local isolation.

### Requirement: Requested Versus Enforced Local Isolation Visibility

Inspection for an accepted local child MUST distinguish between the isolation attributes originally
requested by the parent and the local isolation guarantees actually enforced by the runtime. The
system MUST present enforced local isolation as its own authoritative contract state rather than
forcing the parent to infer enforcement from request echoes alone.

If a requested isolation-related attribute was preserved for traceability but not delivered as an
enforced guarantee in this slice, inspection MUST make that distinction explicit and MUST NOT present
that requested attribute as currently enforced behavior.

#### Scenario: Inspection shows enforced local access posture distinctly from request metadata

- GIVEN a parent launches an accepted local child with isolation-related request metadata
- WHEN the parent later inspects that orchestration run
- THEN the inspection result MUST distinguish the child’s requested local isolation attributes from
  the local guarantees actually enforced for that child
- AND the enforced access posture MUST be visible without relying on request payload reconstruction.

#### Scenario: Deferred stronger isolation is not misreported as enforced

- GIVEN a parent requested stronger isolation characteristics that this slice still treats as deferred
  or unsupported
- WHEN the parent inspects the resulting launch rejection or accepted child record
- THEN the inspection or launch outcome MUST clearly indicate whether those characteristics were
  rejected, deferred, or merely requested
- AND the system MUST NOT misreport them as enforced local guarantees.

### Requirement: No Silent Local Isolation Downgrade

The runtime MUST fail closed when a child request asks for a local isolation guarantee that exceeds
what the delivered local Track 4 contract can enforce. The system MUST NOT silently drop requested
repository/worktree/access constraints, silently convert writable access into broader access, or
silently substitute a less isolated local execution mode.

This prohibition applies equally to in-process local children and mailbox-backed local children. A
change in delivery path MUST NOT become a reason to weaken accepted local isolation semantics.

#### Scenario: Mailbox-backed child does not receive weaker isolation by transport choice alone

- GIVEN two child requests ask for the same accepted local isolation contract
- AND one child is launched through `in_process` transport while the other is launched through
  mailbox-backed local transport
- WHEN both launches are accepted
- THEN the runtime MUST preserve the same local isolation semantics for both children where the
  contract says they are delivered
- AND mailbox-backed transport alone MUST NOT justify a weaker local isolation guarantee.

#### Scenario: Unsupported stronger local mode is rejected without fallback

- GIVEN a child request asks for a stronger local isolation mode than this slice delivers
- WHEN the runtime evaluates that request
- THEN the runtime MUST reject the request as unsupported or unenforceable
- AND the system MUST NOT silently fall back to a broader shared local execution context.

### Requirement: Local Isolation Verification and Regression Coverage

The system MUST include targeted verification or regression coverage for the enforceable local
isolation contract added by this slice. At minimum, coverage MUST exercise accepted local binding to
repository/worktree/access constraints where delivered, launch rejection when those guarantees cannot
be enforced, inspection visibility for requested versus enforced isolation, and prevention of silent
downgrade across local transport modes.

The regression suite MUST be specific enough to detect a future change that broadens child execution
scope beyond the accepted local contract, hides the distinction between requested and enforced
isolation, or weakens rejection behavior for unsupported stronger modes.

#### Scenario: Regression suite catches silent isolation downgrade

- GIVEN a code change causes a child request with enforceable local isolation constraints to be
  admitted under weaker effective local scope than the contract allows
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that local isolation downgrade behavior was violated.

#### Scenario: Regression suite catches missing requested-versus-enforced distinction

- GIVEN a code change causes inspection to report requested local isolation metadata as though it were
  enforced runtime state
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that requested-versus-enforced visibility behavior was violated.

### Requirement: Local Isolation Slice Boundaries

This slice MUST remain limited to enforceable local isolation guarantees for delivered Track 4 child
execution. It MUST NOT be treated as delivery of repository cloning, worktree cloning, sandbox
cloning, repository-per-agent execution, remote bridge isolation, reconnect/resume, or durable
authority reconstruction after parent loss.

The runtime MAY enforce a narrower local contract in this slice than the full Claude Code parity
vision, but it MUST state that narrower contract honestly and fail closed for anything stronger.

#### Scenario: Enforced local contract does not imply cloned repository isolation

- GIVEN a parent inspects an accepted local child with enforced repository and access constraints
- WHEN the parent evaluates that result under this slice
- THEN the inspection result MAY claim only the delivered local isolation guarantees
- AND the system MUST NOT imply that cloned repository, cloned worktree, or repository-per-agent
  execution has been delivered.

#### Scenario: Local isolation slice does not imply remote isolation delivery

- GIVEN a parent requests or expects remote bridge isolation behavior for a child
- WHEN the runtime evaluates that expectation under this slice
- THEN the system MUST treat remote isolation as out of scope
- AND the local isolation contract MUST NOT claim that remote bridge execution or recovery behavior is
  delivered.
