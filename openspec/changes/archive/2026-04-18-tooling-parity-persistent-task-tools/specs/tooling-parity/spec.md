# Delta for Tooling Parity

## ADDED Requirements

### Requirement: Persistent Task Record Model and Slice Boundaries

The system MUST expose a persistent task record model for this tooling-parity slice.

Each persisted task record MUST include exactly these minimum public fields:
- `id`
- `title`
- `description`
- `status`
- `priority`
- `session_id` (optional)
- `created_at`
- `updated_at`

The public `id` MUST be an opaque UUID string. The `status` field MUST allow only `pending`,
`in_progress`, `completed`, or `cancelled`. The `priority` field MUST allow only `low`,
`medium`, or `high`.

Tasks in this slice MUST be global runtime tasks that MAY optionally carry a `session_id`
association. `session_id` association MUST NOT change the global task identity model.

This slice MUST NOT add reopen semantics, subtasks, dependencies, assignees, due dates, comments,
tags, or other project-management features beyond the minimum record defined above.

#### Scenario: created task records use the approved minimal model

- GIVEN a valid `TaskCreate` request
- WHEN the task is created successfully
- THEN the returned task MUST include `id`, `title`, `description`, `status`, `priority`,
  `created_at`, and `updated_at`
- AND `session_id` MUST be present only when the caller supplied one
- AND `id` MUST be a valid UUID string
- AND `status` MUST equal one of the approved status values
- AND `priority` MUST equal one of the approved priority values

#### Scenario: slice rejects unsupported task-management features

- GIVEN a caller attempts to create or update a task with `subtasks`, `dependencies`, or reopen
  semantics
- WHEN the request is validated
- THEN the request MUST fail validation
- AND the response MUST state that the field or behavior is unsupported in this slice

### Requirement: `TaskCreate` MUST Create Persistent Tasks

The system MUST expose a native tool named `TaskCreate` that creates a persistent task record.

`TaskCreate` MUST accept:
- `title` as a required non-empty string;
- `description` as an optional string that defaults to the empty string when omitted;
- `priority` as an optional enum with values `low`, `medium`, or `high`, defaulting to `medium`
  when omitted;
- `session_id` as an optional string association.

On success, `TaskCreate` MUST persist the task and return the full created task record. New tasks
MUST start in `pending` status. `created_at` and `updated_at` MUST be set on creation and MUST be
identical for the initial record.

#### Scenario: `TaskCreate` creates a global task with defaults

- GIVEN persistent task storage is supported for the active runtime backend
- WHEN `TaskCreate` is invoked with `{ "title": "Review parity slice" }`
- THEN the call MUST succeed
- AND the returned task MUST have `status = "pending"`
- AND the returned task MUST have `priority = "medium"`
- AND the returned task MUST have no `session_id`
- AND the task MUST be retrievable by its returned `id`

#### Scenario: `TaskCreate` creates a session-linked task

- GIVEN the caller is permitted to reference session `session-123`
- WHEN `TaskCreate` is invoked with `{ "title": "Summarize logs", "description": "Capture the failure mode", "priority": "high", "session_id": "session-123" }`
- THEN the call MUST succeed
- AND the returned task MUST include `session_id = "session-123"`
- AND the returned task MUST remain a normal global task record with optional session metadata only

#### Scenario: `TaskCreate` rejects invalid input

- GIVEN persistent task storage is available
- WHEN `TaskCreate` is invoked with an empty `title` or an unsupported `priority`
- THEN the call MUST fail validation
- AND no task record MUST be persisted

### Requirement: `TaskGet` MUST Return a Persisted Task by UUID

The system MUST expose a native tool named `TaskGet` that returns a persisted task record by public
UUID.

`TaskGet` MUST accept `id` as a required UUID string. On success, it MUST return the full current
persisted task record for that `id`.

If `id` is not a valid UUID, `TaskGet` MUST fail validation. If no persisted task exists for the
UUID, `TaskGet` MUST fail with a sanitized not-found error.

#### Scenario: `TaskGet` returns an existing task

- GIVEN a persisted task with UUID `11111111-1111-4111-8111-111111111111`
- WHEN `TaskGet` is invoked with that UUID
- THEN the call MUST succeed
- AND the returned task MUST match the current persisted record for that UUID

#### Scenario: `TaskGet` rejects an invalid UUID

- GIVEN a caller provides `id = "not-a-uuid"`
- WHEN `TaskGet` validates the request
- THEN the call MUST fail validation
- AND the runtime MUST NOT query storage with the invalid identifier

#### Scenario: `TaskGet` returns sanitized not-found behavior for an unknown UUID

- GIVEN no task exists for UUID `22222222-2222-4222-8222-222222222222`
- WHEN `TaskGet` is invoked with that UUID
- THEN the call MUST fail
- AND the error MUST indicate that the task was not found
- AND the error MUST NOT leak storage internals

### Requirement: `TaskList` MUST Support Basic Listing, Filtering, and Pagination

The system MUST expose a native tool named `TaskList` for listing persisted tasks.

`TaskList` MUST accept these optional inputs:
- `status` filtered to the approved status enum values;
- `priority` filtered to the approved priority enum values;
- `session_id` as an optional exact-match filter;
- `limit` as an optional positive integer page size;
- `offset` as an optional non-negative integer.

`TaskList` MUST return:
- `tasks` as an array of task records;
- `applied_limit` as the effective positive integer page size;
- `applied_offset` as the effective non-negative integer offset;
- `has_more` as a boolean indicating whether additional matching records remain.

For an unchanged task set, `TaskList` MUST order results deterministically. The ordering MUST be by
`created_at` descending, with `id` ascending as the stable tie-breaker.

#### Scenario: `TaskList` returns the first page in deterministic order

- GIVEN three persisted tasks ordered by creation time newest to oldest
- WHEN `TaskList` is invoked with `{ "limit": 2, "offset": 0 }`
- THEN the call MUST succeed
- AND `tasks` MUST contain the two newest tasks in deterministic order
- AND `applied_limit` MUST equal `2`
- AND `applied_offset` MUST equal `0`
- AND `has_more` MUST be `true`

#### Scenario: `TaskList` filters by status and session association

- GIVEN persisted tasks exist for multiple statuses and sessions
- WHEN `TaskList` is invoked with `{ "status": "in_progress", "session_id": "session-123" }`
- THEN the call MUST succeed
- AND every returned task MUST have `status = "in_progress"`
- AND every returned task MUST have `session_id = "session-123"`

#### Scenario: `TaskList` rejects invalid filters or pagination inputs

- GIVEN a caller provides `status = "paused"`, `limit = 0`, or `offset = -1`
- WHEN `TaskList` validates the request
- THEN the call MUST fail validation
- AND the runtime MUST NOT execute a listing query with those invalid values

### Requirement: `TaskUpdate` MUST Support Valid Non-Cancel Mutations Only

The system MUST expose a native tool named `TaskUpdate` for updating an existing persisted task.

`TaskUpdate` MUST accept:
- `id` as a required UUID string;
- at least one mutable field among `title`, `description`, `priority`, or `status`.

`TaskUpdate` MUST validate all supplied fields before performing any mutation. `updated_at` MUST be
advanced on every successful update.

`TaskUpdate` MUST allow only these status transitions:
- `pending` -> `in_progress`
- `pending` -> `completed`
- `in_progress` -> `completed`

`TaskUpdate` MUST NOT be used to set `status = "cancelled"`; semantic cancellation MUST use
`TaskStop`. `completed` and `cancelled` tasks MUST be terminal in this slice and MUST NOT be
reopened.

`TaskUpdate` MUST NOT set, replace, or clear `session_id` after task creation. Session linkage is
write-on-create only for this slice.

#### Scenario: `TaskUpdate` changes mutable fields and advances `updated_at`

- GIVEN a persisted task in `pending` status
- WHEN `TaskUpdate` is invoked with `{ "id": "11111111-1111-4111-8111-111111111111", "title": "Review runtime parity", "priority": "high" }`
- THEN the call MUST succeed
- AND the returned task MUST contain the updated `title` and `priority`
- AND `updated_at` MUST be later than the previous `updated_at`

#### Scenario: `TaskUpdate` allows a valid forward status transition

- GIVEN a persisted task with `status = "pending"`
- WHEN `TaskUpdate` is invoked with `{ "id": "11111111-1111-4111-8111-111111111111", "status": "in_progress" }`
- THEN the call MUST succeed
- AND the returned task MUST have `status = "in_progress"`

#### Scenario: `TaskUpdate` rejects invalid status mutations

- GIVEN a persisted task with `status = "in_progress"`
- WHEN `TaskUpdate` is invoked with `{ "id": "11111111-1111-4111-8111-111111111111", "status": "cancelled" }`
- THEN the call MUST fail validation or conflict checks
- AND the response MUST direct the caller to use `TaskStop` for cancellation

#### Scenario: `TaskUpdate` rejects invalid identifiers, empty patches, or `session_id` edits

- GIVEN a caller provides `id = "not-a-uuid"`, no mutable fields, or a `session_id` mutation
- WHEN `TaskUpdate` validates the request
- THEN the call MUST fail validation
- AND the runtime MUST NOT mutate any persisted task

#### Scenario: `TaskUpdate` rejects terminal-state reopen semantics

- GIVEN a persisted task with `status = "completed"`
- WHEN `TaskUpdate` is invoked with `{ "id": "11111111-1111-4111-8111-111111111111", "status": "pending" }`
- THEN the call MUST fail validation or conflict checks
- AND the persisted task MUST remain unchanged

### Requirement: `TaskStop` MUST Perform Semantic Cancellation

The system MUST expose a native tool named `TaskStop` that semantically cancels a persisted task.

`TaskStop` MUST accept `id` as a required UUID string. For tasks in `pending` or `in_progress`,
`TaskStop` MUST set `status = "cancelled"`, persist that transition, and advance `updated_at`.

`TaskStop` MUST NOT reopen tasks, and it MUST NOT succeed for tasks already in `completed` or
`cancelled` terminal states.

#### Scenario: `TaskStop` cancels an in-progress task

- GIVEN a persisted task with `status = "in_progress"`
- WHEN `TaskStop` is invoked for that task
- THEN the call MUST succeed
- AND the returned task MUST have `status = "cancelled"`
- AND `updated_at` MUST be later than the previous `updated_at`

#### Scenario: `TaskStop` rejects cancellation of a completed task

- GIVEN a persisted task with `status = "completed"`
- WHEN `TaskStop` is invoked for that task
- THEN the call MUST fail validation or conflict checks
- AND the persisted task MUST remain `completed`

#### Scenario: `TaskStop` rejects an already cancelled task

- GIVEN a persisted task with `status = "cancelled"`
- WHEN `TaskStop` is invoked for that task
- THEN the call MUST fail validation or conflict checks
- AND the call MUST NOT be treated as idempotent success
- AND the persisted task MUST remain `cancelled`

### Requirement: Unsupported Backends MUST Fail Closed for Persistent Task Tools

If the active runtime memory backend does not support persistent tasks in this slice, `TaskCreate`,
`TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` MUST fail closed with a sanitized unsupported
error.

Unsupported backend behavior MUST be explicit. The runtime MUST NOT silently emulate persistence in
volatile memory, MUST NOT redirect to cron/scheduler storage, and MUST NOT claim that persistent
Task tools are available when they are not supported by the active backend.

#### Scenario: task tools are rejected on an unsupported backend

- GIVEN the active runtime backend does not implement persistent task storage for this slice
- WHEN a caller invokes any `Task*` tool
- THEN the call MUST fail with a sanitized unsupported-backend error
- AND no task data MUST be created, mutated, or listed from an alternate fallback store

### Requirement: Session Linkage MUST Respect Security and Scope Boundaries

Persistent task operations MUST remain confined to the active runtime and workspace storage
boundary.

When a caller supplies `session_id` to `TaskCreate`, `TaskList`, or `TaskUpdate`, the runtime MUST
apply the same effective session visibility and permission boundary used for other session-aware
operations. A caller MUST NOT be able to use task tools to discover, attach to, or infer details
about an inaccessible session.

Task responses MUST expose only the task's own `session_id` field when present. They MUST NOT leak
session transcripts, storage paths, raw SQL details, or other runtime internals.

#### Scenario: `TaskCreate` rejects inaccessible session attachment

- GIVEN the caller is not permitted to reference session `foreign-session`
- WHEN `TaskCreate` is invoked with `{ "title": "Investigate", "session_id": "foreign-session" }`
- THEN the call MUST be denied
- AND no task MUST be created
- AND the error MUST NOT reveal whether `foreign-session` exists

#### Scenario: `TaskList` does not leak inaccessible session details

- GIVEN the caller is not permitted to reference session `foreign-session`
- WHEN `TaskList` is invoked with `{ "session_id": "foreign-session" }`
- THEN the call MUST be denied
- AND the response MUST NOT disclose whether that session exists

## MODIFIED Requirements

### Requirement: Tool Inventory and Surfaced Listing Compatibility

The system MUST keep surfaced tool listings consistent with the parity contracts covered by this
specification.

Any surfaced runtime tool inventory relevant to operators or agents, including `/tools` or other
runtime-exposed tool listings, MUST represent `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`,
`TaskList`, `TaskUpdate`, and `TaskStop` as available tools when they are enabled for the current
profile and supported by the active backend.

Such listings MUST remain backward compatible. They MUST NOT require removal of existing
Corvus-native tool names from the runtime, and they MUST distinguish parity-facing names from
legacy or native names clearly enough that operators can understand what is canonical in this
transition slice.

If a profile, permission context, or unsupported backend disables one of the parity tools,
surfaced listings MUST reflect that effective availability rather than advertising unavailable
tools.

(Previously: surfaced listing compatibility for this spec covered only `Glob`, `Grep`, and
`WebFetch`.)

#### Scenario: `/tools` inventory shows enabled task tools

- GIVEN the active runtime profile enables `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and
  `TaskStop`
- AND the active backend supports persistent task storage
- WHEN an operator requests the effective tool inventory
- THEN the surfaced listing MUST include those five task tools by those exact names

#### Scenario: surfaced inventory omits unsupported task tools

- GIVEN the active runtime profile includes the task tools
- AND the active backend does not support persistent tasks for this slice
- WHEN an operator requests the effective tool inventory
- THEN the surfaced listing MUST NOT claim that the task tools are available
- AND the listing MUST remain internally consistent with runtime behavior

### Requirement: Published Parity Mapping and Scope Boundary Documentation

The change MUST publish a parity mapping between Corvus-native tool names and Claude-style tool
names for the parity capabilities covered by this specification.

The published mapping MUST, at minimum, document the relationship between:
- `code_search` and `Grep`;
- `http_request` and `WebFetch`;
- Corvus file-discovery capability and `Glob`;
- persistent task lifecycle capability and `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and
  `TaskStop`.

The mapping MUST identify whether each parity name is additive, canonical for parity-facing
surfaces, legacy/native, or deferred for future consolidation.

The same documentation set MUST also state that:
- persistent task lifecycle is separate from `schedule` and `cron_*` semantics;
- this task slice is limited to persistent tasks with the approved minimal model;
- reopen semantics, subtasks, and dependencies remain out of scope; and
- this slice does not require broad rename or removal of existing internal tool names.

(Previously: this requirement explicitly deferred `TaskCreate`, `TaskGet`, `TaskList`,
`TaskUpdate`, and `TaskStop` from the first slice.)

#### Scenario: parity mapping documents task tools without conflating scheduler behavior

- GIVEN a maintainer reads the parity documentation for this change
- WHEN they inspect the mapping table and scope notes
- THEN they MUST be able to identify the parity-facing task lifecycle tools
- AND they MUST be able to tell that `schedule` and `cron_*` are not equivalent replacements for
  `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, or `TaskStop`

#### Scenario: documentation states the persistent task slice boundaries

- GIVEN a maintainer reviews the documentation for task parity
- WHEN they look for advanced task-management behavior
- THEN the documentation MUST state that reopen semantics, subtasks, and dependencies are out of
  scope
- AND it MUST NOT imply broader project-management support in this slice
