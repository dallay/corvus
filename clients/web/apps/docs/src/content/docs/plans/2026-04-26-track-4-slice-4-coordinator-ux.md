---
title: Track 4 Slice 4 Coordinator UX Implementation Plan
description: Implementation plan for Track 4 Slice 4 orchestration lifecycle visibility and coordinator UX improvements.
owner: team-platform
status: draft
lastReviewed: 2026-04-26
appliesTo: agent-runtime multi-agent orchestration
docType: architecture
---

# Track 4 Slice 4 Coordinator UX Implementation Plan

> **For agentic workers:** Implement this plan task-by-task using the `dispatching-parallel-agents`
> skill for independent tasks, or execute inline with review checkpoints.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose richer parent-visible blocked and approval-needed orchestration state without changing transport scope.

**Architecture:** Build directly on the existing durable local orchestration contract in `clients/agent-runtime/src/agent/coordinator.rs` and the lifecycle tool surfaces in `delegate_launch`, `delegate_inspect`, `delegate_cancel`, and `delegate`. This slice must remain local-only and parent-owned: it improves read models, status summaries, and rejection/blocking semantics, but it must not introduce remote bridge execution, child-owned approvals, or new transport behaviors.

**Tech Stack:** Rust, Tokio, Serde/serde_json, existing agent-runtime tool framework, current Track 4 coordinator/mailbox architecture, inline unit tests, cargo fmt/clippy/test.

---

## File Structure

### Existing files to modify

- `clients/agent-runtime/src/agent/coordinator.rs`
  - Extend orchestration snapshot/read-model types and lifecycle summarization logic.
  - Add explicit blocked/approval-needed/waiting state derivation from live parent-owned orchestration state.
  - Central place for deterministic aggregation logic and blocking reason normalization.
- `clients/agent-runtime/src/tools/delegate_inspect.rs`
  - Surface richer snapshot payloads without breaking the tool contract shape.
  - Ensure inspection output reflects coordinator summary state and child-level blocking/next-action hints.
- `clients/agent-runtime/src/tools/delegate_launch.rs`
  - Tighten launch-time rejection taxonomy for unsupported local requests that should surface as immediate validation failures instead of ambiguous runtime outcomes.
- `clients/agent-runtime/src/tools/delegate_cancel.rs`
  - Confirm cancellation behavior remains coherent for blocked or approval-needed runs; adjust structured output if new lifecycle states require explicit cancel reporting.
- `clients/agent-runtime/src/tools/delegate.rs`
  - Keep the backward-compatible single-child session path aligned with enriched orchestration state/output.
- `openspec/changes/2026-04-23-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md`
  - Use as the implementation source-of-truth for this slice.

### Existing files to read but likely not modify

- `clients/agent-runtime/src/agent/mailbox.rs`
  - Verify no transport behavior changes are needed; this slice should remain read-model focused.
- `openspec/specs/multi-agent-orchestration/spec.md`
  - Confirm compatibility with the base Track 4 contract.

### No new runtime modules unless proven necessary

Prefer extending existing coordinator/read-model structures in place. Only add a new helper module if `coordinator.rs` becomes unreasonably hard to review.

---

## Task 1: Add explicit coordinator summary and child blocking state model

**Files:**
- Modify: `clients/agent-runtime/src/agent/coordinator.rs`
- Test: inline unit tests in `coordinator.rs`

- [ ] **Step 1: Read the current snapshot/read-model types and identify the smallest extension point**

Find the existing orchestration snapshot, child record, lifecycle state, and any inspection-facing serialization structs in `coordinator.rs`.

Document in comments or scratch notes:

```rust
// Existing parent-visible model:
// - orchestration handle
// - coordinator state
// - child states
// - requested/enforced execution metadata
// Needed additions for Slice 4:
// - coordinator summary state
// - child blocking/approval-needed classification
// - blocking reason / next action hint
```

Expected result: a clear list of exact structs/enums to extend before changing behavior.

- [ ] **Step 2: Add an explicit operator-facing coordinator summary enum**

Introduce an enum with states aligned to the slice spec, for example:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinatorSummaryState {
    Running,
    Blocked,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}
```

Requirements:
- keep this distinct from the internal coordinator state machine if internal states are more granular
- expose it only as derived/operator-facing state
- do not break existing internal transition invariants

- [ ] **Step 3: Add child-visible blocking/approval-needed lifecycle classification**

Extend the child inspection model with explicit parent-readable fields, for example:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildProgressState {
    Running,
    Waiting,
    ApprovalNeeded,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildBlockingDetails {
    pub reason_code: String,
    pub message: String,
    pub parent_action_required: bool,
    pub next_action_hint: Option<String>,
}
```

Do not invent child-owned approval authority. `ApprovalNeeded` must mean “the parent/runtime must decide,” not “the child may proceed on its own.”

- [ ] **Step 4: Implement deterministic summary derivation logic**

Add pure functions in `coordinator.rs` that compute:

```rust
fn derive_coordinator_summary(...) -> CoordinatorSummaryState
fn derive_child_progress_state(...) -> ChildProgressState
fn derive_blocking_details(...) -> Option<ChildBlockingDetails>
```

Rules to enforce:
- terminal results win over live summaries
- cancelling maps to `Cancelling`
- a live approval-needed/blocking child with no forward progress maps aggregate summary to `Blocked`
- active children with no surfaced blocking condition map aggregate summary to `Running`
- repeated derivation over unchanged state must be stable/deterministic

- [ ] **Step 5: Add unit tests for summary derivation before wiring tool output**

Create tests that cover at least:

```rust
#[test]
fn summary_is_blocked_when_child_needs_parent_action() {}

#[test]
fn summary_stays_running_when_children_can_still_progress() {}

#[test]
fn terminal_failure_is_not_reported_as_blocked() {}

#[test]
fn approval_needed_child_reports_parent_owned_blocking_details() {}
```

Expected result: these tests fail before implementation and pass after derivation logic is complete.

---

## Task 2: Surface the richer read model through delegate inspection

**Files:**
- Modify: `clients/agent-runtime/src/tools/delegate_inspect.rs`
- Modify: `clients/agent-runtime/src/agent/coordinator.rs`
- Test: inline unit tests in `delegate_inspect.rs` and/or `coordinator.rs`

- [ ] **Step 1: Inspect current `delegate_inspect` structured output contract**

Find how the tool currently serializes snapshots into `ToolResult.structured` and `output`.

Capture the existing shape in scratch notes, then define the smallest additive change, for example:

```json
{
  "snapshot": {
    "handle": "...",
    "state": "supervising",
    "summary_state": "blocked",
    "children": [
      {
        "child_id": "researcher",
        "progress_state": "approval_needed",
        "blocking_details": {
          "reason_code": "permission_escalation_unsupported",
          "message": "Child requires parent approval boundary decision",
          "parent_action_required": true,
          "next_action_hint": "Cancel or relaunch with supported constraints"
        }
      }
    ]
  }
}
```

- [ ] **Step 2: Extend serialization without breaking callers that read older fields**

Implementation constraints:
- keep existing keys present if they already exist
- add `summary_state`, `progress_state`, and `blocking_details` additively
- do not rename or remove fields in this slice

- [ ] **Step 3: Ensure inspection narrative is deterministic and parent-readable**

If the tool generates text output alongside structured output, add concise deterministic language such as:

```text
Run <handle> is blocked: 1 child is awaiting parent action.
- child researcher: approval_needed (permission_escalation_unsupported)
```

Do not dump mailbox internals or ephemeral transport-only details.

- [ ] **Step 4: Add inspection tests covering enriched output**

Write tests for cases like:

```rust
#[tokio::test]
async fn inspect_includes_summary_state_and_child_progress_state() {}

#[tokio::test]
async fn inspect_reports_blocking_details_for_parent_action_case() {}
```

Expected result: inspection results include the new fields and preserve old shape compatibility.

---

## Task 3: Tighten launch rejection taxonomy for unsupported local requests

**Files:**
- Modify: `clients/agent-runtime/src/tools/delegate_launch.rs`
- Modify: `clients/agent-runtime/src/agent/coordinator.rs`
- Test: inline unit tests in `delegate_launch.rs`

- [ ] **Step 1: Audit current validation paths for ambiguous local request failures**

Find where `delegate_launch` and/or coordinator request normalization rejects unsupported:
- escalation/approval patterns
- stronger local isolation than can be enforced today
- `remote_bridge`

List any cases where the result is too generic, e.g. plain validation failure with no machine-readable reason.

- [ ] **Step 2: Introduce stable reason codes for unsupported local launch requests**

Prefer a small, explicit set of reason codes, for example:

```rust
pub const REASON_REMOTE_BRIDGE_DEFERRED: &str = "remote_bridge_deferred";
pub const REASON_PERMISSION_ESCALATION_UNSUPPORTED: &str = "permission_escalation_unsupported";
pub const REASON_LOCAL_ISOLATION_UNENFORCEABLE: &str = "local_isolation_unenforceable";
```

These may live in `delegate_launch.rs` or in a shared coordinator-adjacent location if reused by inspect state.

- [ ] **Step 3: Return richer structured validation errors from `delegate_launch`**

Update validation failure results so callers get machine-readable data, for example:

```json
{
  "error_code": "permission_escalation_unsupported",
  "message": "Requested child execution requires unsupported delegated approval authority",
  "retryable": false
}
```

Requirements:
- do not change success payload shape in this step
- keep human-readable `error` text useful
- use the same reason codes later in inspect/blocking state where appropriate

- [ ] **Step 4: Add validation tests for reason-code coverage**

Write tests covering at least:

```rust
#[tokio::test]
async fn launch_rejects_remote_bridge_with_stable_reason_code() {}

#[tokio::test]
async fn launch_rejects_unsupported_escalation_with_stable_reason_code() {}
```

Expected result: unsupported launch paths are now explicit and machine-readable.

---

## Task 4: Keep cancel and compatibility paths coherent

**Files:**
- Modify: `clients/agent-runtime/src/tools/delegate_cancel.rs`
- Modify: `clients/agent-runtime/src/tools/delegate.rs`
- Test: inline unit tests as needed

- [ ] **Step 1: Verify cancellation of blocked/approval-needed runs uses existing authority correctly**

Review the cancel path and confirm:
- parent can still cancel blocked runs
- cancel does not require the child to “acknowledge approval-needed” first
- terminal blocked/failure states are not misreported as cancel-success when already terminal

If gaps exist, adjust only the minimum needed logic.

- [ ] **Step 2: Ensure single-child compatibility output still makes sense**

For the existing `delegate` session path, verify whether the richer orchestration state is surfaced directly or ignored.

Rule:
- backward-compatible existing behavior must remain valid
- if new structured state appears, it must be additive only

- [ ] **Step 3: Add targeted tests if behavior changed**

Examples:

```rust
#[tokio::test]
async fn cancel_succeeds_for_blocked_live_run() {}

#[tokio::test]
async fn cancel_reports_terminal_run_without_false_success_transition() {}
```

Only add tests where code changed.

---

## Task 5: Run verification and update implementation evidence

**Files:**
- Modify: none required unless tests reveal issues
- Test: cargo fmt, clippy, targeted tests

- [ ] **Step 1: Run targeted tests for Track 4 lifecycle tools and coordinator logic**

Run:

```bash
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_inspect
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_launch
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" delegate_cancel
cargo test --manifest-path "clients/agent-runtime/Cargo.toml" coordinator
```

Expected result: all relevant tests pass.

- [ ] **Step 2: Run formatting and lint verification**

Run:

```bash
cargo fmt --manifest-path "clients/agent-runtime/Cargo.toml" --all -- --check
cargo clippy --manifest-path "clients/agent-runtime/Cargo.toml" --all-targets -- -D warnings
```

Expected result: no formatting drift, no lint regressions.

- [ ] **Step 3: Verify implementation against the slice spec scenarios**

Use this checklist against `openspec/changes/2026-04-23-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md`:

```markdown
- Parent-visible coordinator lifecycle summary exists.
- Child progress state distinguishes approval-needed/blocked where required.
- Blocking details are parent-owned and actionable.
- Launch rejection taxonomy is stable and machine-readable.
- No transport scope widened.
```

- [ ] **Step 4: Prepare a clean commit**

Run:

```bash
git add clients/agent-runtime/src/agent/coordinator.rs clients/agent-runtime/src/tools/delegate_inspect.rs clients/agent-runtime/src/tools/delegate_launch.rs clients/agent-runtime/src/tools/delegate_cancel.rs clients/agent-runtime/src/tools/delegate.rs
git commit -m "feat: expose richer track 4 orchestration lifecycle state"
```

Expected result: one focused implementation commit for Slice 4.

---

## Self-Review

### Spec coverage

This plan maps directly to the Slice 4 spec:
- coordinator summary state
- child blocked/approval-needed visibility
- deterministic parent-readable inspection
- launch rejection taxonomy alignment
- cancel/compatibility coherence

### Risk notes

- Biggest risk: mixing internal state machine semantics with operator-facing summary semantics. Keep them separate.
- Biggest compatibility constraint: `delegate_inspect` output changes must be additive.
- Biggest scope trap: accidentally drifting into real approval workflow or transport changes. Do not.

### Verification standard

Do not claim completion until:
- targeted tests pass
- fmt/clippy pass
- each spec scenario is traceably covered by code/tests

---

Plan complete and saved to `docs/plans/2026-04-26-track-4-slice-4-coordinator-ux.md`.
