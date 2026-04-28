# Tooling Parity Specification

## Purpose

Define the implementation slice for Claude-style search, fetch, and persistent task parity in Corvus.
This specification covers the dedicated `Glob`, `Grep`, read-only `WebFetch`, and persistent
`TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, `TaskStop` tool contracts, their validation and
security boundaries, stable result contracts, and the parity mapping that MUST be surfaced
consistently in documentation and tool inventory outputs.

This slice does not require broad renaming or removal of existing Corvus-native tool names.
Canonical parity names remain additive and MUST continue to coexist with retained native contracts.

## Requirements

### Requirement: Canonical Parity Names and Compatibility Aliases

The system MUST preserve `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`, `TaskList`,
`TaskUpdate`, and `TaskStop` as canonical runtime names for this slice.

The system MUST also accept these additive compatibility aliases:
- `glob`
- `grep`
- `web_fetch`
- `task_create`
- `task_get`
- `task_list`
- `task_update`
- `task_stop`

Alias publication and invocation MUST resolve to the same implementation, permission boundary,
backend support, and result contract as the canonical name.

### Requirement: Published Parity Mapping

Documentation and runtime inventory surfaces MUST publish the canonical parity mapping in a stable,
deterministic format.

At minimum, the published mapping MUST preserve these relationships:

| Canonical parity name | Compatibility alias | Backing native/runtime surface | Mapping status |
| --- | --- | --- | --- |
| `Glob` | `glob` | workspace discovery helpers | additive |
| `Grep` | `grep` | `code_search` search internals | canonical parity + retained native contract |
| `WebFetch` | `web_fetch` | read-only wrapper over `http_request` URL policy boundary | canonical parity + retained native contract |
| `TaskCreate` | `task_create` | persistent task lifecycle service | additive |
| `TaskGet` | `task_get` | persistent task lifecycle service | additive |
| `TaskList` | `task_list` | persistent task lifecycle service | additive |
| `TaskUpdate` | `task_update` | persistent task lifecycle service | additive |
| `TaskStop` | `task_stop` | persistent task lifecycle service | additive |


### Requirement: Dedicated `Glob` Tool Contract

The system MUST expose a dedicated read-only tool named `Glob` for workspace-safe file pattern
discovery.

The `Glob` tool MUST accept:
- `pattern` as a required string glob pattern;
- `path` as an optional workspace-relative directory scope.

The `Glob` tool MUST reject requests when:
- `pattern` is empty;
- `path` is absolute, escapes the workspace, or resolves outside the active workspace boundary.

The `Glob` tool MUST return a stable structured result with:
- `filenames` as an array of workspace-relative paths;
- `durationMs` as a non-negative integer;
- `numFiles` as the total number of returned paths;
- `truncated` as a boolean indicating whether an internal result cap was applied.

Returned paths MUST be workspace-relative and MUST be ordered deterministically. The contract MAY
choose a deterministic ordering strategy such as modification-time ordering, but the chosen
ordering MUST remain stable for repeated runs against an unchanged workspace.

#### Scenario: `Glob` returns workspace-relative matches for a valid pattern

- GIVEN a workspace containing `src/main.ts` and `src/lib/util.ts`
- WHEN `Glob` is invoked with `{ "pattern": "src/**/*.ts" }`
- THEN the call MUST succeed
- AND `structured.filenames` MUST contain only workspace-relative paths
- AND `structured.numFiles` MUST equal the number of returned paths
- AND `structured.truncated` MUST be `false` when no cap was reached

#### Scenario: `Glob` rejects a path that escapes the workspace

- GIVEN an active workspace rooted at `/workspace`
- WHEN `Glob` is invoked with `{ "pattern": "**/*.rs", "path": "../.." }`
- THEN the call MUST fail validation
- AND the tool MUST NOT traverse outside `/workspace`

#### Scenario: `Glob` ordering is stable for an unchanged workspace

- GIVEN the same workspace contents and file metadata across repeated runs
- WHEN `Glob` is invoked repeatedly with the same input
- THEN the returned `structured.filenames` order MUST be identical on every run

### Requirement: Dedicated `Grep` Tool Contract

The system MUST expose a dedicated read-only tool named `Grep` for content search.

The `Grep` contract MUST remain parity-aligned with Claude-style search expectations while staying
behaviorally aligned with Corvus search semantics. The implementation MAY wrap the existing search
engine, but the exposed `Grep` contract MUST be the stable public surface for this slice.

The `Grep` tool MUST accept:
- `pattern` as a required search pattern string;
- `path` as an optional workspace-relative file or directory scope;
- `glob` as an optional include filter;
- `output_mode` as an optional enum with values `content`, `files_with_matches`, or `count`.

The `Grep` tool MAY accept optional context and search modifiers, but any accepted modifier MUST be
validated deterministically and MUST NOT widen filesystem access beyond the workspace boundary.

The `Grep` tool MUST reject requests when:
- `pattern` is empty;
- `path` is absolute, escapes the workspace, or resolves outside the workspace;
- context-only fields are supplied in a combination that the declared `output_mode` does not allow;
- numeric pagination or context fields are negative.

The `Grep` tool MUST return a stable structured result that includes:
- `mode` as the effective output mode;
- `numFiles` as a non-negative integer;
- `filenames` as workspace-relative paths in deterministic order;
- `content` when `mode = "content"`;
- `numLines` when `mode = "content"`;
- `numMatches` when `mode = "count"`;
- `appliedLimit` and `appliedOffset` when relevant.

When `mode = "content"`, returned content MUST preserve deterministic ordering for an unchanged
workspace and MUST remain bounded by the tool's documented output limits.

#### Scenario: `Grep` returns file matches in a deterministic public contract

- GIVEN a workspace where `src/app.ts` and `src/lib.ts` both contain the text `SearchClient`
- WHEN `Grep` is invoked with `{ "pattern": "SearchClient", "output_mode": "files_with_matches" }`
- THEN the call MUST succeed
- AND `structured.mode` MUST equal `files_with_matches`
- AND `structured.filenames` MUST contain only workspace-relative file paths
- AND the filenames MUST appear in deterministic order across repeated runs on the same workspace

#### Scenario: `Grep` rejects invalid output mode combinations

- GIVEN a valid workspace
- WHEN `Grep` is invoked with `{ "pattern": "needle", "output_mode": "count", "-A": 2 }`
- THEN the call MUST fail validation
- AND the failure MUST explain that content context fields are only valid with content output

#### Scenario: `Grep` cannot search outside the workspace

- GIVEN an active workspace rooted at `/workspace`
- WHEN `Grep` is invoked with `{ "pattern": "token", "path": "/etc" }`
- THEN the call MUST fail validation or permission checks
- AND the tool MUST NOT read files outside `/workspace`

#### Scenario: `Grep` preserves zero-match success semantics

- GIVEN a workspace with no files containing `pattern_that_does_not_exist_536`
- WHEN `Grep` is invoked with `{ "pattern": "pattern_that_does_not_exist_536", "output_mode": "count" }`
- THEN the call MUST succeed
- AND the structured result MUST report zero matches
- AND the result shape MUST still match the documented contract for `count` mode

### Requirement: Dedicated Read-Only `WebFetch` Tool Contract

The system MUST expose a dedicated tool named `WebFetch` for read-only fetch-and-extract flows.

The `WebFetch` tool MUST accept:
- `url` as a required absolute URL string;
- `prompt` as a required instruction string describing how fetched content should be summarized or
  transformed for the caller.

`WebFetch` MUST be read-only. It MUST NOT expose mutation semantics for remote resources, and it
MUST NOT be used as a generic arbitrary HTTP write surface.

`WebFetch` MUST preserve the same effective outbound network security boundary required for current
allowlisted HTTP access, including:
- host allowlist enforcement when configured;
- private-host and local-network protections;
- rejection of unsupported or unsafe URL schemes.

The `WebFetch` tool MUST return a stable structured result with:
- `bytes` as the fetched response size in bytes when available;
- `code` as the HTTP status code;
- `codeText` as the status text when available;
- `result` as the extracted or summarized response content;
- `durationMs` as a non-negative integer;
- `url` as the final fetched URL represented to the caller.

If the fetch is denied by policy, the tool MUST fail without making the prohibited network request.

#### Scenario: `WebFetch` returns extracted content for an allowlisted URL

- GIVEN outbound policy permits `https://docs.example.com/page`
- WHEN `WebFetch` is invoked with `{ "url": "https://docs.example.com/page", "prompt": "Summarize the key API limits" }`
- THEN the call MUST succeed
- AND the structured result MUST include `code`, `result`, `durationMs`, and `url`
- AND the tool MUST remain read-only for that request

#### Scenario: `WebFetch` rejects a private-network target

- GIVEN the runtime blocks private-network destinations
- WHEN `WebFetch` is invoked with `{ "url": "http://127.0.0.1:8080/admin", "prompt": "Summarize" }`
- THEN the call MUST be denied
- AND the runtime MUST NOT fetch the target resource

#### Scenario: `WebFetch` rejects an unsupported URL scheme

- GIVEN a valid runtime session
- WHEN `WebFetch` is invoked with `{ "url": "file:///etc/passwd", "prompt": "Summarize" }`
- THEN the call MUST fail validation
- AND the tool MUST NOT treat local file reads as web fetches

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

#### Scenario: `/tools` inventory shows enabled parity tools

- GIVEN the active runtime profile enables `Glob`, `Grep`, and `WebFetch`
- WHEN an operator requests the effective tool inventory
- THEN the surfaced listing MUST include `Glob`, `Grep`, and `WebFetch`
- AND each listed tool MUST use the same names defined by this specification

#### Scenario: surfaced inventory does not advertise disabled parity tools

- GIVEN the active runtime profile disables `WebFetch`
- WHEN an operator requests the effective tool inventory
- THEN the surfaced listing MUST NOT claim that `WebFetch` is available
- AND the listing MUST remain internally consistent with the runtime's actual tool allowlist

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

#### Scenario: parity mapping documentation distinguishes parity and native names

- GIVEN a maintainer reads the parity documentation for this change
- WHEN they inspect the mapping table
- THEN they MUST be able to identify which Corvus-native tool or capability backs `Glob`, `Grep`, and `WebFetch`
- AND they MUST be able to tell whether each mapping is additive or a future consolidation candidate

#### Scenario: documentation explicitly defers task tools

- GIVEN a maintainer or operator reviews the first-slice parity documentation
- WHEN they look for `TaskCreate` and related task lifecycle tools
- THEN the documentation MUST state that `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` are included in this slice as persistent task lifecycle tools
- AND it MUST distinguish them from `schedule` and `cron_*`, which do not satisfy that task-tool contract

---

## Persistent Task Tools Slice

This section covers the persistent task lifecycle portion of the same tooling-parity slice.
Persistent task lifecycle parity remains distinct from `schedule` and `cron_*` capabilities.

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

### Requirement: `TaskGet` MUST Return a Persisted Task by UUID

The system MUST expose a native tool named `TaskGet` that returns a persisted task record by public
UUID.

`TaskGet` MUST accept `id` as a required UUID string. On success, it MUST return the full current
persisted task record for that `id`.

If `id` is not a valid UUID, `TaskGet` MUST fail validation. If no persisted task exists for the
UUID, `TaskGet` MUST fail with a sanitized not-found error.

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

### Requirement: `TaskStop` MUST Perform Semantic Cancellation

The system MUST expose a native tool named `TaskStop` that semantically cancels a persisted task.

`TaskStop` MUST accept `id` as a required UUID string. For tasks in `pending` or `in_progress`,
`TaskStop` MUST set `status = "cancelled"`, persist that transition, and advance `updated_at`.

`TaskStop` MUST NOT reopen tasks, and it MUST NOT succeed for tasks already in `completed` or
`cancelled` terminal states.

### Requirement: Unsupported Backends MUST Fail Closed for Persistent Task Tools

If the active runtime memory backend does not support persistent tasks in this slice, `TaskCreate`,
`TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` MUST fail closed with a sanitized unsupported
error.

Unsupported backend behavior MUST be explicit. The runtime MUST NOT silently emulate persistence in
volatile memory, MUST NOT redirect to cron/scheduler storage, and MUST NOT claim that persistent
Task tools are available when they are not supported by the active backend.

### Requirement: Session Linkage MUST Respect Security and Scope Boundaries

Persistent task operations MUST remain confined to the active runtime and workspace storage
boundary.

When a caller supplies `session_id` to `TaskCreate`, `TaskList`, or `TaskUpdate`, the runtime MUST
apply the same effective session visibility and permission boundary used for other session-aware
operations. A caller MUST NOT be able to use task tools to discover, attach to, or infer details
about an inaccessible session.

Task responses MUST expose only the task's own `session_id` field when present. They MUST NOT leak
session transcripts, storage paths, raw SQL details, or other runtime internals.
