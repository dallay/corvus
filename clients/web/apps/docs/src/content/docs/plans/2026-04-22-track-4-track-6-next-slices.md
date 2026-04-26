---
title: Track 4 and Track 6 Remaining Slices Implementation Plan
description: Planning document for the remaining Track 4 and Track 6 implementation slices after the local orchestration contract landed.
owner: team-platform
status: draft
lastReviewed: 2026-04-26
appliesTo: multi-agent orchestration and bridge remote sessions
docType: architecture
---

# Track 4 and Track 6 Remaining Slices Implementation Plan

> **For agentic workers:** Implement this plan task-by-task using the `dispatching-parallel-agents`
> skill for independent tasks, or execute inline with review checkpoints.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Define concrete next implementation slices for the remaining Multi-Agent Orchestration and Bridge & Remote Sessions gaps after the durable local orchestration contract landed.

**Architecture:** Build on the existing runtime-owned orchestration contract in `clients/agent-runtime/src/agent/coordinator.rs` and the metadata-only bridge seam in `clients/agent-runtime/src/bridge/mod.rs`. The next work should stay split into small reviewable slices: first enrich parent-visible orchestration state and operator UX, then add enforceable local isolation contracts, then introduce a real bridge protocol/auth/transport stack that reuses the same lifecycle and metadata model instead of inventing a second orchestration path.

**Tech Stack:** Rust, Tokio, Serde, existing Corvus agent-runtime tool framework, SQLite mailbox store, existing bridge/session module surfaces, OpenSpec docs, GitHub issue roadmap artifacts.

---

## File Structure

### Existing files to modify

- `openspec/specs/multi-agent-orchestration/spec.md`
  - Source-of-truth for Track 4 behavioral requirements. Add only the next remaining slices, not already-implemented local contract behavior.
- `clients/agent-runtime/src/agent/coordinator.rs`
  - Owns orchestration state, lifecycle inspection model, launch contract validation, and enforced execution guarantees. Future Track 4 slices should extend this first.
- `clients/agent-runtime/src/agent/mailbox.rs`
  - Owns local mailbox transport and endpoint delivery semantics. Future work should reuse this only for local lifecycle/control traffic, not silently expand it into Track 6 transport.
- `clients/agent-runtime/src/tools/delegate_launch.rs`
  - Public launch contract for multi-child orchestration. Future slices should tighten validation and expose richer requested-vs-enforced state here.
- `clients/agent-runtime/src/tools/delegate_inspect.rs`
  - Parent-facing orchestration inspection tool. This is the natural place for coordinator UX/state additions before any terminal UI work.
- `clients/agent-runtime/src/tools/delegate_cancel.rs`
  - Parent-facing cancellation contract. Needed if new states such as approval-needed or isolation-blocked become cancellable lifecycle branches.
- `clients/agent-runtime/src/tools/delegate.rs`
  - Compatibility path for the single-child delegate session flow. Must remain backward compatible while new orchestration features land.
- `clients/agent-runtime/src/bridge/mod.rs`
  - Current metadata-only seam for remote bridge transport. Track 6 should grow from here without claiming unsupported production behavior prematurely.
- `clients/agent-runtime/src/tools/mod.rs`
  - Registry wiring for delegate lifecycle tools and any new bridge-facing tools.
- `tmp/CLAUDIO_ROADMAP.md`
  - Must stay aligned with delivered slices and remaining gaps.

### New files likely to create

- `openspec/changes/<date>-track-4-slice-4-coordinator-ux/proposal.md`
  - Proposal for parent-visible orchestration UX and approval-needed inspection semantics.
- `openspec/changes/<date>-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md`
  - Delta spec for the coordinator UX slice.
- `openspec/changes/<date>-track-4-slice-5-local-isolation-contract/proposal.md`
  - Proposal for enforceable local isolation semantics.
- `openspec/changes/<date>-track-4-slice-5-local-isolation-contract/specs/multi-agent-orchestration/spec.md`
  - Delta spec for stronger isolation guarantees.
- `openspec/changes/<date>-track-6-slice-1-bridge-contract-auth/proposal.md`
  - Proposal for the first real remote bridge slice.
- `openspec/changes/<date>-track-6-slice-1-bridge-contract-auth/specs/<new-bridge-domain>/spec.md`
  - New source-of-truth spec for bridge/session behavior if the team decides Track 6 now warrants its own domain.
- `docs/plans/<follow-up execution plans>.md`
  - Optional downstream execution plans per slice if the spec work is approved.

### Boundaries to preserve

- Do **not** move Track 4 source-of-truth into `gateway`; use `gateway` only as precedent for how a mature source-of-truth spec is structured.
- Do **not** overload `mailbox.rs` into a remote bridge implementation.
- Do **not** make `remote_bridge` silently fall back to `in_process` or `mailbox`.
- Do **not** claim repository/worktree/sandbox isolation as enforced until coordinator launch validation and runtime execution actually guarantee it.

---

### Task 1: Author the Track 4 remaining-scope decomposition

**Files:**
- Modify: `openspec/specs/multi-agent-orchestration/spec.md`
- Modify: `tmp/CLAUDIO_ROADMAP.md`
- Create: `openspec/changes/<date>-track-4-slice-4-coordinator-ux/proposal.md`
- Create: `openspec/changes/<date>-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md`
- Create: `openspec/changes/<date>-track-4-slice-5-local-isolation-contract/proposal.md`
- Create: `openspec/changes/<date>-track-4-slice-5-local-isolation-contract/specs/multi-agent-orchestration/spec.md`
- Test: spec review via markdown diff and requirement traceability checklist

- [ ] **Step 1: Re-read the current Track 4 source-of-truth and list already-delivered vs remaining requirements**

Read and annotate these sections in `openspec/specs/multi-agent-orchestration/spec.md`:

```text
Requirement: Durable Local Orchestration Contract Surface
Requirement: Child Lifecycle State Contract
Requirement: Mailbox Event Visibility and Ordering
Requirement: Parent-Owned Approval Propagation and Permission Broker
Requirement: Execution Metadata and Isolation Contract Boundaries
Requirement: Fail-Closed Remote Bridge Seam
Requirement: Explicit Non-Goals and Deferred Concerns
```

Create a scratch checklist like:

```markdown
Delivered now:
- durable handle launch/inspect/cancel
- mailbox-backed local delivery
- fail-closed remote_bridge validation
- requested vs enforced execution metadata

Still remaining for Track 4:
- richer parent-visible orchestration UX/state
- actionable approval-needed lifecycle exposure
- enforceable local isolation guarantees
- clearer child blocking states for unsupported isolation/escalation paths
```

- [ ] **Step 2: Draft the Track 4 Slice 4 proposal for coordinator UX/state**

Write `openspec/changes/<date>-track-4-slice-4-coordinator-ux/proposal.md` with content shaped like:

```markdown
# Proposal: Track 4 Slice 4 Coordinator UX and Parent-Visible State

## Intent
Turn the existing durable local orchestration contract into a more usable parent-facing surface by
making blocked/approval-needed/isolation-rejected states explicit in inspection and lifecycle tools,
without changing local transport scope.

## In Scope
- richer inspection read model for children and orchestration runs
- explicit approval-needed / blocked status reporting owned by parent authority
- clearer launch/inspection error taxonomy for unsupported execution requests
- delegate lifecycle tool output alignment

## Out of Scope
- remote bridge transport
- repository/worktree cloning
- independent child approval authority
- terminal UI work
```

- [ ] **Step 3: Draft the Track 4 Slice 4 spec delta with concrete requirements**

Write `openspec/changes/<date>-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md` with concrete requirement headings such as:

```markdown
# Delta for Multi-Agent Orchestration

## ADDED Requirements

### Requirement: Parent-Visible Blocked and Approval-Needed Lifecycle States
...

### Requirement: Delegate Lifecycle Tool UX Alignment
...

### Requirement: Launch Rejection Taxonomy for Unsupported Local Requests
...
```

Include scenarios like:

```markdown
#### Scenario: Inspection reports approval-needed child without granting authority
- GIVEN a child request needs unsupported escalation
- WHEN the parent inspects the orchestration run
- THEN the child lifecycle view MUST indicate approval-needed or blocked status
- AND the child MUST NOT be treated as independently authorized to continue.
```

- [ ] **Step 4: Draft the Track 4 Slice 5 proposal for enforceable local isolation**

Write `openspec/changes/<date>-track-4-slice-5-local-isolation-contract/proposal.md` with a minimal scope:

```markdown
# Proposal: Track 4 Slice 5 Local Isolation Contract

## Intent
Upgrade execution metadata from descriptive-only fields into a small set of actually enforceable
local guarantees, starting with the lowest-risk local boundaries.

## In Scope
- define which local isolation knobs are enforceable now
- reject stronger guarantees that cannot be enforced
- expose enforced isolation state through inspection

## Out of Scope
- repository-per-agent cloning
- remote isolation
- bridge transport
```

- [ ] **Step 5: Draft the Track 4 Slice 5 spec delta with enforceable guarantees**

Write `openspec/changes/<date>-track-4-slice-5-local-isolation-contract/specs/multi-agent-orchestration/spec.md` with requirements like:

```markdown
### Requirement: Enforced Local Working Directory Boundary
### Requirement: Read-Only Project Access Enforcement Contract
### Requirement: Inspection Differentiates Enforced Local Isolation from Deferred Isolation
```

Include a scenario like:

```markdown
#### Scenario: Unsupported repository-per-agent request still fails closed
- GIVEN a child launch requests repository-per-agent isolation
- WHEN the runtime evaluates the request
- THEN the launch MUST be rejected
- AND inspection/history MUST NOT imply a downgraded local execution path was accepted.
```

- [ ] **Step 6: Update the canonical Track 4 spec summary text only after the slice deltas exist**

Modify the purpose/intro text in `openspec/specs/multi-agent-orchestration/spec.md` only if needed to reflect the new next-slice decomposition, for example:

```markdown
This specification defines the Track 4 runtime contract for local multi-agent orchestration in
Corvus and records the delivered durable local contract plus the remaining parent-facing UX,
approval visibility, and enforceable local isolation work.
```

Do **not** duplicate the full delta contents into the base spec until the repo’s OpenSpec workflow expects archival/merge.

- [ ] **Step 7: Update roadmap wording to point at the new concrete Track 4 slices**

Modify the Track 4 section of `tmp/CLAUDIO_ROADMAP.md` so the “Main Gaps vs Claude Code” bullets become concrete execution slices, for example:

```markdown
- Slice 4: parent-visible coordinator UX, blocked/approval-needed lifecycle views, tool output alignment
- Slice 5: enforceable local isolation guarantees and requested-vs-enforced validation tightening
- later: remote child execution and broader approval-broker flows
```

- [ ] **Step 8: Review the spec files for overlap and contradiction**

Manual checklist:

```markdown
- Track 4 Slice 4 must not promise isolation enforcement.
- Track 4 Slice 5 must not promise remote bridge transport.
- Base Track 4 spec must still describe current truth, not future wishful behavior.
- Roadmap must point to slices, not vague gaps.
```

Expected result: no contradictory claims between base spec, deltas, and roadmap.

- [ ] **Step 9: Commit the Track 4 planning slice**

Run:

```bash
git add openspec/specs/multi-agent-orchestration/spec.md tmp/CLAUDIO_ROADMAP.md openspec/changes/<date>-track-4-slice-4-coordinator-ux openspec/changes/<date>-track-4-slice-5-local-isolation-contract
git commit -m "docs: define remaining track 4 orchestration slices"
```

Expected: one docs/spec commit containing only Track 4 decomposition work.

---

### Task 2: Author the Track 6 bridge source-of-truth and first executable slice

**Files:**
- Create: `openspec/changes/<date>-track-6-slice-1-bridge-contract-auth/proposal.md`
- Create: `openspec/changes/<date>-track-6-slice-1-bridge-contract-auth/specs/bridge-remote-sessions/spec.md`
- Modify: `clients/agent-runtime/src/bridge/mod.rs`
- Modify: `clients/agent-runtime/src/tools/delegate_launch.rs`
- Modify: `tmp/CLAUDIO_ROADMAP.md`
- Test: spec review + compile-time contract review checklist

- [ ] **Step 1: Decide and document the Track 6 spec domain boundary**

Before writing the spec, capture this decision in the proposal:

```markdown
Track 6 should graduate from a seam-only dependency of `multi-agent-orchestration` into its own
source-of-truth domain because bridge auth, transport, reconnect semantics, and remote session
isolation are a separate product surface with their own lifecycle and threat model.
```

If the team rejects a new domain, document why and keep the bridge work as a delta under the Track 4 spec instead.

- [ ] **Step 2: Draft the Track 6 Slice 1 proposal**

Write `openspec/changes/<date>-track-6-slice-1-bridge-contract-auth/proposal.md` with content like:

```markdown
# Proposal: Track 6 Slice 1 Bridge Contract and Authentication

## Intent
Turn the current metadata-only `remote_bridge` seam into a real but still narrow remote-session
contract: authenticated bridge admission, session-scoped transport negotiation, and explicit
rejection/availability behavior before streaming child execution lands.

## In Scope
- bridge protocol contract
- JWT-backed bridge authentication contract
- session admission / rejection semantics
- shared lifecycle metadata reuse with delegate orchestration

## Out of Scope
- full child execution streaming
- reconnect/resume
- historical replay
- terminal UI integration
```

- [ ] **Step 3: Create the first bridge source-of-truth spec**

Write `openspec/changes/<date>-track-6-slice-1-bridge-contract-auth/specs/bridge-remote-sessions/spec.md` with a full header and requirements such as:

```markdown
# Bridge Remote Sessions Specification

## Purpose
Define the Track 6 remote bridge contract for authenticated remote session admission and transport
negotiation that reuses the existing orchestration lifecycle model without claiming full streaming
execution yet.

## Requirements
### Requirement: JWT-Authenticated Bridge Session Admission
### Requirement: Session-Scoped Remote Bridge Negotiation
### Requirement: Fail-Closed Transport Availability Reporting
### Requirement: Shared Orchestration Metadata Shape Across Local and Remote Requests
```

- [ ] **Step 4: Write concrete acceptance scenarios for bridge auth and negotiation**

Include exact scenarios like:

```markdown
#### Scenario: Missing bridge token is rejected
- GIVEN a remote bridge request omits authentication credentials
- WHEN the bridge admission endpoint evaluates the request
- THEN the request MUST be rejected
- AND the system MUST NOT create a remote session scope.

#### Scenario: Unsupported websocket bridge request reports explicit availability
- GIVEN a remote bridge request asks for `websocket`
- WHEN the runtime has not enabled websocket transport
- THEN the result MUST report the transport as unavailable or deferred
- AND the system MUST NOT silently downgrade to SSE or local mailbox delivery.
```

- [ ] **Step 5: Align the code seam inventory with the new spec**

Inspect and annotate the current fields in `clients/agent-runtime/src/bridge/mod.rs` and `clients/agent-runtime/src/tools/delegate_launch.rs`:

```rust
pub struct RemoteBridgeRequest {
    pub protocol_version: BridgeProtocolVersion,
    pub transport: BridgeTransportKind,
    pub session_scope: String,
}
```

Document in the proposal/spec what is missing:

```markdown
Missing contract fields to consider in later implementation:
- auth material reference or verified claims context
- session admission result shape
- negotiated capabilities
- reconnect policy
- bridge endpoint identity
```

- [ ] **Step 6: Tighten roadmap language so Track 6 becomes executable**

Modify `tmp/CLAUDIO_ROADMAP.md` Track 6 section to replace vague bullets with slice language, for example:

```markdown
- Slice 1: bridge contract + JWT auth + session admission/rejection
- Slice 2: SSE transport for remote session event delivery
- Slice 3: WebSocket transport and bidirectional control plane
- Slice 4: reconnect/resume and remote session recovery semantics
```

- [ ] **Step 7: Review that Track 6 claims do not outrun current code**

Manual review checklist:

```markdown
- bridge/mod.rs currently remains metadata-only and must stay described that way until implementation lands
- delegate_launch must continue fail-closed behavior for remote_bridge until Track 6 code exists
- roadmap can point to JWT/SSE/WebSocket slices, but current-state notes must still say incomplete
```

Expected result: spec is actionable without falsely claiming delivery.

- [ ] **Step 8: Commit the Track 6 planning slice**

Run:

```bash
git add openspec/changes/<date>-track-6-slice-1-bridge-contract-auth tmp/CLAUDIO_ROADMAP.md
git commit -m "docs: define initial track 6 bridge slices"
```

Expected: one docs/spec commit with the Track 6 decomposition only.

---

### Task 3: Prepare the first implementation-ready execution plan for the chosen next slice

**Files:**
- Create: `docs/plans/<date>-track-4-slice-4-coordinator-ux.md` or `docs/plans/<date>-track-6-slice-1-bridge-contract-auth.md`
- Modify: none unless clarifying links are needed in roadmap/spec docs
- Test: plan quality review against spec requirements

- [ ] **Step 1: Choose the very next slice to execute**

Decision rule:

```markdown
If the team wants immediate Claude Code parity improvement in local agent workflows, choose Track 4 Slice 4.
If the team wants to unblock remote architecture first, choose Track 6 Slice 1.
Default recommendation: Track 4 Slice 4 first, because it improves existing shipped capability instead of opening a second runtime surface.
```

- [ ] **Step 2: Map the execution files for that slice**

If Track 4 Slice 4 is chosen, use this file map:

```markdown
- Modify: `clients/agent-runtime/src/agent/coordinator.rs`
- Modify: `clients/agent-runtime/src/tools/delegate_launch.rs`
- Modify: `clients/agent-runtime/src/tools/delegate_inspect.rs`
- Modify: `clients/agent-runtime/src/tools/delegate_cancel.rs`
- Modify: `clients/agent-runtime/src/tools/delegate.rs`
- Test: inline unit tests in those modules plus cargo test filtering for delegate/coordinator paths
```

If Track 6 Slice 1 is chosen, use this file map:

```markdown
- Modify: `clients/agent-runtime/src/bridge/mod.rs`
- Modify: `clients/agent-runtime/src/agent/coordinator.rs`
- Modify: `clients/agent-runtime/src/tools/delegate_launch.rs`
- Modify: `clients/agent-runtime/src/tools/mod.rs`
- Test: inline bridge tests, delegate launch validation tests, cargo test filters for bridge/delegate modules
```

- [ ] **Step 3: Write the implementation plan in the repo plan directory**

Create one plan file in `docs/plans/` using the required header and task-by-task TDD steps. Start with a header like:

```markdown
# Track 4 Slice 4 Coordinator UX Implementation Plan

> **For agentic workers:** Implement this plan task-by-task using the `dispatching-parallel-agents`
> skill for independent tasks, or execute inline with review checkpoints.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose richer parent-visible blocked and approval-needed orchestration state without changing transport scope.
```

- [ ] **Step 4: Include exact test commands for the execution slice**

For Track 4 Slice 4, include commands like:

```bash
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_inspect
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_launch
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_cancel
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" coordinator
cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings
cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check
```

For Track 6 Slice 1, include commands like:

```bash
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" bridge
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_launch
cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings
cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check
```

- [ ] **Step 5: Verify the plan covers every scenario in the chosen slice spec**

Manual coverage checklist:

```markdown
- Each requirement in the chosen slice spec maps to at least one task.
- Each task lists exact file paths.
- Each code step includes concrete code, not placeholders.
- Each test step includes exact commands and expected pass/fail results.
```

Expected result: implementation plan is ready for execution without requiring fresh discovery.

- [ ] **Step 6: Commit the execution plan**

Run:

```bash
git add docs/plans/<date>-<chosen-slice>.md
git commit -m "docs: add execution plan for next claudio slice"
```

Expected: one isolated planning commit.

---

## Self-Review

### Spec coverage

This plan covers:
- decomposition of the remaining Track 4 scope into concrete slices
- creation of a dedicated Track 6 source-of-truth and first auth/contract slice
- preparation of a follow-up implementation plan for the chosen next slice

### Placeholder scan

Checked for forbidden placeholders:
- no `TODO`
- no `implement later`
- no vague “write tests” without commands
- all file paths are explicit, with `<date>` placeholders only where the repo naming convention requires the actual current date slug at creation time

### Type consistency

Names align with current code and spec vocabulary:
- `remote_bridge`
- `delegate_launch`
- `delegate_inspect`
- `delegate_cancel`
- `SupervisedOrchestrationService`
- `ChildExecutionSpec`
- `NormalizedExecutionRequest`
- `EnforcedExecutionGuarantees`

---

Plan complete and saved to `docs/plans/2026-04-22-track-4-track-6-next-slices.md`. Two execution options:

**1. Parallel Agents (recommended for independent tasks)** — use `dispatching-parallel-agents` skill, dispatch fresh agent per independent task, review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session step by step with checkpoints for review.

**Which approach?**
