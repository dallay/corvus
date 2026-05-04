# Technical Design: Dream System — Long-Term Memory Consolidation

> **Change:** 2026-04-26-dream-system
> **Issue:** #526
> **Date:** 2026-04-26
> **Status:** Design

---

## 1. Technical Approach

Implement Dream as a **runtime-owned post-session consolidation pipeline** centered in
`clients/agent-runtime/src/memory/`. The gateway remains a trigger source, sessions remain the
lifecycle source for completion state, and Dream itself becomes the source-of-truth for
eligibility, consolidation, persistence, replay safety, and backend parity.

The design intentionally upgrades the existing seed implementation in `memory/dream.rs` rather than
introducing a new subsystem. The current file-oriented consolidation flow and state file/lock file
mechanics provide the implementation seam, but the behavior shifts from coarse workspace-level
maintenance into **per-completed-session deterministic consolidation**.

**Implementation phases:**

1. **Dream runtime contract** — define per-session eligibility, artifact shape, and Dream state
2. **Session-integrated triggering** — ensure completion recording precedes Dream evaluation
3. **Backend parity** — persist/hydrate/export Dream artifacts and replay state across sqlite,
   markdown, and snapshot flows
4. **Hardening** — locking, idempotency, observability, and end-to-end tests

All changes are additive and rollback-safe. Existing session completion continues to work if Dream is
skipped or disabled.

---

## 2. Architecture Decisions

### ADR-1: Dream belongs to the memory/runtime domain, not gateway

**Decision:** The primary behavioral contract for Dream will live in runtime memory modules and be
specified under the gateway source-of-truth already used for runtime memory behavior, with gateway
changes limited to trigger integration.

**Rationale:**

- Proposal and delta specs require gateway to be trigger-only.
- Current hooks already call `record_session_completion` and `run_dream_if_triggered` from gateway,
  which is sufficient as an integration seam.
- Consolidation semantics, backend persistence, hydration, and replay are memory concerns, not HTTP
  concerns.

**Consequence:**

- Gateway code should not decide whether a session is Dream-eligible.
- Gateway code should not format or persist Dream artifacts directly.
- Runtime modules define ordering and idempotency; gateway only invokes them.

### ADR-2: Dream is keyed by completed session identity

**Decision:** The unit of consolidation is a single recorded completed session, identified by
`session_id`.

**Rationale:**

- The delta specs require Dream to target a specific completed session.
- Per-session identity gives a natural idempotency key and simplifies replay safety.
- It lets the runtime distinguish “completion observed”, “consolidation in progress”, and
  “consolidation completed” for the same session.

**Consequence:**

- `record_session_completion` must evolve from a global counter update into a session-aware write.
- Dream state must include enough information to determine whether a given `session_id` is pending,
  running, completed, or failed.
- Trigger logic can still batch or defer work, but artifact ownership remains session-scoped.

### ADR-3: Consolidation output is distilled memory plus replay metadata

**Decision:** Dream produces two durable outputs:

1. **long-term memory artifacts** containing distilled high-value information, and
2. **Dream replay metadata** describing the consolidation outcome for that completed session.

**Rationale:**

- The gateway spec requires durable distilled memory, not transcript retention.
- The same spec also requires enough persisted state to avoid ambiguous replay.
- Separating memory artifact content from execution metadata keeps retrieval surfaces clean while
  preserving exact replay semantics.

**Consequence:**

- Dream artifacts are stored as normal long-term memory entries/files.
- Replay metadata is stored separately and never treated as user-facing knowledge.
- Snapshot/export flows must preserve both.

### ADR-4: Deterministic ordering is completion-recorded-first, Dream-evaluated-second

**Decision:** The runtime contract is:

1. session completion is recorded in the session system,
2. Dream completion state is recorded or enqueued against that completed session,
3. Dream evaluates eligibility and consolidates afterward.

**Rationale:**

- Required by the sessions and gateway integration deltas.
- Avoids in-flight notions of completion derived only from transport success.
- Makes stale-session auto-close compatible with the same post-completion path.

**Consequence:**

- `end_session` and stale auto-close become valid producers of the same Dream trigger input.
- Dream must refuse sessions that are not present as recorded completed sessions.
- Retry paths can safely re-run because recorded completion is the durable precondition.

### ADR-5: Locking is two-level: global runner lock plus per-session idempotency state

**Decision:** Dream will use:

- a **global runner lock** to serialize filesystem/maintenance-style Dream execution where needed,
  and
- **per-session durable status markers** to prevent duplicate consolidation for individual sessions.

**Rationale:**

- Current `dream.lock` behavior is already useful for coarse serialization.
- A global lock alone is insufficient for replay-safe idempotency.
- A per-session state machine makes repeated gateway callbacks and post-restore retries safe.

**Consequence:**

- “Busy” remains a valid Dream result when another run is already active.
- Retrying after a busy/failed state consults per-session Dream state first.
- Global serialization can be relaxed later without changing the session-level contract.

### ADR-6: Backend parity means preserving artifacts and Dream replay state, not identical storage format

**Decision:** SQLite, markdown, and snapshot support may persist Dream differently, but all supported
backends must preserve:

- durable Dream artifacts,
- mapping from artifact(s) to completed session identity, and
- enough Dream replay state to keep repeat triggers unambiguous after restart/export/hydration.

**Rationale:**

- Specs require behavioral parity, not identical implementation details.
- SQLite naturally supports relational state; markdown requires explicit sidecar/state-file
  conventions; snapshot needs an exportable representation.

**Consequence:**

- We keep one semantic contract across backends.
- We do not force markdown into a relational shape, but we do require explicit persisted replay
  metadata.

---

## 3. Runtime and Data Flow

### 3.1 Primary Dream trigger flow

```text
┌─────────────┐     request/session end     ┌─────────────┐
│ Gateway /   │ ───────────────────────────►│ Sessions /  │
│ Hygiene     │                             │ Memory API  │
└──────┬──────┘                             └──────┬──────┘
       │                                           │
       │ record completed session                  │
       │                                           ▼
       │                                   sessions table / recorded
       │                                   completed session state
       │                                           │
       │ call Dream trigger hook                   │
       ▼                                           ▼
┌─────────────┐                             ┌──────────────┐
│ dream.rs    │ ─ eligibility lookup ────► │ transcript + │
│ coordinator │                             │ session data │
└──────┬──────┘                             └──────┬───────┘
       │                                           │
       │ synthesize distilled memories             │
       ▼                                           │
┌──────────────┐                                   │
│ backend      │ ◄──────── persist artifacts ──────┘
│ persistence  │
└──────┬───────┘
       │ persist replay metadata
       ▼
┌──────────────┐
│ Dream state  │
│ completed    │
└──────────────┘
```

### 3.2 Completion-to-Dream ordering

1. A runtime path ends a session (`end_session`) or the hygiene pass auto-closes a stale session.
2. The runtime persists completed-session state (`ended_at`, `status='ended'`).
3. Dream trigger input is recorded for that `session_id`.
4. Dream checks whether that completed session has already been consolidated.
5. If not consolidated, Dream loads session transcript and relevant contextual memory.
6. Dream distills stable high-value facts/summaries.
7. Dream writes memory artifacts through the active backend.
8. Dream writes replay metadata marking the session as completed for Dream purposes.

### 3.3 Eligibility model

For MVP, Dream eligibility is intentionally simple and deterministic:

- session must exist,
- session must be recorded completed,
- session must not already have a successful Dream completion record,
- required source material for consolidation must be readable,
- optional trigger thresholds may decide **when** to run, but not **whether** the already completed
  session is conceptually a candidate.

This keeps the spec-aligned distinction between:

- **candidate selection**: based on recorded completed sessions, and
- **scheduler timing**: whether the runtime executes immediately or as part of a due run.

### 3.4 Consolidation behavior

Dream does not store a raw transcript as the durable output. Instead it distills:

- durable user preferences,
- project facts and decisions,
- unresolved follow-up items that matter beyond the session,
- stable environment/context details worth future recall,
- other high-signal summaries appropriate for long-term memory.

MVP consolidation should bias toward predictable additive summaries rather than sophisticated
ranking or aggressive pruning heuristics.

### 3.5 Trigger batching vs session ownership

The current seed implementation uses session-count/time thresholds and a workspace-wide run. This can
be retained as the **scheduler policy**, but the consolidation contract changes to session ownership:

- the scheduler decides when Dream work wakes up,
- Dream then drains eligible completed sessions deterministically,
- each drained session gets exactly one logical Dream result.

This allows today’s `sessions_since_last_run` / time-based triggers to survive as an implementation
optimization without remaining the logical source-of-truth.

---

## 4. File and Module Changes Likely Needed

### 4.1 `clients/agent-runtime/src/memory/dream.rs`

**Primary changes:**

- Introduce session-aware Dream state types, likely including:
  - pending/completed/failed per-session records
  - artifact references
  - timestamps / last-attempt metadata
- Refactor `record_session_completion(workspace_dir)` into a session-aware API, likely accepting at
  least `session_id`
- Refactor `run_if_triggered(workspace_dir)` to enumerate eligible completed sessions rather than
  only doing workspace-level line merging
- Preserve current lock semantics where useful, but apply them around session draining
- Emit a Dream report that can summarize per-run and per-session outcomes

**Why:** This file is already the seed implementation and exported as the canonical Dream module.

### 4.2 `clients/agent-runtime/src/memory/mod.rs`

**Likely changes:**

- Re-export revised Dream APIs/types
- Invoke the new Dream trigger API in startup or due-run paths without changing ownership
- Keep hydration/export sequencing compatible with Dream state restore

### 4.3 `clients/agent-runtime/src/memory/sqlite.rs`

**Likely changes:**

- Add Dream persistence schema, likely a `dream_runs` and/or `dream_sessions` table
- Store Dream artifact references and Dream status by `session_id`
- Expose helper methods used by Dream to:
  - discover completed sessions without completed Dream state,
  - read session-scoped memories/transcript inputs,
  - persist Dream artifacts and replay metadata atomically where possible
- Extend migration logic additively

**Possible schema direction:**

```sql
CREATE TABLE IF NOT EXISTS dream_sessions (
    session_id         TEXT PRIMARY KEY,
    status             TEXT NOT NULL,         -- pending | running | completed | failed
    trigger_reason     TEXT,
    artifact_refs      TEXT,                  -- JSON array or normalized child rows
    last_attempt_at    TEXT,
    completed_at       TEXT,
    failure_reason     TEXT,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);
CREATE INDEX IF NOT EXISTS idx_dream_sessions_status ON dream_sessions(status);
```

The exact schema can vary, but the important part is durable replay-safe session identity.

### 4.4 `clients/agent-runtime/src/memory/markdown.rs`

**Likely changes:**

- Define where Dream metadata lives for markdown-backed persistence
- Preserve Dream artifact/session association in a file-compatible way
- Continue storing user-facing long-term memory in markdown while keeping replay metadata separate

**Recommended layout:**

```text
workspace/
├── MEMORY.md
├── memory/
│   ├── 2026-04-26.md
│   └── ...
└── .corvus/
    └── dream_state.json
```

The current `dream_state.json` is a reasonable sidecar for markdown replay metadata. If Dream
artifacts need explicit per-session structure, that can also be captured in sidecar JSON while
keeping `MEMORY.md` human-readable.

### 4.5 `clients/agent-runtime/src/memory/snapshot.rs`

**Likely changes:**

- Export Dream artifacts as part of snapshot-visible long-term memory
- Export enough Dream replay state for later hydration/restart safety
- Hydrate Dream replay metadata when restoring a workspace

Because the current snapshot exports only core memories, the design needs either:

1. embedding Dream artifacts directly as core memory entries plus a Dream metadata section, or
2. extending snapshot export to include a sidecar state file alongside `MEMORY_SNAPSHOT.md`.

The simplest MVP is to keep `MEMORY_SNAPSHOT.md` for user-visible memory and add a machine-readable
Dream state companion file if required.

### 4.6 `clients/agent-runtime/src/memory/hygiene.rs`

**Likely changes:**

- After stale-session auto-close, ensure those closed sessions can enter the same Dream trigger path
- Optionally return affected session IDs, not only an affected count, if Dream needs direct enqueue
  semantics

Today hygiene closes stale sessions in SQL but does not identify which sessions now require Dream
follow-up. This is the main integration gap for spec compliance.

### 4.7 Gateway integration points

**Likely changes:**

- Update `gateway/mod.rs` completion path around lines already invoking `record_session_completion`
  and `run_dream_if_triggered`
- Pass `session_id` into the Dream completion-record API
- Keep gateway behavior best-effort and trigger-only

### 4.8 Session service / command modules

Even though there is no dedicated `src/session/` directory, session lifecycle behavior exists across
SQLite/session command services and must align with Dream:

- consistent end-of-session semantics,
- consistent idempotent completion recording,
- optional helper to fetch completed session transcript/source content.

---

## 5. Backend Behavior Across SQLite, Markdown, Snapshot Hydration, and Export

### 5.1 SQLite backend

SQLite is the strongest backend for Dream and should be treated as the reference implementation.

**Behavior:**

- Session completion is read from the `sessions` table.
- Dream replay state is persisted in dedicated Dream tables.
- Dream memory artifacts are stored either in `memories` with a Dream-distinguishing key/category
  convention or in a normalized Dream artifact table plus memory entry rows.
- Artifact creation and Dream completion metadata should be written in one transaction where
  practical.

**Recommended invariant:** after a successful transaction, both the artifact and the completed Dream
status are visible together.

### 5.2 Markdown backend

Markdown cannot rely on SQL transactions, so parity is semantic rather than structural.

**Behavior:**

- Distilled Dream output is appended/merged into `MEMORY.md` or another established long-term memory
  markdown file.
- Replay metadata persists separately in `dream_state.json` (or a similar sidecar file) keyed by
  `session_id`.
- On restart, Dream reads replay metadata before considering a completed session eligible again.

**Recommended invariant:** if artifact write succeeds but replay metadata write fails, the next run
must reconcile conservatively and avoid producing ambiguous duplicate output.

A practical recovery strategy is to store artifact references or a deterministic Dream key derived
from `session_id`, allowing Dream to detect that the durable artifact already exists before writing a
second one.

### 5.3 Snapshot export

Snapshot export currently focuses on core memory visibility. Dream extends the export surface.

**Behavior required:**

- Dream artifacts that are part of durable long-term memory must appear in exportable state.
- Dream replay metadata must also be exportable or reconstructable.

**Preferred MVP:**

- continue exporting user-visible durable memory to `MEMORY_SNAPSHOT.md`,
- add a machine-readable Dream metadata companion if existing markdown snapshot format is too limited
  for replay state.

### 5.4 Snapshot hydration

Hydration must restore enough state that repeated Dream triggers do not treat already consolidated
sessions as fresh work.

**Behavior required:**

- restore Dream artifacts into the target backend,
- restore Dream replay metadata, or reconstruct it unambiguously from imported Dream artifacts.

**Design bias:** explicit Dream replay metadata is safer than inference from prose memory content.

### 5.5 Export/hydration non-goal

We are **not** trying to make snapshot files an exhaustive forensic dump of every Dream internal.
The requirement is only to preserve long-term memory outputs and enough replay state for idempotent
behavior.

---

## 6. Idempotency and Locking Strategy

### 6.1 Idempotency model

Idempotency key: **`session_id` for a recorded completed session**.

A session’s Dream lifecycle is:

- `pending` — completion observed, Dream not yet finalized
- `running` — Dream currently consolidating
- `completed` — durable artifact(s) and replay metadata recorded
- `failed` — last attempt failed; retry policy may re-enter deterministically

### 6.2 Completion-path idempotency

Repeated completion handling for the same session must be safe.

**Rules:**

- Recording completion in the session system must stay idempotent (`ended_at` not rewritten once set).
- Recording Dream trigger input for the same `session_id` must be upsert-like.
- If Dream already completed for the session, subsequent completion hooks are no-ops for
  consolidation.

### 6.3 Consolidation-path idempotency

Before writing any output, Dream checks persisted replay state.

- If `completed`, skip.
- If `running`, skip or treat as busy depending on lock ownership.
- If `pending`/`failed`, attempt deterministic consolidation.

Artifacts should be written with deterministic references where feasible, for example a stable Dream
key derived from `session_id`, so duplicate writes can be recognized by storage layers that lack full
transactions.

### 6.4 Locking strategy

**Global lock:** retain a coarse lock such as `dream.lock` to serialize a Dream run over a workspace.
This prevents overlapping scanners/pruners and is especially important for markdown/file backends.

**Stale-lock recovery:** ADR-5, §6.4, and §11 require a documented recovery posture for `dream.lock`.
Use OS-level advisory file locking where available so process death automatically releases the held lock even if the lock file path remains on disk. When a reclaim attempt detects an existing `dream.lock` file but cannot acquire the OS lock, treat the run as busy and log the refusal. If an operator suspects a stale file-path artifact, validate that no active Corvus process still holds the lock (for example via process inspection or a retry after the suspected owner exits) before deleting `dream.lock`. Any manual reclaim should log the workspace path, detection reason, and validation outcome so postmortems can distinguish a safe cleanup from a live-lock contention event.

**Per-session state:** use durable state to prevent duplicate output even if:

- gateway retries completion,
- the process crashes after completion recording,
- snapshot restore is followed by another Dream trigger,
- stale-session auto-close re-enters the same logic.

### 6.5 Failure windows and recovery

Potential failure windows:

1. completion recorded, Dream state not yet marked pending
2. Dream pending/running marked, artifact not written
3. artifact written, replay metadata not finalized

**Recovery posture:**

- favor deterministic artifact keys and replay metadata updates,
- re-check storage before writing duplicate artifacts,
- mark failed attempts explicitly so retries remain visible and bounded.

This keeps the system conservative: better to skip duplicate work than to emit ambiguous duplicate
memories.

---

## 7. Integration Boundaries with Sessions and Gateway

### 7.1 Sessions boundary

Sessions own:

- whether a session exists,
- whether it is active or completed,
- completion timestamps and lifecycle state,
- transcript/snapshot sources used by Dream as inputs.

Dream owns:

- whether a recorded completed session should be consolidated now,
- what durable long-term artifact is produced,
- replay metadata and duplicate suppression.

**Boundary rule:** Dream never infers completion solely from message flow or gateway success; it only
acts on recorded completion.

### 7.2 Gateway boundary

Gateway owns:

- invoking session completion paths when generated sessions finish,
- invoking Dream hooks in runtime-defined order,
- preserving runtime idempotency by delegating duplicate safety downward.

Gateway does **not** own:

- Dream eligibility rules,
- Dream content selection,
- Dream persistence format,
- public Dream-specific HTTP contracts for this slice.

### 7.3 Hygiene boundary

Hygiene is another completion producer, not a separate Dream implementation.

When stale sessions are auto-closed, they must feed the same Dream trigger path as gateway-driven
completion so there is one semantic notion of “completed session became a Dream candidate.”

### 7.4 Session snapshots / slash-session lifecycle boundary

Existing session snapshot/tldr/compact machinery may be used as Dream inputs, but this change does
not redefine slash-session lifecycle behavior. Dream may consume those artifacts opportunistically
when they are the best available summary source.

---

## 8. Testing Strategy

### 8.1 Unit tests

Add focused tests in `memory/dream.rs` and relevant backend modules for:

- rejecting active/non-completed sessions,
- recording completion for a specific `session_id`,
- duplicate trigger suppression,
- replay-safe behavior after state reload,
- lock contention returning a safe busy/skipped result,
- deterministic artifact key/reference generation.

### 8.2 SQLite integration tests

Add tests around SQLite-backed Dream state covering:

- completed session becomes pending and then completed,
- repeated completion hook does not create duplicate Dream rows or artifacts,
- transaction/atomicity expectations for artifact + Dream metadata,
- stale auto-closed session later gets Dream eligibility,
- restore from snapshot/export does not reconsolidate ambiguously.

### 8.3 Markdown backend tests

Add tests covering:

- Dream artifact persistence into markdown long-term memory,
- replay metadata in sidecar state,
- restart behavior using only persisted files,
- conservative recovery if artifact exists but metadata is incomplete.

### 8.4 Snapshot tests

Add tests for:

- export includes Dream-visible durable memory,
- hydration restores enough Dream replay state,
- post-hydration re-trigger remains idempotent.

### 8.5 Gateway/session integration tests

Add tests around the completion path already present in `gateway/mod.rs`:

- gateway calls completion recording before Dream trigger,
- retrying the same generated-session completion path stays safe,
- no Dream-specific HTTP contract is required.

### 8.6 Regression tests for hygiene

Add tests ensuring:

- stale session auto-close produces a valid downstream Dream trigger input,
- hygiene-triggered completion and gateway-triggered completion converge on the same Dream semantics.

### 8.7 Validation scope

For code changes, the expected validation baseline is:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If snapshot/export tests are expensive or platform-sensitive, document any scoped subset explicitly.

---

## 9. Explicit Non-Goals and Deferred Items

### Non-goals for this change

- Re-architecting orchestration around Dream
- Adding a new public Dream-specific gateway API
- Building operator/admin UX for Dream inspection beyond what existing runtime verification needs
- Redesigning general memory retrieval/ranking outside consolidation requirements
- Implementing speculative autonomous background schedulers beyond the approved trigger model
- Storing full transcripts as the durable Dream output contract

### Deferred items

- richer heuristics for selecting/highlighting “important” memories,
- multi-session thematic consolidation,
- advanced scoring or semantic deduplication,
- operator tooling to inspect Dream run history/artifacts,
- orchestration reuse once the runtime contract stabilizes,
- finer-grained concurrency beyond the initial global-lock + per-session-state model.

---

## 10. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Current seed Dream implementation is workspace-oriented rather than session-oriented | Medium | Refactor around session identity while preserving trigger thresholds only as scheduler inputs |
| Hygiene auto-close does not currently surface session IDs for Dream follow-up | Medium | Extend hygiene/session integration to return or enumerate newly completed sessions deterministically |
| Markdown backend lacks transactional guarantees | Medium | Use deterministic artifact keys plus durable replay metadata sidecar and conservative recovery logic |
| Snapshot export may not currently encode enough replay metadata | Medium | Add explicit Dream metadata export/hydration path rather than inferring from prose alone |
| Gateway retries may re-enter completion logic | Medium | Keep completion recording idempotent and move duplicate suppression into runtime Dream state |
| Distillation quality may vary | Low/Medium | Scope MVP to stable summaries/facts and defer advanced heuristics |

---

## 11. Rollout and Rollback Notes

### Rollout

- Land Dream state contract first.
- Wire session-aware trigger recording next.
- Add backend parity and hydration/export support.
- Finish with end-to-end hardening tests.

### Rollback

If Dream produces incorrect or duplicate memories:

1. disable Dream trigger invocation while leaving normal session completion intact,
2. retain additive schema/state files where safe,
3. ignore Dream replay metadata during runtime startup if necessary,
4. stop producing new Dream artifacts until corrected.

Because the design is additive, rollback should isolate Dream without regressing ordinary session and
memory behavior.
