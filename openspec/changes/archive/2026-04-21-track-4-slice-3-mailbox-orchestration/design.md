# Design: Track 4 Slice 3 — Mailbox-on-Disk Orchestration Messaging

## Technical Approach

This change keeps the existing Slice 2 orchestration contract intact while adding the smallest durable mailbox seam needed for internal coordinator↔child messaging.

The implementation keeps `Coordinator` as the authoritative reducer, keeps `OrchestrationHandle` and `SupervisedOrchestrationService` stable, and adds a dedicated SQLite mailbox module adjacent to `coordinator.rs`. The new mailbox layer owns persistence, leasing, ack, redelivery, and endpoint addressing. The coordinator continues to decide state transitions and aggregate outcomes, while mailbox delivery is treated as a transport detail.

The preferred first implementation remains narrow:

- no restart recovery or reattach,
- no remote bridge,
- no streaming tool payloads,
- no user-facing transport selector,
- process-local inspection/cancel remains authoritative.

## Architecture Decisions

### Decision: Keep lifecycle entry points stable and add mailbox transport under them

**Choice**: Preserve `OrchestrationHandle`, `SupervisedOrchestrationService`, `DelegateTool`, `delegate_launch`, `delegate_inspect`, and `delegate_cancel` contracts. Add mailbox transport behind the existing runner/service seam.

**Alternatives considered**:
- Replace the service API with mailbox-native launch/inspect/cancel contracts.
- Make mailbox rows the primary orchestration state source.

**Rationale**: Slice 2 callers already depend on handle-based lifecycle APIs. Replacing them would widen scope and increase regression risk. The service can continue owning live-process state while delegating transport to a new mailbox layer.

### Decision: Introduce one dedicated SQLite mailbox module under `agent/`

**Choice**: Add a dedicated mailbox persistence/driver module adjacent to coordinator code, centered on a SQLite-backed store plus a small delivery driver.

**Alternatives considered**:
- Embed mailbox logic directly into `coordinator.rs`.
- Reuse memory/session/task SQLite tables.

**Rationale**: The repo has good SQLite patterns, but no reusable lease/ack queue abstraction. A focused mailbox module keeps persistence mechanics isolated and avoids bloating the coordinator state machine or stretching unrelated tables.

### Decision: Use logical endpoints for all internal orchestration mail routing

**Choice**: Address mailbox rows by logical endpoint, not by ad hoc child IDs alone.

**Alternatives considered**:
- Use only `child_id` and infer direction implicitly.
- Use physical process identifiers as recipients.

**Rationale**: Slice 3 needs durable coordinator→child and child→coordinator addressing that survives transport differences without introducing remote bridge semantics. Logical endpoints keep addressing explicit while remaining internal and transport-agnostic.

### Decision: At-least-once delivery with lease/ack/redelivery and coordinator idempotency

**Choice**: The mailbox store leases messages to a consumer, requires explicit ack, and makes expired leases eligible for redelivery. The coordinator becomes idempotent for duplicate logical messages.

**Alternatives considered**:
- At-most-once delivery.
- Exactly-once delivery via distributed transactions.

**Rationale**: At-most-once would lose messages on crash or mid-processing failure. Exactly-once is unnecessary complexity for this slice. At-least-once is the smallest durable behavior that fits SQLite and the current runtime shape, as long as duplicates are treated as safe no-ops.

### Decision: Polling is the correctness path; wakeup is an optimization only

**Choice**: Consumers poll SQLite for eligible rows. Optional in-memory wakeup hints may shorten latency for same-process participants but are never required for correctness.

**Alternatives considered**:
- Depend on a wakeup channel for delivery.
- Add filesystem watchers or a durable signal bus.

**Rationale**: Cross-process correctness must not depend on shared in-memory notification. Polling is deterministic and sufficient. A best-effort wakeup hub can improve latency when parent and child loops share a process, without creating a new correctness dependency.

## Data Flow

### Launch and mailbox-backed child execution

```text
delegate / delegate_launch
        │
        ▼
SupervisedOrchestrationService
        │ creates handle + live registry entry
        ▼
Coordinator::run_with_cancellation(...)
        │
        ├─ admit child / apply local dispatch + started bookkeeping
        │
        └─ MailboxBackedChildRunner::run_child(...)
              │
              ├─ enqueue DispatchChild to child endpoint inbox
              ├─ optionally emit wakeup hint
              ├─ spawn child mailbox loop / delegated execution path
              └─ wait on coordinator endpoint inbox for terminal reply
                        │
                        ▼
                SqliteMailboxStore (poll + lease + ack)
                        │
                        ▼
                terminal envelope returned to coordinator
                        │ resequence + dedupe + apply
                        ▼
                 Coordinator authoritative state
```

### Cancel path

```text
delegate_cancel
    │
    ▼
SupervisedOrchestrationService::cancel(handle)
    │ cancels parent-owned token
    ▼
Coordinator enters Cancelling
    │
    ├─ enqueue CancelChild to each active child endpoint
    ├─ child loop observes cancel via mailbox polling or token
    └─ child emits ChildCancelled (idempotent if repeated)
            │
            ▼
      coordinator applies terminal updates once
```

### Duplicate-delivery handling

```text
leased row redelivered after timeout
          │
          ▼
Coordinator receives same logical message_id again
          │
          ├─ if message_id already applied with same payload digest → no-op + ack
          └─ if sequence regresses with a different message_id/payload → fail closed
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/mailbox.rs` | Create | New SQLite mailbox persistence + driver layer: schema init, enqueue, lease, ack, release/redelivery, logical endpoints, optional wakeup hub, and mailbox-backed child runner helpers. |
| `clients/agent-runtime/src/agent/mod.rs` | Modify | Export the new mailbox module. |
| `clients/agent-runtime/src/agent/coordinator.rs` | Modify | Extend transport metadata for mailbox delivery, add logical endpoint-aware envelope validation, and make inbound application idempotent for at-least-once redelivery while preserving stable outcome ordering. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Build one shared mailbox store/driver next to the shared `SupervisedOrchestrationService`, choose the mailbox-backed runner for supervised delegate execution, and keep lifecycle tools on the same service instance. |
| `clients/agent-runtime/src/tools/delegate.rs` | Modify | Keep the single-child `delegate` session path on `run_to_completion()` while routing its supervised executor through the mailbox-backed runner under the same stable result contract. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Modify | Keep JSON input/output stable while launching runs against the shared mailbox-backed orchestration runner; extend tests for mailbox transport behavior without changing the external schema. |
| `clients/agent-runtime/src/tools/delegate_cancel.rs` | Modify | Preserve handle-based cancel semantics and add regression coverage for mailbox-backed cancellation races. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Modify | Preserve process-local inspection semantics and add regression coverage proving inspection does not fall back to mailbox persistence after parent loss. |
| `clients/agent-runtime/src/config/schema.rs` | No change preferred | Slice 3 should avoid new user-facing transport config. If a knob becomes necessary, keep it internal-only (DB path / poll interval), but the preferred implementation uses code defaults under the workspace state directory. |

## Persistence Schema

The mailbox database should live under the workspace state area, not under long-term memory tables:

- path: `<workspace>/state/orchestration/mailbox.db`
- connection profile: SQLite WAL mode, `synchronous = NORMAL`, bounded `busy_timeout`, short blocking sections via `spawn_blocking` following existing repo patterns.

### Practical schema

```sql
CREATE TABLE IF NOT EXISTS mailbox_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mailbox_messages (
  message_id TEXT PRIMARY KEY,
  coordinator_id TEXT NOT NULL,
  child_id TEXT,
  sender_endpoint TEXT NOT NULL,
  recipient_endpoint TEXT NOT NULL,
  correlation_id TEXT NOT NULL,
  sequence INTEGER NOT NULL,
  transport TEXT NOT NULL CHECK (transport IN ('mailbox')),
  payload_kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  payload_digest TEXT NOT NULL,
  created_at TEXT NOT NULL,
  available_at TEXT NOT NULL,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  lease_owner TEXT,
  lease_expires_at TEXT,
  acked_at TEXT,
  terminal_error TEXT
);

CREATE INDEX IF NOT EXISTS idx_mailbox_poll
  ON mailbox_messages(recipient_endpoint, acked_at, available_at, lease_expires_at, created_at);

CREATE INDEX IF NOT EXISTS idx_mailbox_coordinator
  ON mailbox_messages(coordinator_id, created_at);
```

### Notes

- `message_id` is the stable logical identity used for idempotency across retries.
- `sender_endpoint` / `recipient_endpoint` store serialized logical endpoints.
- `payload_digest` lets the coordinator distinguish a harmless duplicate from a conflicting replay.
- `terminal_error` is the narrow poison-message escape hatch for unrecoverable decode/apply failures; there is no broad dead-letter subsystem in this slice.

## Interfaces / Contracts

### Logical addressing and envelope metadata

```rust
pub enum LogicalEndpoint {
    Coordinator { coordinator_id: String },
    Child {
        coordinator_id: String,
        child_id: ChildAgentId,
    },
}

pub enum CoordinatorTransport {
    InProcess,
    Mailbox,
}

pub struct EnvelopeMeta {
    pub coordinator_id: String,
    pub child_id: Option<ChildAgentId>,
    pub sequence: u64,
    pub message_id: String,
    pub correlation_id: String,
    pub sender: LogicalEndpoint,
    pub recipient: LogicalEndpoint,
    pub sent_at: DateTime<Utc>,
    pub transport: CoordinatorTransport,
}
```

### Mailbox store contract

```rust
pub struct MailboxLease {
    pub message_id: String,
    pub lease_owner: String,
    pub lease_expires_at: DateTime<Utc>,
    pub envelope: MessageEnvelope<CoordinatorMessage>,
}

#[async_trait]
pub trait OrchestrationMailbox: Send + Sync {
    async fn enqueue(
        &self,
        envelope: MessageEnvelope<CoordinatorMessage>,
        recipient: LogicalEndpoint,
    ) -> Result<(), CoordinatorError>;

    async fn lease_next(
        &self,
        recipient: &LogicalEndpoint,
        lease_owner: &str,
        lease_ttl: Duration,
    ) -> Result<Option<MailboxLease>, CoordinatorError>;

    async fn ack(&self, lease: &MailboxLease) -> Result<(), CoordinatorError>;

    async fn release(&self, lease: &MailboxLease) -> Result<(), CoordinatorError>;

    async fn record_terminal_error(
        &self,
        lease: &MailboxLease,
        error: &str,
    ) -> Result<(), CoordinatorError>;
}
```

### Mailbox-backed runner seam

```rust
pub struct MailboxBackedChildRunner {
    mailbox: Arc<dyn OrchestrationMailbox>,
    delegated: Arc<DelegatedAgentRunner>,
    wakeups: Arc<MailboxWakeupHub>,
}
```

`MailboxBackedChildRunner` still implements the existing `CoordinatorChildRunner` trait so `Coordinator::run()` and `SupervisedOrchestrationService` do not need a public API change.

## Lease / Ack / Redelivery Behavior

1. **Enqueue**
   - Parent or child writes a mailbox row with a stable `message_id` and `available_at = now`.
   - Optional wakeup hint is emitted after commit.

2. **Lease**
   - Consumer polls its endpoint.
   - In one transaction, the store selects one eligible row (`acked_at IS NULL`, available, and unleased or expired), sets `lease_owner`, `lease_expires_at`, increments `attempt_count`, and returns the envelope.

3. **Apply**
   - Consumer applies the message.
   - If apply succeeds, consumer `ack`s the lease.
   - If processing fails transiently, consumer releases the lease or lets it expire.

4. **Redelivery**
   - Any unacked row whose lease expires becomes visible again on the next poll.
   - The same `message_id` is preserved across attempts.

5. **Poison rows**
   - If a row cannot be decoded or is semantically conflicting, the store records `terminal_error`, the owning run fails closed, and the row is no longer retried in a tight loop.

### Idempotency strategy

The coordinator needs a duplicate-safe path before monotonic sequence checks.

- Add an internal `applied_messages` map keyed by `child_id + message_id`.
- Store the first-seen `payload_digest` and resulting terminal/non-terminal classification.
- If a duplicate arrives with the same `message_id` and same digest, return `Ok(())` without mutating child state again.
- If a message arrives with `sequence <= last_sequence` **and** it is not a known duplicate, reject it as a conflicting replay and fail closed.
- Terminal child updates remain once-only; repeated `ChildCompleted` / `ChildFailed` / `ChildCancelled` for the same logical message must be idempotent no-ops.

This keeps at-least-once safe without weakening the state machine.

## Backward Compatibility

### `OrchestrationHandle` and `SupervisedOrchestrationService`

- Handle format stays opaque.
- The service registry remains the live process source of truth for inspect/cancel.
- `launch`, `inspect`, `cancel`, and `run_to_completion` signatures remain unchanged.

### `delegate_launch`

- Input JSON stays the same (`children[]` with `child_id`, `agent_name`, `prompt`, optional `context`).
- Output still returns `{ handle, snapshot }`.
- Only the internal runner implementation changes.

### `delegate`

- The current single-child session path still routes through `run_to_completion()`.
- Success/failure output remains the same `ToolResult` shape.
- Callers are not forced to understand mailbox transport.

### `delegate_cancel` and `delegate_inspect`

- Both remain parent-process operations over the in-memory service registry.
- Neither API reads mailbox rows as authoritative orchestration state.
- After parent loss, handles remain unknown for this slice; no restart recovery is implied.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Mailbox schema init, endpoint serialization, enqueue/lease/ack/release semantics, expired lease redelivery, poison-row handling | New tests in `agent/mailbox.rs` using temp SQLite DBs and real `rusqlite` transactions. |
| Unit | Coordinator duplicate delivery/idempotency rules | Extend `coordinator.rs` tests to cover same-`message_id` replay, conflicting replay, duplicate terminal envelopes, and support for both `InProcess` and `Mailbox` transport metadata. |
| Integration | Mailbox-backed child execution still yields deterministic fan-in order | Add coordinator/service tests with a mailbox-backed stub runner where child completion order differs from launch order. |
| Integration | Parent cancellation remains authoritative with mailbox polling | Add tests that race cancel vs leased stale work and assert final child/outcome state is deterministic. |
| Integration | Lifecycle tools remain backward compatible | Extend `delegate.rs`, `delegate_launch.rs`, `delegate_cancel.rs`, and `delegate_inspect.rs` tests to verify unchanged schemas/results while using the mailbox-backed path. |
| E2E | No new external E2E surface required | This slice is runtime-internal; focused Rust integration/regression tests are the primary gate. |

## Migration / Rollout

No migration required.

The mailbox database is created lazily under the workspace state directory when delegate lifecycle tooling is initialized. Existing callers keep the same APIs and only pick up the new transport internally.

## Open Questions

- [ ] None blocking for implementation.
