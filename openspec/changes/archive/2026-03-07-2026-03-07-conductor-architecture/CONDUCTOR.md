# Corvus Conductor — Architecture & Specification

> **Status:** APPROVED — All blocking issues resolved, all open questions closed
> **Date:** 2026-03-07
> **Author:** Architecture Team
> **Audience:** Architecture review board, implementation agents
> **Revision:** v1.2 — Applied Gap 1 (WaitingForApproval), Gap 2 (RuleBasedClassifier fast-path),
>   M1 (JoinHandle panic detection), Phase 3↔4 swap, Q1-Q10 closure,
>   v1.2: Backported recovery semantics (WaitingForApproval/Scheduled crash survival),
>   failure mode requirements (SQLite write failure, concurrency starvation, unauthorized API,
>   least-privilege sandbox scope) from openspec gap analysis

---

## Table of Contents

1. [Vision & Context](#1-vision--context)
2. [Architecture Overview](#2-architecture-overview)
3. [Core Components](#3-core-components)
4. [Rust Interfaces](#4-rust-interfaces)
5. [Integration with Existing Runtime](#5-integration-with-existing-runtime)
6. [Task Lifecycle & State Machine](#6-task-lifecycle--state-machine)
7. [Concurrency Model](#7-concurrency-model)
8. [Configuration](#8-configuration)
9. [Deployment Shape & Migration Path](#9-deployment-shape--migration-path)
10. [Formal Specification](#10-formal-specification)
11. [Phased Implementation Plan](#11-phased-implementation-plan)
12. [Decisions](#12-decisions-formerly-open-questions)

---

## 1. Vision & Context

### 1.1 What We Are Building

The **Conductor** is a general-purpose task orchestrator embedded within the Corvus runtime. It
receives work from multiple sources (chat channels, scheduled jobs, manual dashboard commands,
`/task` CLI commands), decomposes complex tasks into executable steps via LLM-powered planning,
dispatches those steps to specialized **Performers** (Coding, Research, Browser, System), and
reports progress back through the originating surface.

The Conductor is NOT:

- A multi-tenant job scheduler (it runs within a single Corvus instance)
- A replacement for the existing AgentLoop (which handles conversational interaction)
- A distributed workflow engine (no cross-node coordination)

The Conductor IS:

- An intelligent task router that classifies, plans, and executes heterogeneous work
- A supervisor that tracks task progress, handles failures, and coordinates dependencies
- A bridge that connects Corvus's existing primitives (Agent, Provider, Tool, Memory, Observer)
  into higher-level autonomous workflows

### 1.2 Inspiration: OpenAI Symphony — What We Take, What We Don't

OpenAI's Symphony is a daemon that polls Linear for issues, creates isolated workspaces per issue,
and launches Codex agents to work on them autonomously. Its key ideas:

| Symphony Concept                         | Corvus Conductor Equivalent                                 | Adaptation                                                                                           |
|------------------------------------------|-------------------------------------------------------------|------------------------------------------------------------------------------------------------------|
| `WORKFLOW.md` (config+prompt)            | `CONDUCTOR.md` + `config.toml`                              | We split config from prompt; CONDUCTOR.md is the behavioral prompt, config.toml holds typed settings |
| Poll-based tick loop (30s)               | `TickLoop` with configurable interval                       | Same pattern — reconcile, schedule, dispatch on each tick                                            |
| Issue tracker as source                  | `SourceRouter` with multiple source types                   | We support chat, cron, dashboard, `/task` CLI, not just one tracker                                  |
| One agent per issue, fully isolated      | `Performer` per step, shared memory                         | We share `Arc<Memory>` for cross-step context; isolation is at workspace level                       |
| Workspace Manager (filesystem lifecycle) | `WorkspaceManager` per task                                 | Same — create, setup, teardown per task with hooks                                                   |
| In-memory state, no DB                   | `TaskStore` with in-memory primary + SQLite WAL persistence | We add crash recovery via SQLite; in-memory for hot path                                             |
| Exponential backoff retry                | Same, integrated with daemon supervisor                     | Leverage existing daemon restart infrastructure                                                      |
| Single domain (coding)                   | Multi-domain: Coding, Research, Browser, System, Composite  | Each domain has a specialized Performer with appropriate tools                                       |

What we explicitly **do NOT take** from Symphony:

- **Linear-only integration** — We are source-agnostic
- **No task decomposition** — Symphony treats each issue as atomic. We decompose via LLM Planner
- **No inter-task coordination** — Symphony agents are fully isolated. Our steps within a task
  can share context and have dependency graphs
- **Elixir/OTP supervision trees** — We use Tokio tasks with `JoinHandle` panic detection + daemon supervisor

### 1.3 Relationship to Existing Corvus Components

```
┌───────────────────────────────────────────────────────────────┐
│                     Corvus Runtime                            │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐   │
│  │  AgentLoop   │  │  Conductor   │  │   Gateway/Server   │   │
│  │  (chat turns) │  │  (task mgmt) │  │   (HTTP/WS API)    │   │
│  └──────┬───────┘  └──────┬───────┘  └────────┬───────────┘   │
│         │                 │                    │               │
│  ┌──────┴─────────────────┴────────────────────┴───────────┐  │
│  │                 Shared Infrastructure                    │  │
│  │  Arc<Provider>  Arc<Memory>  Arc<Config>  Arc<Observer>  │  │
│  │  Arc<Sandbox>   Arc<SecurityPolicy>   Tool Registry      │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                    Daemon Supervisor                     │  │
│  │  [gateway] [channels] [heartbeat] [scheduler]           │  │
│  │  [mission-checkpoints] [updater] [conductor] ← NEW      │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

The Conductor operates **alongside** the AgentLoop, not replacing it:

- **AgentLoop**: Handles interactive conversation — short messages, low latency, user-facing
- **Conductor**: Handles autonomous task execution — long-running, multi-step, background

Both share the same `Arc<Memory>`, `Arc<Provider>`, tool registry, and observer infrastructure.

---

## 2. Architecture Overview

### 2.1 High-Level Component Diagram

```
                          SOURCES (inbound)
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
    ┌─────▼──────┐   ┌────────▼───────┐   ┌───────▼────────┐
    │    Chat     │   │   /task CLI    │   │   Dashboard    │
    │  Channels   │   │   command      │   │   Manual       │
    └─────┬──────┘   └────────┬───────┘   └───────┬────────┘
          │                    │                    │
          └────────────────────┼────────────────────┘
                               │
                               ▼
                 ┌──────────────────────────┐
                 │      SourceRouter        │
                 │                          │
                 │  • Normalize raw input   │
                 │  • Classify domain       │
                 │  • Create TaskRequest    │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │        Planner           │
                 │                          │
                 │  • LLM-powered decomp    │
                 │  • Build dependency DAG  │
                 │  • Cost estimation       │
                 │  • Security pre-check    │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │       TaskStore          │
                 │                          │
                 │  • In-memory (DashMap)   │
                 │  • SQLite WAL persist    │
                 │  • State transitions     │
                 │  • Crash recovery        │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │       TickLoop           │
                 │                          │
                 │  • 30s interval (config) │
                 │  • reconcile() → stalls  │
                 │  • schedule() → ready    │
                 │  • dispatch() → launch   │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌──────────────────────────┐
                 │     PerformerPool        │
                 │                          │
                 │  tokio::spawn per step   │
                 │                          │
                 │  ┌────────┐ ┌─────────┐  │
                 │  │ Coding │ │Research │  │
                 │  └────────┘ └─────────┘  │
                 │  ┌────────┐ ┌─────────┐  │
                 │  │Browser │ │ System  │  │
                 │  └────────┘ └─────────┘  │
                 └────────────┬─────────────┘
                              │
                              ▼
                       SINKS (outbound)
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
    ┌─────▼──────┐   ┌───────▼──────┐   ┌────────▼───────┐
    │  Channel   │   │   Observer   │   │   Dashboard    │
    │  Reply     │   │   Events     │   │   WebSocket    │
    └────────────┘   └──────────────┘   └────────────────┘
```

### 2.2 Data Flow — Happy Path

```
User (Telegram): "Refactoriza el módulo de auth, escribe tests y documenta los cambios"
  │
  ├─1→ ChannelSource captures ChannelMessage
  │     SourceRouter classifies → TaskDomain::Composite
  │     Creates TaskRequest { origin: Chat("telegram", "chan_123"), raw: "..." }
  │
  ├─2→ Planner receives TaskRequest
  │     LLM call with CONDUCTOR.md system prompt + task context
  │     Returns TaskPlan:
  │       Step A: Research  "Analyze current auth module structure"    deps: []
  │       Step B: Coding    "Refactor auth module"                    deps: [A]
  │       Step C: Coding    "Write unit tests for refactored auth"    deps: [B]
  │       Step D: Research  "Generate documentation for changes"      deps: [B]
  │
  ├─3→ TaskStore persists plan, all steps status = Queued
  │     Notifies TickLoop of new work
  │
  ├─4→ TickLoop.schedule()
  │     Step A has no deps → status = Scheduled
  │     Steps B, C, D blocked → status = WaitingForDependency
  │
  ├─5→ TickLoop.dispatch()
  │     Step A → ResearchPerformer.execute()
  │     tokio::spawn with timeout, progress channel wired
  │
  ├─6→ ResearchPerformer completes Step A
  │     StepOutcome { success: true, artifacts: ["analysis.md"] }
  │     TickLoop.reconcile() → unblock Step B
  │     Step B → CodingPerformer.execute()
  │
  ├─7→ CodingPerformer completes Step B
  │     TickLoop.reconcile() → unblock Steps C and D (parallel!)
  │     Step C → CodingPerformer.execute()   (concurrent)
  │     Step D → ResearchPerformer.execute()  (concurrent)
  │
  ├─8→ Steps C and D complete
  │     All steps done → Task status = Completed
  │     TaskOutcome aggregated
  │
  └─9→ ConductorEvent::TaskCompleted emitted
        ChannelSink sends summary to Telegram chat
        Observer records telemetry
```

### 2.3 Data Flow — Failure & Retry

```
Step B (Coding) fails after 3 minutes:
  │
  ├─1→ PerformerPool receives error from tokio::spawn JoinHandle
  │     TickLoop.reconcile() detects failure
  │
  ├─2→ Retry policy evaluated:
  │     retries_remaining > 0?
  │       YES → Step B status = RetryQueued { attempt: 2, backoff: 10s }
  │              After backoff → re-dispatch to CodingPerformer
  │       NO  → Step B status = Failed { error: "...", retries: 3 }
  │              Dependent steps C, D → status = Cancelled { reason: "dependency_failed" }
  │              Task status = Failed
  │
  ├─3→ ConductorEvent::StepFailed emitted
  │     ConductorEvent::TaskFailed emitted (if terminal)
  │
  └─4→ ChannelSink sends failure summary to user
        Observer records failure telemetry
```

---

## 3. Core Components

### 3.1 SourceRouter

**Responsibility:** Receive raw input from heterogeneous sources, normalize it into a typed
`TaskRequest`, and classify which domain(s) it belongs to.

**Sources supported (MVP):**

| Source              | Trigger                               | Integration                                             |
|---------------------|---------------------------------------|---------------------------------------------------------|
| Chat Channels       | User message containing task intent   | Existing `Channel` trait → `ChannelMessage`             |
| `/task` CLI command | User runs `corvus task "description"` | New CLI subcommand → direct `ConductorHandle::submit()` |
| Dashboard Manual    | User clicks "New Task" in web UI      | Gateway HTTP endpoint → `ConductorHandle::submit()`     |
| Cron Scheduler      | Scheduled job fires                   | Existing `CronScheduler` → new `ConductorJobType`       |

**Classification strategy:** The SourceRouter uses a lightweight LLM call (or rule-based heuristics
for simple cases) to determine:

1. **Is this a task or a conversation?** — "What time is it?" → Conversation (route to AgentLoop).
   "Deploy the staging environment" → Task (route to Conductor).
2. **Which domain(s)?** — Single domain tasks go directly to a Performer. Multi-domain tasks go
   through the Planner for decomposition.

**Critical design decision:** The SourceRouter does NOT intercept all channel messages. It only
processes messages explicitly tagged as tasks (via `/task` command, dashboard submission, or cron).
Regular chat stays in the existing AgentLoop pipeline. This avoids breaking the current
conversational UX.

### 3.2 Planner

**Responsibility:** Decompose a `TaskRequest` into an executable `TaskPlan` with a dependency graph.

**How it works:**

1. Receives `TaskRequest` with raw description and domain classification
2. **Fast-path check (NO network):** The `RuleBasedClassifier` is invoked FIRST. If it returns
   a single domain with high confidence, a single-step plan is created immediately — NO LLM call.
   This handles 90%+ of simple tasks like `/task fix typo in README` in <10ms.
3. **Slow-path (LLM decomposition):** Only reached if the `RuleBasedClassifier` returns
   `Composite` or low confidence. In this case:
   a. Loads the `CONDUCTOR.md` prompt as system context
   b. Queries `Memory` for relevant past tasks and workspace context
   c. Makes an LLM call to the configured planner model asking for:

   - A list of discrete steps
   - Domain classification per step
   - Dependency relationships between steps
   - Estimated complexity per step

   d. Validates the plan:

   - Dependency graph is a DAG (no cycles)
   - All referenced domains have available Performers
   - Total estimated cost within governance budget

   e. Returns `TaskPlan` or `PlanningError`

**Decision flow:**

```
TaskRequest
    │
    ▼
RuleBasedClassifier.classify(description)
    │
    ├── High confidence + single domain ──► Single-step plan (NO LLM, <10ms)
    │
    └── Composite / low confidence ──► LLM Planner (2-5s)
                                           │
                                           ▼
                                       TaskPlan with N steps + dependency DAG
```

**Planner prompt structure (injected from CONDUCTOR.md + dynamic context):**

```
[CONDUCTOR.md content — behavioral instructions, domain descriptions, planning rules]
[Workspace context from Memory — recent tasks, project structure]
[Available performers and their capabilities]
[Task request from user]

Output format: JSON with steps, dependencies, domain assignments
```

### 3.3 TaskStore

**Responsibility:** Authoritative source of truth for all task and step state.

**Design:**

- **Hot path:** `DashMap<TaskId, TaskState>` — zero-copy concurrent reads, lock-free
- **Persistence:** SQLite WAL mode — write-ahead log for crash recovery
- **Consistency:** State transitions are atomic (DashMap update + SQLite write in same call)
- **Recovery:** On startup, load incomplete tasks from SQLite. Tasks in `Running` state get
  reset to `Queued` (the performer that was running them is gone after a crash)

**Tables:**

```sql
CREATE TABLE tasks
(
  id           TEXT PRIMARY KEY,
  status       TEXT NOT NULL,
  request_json TEXT NOT NULL,
  plan_json    TEXT,
  origin_json  TEXT NOT NULL,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE steps
(
  id              TEXT PRIMARY KEY,
  task_id         TEXT    NOT NULL REFERENCES tasks (id),
  domain          TEXT    NOT NULL,
  description     TEXT    NOT NULL,
  status          TEXT    NOT NULL,
  depends_on_json TEXT    NOT NULL DEFAULT '[]',
  inputs_json     TEXT,
  outcome_json    TEXT,
  attempt         INTEGER NOT NULL DEFAULT 0,
  approval_reason TEXT,           -- non-null when status = 'WaitingForApproval'
  approval_tool   TEXT,           -- tool that triggered the approval request
  started_at      TEXT,
  completed_at    TEXT
);

CREATE TABLE step_artifacts
(
  id         TEXT PRIMARY KEY,
  step_id    TEXT NOT NULL REFERENCES steps (id),
  kind       TEXT NOT NULL,
  content    TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

### 3.4 TickLoop

**Responsibility:** Periodic scheduler that reconciles state, identifies ready work, and dispatches
to Performers.

**Tick phases (executed in order on each interval):**

```
┌─────────────────────────────────────────────────────────┐
│                    TICK (every 30s)                      │
│                                                         │
│  Phase 1: RECONCILE                                     │
│  ├── Stall detection: steps running > step.timeout      │
│  ├── Progress check: poll performer progress channels   │
│  ├── Completion: collect finished steps, update store    │
│  ├── Failure: collect failed steps, evaluate retry       │
│  └── Dependency resolution: unblock waiting steps        │
│                                                         │
│  Phase 2: SCHEDULE                                      │
│  ├── Query store for Queued steps with all deps met     │
│  ├── Check concurrency limits (global + per-domain)     │
│  ├── Priority sort: urgency → age → domain affinity     │
│  └── Move eligible steps to Scheduled status             │
│                                                         │
│  Phase 3: DISPATCH                                      │
│  ├── Match Scheduled steps to available performers      │
│  ├── Create PerformerContext with workspace, memory      │
│  ├── tokio::spawn performer execution                   │
│  └── Wire progress channels, store JoinHandle            │
│                                                         │
│  Phase 4: NOTIFY                                        │
│  ├── Emit ConductorEvents for all state changes         │
│  ├── Aggregate task-level status from step statuses      │
│  └── Send progress to originating channels               │
└─────────────────────────────────────────────────────────┘
```

**Reactive dispatch (optimization):** In addition to the periodic tick, the TickLoop listens on an
`mpsc` channel for "nudge" signals. When a step completes, its completion handler sends a nudge,
causing an immediate mini-tick (reconcile + schedule + dispatch) without waiting for the next
interval. This reduces latency for multi-step plans from `O(steps * tick_interval)` to near-instant
cascade.

### 3.5 PerformerPool

**Responsibility:** Manage the lifecycle of Performer instances and enforce concurrency limits.

**Design:**

```
PerformerPool
├── performers: HashMap<TaskDomain, Box<dyn Performer>>
├── running: DashMap<StepId, RunningStep>
├── semaphores: HashMap<TaskDomain, Arc<Semaphore>>  // per-domain limits
└── global_semaphore: Arc<Semaphore>                  // total limit
```

Each dispatch acquires both the global and domain-specific semaphore permits before spawning.
Permits are released when the performer task completes (or is cancelled).

### 3.6 Performers (4 specialized + 1 composite)

Each Performer wraps the existing `Agent` + `Tool` infrastructure with domain-specific
configuration.

#### 3.6.1 CodingPerformer

- **Tools:** `shell`, `file_read`, `file_write`, `git_operations`, MCP tools (Codex CLI if
  configured)
- **Agent config:** High max iterations, code-focused system prompt, structured output preference
- **Workspace:** Isolated directory per task, git branch per coding step
- **Speciality:** Can launch external coding agents (Codex CLI, Claude Code) via subprocess
  and stream their output as progress events

#### 3.6.2 ResearchPerformer

- **Tools:** `web_search`, `http_request`, `browser_open`, `memory_store`, `memory_recall`,
  `file_read`, `file_write`
- **Agent config:** Analytical system prompt, high-quality model preference
- **Output:** Structured analysis documents, stored as step artifacts
- **Memory integration:** Direct access to `Arc<Memory>` for contextual recall and storage

#### 3.6.3 BrowserPerformer

- **Tools:** `browser`, `browser_open`, `screenshot`, `http_request`
- **Agent config:** Browser automation prompt, visual reasoning capable model
- **Speciality:** Headless browser automation, scraping, UI testing
- **Isolation:** Each browser session is sandboxed

#### 3.6.4 SystemPerformer

- **Tools:** `shell`, `file_read`, `file_write`, `http_request`, `cron_add`
- **Agent config:** DevOps/sysadmin prompt, safety-first
- **Sandbox:** All shell commands wrapped through `Sandbox` trait — MANDATORY, no bypass
- **Governance:** Higher approval threshold — destructive commands require explicit user approval

#### 3.6.5 CompositePerformer (internal)

Not a real performer — a virtual domain that signals the Planner to decompose. A task classified
as `Composite` never reaches the PerformerPool directly; it always goes through planning first.

### 3.7 WorkspaceManager

**Responsibility:** Filesystem lifecycle per task — creation, setup, isolation, teardown.

**Layout:**

```
~/.corvus/workspaces/
├── task-abc123/
│   ├── .conductor/
│   │   ├── plan.json          # Serialized TaskPlan
│   │   ├── state.json         # Step statuses snapshot
│   │   └── artifacts/         # Step outputs
│   │       ├── step-1-analysis.md
│   │       └── step-2-diff.patch
│   ├── workspace/             # Actual working directory
│   │   └── (cloned repo, generated files, etc.)
│   └── logs/
│       ├── step-1.log
│       └── step-2.log
└── task-def456/
    └── ...
```

**Hooks (configurable in CONDUCTOR.md):**

| Hook             | When                              | Example                               |
|------------------|-----------------------------------|---------------------------------------|
| `after_create`   | After workspace directory created | `git clone <repo> .`                  |
| `before_step`    | Before each step executes         | `git checkout -b conductor/<step-id>` |
| `after_step`     | After each step completes         | `git add -A && git stash`             |
| `after_complete` | After all steps complete          | `rm -rf workspace/node_modules`       |
| `on_failure`     | When task fails terminally        | `git checkout main`                   |

All hooks run through `Sandbox::wrap_command()` — no unsandboxed filesystem access.
All filesystem operations in hooks use `tokio::task::spawn_blocking` to avoid blocking the
Tokio runtime.

---

## 4. Rust Interfaces

### 4.1 Core Types

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// ─── Identifiers ───────────────────────────────────────────────────

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct StepId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerformerId(pub String);

impl TaskId {
  pub fn new() -> Self { Self(format!("task-{}", Uuid::new_v4().as_simple())) }
}

impl StepId {
  pub fn new() -> Self { Self(format!("step-{}", Uuid::new_v4().as_simple())) }
}

// ─── Domain Classification ────────────────────────────────────────

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TaskDomain {
  Coding,
  Research,
  Browser,
  System,
  Composite,  // Requires decomposition — never dispatched directly
}

// ─── Task Request (input from sources) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
  pub description: String,
  pub origin: TaskOrigin,
  pub priority: TaskPriority,
  pub context: Option<String>,       // Additional context from user
  pub workspace_hint: Option<String>, // e.g., repo URL or local path
  pub timeout: Option<Duration>,
  pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOrigin {
  Chat {
    channel_name: String,
    channel_id: String,
    sender: String,
    thread_id: Option<String>,
  },
  Cli {
    working_dir: PathBuf,
  },
  Dashboard {
    session_id: String,
  },
  Cron {
    job_id: String,
    schedule_name: String,
  },
  Internal {
    component: String,
  },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
  Low = 0,
  Normal = 1,
  High = 2,
  Urgent = 3,
}

impl Default for TaskPriority {
  fn default() -> Self { Self::Normal }
}

// ─── Task Plan (output from Planner) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
  pub id: PlanId,
  pub steps: Vec<PlannedStep>,
  pub estimated_total_duration: Option<Duration>,
  pub estimated_cost_cents: Option<u32>,
  pub rationale: String,  // LLM explanation of the decomposition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
  pub id: StepId,
  pub domain: TaskDomain,
  pub description: String,
  pub depends_on: Vec<StepId>,
  pub expected_output: String,      // What this step should produce
  pub tool_hints: Vec<String>,       // Suggested tools (advisory, not binding)
  pub timeout: Duration,
  pub max_retries: u32,
}

// ─── Task State (managed by TaskStore) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
  pub id: TaskId,
  pub request: TaskRequest,
  pub status: TaskStatus,
  pub plan: Option<TaskPlan>,
  pub step_states: HashMap<StepId, StepState>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
  pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
  /// Just received, not yet planned
  Received,
  /// Planner is decomposing the task
  Planning,
  /// Plan created, steps are being scheduled/executed
  Active,
  /// All steps completed successfully
  Completed { outcome: TaskOutcome },
  /// One or more steps failed terminally
  Failed { error: String },
  /// User or system requested cancellation
  Cancelled { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutcome {
  pub summary: String,
  pub artifacts: Vec<Artifact>,
  pub steps_completed: usize,
  pub total_duration: Duration,
  pub total_cost_cents: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
  pub kind: ArtifactKind,
  pub description: String,
  pub path: Option<PathBuf>,
  pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactKind {
  File,
  Diff,
  Analysis,
  Documentation,
  Screenshot,
  Log,
  Custom(String),
}

// ─── Step State ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepState {
  pub id: StepId,
  pub status: StepStatus,
  pub attempt: u32,
  pub started_at: Option<chrono::DateTime<chrono::Utc>>,
  pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
  pub outcome: Option<StepOutcome>,
  pub progress: Option<StepProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
  Queued,
  WaitingForDependency { blocked_by: Vec<StepId> },
  Scheduled,
  Running { performer_id: PerformerId },
  RetryQueued { attempt: u32, retry_after: chrono::DateTime<chrono::Utc> },
  WaitingForApproval { reason: String, tool_name: String },
  Completed,
  Failed { error: String },
  Cancelled { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutcome {
  pub success: bool,
  pub summary: String,
  pub artifacts: Vec<Artifact>,
  pub output_context: Option<String>,  // Passed to dependent steps as input
  pub tokens_used: Option<u64>,
  pub cost_cents: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepProgress {
  pub message: String,
  pub percentage: Option<u8>,  // 0-100
  pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

### 4.2 Core Traits

```rust
use async_trait::async_trait;
use tokio::sync::{mpsc, broadcast};

// ─── ConductorHandle — External interface to Conductor ────────────
// This trait enables the Same Process → Remote Process migration.
// Corvus code calls this trait; it doesn't know the implementation.

#[async_trait]
pub trait ConductorHandle: Send + Sync {
  /// Submit a new task. Returns immediately with an ID.
  async fn submit(&self, request: TaskRequest) -> anyhow::Result<TaskId>;

  /// Cancel a running or queued task.
  async fn cancel(&self, id: &TaskId) -> anyhow::Result<()>;

  /// Get current status of a task.
  async fn status(&self, id: &TaskId) -> anyhow::Result<TaskState>;

  /// Get detailed status of a specific step.
  async fn step_status(&self, task_id: &TaskId, step_id: &StepId)
                       -> anyhow::Result<StepState>;

  /// Subscribe to real-time conductor events.
  async fn subscribe(&self) -> broadcast::Receiver<ConductorEvent>;

  /// Get a snapshot of all active tasks and conductor health.
  async fn snapshot(&self) -> anyhow::Result<ConductorSnapshot>;

  /// List tasks with optional filtering.
  async fn list_tasks(&self, filter: TaskFilter) -> anyhow::Result<Vec<TaskState>>;
}

// MVP implementation: same-process via channels
pub struct LocalConductorHandle {
  cmd_tx: mpsc::Sender<ConductorCmd>,
  event_tx: broadcast::Sender<ConductorEvent>,
}

// Future implementation: remote process via HTTP/Unix socket
// pub struct RemoteConductorHandle {
//     base_url: url::Url,
//     client: reqwest::Client,
// }

// ─── Performer — Domain-specific task executor ────────────────────

#[async_trait]
pub trait Performer: Send + Sync {
  /// Which domain this performer handles.
  fn domain(&self) -> TaskDomain;

  /// Human-readable name for logging/display.
  fn name(&self) -> &str;

  /// Maximum concurrent steps this performer can handle.
  fn max_concurrent(&self) -> usize;

  /// Execute a planned step within the given context.
  /// MUST send progress updates via the progress channel.
  /// MUST respect the step timeout (ctx.deadline).
  /// MUST NOT panic — return Err instead.
  async fn execute(
    &self,
    step: &PlannedStep,
    ctx: PerformerContext,
    progress_tx: mpsc::Sender<ProgressEvent>,
  ) -> anyhow::Result<StepOutcome>;

  /// Request cancellation of a running step.
  /// The performer SHOULD attempt graceful shutdown.
  async fn cancel(&self, step_id: &StepId) -> anyhow::Result<()>;

  /// Health check — is this performer ready to accept work?
  async fn health_check(&self) -> anyhow::Result<()> { Ok(()) }
}

/// Context provided to every performer execution.
/// All fields are cheaply cloneable (Arc-wrapped).
pub struct PerformerContext {
  pub task_id: TaskId,
  pub memory: Arc<dyn Memory>,
  pub config: Arc<Config>,
  pub observer: Arc<dyn Observer>,
  pub sandbox: Arc<dyn Sandbox>,
  pub provider: Arc<dyn Provider>,
  pub tool_registry: Arc<Vec<Box<dyn Tool>>>,
  pub workspace: PathBuf,
  pub deadline: Instant,
  /// Output from completed dependency steps (step_id → output_context)
  pub dependency_outputs: HashMap<StepId, String>,
}

// ─── Source — Inbound task origin ─────────────────────────────────

#[async_trait]
pub trait Source: Send + Sync {
  fn name(&self) -> &str;

  /// Called on each tick to check for new task requests.
  /// Returns empty vec if no new work.
  async fn poll(&self) -> anyhow::Result<Vec<TaskRequest>>;

  /// Some sources support push (channels). This returns a receiver
  /// for immediate task submissions between ticks.
  fn subscribe(&self) -> Option<mpsc::Receiver<TaskRequest>> { None }
}

// ─── TaskClassifier — Domain classification ───────────────────────

#[async_trait]
pub trait TaskClassifier: Send + Sync {
  /// Classify a raw task description into one or more domains.
  /// Returns Composite if multiple domains detected.
  async fn classify(&self, description: &str) -> anyhow::Result<ClassificationResult>;
}

#[derive(Debug, Clone)]
pub struct ClassificationResult {
  pub domain: TaskDomain,
  pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
  /// Rule matched unambiguously — skip LLM planning
  High,
  /// Likely correct but LLM validation recommended
  Medium,
  /// Ambiguous — LLM planning required
  Low,
}

/// First-class fast-path classifier. Runs BEFORE the LLM Planner.
/// Resolves 90%+ of simple, single-domain tasks without any network call.
///
/// Pattern matching strategy (checked in order):
///   1. Explicit domain prefix: "/task code: ..." → Coding
///   2. Keyword signals: "fix", "refactor", "write test" → Coding (High)
///   3. Keyword signals: "search", "analyze", "summarize" → Research (High)
///   4. Keyword signals: "open browser", "scrape", "screenshot" → Browser (High)
///   5. Keyword signals: "deploy", "restart", "install" → System (High)
///   6. Multi-domain keywords detected → Composite (High)
///   7. No strong signal → Composite (Low) — defers to LLM
pub struct RuleBasedClassifier;

/// Falls back to LLM when RuleBasedClassifier returns Low confidence.
pub struct LlmClassifier {
  provider: Arc<dyn Provider>,
  model: String,
}

/// Chains RuleBasedClassifier → LlmClassifier.
/// Used by the Planner as its classification strategy.
pub struct ChainedClassifier {
  rule_based: RuleBasedClassifier,
  llm: LlmClassifier,
}

#[async_trait]
impl TaskClassifier for ChainedClassifier {
  async fn classify(&self, description: &str) -> anyhow::Result<ClassificationResult> {
    let result = self.rule_based.classify(description).await?;
    if result.confidence == Confidence::High {
      return Ok(result); // Fast path — no network
    }
    // Slow path — LLM classification
    self.llm.classify(description).await
  }
}
```

### 4.3 Events & Commands

```rust
// ─── ConductorEvent — Observable state changes ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConductorEvent {
  // Task lifecycle
  TaskReceived { task_id: TaskId, origin: TaskOrigin },
  TaskPlanned { task_id: TaskId, step_count: usize },
  TaskCompleted { task_id: TaskId, outcome: TaskOutcome },
  TaskFailed { task_id: TaskId, error: String },
  TaskCancelled { task_id: TaskId, reason: String },

  // Step lifecycle
  StepScheduled { task_id: TaskId, step_id: StepId, domain: TaskDomain },
  StepStarted { task_id: TaskId, step_id: StepId, performer: String },
  StepProgress { task_id: TaskId, step_id: StepId, progress: StepProgress },
  StepCompleted { task_id: TaskId, step_id: StepId, outcome: StepOutcome },
  StepFailed { task_id: TaskId, step_id: StepId, error: String, will_retry: bool },
  StepCancelled { task_id: TaskId, step_id: StepId },

  // System
  ConductorStarted,
  ConductorStopped,
  TickCompleted { active_tasks: usize, running_steps: usize },
  StallDetected { task_id: TaskId, step_id: StepId, duration: Duration },
}

// Bridge to existing Observer infrastructure
impl From<&ConductorEvent> for ObserverEvent {
  fn from(event: &ConductorEvent) -> Self {
    // Map conductor events to observer events for telemetry
    // This preserves all existing observability pipelines
    todo!()
  }
}

// ─── ConductorCmd — Internal command channel ──────────────────────

pub(crate) enum ConductorCmd {
  Submit { request: TaskRequest, reply: oneshot::Sender<anyhow::Result<TaskId>> },
  Cancel { task_id: TaskId, reply: oneshot::Sender<anyhow::Result<()>> },
  Status { task_id: TaskId, reply: oneshot::Sender<anyhow::Result<TaskState>> },
  StepStatus { task_id: TaskId, step_id: StepId, reply: oneshot::Sender<anyhow::Result<StepState>> },
  Snapshot { reply: oneshot::Sender<anyhow::Result<ConductorSnapshot>> },
  ListTasks { filter: TaskFilter, reply: oneshot::Sender<anyhow::Result<Vec<TaskState>>> },
  Nudge,  // Trigger immediate mini-tick (step completed, new work available)
}

// ─── ConductorSnapshot — Dashboard/status view ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorSnapshot {
  pub active_tasks: Vec<TaskState>,
  pub completed_today: usize,
  pub failed_today: usize,
  pub performer_health: HashMap<TaskDomain, PerformerHealth>,
  pub uptime: Duration,
  pub tick_count: u64,
  pub config: ConductorConfigView,  // Sanitized, no secrets
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformerHealth {
  pub domain: TaskDomain,
  pub name: String,
  pub running_steps: usize,
  pub max_concurrent: usize,
  pub healthy: bool,
}

// ─── Task Filtering ───────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
  pub status: Option<Vec<TaskStatus>>,
  pub domain: Option<Vec<TaskDomain>>,
  pub origin: Option<String>,
  pub created_after: Option<chrono::DateTime<chrono::Utc>>,
  pub created_before: Option<chrono::DateTime<chrono::Utc>>,
  pub limit: Option<usize>,
}

// ─── Progress Events (performer → tick loop) ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
  Update { message: String, percentage: Option<u8> },
  Log { level: LogLevel, message: String },
  ArtifactProduced { artifact: Artifact },
  ApprovalRequired { reason: String, tool_name: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogLevel {
  Debug,
  Info,
  Warn,
  Error,
}
```

### 4.4 ConductorService (Main Orchestrator)

```rust
pub struct ConductorService {
  config: ConductorConfig,
  store: Arc<TaskStore>,
  planner: Arc<Planner>,
  pool: Arc<PerformerPool>,
  source_router: SourceRouter,
  workspace_manager: WorkspaceManager,
  cmd_rx: mpsc::Receiver<ConductorCmd>,
  event_tx: broadcast::Sender<ConductorEvent>,
  observer: Arc<dyn Observer>,
  nudge_rx: mpsc::Receiver<()>,
  nudge_tx: mpsc::Sender<()>,
}

impl ConductorService {
  pub fn new(
    config: ConductorConfig,
    memory: Arc<dyn Memory>,
    provider: Arc<dyn Provider>,
    observer: Arc<dyn Observer>,
    sandbox: Arc<dyn Sandbox>,
    tool_registry: Arc<Vec<Box<dyn Tool>>>,
    app_config: Arc<Config>,
  ) -> (Self, LocalConductorHandle) {
    let (cmd_tx, cmd_rx) = mpsc::channel(256);
    let (event_tx, _) = broadcast::channel(1024);
    let (nudge_tx, nudge_rx) = mpsc::channel(64);

    let handle = LocalConductorHandle {
      cmd_tx,
      event_tx: event_tx.clone(),
    };

    let service = Self {
      config,
      store: Arc::new(TaskStore::new(/* db_path */)),
      planner: Arc::new(Planner::new(provider.clone(), memory.clone())),
      pool: Arc::new(PerformerPool::new(/* performer configs */)),
      source_router: SourceRouter::new(/* sources, classifier */),
      workspace_manager: WorkspaceManager::new(/* workspace_root */),
      cmd_rx,
      event_tx,
      observer,
      nudge_rx,
      nudge_tx,
    };

    (service, handle)
  }

  /// Main run loop — called inside tokio::spawn from daemon
  pub async fn run(mut self, mut shutdown: broadcast::Receiver<()>) {
    self.event_tx.send(ConductorEvent::ConductorStarted).ok();
    self.recover_from_crash().await;

    let mut tick = tokio::time::interval(self.config.tick_interval);
    let mut tick_count: u64 = 0;

    loop {
      tokio::select! {
                // Periodic tick
                _ = tick.tick() => {
                    tick_count += 1;
                    self.full_tick(tick_count).await;
                }

                // Reactive nudge (step completed → immediate cascade)
                _ = self.nudge_rx.recv() => {
                    self.mini_tick().await;
                }

                // Command from ConductorHandle
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle_cmd(cmd).await;
                }

                // Shutdown signal
                _ = shutdown.recv() => {
                    self.graceful_shutdown().await;
                    break;
                }
            }
    }

    self.event_tx.send(ConductorEvent::ConductorStopped).ok();
  }

  async fn full_tick(&self, tick_count: u64) {
    self.poll_sources().await;
    self.reconcile().await;
    self.schedule().await;
    self.dispatch().await;
    self.notify(tick_count).await;
  }

  async fn mini_tick(&self) {
    // Lightweight: just reconcile completed work and dispatch ready steps
    self.reconcile().await;
    self.schedule().await;
    self.dispatch().await;
  }

  async fn recover_from_crash(&self) {
    // Load incomplete tasks from SQLite
    // Reset Running steps to Queued
    // Log recovery actions
  }

  async fn graceful_shutdown(&self) {
    // 1. Stop accepting new tasks
    // 2. Cancel all running performers (with timeout)
    // 3. Persist final state to SQLite
    // 4. Clean up workspaces (optional, configurable)
  }
}
```

---

## 5. Integration with Existing Runtime

### 5.1 Daemon Integration

The Conductor registers as a new supervised worker in the existing daemon infrastructure:

```rust
// In src/daemon/mod.rs — addition to existing code

// Add to the component supervisor list:
spawn_component_supervisor(
"conductor",
{
let config = config.clone();
let memory = memory.clone();
let provider = provider.clone();
let observer = observer.clone();
let sandbox = sandbox.clone();
let tools = tools.clone();
let app_config = app_config.clone();
let shutdown = shutdown_tx.subscribe();
move | | {
let (service, handle) = ConductorService::new(
config.conductor.clone(),
memory.clone(),
provider.clone(),
observer.clone(),
sandbox.clone(),
tools.clone(),
app_config.clone(),
);
// Store handle for other components to use
conductor_handle.store(Arc::new(handle));
async move { service.run(shutdown).await; Ok(()) }
}
},
shutdown_tx.subscribe(),
).await;
```

The daemon supervisor handles:

- Automatic restart with exponential backoff if Conductor panics
- Health monitoring via the existing heartbeat system
- Graceful shutdown propagation via broadcast channel

### 5.2 Channel Integration (Chat → Conductor)

The existing channel message pipeline gains a routing check:

```rust
// In src/channels/mod.rs — modification to message handling

async fn handle_channel_message(msg: ChannelMessage, /* ... */) {
  // New: check if message is a task submission
  if let Some(task_request) = try_parse_task_command(&msg) {
    // Route to Conductor
    if let Some(handle) = conductor_handle.load().as_ref() {
      match handle.submit(task_request).await {
        Ok(task_id) => {
          channel.send(SendMessage {
            text: format!("Task accepted: {} — I'll work on it.", task_id.0),
            channel_id: msg.channel_id.clone(),
            thread_id: msg.thread_id.clone(),
          }).await.ok();
        }
        Err(e) => {
          channel.send(SendMessage {
            text: format!("Failed to submit task: {}", e),
            channel_id: msg.channel_id.clone(),
            thread_id: msg.thread_id.clone(),
          }).await.ok();
        }
      }
    }
    return; // Don't process as regular chat
  }

  // Existing: route to AgentLoop for conversational handling
  // ... (unchanged)
}

/// Parse `/task <description>` command from channel message
fn try_parse_task_command(msg: &ChannelMessage) -> Option<TaskRequest> {
  let text = msg.text.trim();
  if text.starts_with("/task ") {
    Some(TaskRequest {
      description: text.strip_prefix("/task ").unwrap().to_string(),
      origin: TaskOrigin::Chat {
        channel_name: "unknown".to_string(), // filled by caller
        channel_id: msg.channel_id.clone(),
        sender: msg.sender.clone(),
        thread_id: msg.thread_id.clone(),
      },
      priority: TaskPriority::Normal,
      context: None,
      workspace_hint: None,
      timeout: None,
      tags: vec![],
    })
  } else {
    None
  }
}
```

### 5.3 CLI Integration (`/task` command)

```rust
// New CLI subcommand: corvus task

/// Submit a task to the Conductor
#[derive(clap::Args)]
pub struct TaskCmd {
  /// Task description
  description: String,

  /// Priority level
  #[arg(short, long, default_value = "normal")]
  priority: String,

  /// Additional context
  #[arg(short, long)]
  context: Option<String>,

  /// Wait for completion and show results
  #[arg(short, long)]
  wait: bool,

  /// Workspace path or repo URL
  #[arg(short = 'w', long)]
  workspace: Option<String>,
}

/// List active tasks
#[derive(clap::Args)]
pub struct TaskListCmd {
  /// Filter by status
  #[arg(short, long)]
  status: Option<String>,
}

/// Check task status
#[derive(clap::Args)]
pub struct TaskStatusCmd {
  /// Task ID
  id: String,
}

/// Cancel a task
#[derive(clap::Args)]
pub struct TaskCancelCmd {
  /// Task ID
  id: String,
}
```

### 5.4 Gateway Integration (Dashboard)

New HTTP endpoints for the web dashboard:

```
POST   /api/conductor/tasks          Submit a new task
GET    /api/conductor/tasks          List tasks (with query filters)
GET    /api/conductor/tasks/:id      Get task details
DELETE /api/conductor/tasks/:id      Cancel a task
GET    /api/conductor/tasks/:id/steps/:step_id  Get step details
GET    /api/conductor/snapshot       Get conductor status snapshot
WS     /api/conductor/events         Real-time event stream (WebSocket)
```

### 5.5 Observer Integration

Conductor events flow through the existing observability pipeline:

```rust
// Every ConductorEvent is also emitted as an ObserverEvent
impl ConductorService {
  fn emit_event(&self, event: ConductorEvent) {
    // 1. Broadcast to subscribers (dashboard, channels)
    self.event_tx.send(event.clone()).ok();

    // 2. Record in observer for telemetry
    self.observer.record_event(&ObserverEvent::from(&event));
  }
}
```

New `ObserverEvent` variants to add:

```rust
pub enum ObserverEvent {
  // ... existing variants ...

  // Conductor events (NEW)
  ConductorTaskReceived { task_id: String, origin: String, domain: String },
  ConductorTaskPlanned { task_id: String, step_count: u32 },
  ConductorTaskCompleted { task_id: String, duration: Duration, steps_completed: u32 },
  ConductorTaskFailed { task_id: String, error: String, steps_completed: u32 },
  ConductorStepStarted { task_id: String, step_id: String, domain: String, performer: String },
  ConductorStepCompleted { task_id: String, step_id: String, duration: Duration },
  ConductorStepFailed { task_id: String, step_id: String, error: String, will_retry: bool },
  ConductorStallDetected { task_id: String, step_id: String, stall_duration: Duration },
  ConductorTickCompleted { active_tasks: u32, running_steps: u32 },
}
```

### 5.6 Cron Scheduler Integration

The existing `CronScheduler` gains a new job type:

```rust
// In src/cron/types.rs — extend existing enum

pub enum JobType {
  Shell { command: String },
  Agent { prompt: String, model: Option<String> },
  ConductorTask { description: String, domain: Option<String>, priority: Option<String> },  // NEW
}
```

When a `ConductorTask` cron job fires, it creates a `TaskRequest` with `TaskOrigin::Cron` and
submits it via `ConductorHandle::submit()`.

---

## 6. Task Lifecycle & State Machine

### 6.1 Task-Level State Machine

```
                    ┌──────────┐
          submit()  │ Received │
         ──────────►│          │
                    └────┬─────┘
                         │
                    plan()│
                         ▼
                    ┌──────────┐
                    │ Planning │──── PlanningError ────► Failed
                    │          │
                    └────┬─────┘
                         │
              plan ready │
                         ▼
                    ┌──────────┐
         ┌─────────│  Active   │◄────────────┐
         │         │          │              │
         │         └────┬─────┘              │
         │              │                    │
         │   all steps  │       step failed  │
         │   complete   │       (retryable)  │
         │              ▼                    │
         │         ┌──────────┐              │
         │         │Completed │              │
         │         └──────────┘              │
         │                                   │
         │  step failed (terminal)           │
         ▼                                   │
    ┌──────────┐                             │
    │  Failed  │                             │
    └──────────┘                             │
                                             │
    ┌──────────┐                             │
    │Cancelled │ ◄──── cancel() at any state │
    └──────────┘                             │
```

### 6.2 Step-Level State Machine

```
                    ┌──────────┐
                    │  Queued  │
                    └────┬─────┘
                         │
           deps check    │
                         ▼
              ┌──────────────────────┐
              │WaitingForDependency  │◄─── deps not yet met
              └──────────┬───────────┘
                         │
              all deps   │
              satisfied  │
                         ▼
                    ┌──────────┐
                    │Scheduled │
                    └────┬─────┘
                         │
              dispatch() │
                         ▼
                    ┌──────────┐
              ┌────►│ Running  │
              │     └────┬─────┘
              │          │
              │     ┌────┴──────────────┬──────────────────┐
              │     │         │         │                  │
              │  success   failure   cancel            approval
              │     │         │         │              required
              │     ▼         ▼         ▼                  │
              │ ┌─────────┐ ┌────────┐ ┌──────────┐        ▼
              │ │Completed│ │ Failed │ │Cancelled │  ┌───────────────────┐
              │ └─────────┘ └───┬────┘ └──────────┘  │WaitingForApproval │
              │                 │                     └────────┬──────────┘
              │  retries > 0    │                              │
              │                 ▼                     ┌────────┴────────┐
              │         ┌──────────────┐              │                 │
              └─────────│ RetryQueued  │           approved          denied/
                backoff │              │              │              timeout
                elapsed └──────────────┘              │                 │
                                                      ▼                 ▼
                                                  ┌────────┐      ┌────────┐
                                                  │Running │      │Failed  │
                                                  └────────┘      └────────┘
```

### 6.3 State Transition Rules

| From                   | To                     | Trigger                                | Side Effects                                  |
|------------------------|------------------------|----------------------------------------|-----------------------------------------------|
| `Queued`               | `WaitingForDependency` | Step has unresolved deps               | None                                          |
| `Queued`               | `Scheduled`            | No deps, or all deps met               | None                                          |
| `WaitingForDependency` | `Scheduled`            | All blocking deps completed            | Inherit dependency outputs                    |
| `WaitingForDependency` | `Cancelled`            | A blocking dep failed terminally       | Set reason = "dependency_failed"              |
| `Scheduled`            | `Running`              | Performer dispatch                     | Set started_at, assign performer_id           |
| `Running`              | `Completed`            | Performer returns Ok(outcome)          | Set completed_at, store artifacts, send nudge |
| `Running`              | `Failed`               | Performer returns Err, no retries left | Set error, cascade to dependents              |
| `Running`              | `RetryQueued`          | Performer returns Err, retries remain  | Increment attempt, compute backoff            |
| `Running`              | `Cancelled`            | Cancel command received                | Call performer.cancel()                       |
| `Running`              | `WaitingForApproval`   | Performer emits ApprovalRequired       | Pause step, notify user via origin channel    |
| `WaitingForApproval`   | `Running`              | User approves through origin channel   | Resume performer execution                    |
| `WaitingForApproval`   | `Failed`               | User denies or approval times out      | Set error = "approval_denied" or "approval_timeout" |
| `WaitingForApproval`   | `Cancelled`            | Cancel command received                | Call performer.cancel()                       |
| `RetryQueued`          | `Scheduled`            | Backoff elapsed                        | Reset for new attempt                         |

### 6.4 Dependency Resolution Algorithm

```rust
/// Called during reconcile phase when a step completes.
/// Finds all steps waiting on this step and checks if they can be unblocked.
fn resolve_dependencies(&self, completed_step_id: &StepId) {
  for (step_id, step_state) in self.store.iter_steps() {
    if let StepStatus::WaitingForDependency { blocked_by } = &step_state.status {
      if blocked_by.contains(completed_step_id) {
        let remaining: Vec<_> = blocked_by.iter()
          .filter(|id| *id != completed_step_id)
          .filter(|id| !self.store.is_step_completed(id))
          .cloned()
          .collect();

        if remaining.is_empty() {
          // All deps satisfied — promote to Scheduled
          self.store.update_step_status(step_id, StepStatus::Scheduled);
          self.nudge_tx.send(()).await.ok();
        } else {
          // Update remaining deps
          self.store.update_step_status(step_id,
                                        StepStatus::WaitingForDependency { blocked_by: remaining });
        }
      }
    }
  }
}

/// Called when a step fails terminally (no more retries).
/// Cascades cancellation to all transitively dependent steps.
fn cascade_failure(&self, failed_step_id: &StepId) {
  let mut to_cancel = vec![failed_step_id.clone()];
  let mut visited = HashSet::new();

  while let Some(step_id) = to_cancel.pop() {
    if !visited.insert(step_id.clone()) { continue; }

    for (dep_step_id, dep_step) in self.store.iter_steps() {
      if let StepStatus::WaitingForDependency { blocked_by } = &dep_step.status {
        if blocked_by.contains(&step_id) {
          self.store.update_step_status(dep_step_id,
                                        StepStatus::Cancelled { reason: "dependency_failed".into() });
          to_cancel.push(dep_step_id.clone());
        }
      }
    }
  }
}
```

---

## 7. Concurrency Model

### 7.1 How Conductor and AgentLoop Coexist

```
                    Corvus Tokio Runtime
                         │
         ┌───────────────┴──────────────────┐
         │                                  │
   tokio::spawn                       tokio::spawn
         │                                  │
   ┌─────▼──────┐                    ┌──────▼──────┐
   │ AgentLoop  │                    │  Conductor  │
   │  worker    │                    │  service    │
   │            │                    │             │
   │ Processes  │                    │ TickLoop    │
   │ short chat │                    │ 30s default │
   │ messages   │                    │             │
   │ ~ms        │                    │ Long tasks  │
   │            │                    │ ~min/hours  │
   └─────┬──────┘                    └──────┬──────┘
         │                                  │
         │     Shared Arc<T> resources      │
         └──────────►◄──────────────────────┘
              Memory, Provider, Config,
              Observer, Sandbox, Tools
```

**Critical rules:**

1. **AgentLoop has priority for latency-sensitive work.** The Conductor MUST NOT starve the
   AgentLoop. All performer work happens in `tokio::spawn` tasks, never inline in the tick loop.

2. **The tick loop is lightweight.** Reconcile/schedule/dispatch only update state and spawn tasks.
   No LLM calls or network I/O happen directly in the tick loop body.

3. **Performers are I/O-bound, not CPU-bound.** Coding performers launch subprocesses (Codex CLI).
   Research performers make HTTP calls. Browser performers control a headless browser. None of
   these block the Tokio runtime.

4. **If a performer MUST do CPU work** (e.g., parsing large files), it MUST use
   `tokio::task::spawn_blocking`.

5. **If a performer panics**, Tokio catches the panic at the `tokio::spawn` boundary. The
   `JoinHandle` returns `Err(JoinError)` and `JoinError::is_panic()` detects it. The panic is
   logged and the step is marked as Failed. The Conductor tick loop continues unaffected.
   **Note:** We do NOT use `AssertUnwindSafe` + `catch_unwind` — this pattern is technically
   incorrect for async Rust (futures are not `UnwindSafe` by design, and partial state corruption
   can occur). Tokio's spawn boundary already provides panic isolation.

### 7.2 Concurrency Limits

```rust
pub struct ConcurrencyLimits {
  /// Maximum total steps running across all performers
  pub global_max: usize,  // Default: 10

  /// Maximum steps per domain
  pub per_domain: HashMap<TaskDomain, usize>,
  // Defaults:
  //   Coding:   3  (each may use significant RAM/subprocesses)
  //   Research: 5  (lightweight HTTP/LLM calls)
  //   Browser:  2  (headless browser instances are heavy)
  //   System:   2  (safety — limit concurrent shell operations)
}
```

Enforcement via `tokio::sync::Semaphore`:

```rust
async fn dispatch_step(&self, step: &PlannedStep) -> anyhow::Result<()> {
  // Acquire global permit
  let _global_permit = self.global_semaphore
    .acquire()
    .await
    .map_err(|_| anyhow::anyhow!("Conductor shutting down"))?;

  // Acquire domain permit
  let domain_semaphore = self.domain_semaphores
    .get(&step.domain)
    .ok_or_else(|| anyhow::anyhow!("No performer for domain {:?}", step.domain))?;

  let _domain_permit = domain_semaphore
    .acquire()
    .await
    .map_err(|_| anyhow::anyhow!("Conductor shutting down"))?;

  // Spawn performer execution — permits released on drop when task completes
  let handle = tokio::spawn(async move {
    let _global = _global_permit;  // Move permits into task
    let _domain = _domain_permit;

    // No catch_unwind needed — Tokio catches panics at the spawn boundary.
    // If the performer panics, the JoinHandle returns Err(JoinError)
    // and JoinError::is_panic() == true. This is the correct pattern
    // for async Rust (AssertUnwindSafe + catch_unwind is unsound for futures).
    performer.execute(step, ctx, progress_tx).await
  });

  // The reconcile phase collects results from JoinHandles:
  //   match handle.await {
  //     Ok(Ok(outcome)) => mark step Completed,
  //     Ok(Err(e))      => mark step Failed (performer error),
  //     Err(join_err) if join_err.is_panic() => {
  //       let panic_msg = join_err.into_panic();
  //       mark step Failed with "Performer panicked: {panic_msg:?}"
  //       observer.record_error(...)
  //     }
  //     Err(join_err) => mark step Failed with "Task cancelled"
  //   }

  self.running_steps.insert(step.id.clone(), handle);
  Ok(())
}
```

### 7.3 Backpressure

If all semaphore permits are taken:

- New steps stay in `Scheduled` status
- The tick loop logs a warning: "All performer slots occupied, N steps queued"
- The next tick will attempt dispatch again
- The user receives a progress message: "Your task is queued — N tasks ahead"

### 7.4 Shutdown Sequence

```
1. Conductor receives shutdown signal via broadcast channel
2. Stop accepting new tasks (drain cmd_rx)
3. Stop polling sources
4. For each running step:
   a. Call performer.cancel(step_id)
   b. Wait up to 10s for graceful completion
   c. If still running after 10s, abort the tokio task
5. Persist all current state to SQLite
6. Emit ConductorStopped event
7. Return from run()
```

---

## 8. Configuration

### 8.1 config.toml (Typed Settings)

```toml
[conductor]
enabled = false                    # Must be explicitly enabled
tick_interval_ms = 30000           # 30 seconds
stall_timeout_ms = 300000          # 5 minutes
max_retries = 3
workspace_root = "~/.corvus/workspaces"

[conductor.planner]
model = "claude-sonnet-4-20250514"       # Model for task decomposition
temperature = 0.3                  # Low temp for structured planning
max_planning_time_ms = 30000       # 30s timeout for planning

[conductor.concurrency]
global_max = 10
coding_max = 3
research_max = 5
browser_max = 2
system_max = 2

[conductor.retry]
max_retries = 3
initial_backoff_ms = 5000          # 5s
max_backoff_ms = 300000            # 5min
backoff_multiplier = 2.0

[conductor.performers.coding]
model = "claude-sonnet-4-20250514"
max_iterations = 50
timeout_ms = 600000                # 10 min per step
tools = ["shell", "file_read", "file_write", "git_operations"]

[conductor.performers.research]
model = "claude-sonnet-4-20250514"
max_iterations = 30
timeout_ms = 300000                # 5 min per step
tools = ["web_search", "http_request", "memory_store", "memory_recall", "file_write"]

[conductor.performers.browser]
model = "claude-sonnet-4-20250514"
max_iterations = 20
timeout_ms = 300000
tools = ["browser", "browser_open", "screenshot", "http_request"]

[conductor.performers.system]
model = "claude-sonnet-4-20250514"
max_iterations = 20
timeout_ms = 180000                # 3 min — shorter for safety
tools = ["shell", "file_read", "file_write"]
approval_required = true           # ALL system actions need user approval
```

### 8.2 CONDUCTOR.md (Behavioral Prompt)

The `CONDUCTOR.md` file lives in the project root (or `~/.corvus/CONDUCTOR.md` for global config).
It is hot-reloaded on file change via `notify` crate filesystem watcher.

**Purpose:** Defines HOW the Conductor thinks about task decomposition, performer behavior, and
output quality. This is the "soul" of the Conductor — the typed config is the "body."

**Structure:**

```markdown
---
# YAML front matter for overrides (optional, merged with config.toml)
tick_interval_ms: 15000
---

# Conductor System Prompt

You are Corvus Conductor, an autonomous task orchestrator. You decompose complex
tasks into executable steps and coordinate specialized performers to complete them.

## Planning Rules

1. Always analyze the task before decomposing. If it's simple and single-domain,
   create a single step — don't over-decompose.
2. Identify dependencies between steps explicitly. Steps without dependencies
   can run in parallel.
3. Each step MUST have a clear, measurable expected output.
4. Prefer fewer, larger steps over many tiny steps.
5. Never create a step that requires human intervention mid-execution.

## Domain Guidelines

### Coding

- Use for: code changes, refactoring, writing tests, creating files
- Always create a git branch for non-trivial changes
- Run tests after code changes when possible

### Research

- Use for: analysis, documentation, web lookups, data gathering
- Store findings in memory for future reference
- Produce structured output (markdown, JSON)

### Browser

- Use for: web automation, UI testing, scraping, visual verification
- Always capture screenshots as evidence
- Respect robots.txt and rate limits

### System

- Use for: infrastructure, DevOps, file management, process control
- ALWAYS sandbox shell commands
- Require approval for destructive operations (rm -rf, service restart, etc.)

## Output Quality

- Every task completion MUST include a human-readable summary
- Include artifacts (files, diffs, screenshots) whenever produced
- Report what was done, what changed, and any concerns
```

**Hot-reload mechanism:**

```rust
impl ConductorService {
  fn watch_conductor_md(&self) -> notify::RecommendedWatcher {
    let planner = self.planner.clone();
    let mut watcher = notify::recommended_watcher(move |event| {
      if let Ok(notify::Event { kind: notify::EventKind::Modify(_), .. }) = event {
        if let Ok(content) = std::fs::read_to_string("CONDUCTOR.md") {
          planner.update_system_prompt(content);
          tracing::info!("CONDUCTOR.md hot-reloaded");
        }
      }
    }).unwrap();
    watcher.watch(Path::new("CONDUCTOR.md"), notify::RecursiveMode::NonRecursive).ok();
    watcher
  }
}
```

### 8.3 Configuration Precedence

```
CONDUCTOR.md front matter  >  config.toml [conductor]  >  defaults
```

Environment variable overrides follow existing Corvus convention:

```
CORVUS_CONDUCTOR_ENABLED=true
CORVUS_CONDUCTOR_TICK_INTERVAL_MS=15000
CORVUS_CONDUCTOR_GLOBAL_MAX=20
```

---

## 9. Deployment Shape & Migration Path

### 9.1 Phase 1-2: Same Process (MVP → Production)

```
┌─────────────────────────────────────────────────────┐
│                  corvus binary                      │
│                                                     │
│  ┌──────────────┐      mpsc/broadcast channels      │
│  │  Agent Loop  │ ◄──────────────────────────────┐  │
│  │   (queue)    │                                │  │
│  └──────────────┘                                │  │
│                                                  │  │
│  ┌──────────────────────────────────────────┐    │  │
│  │           Conductor                      │    │  │
│  │  tokio::spawn per performer              │────┘  │
│  │  tick loop = tokio::interval             │       │
│  └──────────────────────────────────────────┘       │
│                                                     │
│  Shared: Arc<Memory>, Arc<Sandbox>, Arc<Config>     │
└─────────────────────────────────────────────────────┘
```

**Why this is correct:**

- Performers are I/O-bound (subprocesses, HTTP) — no Tokio runtime contention
- Zero IPC overhead — `Arc<T>` sharing is free
- One binary, one deploy, one systemd unit
- Hot-reload CONDUCTOR.md affects everything instantly
- Direct access to Memory for cross-step context

**Mitigations for in-process risks:**

- `JoinHandle` panic detection around all performer spawns → panics don't crash Corvus
- `tokio::task::spawn_blocking` for any synchronous filesystem work
- Per-performer timeouts via `tokio::time::timeout`
- Daemon supervisor auto-restarts Conductor if it fails

### 9.2 When to Extract (Concrete Signals)

Extract the Conductor to a separate process when ANY of these are true:

1. Performers consume >50% of the machine's RAM, affecting AgentLoop latency
2. The team needs independent CI/CD for Conductor (different release cadence)
3. Multi-tenant requirement: multiple Corvus instances sharing one Conductor
4. Conductor needs to run on a different machine (GPU server for heavy LLM work)

### 9.3 Migration Path (Zero Rewrite)

The `ConductorHandle` trait enables this migration:

```rust
// Day 1 (MVP): LocalConductorHandle
//   - mpsc channel to in-process ConductorService
//   - Zero serialization overhead

// Day N (if needed): RemoteConductorHandle
//   - HTTP/Unix socket to separate conductor binary
//   - Same trait, different transport

// Corvus code NEVER changes — only the handle creation in daemon.rs:
let handle: Arc<dyn ConductorHandle> = if config.conductor.remote {
Arc::new(RemoteConductorHandle::new(config.conductor.url.clone()))
} else {
let (service, handle) = ConductorService::new(/* ... */);
tokio::spawn(service.run(shutdown));
Arc::new(handle)
};
```

---

## 10. Formal Specification

This section follows the project's spec convention: RFC 2119 keywords (MUST, SHALL, SHOULD, MAY)
and Given/When/Then scenarios.

### Requirement 1: Task Submission

The Conductor MUST accept task submissions from chat channels, CLI commands, dashboard, and
cron scheduler. Each submission MUST return a `TaskId` immediately without blocking on planning
or execution.

#### Scenario 1.1: Chat channel task submission

- GIVEN a user sends `/task Refactorizar el módulo de auth` in a Telegram chat
- WHEN the channel message handler detects the `/task` prefix
- THEN the system MUST create a `TaskRequest` with `TaskOrigin::Chat`
- AND submit it to the Conductor via `ConductorHandle::submit()`
- AND respond in the chat with the assigned `TaskId`
- AND the task status MUST be `Received`

#### Scenario 1.2: CLI task submission

- GIVEN a user runs `corvus task "Deploy staging environment" --priority high`
- WHEN the CLI parses the command
- THEN the system MUST create a `TaskRequest` with `TaskOrigin::Cli` and `TaskPriority::High`
- AND submit it to the Conductor
- AND print the `TaskId` to stdout

#### Scenario 1.3: Dashboard task submission

- GIVEN a user clicks "New Task" in the web dashboard and submits a description
- WHEN the gateway receives `POST /api/conductor/tasks`
- THEN the system MUST create a `TaskRequest` with `TaskOrigin::Dashboard`
- AND return the `TaskId` in the HTTP response with status 202 Accepted

#### Scenario 1.4: Cron-triggered task

- GIVEN a cron job of type `ConductorTask` fires on schedule
- WHEN the scheduler processes the job
- THEN the system MUST create a `TaskRequest` with `TaskOrigin::Cron`
- AND submit it to the Conductor
- AND log the submission

### Requirement 2: Task Planning

The Conductor MUST decompose composite tasks into ordered steps with an acyclic dependency graph.
Single-domain atomic tasks SHOULD bypass LLM planning and create a single-step plan.

#### Scenario 2.1: Composite task decomposition

- GIVEN a `TaskRequest` classified as `TaskDomain::Composite`
- WHEN the Planner processes it
- THEN it MUST make an LLM call with the CONDUCTOR.md system prompt
- AND produce a `TaskPlan` with two or more `PlannedStep`s
- AND the dependency graph MUST be a valid DAG (no cycles)
- AND each step MUST have a `TaskDomain` other than `Composite`
- AND the task status MUST transition from `Received` to `Planning` to `Active`

#### Scenario 2.2: Atomic task fast path (RuleBasedClassifier)

- GIVEN a `TaskRequest` with description "fix typo in README"
- WHEN the `RuleBasedClassifier` processes it
- THEN it MUST match keyword "fix" → `TaskDomain::Coding` with `Confidence::High`
- AND the Planner MUST skip the LLM call entirely
- AND it MUST create a single-step plan inheriting the task's domain and description
- AND total classification + plan creation MUST complete in under 10ms
- AND no network call MUST be made

#### Scenario 2.3: Planning failure

- GIVEN a `TaskRequest` that the Planner cannot decompose (LLM error, timeout, invalid DAG)
- WHEN planning fails
- THEN the task status MUST transition to `Failed` with a descriptive error
- AND the system MUST emit `ConductorEvent::TaskFailed`
- AND the originating source MUST be notified of the failure

#### Scenario 2.4: Dependency cycle detection

- GIVEN the Planner produces steps where step A depends on step B and step B depends on step A
- WHEN the plan is validated
- THEN the system MUST reject the plan with error "Dependency cycle detected"
- AND the Planner SHOULD retry once with explicit instructions to avoid cycles
- AND if retry also fails, the task MUST be marked Failed

### Requirement 3: Step Scheduling & Dispatch

The Conductor MUST schedule steps whose dependencies are satisfied and dispatch them to
appropriate Performers respecting concurrency limits.

#### Scenario 3.1: Independent steps run in parallel

- GIVEN a plan with steps A (no deps), B (no deps), and C (depends on A and B)
- WHEN the TickLoop schedules
- THEN steps A and B MUST both be dispatched concurrently
- AND step C MUST remain in `WaitingForDependency` until both A and B complete

#### Scenario 3.2: Concurrency limit enforcement

- GIVEN `coding_max = 3` and 3 coding steps are already running
- WHEN a 4th coding step becomes schedulable
- THEN it MUST remain in `Scheduled` status until a slot opens
- AND the system SHOULD log "Coding performer at capacity: 3/3"

#### Scenario 3.3: Dependency cascade on completion

- GIVEN step A completes, and steps B and C both depend only on A
- WHEN the TickLoop reconciles
- THEN both B and C MUST transition to `Scheduled`
- AND a nudge MUST be sent to trigger immediate dispatch without waiting for next tick

#### Scenario 3.4: Dependency cascade on failure

- GIVEN step A fails terminally (no retries remaining)
- WHEN the TickLoop reconciles
- THEN all steps transitively dependent on A MUST be cancelled with reason "dependency_failed"
- AND the task MUST be marked Failed if no alternative path exists

### Requirement 4: Performer Execution

Performers MUST execute steps within their configured timeout, report progress, and handle
cancellation gracefully.

#### Scenario 4.1: Successful step execution

- GIVEN a `PlannedStep` dispatched to `CodingPerformer`
- WHEN the performer completes successfully
- THEN it MUST return `StepOutcome` with `success: true`
- AND include any artifacts produced (files, diffs)
- AND include `output_context` for dependent steps to consume
- AND the step status MUST transition to `Completed`

#### Scenario 4.2: Step timeout

- GIVEN a step with `timeout = 600s` (10 minutes)
- WHEN the step has been running for 600 seconds without completion
- THEN the system MUST cancel the performer execution
- AND the step MUST be marked as Failed with error "Step timed out after 600s"
- AND retry policy MUST be evaluated

#### Scenario 4.3: Step retry with backoff

- GIVEN a step that failed with `attempt = 1` and `max_retries = 3`
- WHEN the TickLoop processes the failure
- THEN the step MUST transition to `RetryQueued` with attempt = 2
- AND the backoff MUST be `initial_backoff * multiplier^(attempt-1)` = 5s * 2^1 = 10s
- AND after 10s, the step MUST be re-scheduled

#### Scenario 4.4: Performer panic

- GIVEN a performer that panics during execution
- WHEN the `JoinHandle` returns `Err(JoinError)` with `is_panic() == true`
- THEN the reconcile phase MUST extract the panic payload via `into_panic()`
- AND the step MUST be marked as Failed with the panic message
- AND the TickLoop MUST continue operating normally (Tokio isolates panics at spawn boundary)
- AND the observer MUST record an error event
- AND the system MUST NOT use `AssertUnwindSafe` + `catch_unwind` (unsound for async futures)

### Requirement 5: Progress Reporting & Observability

The Conductor MUST emit events for all state transitions and provide real-time progress to the
originating source.

#### Scenario 5.1: Chat user receives progress

- GIVEN a task submitted from Telegram chat
- WHEN a step completes
- THEN the system MUST send a progress message to the originating chat thread
- AND the message MUST include: step description, status, and remaining steps count

#### Scenario 5.2: Dashboard real-time updates

- GIVEN a dashboard user connected via WebSocket to `/api/conductor/events`
- WHEN any `ConductorEvent` is emitted
- THEN the event MUST be delivered to the WebSocket within 1 second
- AND the event MUST include task_id, event type, and relevant payload

#### Scenario 5.3: Observer telemetry

- GIVEN any conductor event
- WHEN the event is emitted
- THEN a corresponding `ObserverEvent` MUST be recorded
- AND existing telemetry pipelines (prometheus, otel, logging) MUST receive it

### Requirement 6: Crash Recovery

The Conductor MUST recover gracefully from process crashes without losing task state.

#### Scenario 6.1: Recovery on startup

- GIVEN the Conductor crashed while tasks were active
- WHEN the Conductor restarts (via daemon supervisor)
- THEN it MUST load all incomplete tasks from SQLite
- AND steps in `Running` status MUST be reset to `Queued` (the performer is gone)
- AND steps in `Scheduled` status MUST be reset to `Queued` (the dispatch was interrupted)
- AND steps in `WaitingForApproval` MUST remain in `WaitingForApproval` (requires explicit user
  action — approval state survives crash)
- AND steps in `Completed` or `Failed` MUST retain their state
- AND the tick loop MUST resume scheduling
- AND the observer MUST record a recovery event

#### Scenario 6.2: SQLite persistence

- GIVEN a step transitions from `Scheduled` to `Running`
- WHEN the state is updated
- THEN the in-memory DashMap and SQLite MUST be updated atomically
- AND SQLite MUST use WAL mode for concurrent reads during writes

### Requirement 7: Configuration & Hot-Reload

#### Scenario 7.1: CONDUCTOR.md hot-reload

- GIVEN the Conductor is running
- WHEN the user modifies `CONDUCTOR.md`
- THEN the filesystem watcher MUST detect the change
- AND the Planner's system prompt MUST be updated within 5 seconds
- AND no running tasks MUST be affected (only future planning calls use the new prompt)
- AND the observer MUST log "CONDUCTOR.md reloaded"

#### Scenario 7.2: Configuration precedence

- GIVEN `CONDUCTOR.md` front matter sets `tick_interval_ms: 15000`
- AND `config.toml` sets `conductor.tick_interval_ms = 30000`
- WHEN the Conductor reads configuration
- THEN `CONDUCTOR.md` front matter MUST take precedence
- AND the tick interval MUST be 15000ms

#### Scenario 7.3: Disabled by default

- GIVEN a fresh Corvus installation
- WHEN the daemon starts
- THEN the Conductor MUST NOT start unless `conductor.enabled = true` in config
- AND no Conductor-related resources (TaskStore, WorkspaceManager) MUST be allocated

### Requirement 8: Security

#### Scenario 8.1: System performer sandboxing

- GIVEN a SystemPerformer executing a shell command
- WHEN the command is dispatched
- THEN it MUST be wrapped through `Sandbox::wrap_command()` — no exceptions
- AND if sandboxing is unavailable, the step MUST fail with "Sandbox required for system tasks"

#### Scenario 8.2: Approval for destructive operations

- GIVEN `approval_required = true` for the System performer
- WHEN the performer attempts a destructive operation
- THEN the step MUST pause and emit `ProgressEvent::ApprovalRequired`
- AND the step status MUST transition to `WaitingForApproval { reason, tool_name }`
- AND the user MUST be prompted through the originating channel
- AND the step MUST NOT proceed until approval is received or timeout occurs

#### Scenario 8.2a: Approval granted resumes step

- GIVEN a step in `WaitingForApproval` status
- WHEN the user approves through the originating channel
- THEN the step status MUST transition back to `Running`
- AND the performer MUST resume execution from where it paused
- AND the observer MUST record the approval event

#### Scenario 8.2b: Approval denied or timed out fails step

- GIVEN a step in `WaitingForApproval` status
- WHEN the user denies or the approval timeout (configurable, default 5 minutes) expires
- THEN the step MUST transition to `Failed` with error "approval_denied" or "approval_timeout"
- AND retry policy MUST be evaluated (the step MAY be retried if retries remain)
- AND the observer MUST record the denial/timeout event

#### Scenario 8.3: Workspace isolation

- GIVEN two concurrent tasks T1 and T2
- WHEN both create workspaces
- THEN each MUST have a unique directory under `workspace_root`
- AND no performer from T1 MUST be able to access T2's workspace
- AND workspace paths MUST be sanitized to prevent path traversal

### Requirement 9: Failure Mode Handling

The Conductor MUST handle known failure modes deterministically with observable outcomes.

#### Scenario 9.1: SQLite write failure halts affected task

- GIVEN a step transition requires SQLite persistence
- WHEN the SQLite write fails (disk full, corruption, I/O error)
- THEN the transition MUST NOT be committed to in-memory state
- AND the affected task MUST be halted (no further step dispatches)
- AND the system MUST emit a critical observer event with the storage fault details
- AND unaffected tasks MUST continue operating normally

#### Scenario 9.2: Concurrency starvation detection

- GIVEN per-domain concurrency limits are fully consumed for an extended period
- WHEN eligible steps have been queued beyond a stall threshold
- THEN the system MUST detect the stall via queue depth and latency metrics
- AND the system MUST emit a `stall_detected` scheduler health event
- AND per-domain fairness MUST be enforced to prevent one domain from monopolizing slots

#### Scenario 9.3: Unauthorized task API access is rejected

- GIVEN a client attempts to use task API endpoints (gateway task CRUD, event stream)
- WHEN the client has not satisfied the existing pairing/auth policy
- THEN the request MUST be rejected with 401 or 403 status
- AND an audit event MUST be recorded with the rejected client context
- AND rejection MUST NOT leak internal state or task details

#### Scenario 9.4: Least-privilege sandbox scope enforcement

- GIVEN a system step has declared filesystem or process needs
- WHEN sandbox policy is resolved for that step
- THEN the runtime MUST apply only the minimum privileges required by policy
- AND privileges beyond declared needs MUST NOT be granted implicitly
- AND the sandbox scope MUST be narrower than or equal to the security policy defaults

---

## 11. Phased Implementation Plan

### Phase 1: Foundation (1 session)

**Goal:** Core types, TaskStore, ConductorConfig, daemon integration scaffold.

| #   | Task                                           | TDD       | Notes                                        |
|-----|------------------------------------------------|-----------|----------------------------------------------|
| 1.1 | Create `src/conductor/mod.rs` module structure | -         | Module declaration in lib.rs                 |
| 1.2 | Define all types from Section 4.1              | RED/GREEN | Unit tests for serialization round-trip      |
| 1.3 | Implement `TaskStore` with DashMap + SQLite    | RED/GREEN | Test CRUD, state transitions, crash recovery |
| 1.4 | Implement `ConductorConfig` with defaults      | RED/GREEN | Test config parsing from TOML                |
| 1.5 | Define `ConductorHandle` trait                 | -         | Trait only, no impl yet                      |
| 1.6 | Add `[conductor]` section to config schema     | -         | Extend existing schema.rs                    |
| 1.7 | Add `conductor` worker to daemon supervisor    | -         | Scaffold, returns Ok immediately             |

**Verification:** `make test` passes, `make build` succeeds, TaskStore CRUD works.

### Phase 2: Tick Loop & Performer Pool (1 session)

**Goal:** The heartbeat of the Conductor — scheduling, dispatch, reconciliation.

| #   | Task                                                       | TDD       | Notes                                      |
|-----|------------------------------------------------------------|-----------|--------------------------------------------|
| 2.1 | Implement `TickLoop` with reconcile/schedule/dispatch      | RED/GREEN | Test with mock performers                  |
| 2.2 | Implement `PerformerPool` with semaphore-based concurrency | RED/GREEN | Test limit enforcement                     |
| 2.3 | Implement `ConductorService::run()` main loop              | RED/GREEN | Test select! behavior                      |
| 2.4 | Implement `LocalConductorHandle`                           | RED/GREEN | Test submit/cancel/status through channels |
| 2.5 | Implement nudge mechanism for reactive dispatch            | RED/GREEN | Test cascading unblock                     |
| 2.6 | Implement dependency resolution algorithm                  | RED/GREEN | Test DAG traversal, cycle detection        |
| 2.7 | Implement failure cascade                                  | RED/GREEN | Test transitive cancellation               |

**Verification:** Full tick cycle test with mocked performers, dependency resolution tests.

### Phase 3: Planner & Classifier (1 session)

**Goal:** LLM-powered task decomposition with dependency graphs, plus fast-path classifier.

**Rationale for ordering:** The Planner defines `PlannedStep` structure that Performers consume.
Building Planner before Performers prevents Phase 4 tests from hardcoding `PlannedStep` fields
that don't match what the real Planner produces.

| #   | Task                                                | TDD       | Notes                   |
|-----|-----------------------------------------------------|-----------|-------------------------|
| 3.1 | Implement `RuleBasedClassifier` (fast-path)         | RED/GREEN | Test keyword matching, confidence levels |
| 3.2 | Implement `LlmClassifier` for ambiguous tasks       | RED/GREEN | Test with mock provider |
| 3.3 | Implement `ChainedClassifier` (rule→LLM fallback)   | RED/GREEN | Test chain behavior     |
| 3.4 | Implement `Planner` struct with LLM call            | RED/GREEN | Test with mock provider |
| 3.5 | Implement plan validation (DAG check, domain check) | RED/GREEN | Test cycle detection    |
| 3.6 | Implement atomic task fast-path (single-step plan)  | RED/GREEN | Test bypass behavior, <10ms |
| 3.7 | Implement cost estimation                           | RED/GREEN |                         |
| 3.8 | Implement CONDUCTOR.md prompt loading               | RED/GREEN | Test file parsing       |

**Verification:** Planner produces valid plans for composite and atomic tasks. Fast-path resolves
simple tasks without network calls.

### Phase 4: Performers (1 session)

**Goal:** Four specialized performers using existing Agent infrastructure.

| #   | Task                                                  | TDD       | Notes                       |
|-----|-------------------------------------------------------|-----------|-----------------------------|
| 4.1 | Define `Performer` trait                              | -         | As specified in Section 4.2 |
| 4.2 | Implement `CodingPerformer`                           | RED/GREEN | Test with mock Agent        |
| 4.3 | Implement `ResearchPerformer`                         | RED/GREEN | Test with mock Agent        |
| 4.4 | Implement `BrowserPerformer`                          | RED/GREEN | Test with mock Agent        |
| 4.5 | Implement `SystemPerformer` with mandatory sandboxing | RED/GREEN | Test sandbox enforcement    |
| 4.6 | Implement `PerformerContext` construction             | -         | Wire up Arc<T> sharing      |
| 4.7 | Implement progress reporting via mpsc channel         | RED/GREEN | Test progress events        |

**Verification:** Each performer can execute a PlannedStep (from real Planner) and return StepOutcome.

### Phase 5: Sources & Sinks (1 session)

**Goal:** Connect inputs and outputs to existing infrastructure.

| #   | Task                                            | TDD       | Notes                         |
|-----|-------------------------------------------------|-----------|-------------------------------|
| 5.1 | Implement `SourceRouter`                        | RED/GREEN | Test routing logic            |
| 5.2 | Implement channel integration (`/task` command) | RED/GREEN | Test message parsing          |
| 5.3 | Implement CLI `corvus task` subcommand          | RED/GREEN | Test arg parsing              |
| 5.4 | Implement Gateway HTTP endpoints                | RED/GREEN | Test CRUD endpoints           |
| 5.5 | Implement cron `ConductorTask` job type         | RED/GREEN | Test job dispatch             |
| 5.6 | Implement channel reply sink (progress to user) | RED/GREEN | Test reply formatting         |
| 5.7 | Implement `WorkspaceManager`                    | RED/GREEN | Test create/cleanup lifecycle |

**Verification:** End-to-end: `/task` in channel → plan → execute → reply.

### Phase 6: Observability & Polish (1 session)

**Goal:** Telemetry, dashboard integration, hot-reload, crash recovery.

| #   | Task                                            | TDD       | Notes                             |
|-----|-------------------------------------------------|-----------|-----------------------------------|
| 6.1 | Add `ObserverEvent` variants for conductor      | RED/GREEN | Test event emission               |
| 6.2 | Implement ConductorEvent → ObserverEvent bridge | RED/GREEN | Test mapping                      |
| 6.3 | Implement WebSocket event stream for dashboard  | RED/GREEN | Test real-time delivery           |
| 6.4 | Implement CONDUCTOR.md hot-reload via notify    | RED/GREEN | Test file watch                   |
| 6.5 | Implement crash recovery (SQLite → DashMap)     | RED/GREEN | Test recovery scenarios           |
| 6.6 | Implement graceful shutdown sequence            | RED/GREEN | Test cleanup                      |
| 6.7 | Add Prometheus metrics for conductor            | -         | active_tasks, step_duration, etc. |

**Verification:** Full integration test: submit → plan → execute → observe → recover.

---

## 12. Decisions (formerly Open Questions)

All questions were reviewed and closed by the architecture team on 2026-03-07.

### Q1: Should the Planner use the same Provider as Performers?

**Context:** The Planner needs structured JSON output (step list + dependency graph). Not all
models are equally good at this. Performers may need different models for different domains.

**Recommendation:** Separate config. `conductor.planner.model` for planning,
`conductor.performers.<domain>.model` for execution. This allows using a cheap, fast model for
planning and a more capable model for actual work.

**Decision:** ACCEPTED. Separate model config per component. `conductor.planner.model` for planning,
`conductor.performers.<domain>.model` for execution. This is already reflected in the config schema
(Section 8.1).

### Q2: Should `/task` in chat channels be the only trigger, or should the Conductor also intercept natural language task intent?

**Context:** A user saying "Can you refactor the auth module?" in chat is a task, but they didn't
use `/task`. Should the Conductor have an intent classifier that intercepts these?

**Recommendation:** MVP uses explicit `/task` prefix only. Phase 2 can add an intent classifier
that asks the user "Should I run this as a background task?" before routing to Conductor. This
avoids surprising users and keeps the existing chat UX intact.

**Decision:** ACCEPTED. MVP uses explicit `/task` prefix only. Natural language intent detection is
deferred to a future phase. When added, it MUST ask for confirmation ("Run as background task?")
before routing to Conductor — never silently intercept.

### Q3: What happens when a Performer needs user input mid-execution?

**Context:** A CodingPerformer might discover it needs clarification. A SystemPerformer might need
approval for a destructive command.

**Recommendation:** The step pauses (status = `WaitingForApproval`), emits a progress event with
the question, and the user replies through the originating channel. The Conductor routes the reply
back to the performer. This requires a "reply channel" mechanism in `PerformerContext`.

**Decision:** ACCEPTED. Steps pause via `WaitingForApproval` status (now a first-class state — see
Section 4.1, 6.2, 6.3). The performer emits `ProgressEvent::ApprovalRequired`, the originating
channel prompts the user, and a reply channel in `PerformerContext` routes the response back.
Approval timeout is configurable (default 5 minutes).

### Q4: Should step artifacts persist beyond task completion?

**Context:** Steps produce artifacts (analysis docs, diffs, screenshots). Should these survive
after the task is marked complete? If so, for how long?

**Recommendation:** Artifacts persist in SQLite + workspace directory for 7 days (configurable).
After that, workspace is cleaned up but SQLite metadata (without content) is retained for audit.

**Decision:** ACCEPTED. 7-day retention for workspace + artifact content (configurable via
`conductor.artifact_retention_days`). SQLite metadata (kind, description, step_id) persists
indefinitely for audit trail. Workspace cleanup runs as a periodic task in the TickLoop.

### Q5: Should the Conductor support task templates?

**Context:** Users may want to define reusable task templates. "Deploy staging" always means the
same 5 steps. Templates would skip planning.

**Recommendation:** Out of scope for MVP. Note it in the roadmap for Phase 3.

**Decision:** ACCEPTED. Out of scope for MVP. Added to post-MVP roadmap. Templates will be YAML
files in `.corvus/templates/` that define pre-built TaskPlans.

### Q6: How should the Conductor handle tasks that span multiple repositories?

**Context:** "Update the auth module in backend AND the login page in frontend" touches two repos.

**Recommendation:** MVP treats this as two separate workspaces within one task. The WorkspaceManager
supports multiple workspace directories per task. The Planner can assign different workspace hints
per step.

**Decision:** ACCEPTED. WorkspaceManager supports multiple workspace directories per task. The
`PlannedStep` struct already has access to workspace hints via `PerformerContext.workspace`. The
Planner assigns per-step workspace paths when multiple repos are detected.

### Q7: Should performers share conversation context between steps?

**Context:** Step A (research) produces an analysis. Step B (coding) needs that analysis. Currently,
this is passed via `output_context` in `StepOutcome`. But should performers also share a persistent
conversation history (like a shared chat thread)?

**Recommendation:** MVP uses `output_context` only (explicit data passing). This is simpler and
more predictable. A shared conversation thread can lead to context window bloat and unexpected
behavior. If needed later, Memory can serve as the shared context store.

**Decision:** ACCEPTED. MVP uses explicit `output_context` passing only. This keeps step boundaries
clean and context windows predictable. Post-MVP, `Arc<Memory>` can serve as an opt-in shared
context store for steps that need richer inter-step communication.

### Q8: Rate limiting for LLM calls across concurrent performers

**Context:** 5 performers running simultaneously could burn through API rate limits fast.

**Recommendation:** Implement a shared `RateLimiter` (token bucket) in `PerformerContext` that all
performers respect. This uses the existing `Provider` infrastructure — the resilient provider
already has retry logic for 429 responses. Add a Conductor-level token budget that's checked
before dispatch.

**Decision:** ACCEPTED. Leverage the existing resilient `Provider` with 429 retry logic as the
primary defense. Add a Conductor-level `token_budget_per_tick` config (optional) as a secondary
safeguard. The existing `MissionGovernance.budget` pattern provides the model for this.

### Q9: Should the Conductor integrate with the existing MissionCoordinator?

**Context:** The `MissionCoordinator` already has a state machine for autonomous multi-step
execution with governance (budget, SLA, max steps). Should the Conductor USE the
MissionCoordinator internally, or is it a parallel system?

**Recommendation:** They are complementary, not overlapping:

- `MissionCoordinator` = single-agent, multi-checkpoint, conversation-driven
- `Conductor` = multi-agent, multi-domain, task-driven

The Conductor MAY create missions for complex single-domain steps (a coding step that needs
multiple checkpoints). But the Conductor itself is not a mission — it's a higher-level orchestrator.

Long-term, consider making MissionCoordinator a performer-internal detail.

**Decision:** ACCEPTED. Complementary systems, not overlapping. For MVP, Performers use `Agent`
directly (not `MissionCoordinator`). Post-MVP, complex performers MAY wrap their Agent execution
in a `MissionCoordinator` for governance (budget limits, checkpoint-based progress). The Conductor
never uses MissionCoordinator at the task level.

### Q10: Naming — "Conductor" or something else?

**Context:** "Conductor" implies orchestra/music. Alternatives: "Orchestrator", "Dispatcher",
"TaskEngine", "Foreman".

**Recommendation:** Keep "Conductor" — it's evocative, distinctive, and already used in all
design docs. The orchestra metaphor (conductor, performers, composition) is intuitive.

**Decision:** ACCEPTED. Name is "Conductor". The orchestra metaphor (Conductor → Performers →
Composition) provides a natural, intuitive vocabulary for the entire subsystem.

---

## Appendix A: Module Structure

```
src/conductor/
├── mod.rs              # Public API, re-exports
├── types.rs            # All types from Section 4.1
├── traits.rs           # ConductorHandle, Performer, Source, TaskClassifier
├── events.rs           # ConductorEvent, ConductorCmd, ProgressEvent
├── service.rs          # ConductorService (main orchestrator)
├── tick_loop.rs        # TickLoop with reconcile/schedule/dispatch
├── task_store.rs       # TaskStore (DashMap + SQLite)
├── planner.rs          # Planner (LLM decomposition)
├── classifier.rs       # RuleBasedClassifier, LlmClassifier
├── performer/
│   ├── mod.rs          # PerformerPool
│   ├── coding.rs       # CodingPerformer
│   ├── research.rs     # ResearchPerformer
│   ├── browser.rs      # BrowserPerformer
│   └── system.rs       # SystemPerformer
├── source/
│   ├── mod.rs          # SourceRouter
│   ├── channel.rs      # ChannelSource
│   ├── cli.rs          # CliSource
│   └── cron.rs         # CronSource
├── workspace.rs        # WorkspaceManager
├── config.rs           # ConductorConfig parsing
└── recovery.rs         # Crash recovery from SQLite
```

## Appendix B: New Dependencies

```toml
# Addition to Cargo.toml

# Already in use (no new deps):
# tokio, async-trait, serde, serde_json, rusqlite, dashmap, uuid, chrono, tracing, anyhow

# New:
notify = "6"          # Filesystem watcher for CONDUCTOR.md hot-reload
```

Only ONE new dependency. Everything else reuses existing crates.

## Appendix C: Glossary

| Term                 | Definition                                                        |
|----------------------|-------------------------------------------------------------------|
| **Conductor**        | The top-level orchestrator service that manages task lifecycle    |
| **Task**             | A user-submitted unit of work that may be decomposed into steps   |
| **Step**             | An atomic unit of execution assigned to a single Performer        |
| **Performer**        | A domain-specialized executor (Coding, Research, Browser, System) |
| **Planner**          | The component that decomposes tasks into steps via LLM            |
| **TickLoop**         | The periodic scheduler that reconciles, schedules, and dispatches |
| **TaskStore**        | The persistence layer for task and step state                     |
| **SourceRouter**     | The ingestion layer that normalizes input from multiple sources   |
| **WorkspaceManager** | Manages filesystem lifecycle per task                             |
| **Artifact**         | An output produced by a step (file, diff, screenshot, analysis)   |
| **Nudge**            | A signal that triggers immediate scheduling after step completion |
| **CONDUCTOR.md**     | The behavioral prompt file defining how the Conductor thinks      |

---

*End of document. APPROVED by architecture team — ready for implementation.*
