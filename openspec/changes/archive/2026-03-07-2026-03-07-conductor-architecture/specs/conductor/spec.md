# Delta for Conductor Runtime

## ADDED Requirements

### Requirement: Explicit Task Submission and Routing Boundaries

The runtime MUST accept explicit task submissions from approved MVP surfaces (`/task` channel command,
CLI task commands, gateway task APIs, and cron `ConductorTask`) and MUST keep non-task
conversational traffic on existing AgentLoop paths.

#### Scenario: Explicit `/task` message routes to Conductor

- GIVEN Conductor is enabled and a channel message starts with an explicit `/task` command
- WHEN channel routing evaluates the message
- THEN the runtime MUST submit a `TaskRequest` to Conductor and return a task acceptance or rejection
  result
- AND the message MUST NOT be processed by the regular chat turn path.

#### Scenario: Regular chat remains on AgentLoop

- GIVEN Conductor is enabled and a channel message does not use an explicit task submission contract
- WHEN routing evaluates the message
- THEN the runtime MUST preserve existing AgentLoop chat behavior
- AND Conductor MUST NOT intercept or mutate the conversational request.

### Requirement: Secure-by-Default Enablement and Fail-Closed Startup

Conductor execution MUST be disabled by default and runtime startup MUST fail closed for invalid or
unsafe conductor configuration values.

#### Scenario: Default configuration keeps Conductor inactive

- GIVEN runtime configuration does not explicitly enable Conductor
- WHEN the runtime starts
- THEN the Conductor worker MUST remain inactive
- AND existing non-Conductor runtime contracts MUST remain unchanged.

#### Scenario: Invalid conductor configuration is rejected

- GIVEN conductor configuration contains invalid security, timeout, or concurrency values
- WHEN configuration validation runs
- THEN the runtime MUST reject invalid values with structured diagnostics
- AND the runtime MUST NOT start Conductor in a degraded unsafe mode.

### Requirement: Least-Privilege Sandboxing for System and Risky Actions

System-domain and other risky execution steps MUST run through sandbox-enforced wrappers with
least-privilege defaults, and any unsandboxed execution path MUST be denied.

#### Scenario: Unsandboxed system execution is blocked

- GIVEN a planned step requires system command execution
- WHEN performer execution is about to launch
- THEN the runtime MUST require sandbox wrapping for the command path
- AND if sandbox wrapping is unavailable, execution MUST fail closed with a structured denial.

#### Scenario: Sandbox scope is least privilege

- GIVEN a system step has declared filesystem or process needs
- WHEN sandbox policy is resolved for that step
- THEN the runtime MUST apply only the minimum privileges required by policy
- AND privileges beyond policy MUST NOT be granted implicitly.

### Requirement: Approval-Gated High-Risk Operations

Destructive, high-risk, or policy-unknown actions MUST require explicit approval before execution.
Approval wait, denial, and timeout outcomes MUST be deterministic and observable.

#### Scenario: Approval-required step pauses and resumes

- GIVEN a step is classified as requiring approval
- WHEN execution reaches the approval gate
- THEN the step MUST transition to `WaitingForApproval` with reason metadata
- AND execution MUST resume only after an explicit allow decision is recorded.

#### Scenario: Approval timeout or denial fails closed

- GIVEN a step is in `WaitingForApproval`
- WHEN approval is denied or approval timeout expires
- THEN the runtime MUST transition the step to a terminal denied or failed state
- AND dependent execution MUST NOT continue through an unauthorized bypass path.

### Requirement: Fair Scheduling, Bounded Concurrency, and Backpressure

Conductor scheduling MUST enforce global and per-domain concurrency ceilings, maintain fair
progress across eligible tasks, and apply bounded backpressure when intake exceeds capacity.

#### Scenario: Concurrency caps are enforced under mixed load

- GIVEN multiple ready steps across domains exceed configured limits
- WHEN scheduling and dispatch run
- THEN the runtime MUST enforce both global and per-domain caps before dispatch
- AND tasks outside available capacity MUST remain queued without starvation of older eligible work.

#### Scenario: Intake pressure is handled without unbounded growth

- GIVEN submission rate exceeds executable throughput for a sustained period
- WHEN intake buffers reach configured limits
- THEN the runtime MUST apply deterministic backpressure by queueing submissions with bounded
  buffer capacity
- AND the runtime MUST NOT reject or silently drop submissions under normal backpressure
- AND the runtime MUST avoid unbounded memory growth.

### Requirement: Latency Protections via Planner Fast Path and Bounded Slow Path

The planner MUST use a no-network fast path for high-confidence single-domain tasks and MUST bound
slow-path planning latency for composite or ambiguous work.

#### Scenario: Fast path avoids planner network call

- GIVEN a submission matches high-confidence rule-based classification for a single domain
- WHEN planning executes
- THEN the runtime MUST generate a valid single-step plan without external planner network calls
- AND planning latency MUST stay within configured fast-path budget.

#### Scenario: Slow path is latency-bounded

- GIVEN a submission requires composite or low-confidence planning
- WHEN the planner uses an external model
- THEN planning MUST complete, timeout, or fail within configured latency bounds
- AND timeout or failure MUST return a structured non-success result without blocking scheduler
  progress.

### Requirement: Durable State Transitions and Deterministic Crash Recovery

Task and step transitions MUST be persisted atomically across in-memory hot state and SQLite WAL
state, with deterministic recovery for incomplete work after restart.

#### Scenario: Crash recovery re-queues interrupted running steps

- GIVEN one or more steps were in `Running` state before process termination
- WHEN Conductor restarts and recovery runs
- THEN those steps MUST transition to a recoverable queued state according to policy
- AND no step MAY remain permanently in a stale `Running` state.

#### Scenario: Dependency failure propagation is deterministic

- GIVEN a step fails terminally after retries are exhausted
- WHEN dependent steps are evaluated
- THEN dependent steps MUST transition to deterministic cancellation or blocked terminal outcomes
- AND task terminal status MUST reflect the dependency failure cause.

### Requirement: Runtime Contract Compatibility and Additive Interfaces

Conductor integration MUST preserve compatibility with existing runtime contracts. New CLI and
gateway task interfaces MUST be additive and MUST NOT break existing non-task endpoints,
administrative contracts, or conversational semantics.

#### Scenario: Existing gateway and admin contracts remain stable

- GIVEN clients use existing gateway health, metrics, webhook, and admin endpoints
- WHEN Conductor-enabled runtime is deployed
- THEN existing endpoint request/response behavior MUST remain backward compatible
- AND new Conductor endpoints MUST be additive rather than breaking existing routes.

#### Scenario: Existing CLI and channel non-task behavior remains unchanged

- GIVEN users execute existing CLI commands and non-task channel interactions
- WHEN Conductor is present
- THEN prior command semantics and non-task interaction flows MUST remain behaviorally stable
- AND task-specific behavior MUST activate only through explicit task interfaces.

### Requirement: Task Planning and Decomposition

The Conductor MUST decompose composite tasks into ordered steps with an acyclic dependency graph.
Single-domain atomic tasks SHOULD bypass LLM planning and create a single-step plan via rule-based
fast-path classification.

#### Scenario: Composite task decomposition produces valid DAG

- GIVEN a `TaskRequest` classified as `Composite`
- WHEN the Planner processes it
- THEN it MUST produce a `TaskPlan` with two or more `PlannedStep`s
- AND the dependency graph MUST be a valid DAG (no cycles)
- AND each step MUST have a domain other than `Composite`
- AND the task status MUST transition from `Received` to `Planning` to `Active`.

#### Scenario: DAG cycle detection rejects invalid plan

- GIVEN the Planner produces steps where step A depends on step B and step B depends on step A
- WHEN the plan is validated
- THEN the system MUST reject the plan with error "Dependency cycle detected"
- AND the Planner SHOULD retry once with explicit instructions to avoid cycles
- AND if retry also fails, the task MUST be marked Failed.

#### Scenario: Planning failure marks task terminal

- GIVEN a `TaskRequest` that the Planner cannot decompose (LLM error, timeout, invalid DAG)
- WHEN planning fails
- THEN the task status MUST transition to `Failed` with a descriptive error
- AND the originating source MUST be notified of the failure
- AND no partial execution MUST occur.

### Requirement: Performer Execution with Timeout, Retry, and Panic Safety

Performers MUST execute steps within their configured timeout, apply retry policy on transient
failures, and isolate panics without crashing the runtime.

#### Scenario: Step timeout cancels execution

- GIVEN a step with a configured timeout
- WHEN the step has been running past its timeout without completion
- THEN the system MUST cancel the performer execution
- AND the step MUST be marked as Failed with error indicating timeout
- AND retry policy MUST be evaluated.

#### Scenario: Step retry with exponential backoff

- GIVEN a step that failed with `attempt = 1` and retries remaining
- WHEN the scheduler processes the failure
- THEN the step MUST transition to `RetryQueued` with incremented attempt
- AND the backoff MUST follow `initial_backoff * multiplier^(attempt-1)` with jitter
- AND after backoff elapses, the step MUST be re-scheduled.

#### Scenario: Performer panic is isolated and recorded

- GIVEN a performer that panics during execution
- WHEN the `JoinHandle` returns a panic error
- THEN the step MUST be marked as Failed with the panic message
- AND the tick loop MUST continue operating normally
- AND the observer MUST record an error event
- AND the system MUST NOT use `AssertUnwindSafe` + `catch_unwind` (unsound for async futures).

### Requirement: Progress Reporting to Originating Source

The Conductor MUST emit events for all state transitions and provide real-time progress to the
originating source through its native surface.

#### Scenario: Chat user receives step progress

- GIVEN a task submitted from a chat channel
- WHEN a step completes
- THEN the system MUST send a progress message to the originating chat thread
- AND the message MUST include step description, status, and remaining steps count.

#### Scenario: Dashboard receives real-time events via WebSocket

- GIVEN a dashboard user connected via WebSocket to `/api/conductor/events`
- WHEN any `ConductorEvent` is emitted
- THEN the event MUST be delivered to the WebSocket within 1 second
- AND the event MUST include task_id, event type, and relevant payload.

#### Scenario: Observer telemetry pipeline receives conductor events

- GIVEN any conductor event
- WHEN the event is emitted
- THEN a corresponding `ObserverEvent` MUST be recorded
- AND existing telemetry pipelines (prometheus, otel, logging) MUST receive it.

### Requirement: Configuration Hot-Reload

The Conductor MUST support hot-reload of `CONDUCTOR.md` behavioral prompt without restarting the
runtime or affecting running tasks.

#### Scenario: CONDUCTOR.md change updates planner prompt

- GIVEN the Conductor is running
- WHEN the user modifies `CONDUCTOR.md`
- THEN the filesystem watcher MUST detect the change
- AND the Planner's system prompt MUST be updated within 5 seconds
- AND no running tasks MUST be affected (only future planning calls use the new prompt)
- AND the observer MUST log the reload event.

#### Scenario: Configuration precedence is respected

- GIVEN `CONDUCTOR.md` front matter sets `tick_interval_ms: 15000`
- AND `config.toml` sets `conductor.tick_interval_ms = 30000`
- WHEN the Conductor reads configuration
- THEN `CONDUCTOR.md` front matter MUST take precedence
- AND the tick interval MUST be 15000ms.

### Requirement: Workspace Isolation Between Concurrent Tasks

Concurrent tasks MUST have isolated filesystem workspaces with no cross-task access.

#### Scenario: Concurrent tasks have unique isolated workspaces

- GIVEN two concurrent tasks T1 and T2
- WHEN both create workspaces
- THEN each MUST have a unique directory under `workspace_root`
- AND no performer from T1 MUST be able to access T2's workspace
- AND workspace paths MUST be sanitized to prevent path traversal.
