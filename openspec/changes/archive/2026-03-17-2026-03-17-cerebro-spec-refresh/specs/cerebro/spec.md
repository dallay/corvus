# Delta for Cerebro

## ADDED Requirements

### Requirement: MCP Tool Inventory

The Cerebro MCP service MUST expose the following 13 tools as the canonical tool surface, and tool
introspection MUST return this list without omissions or extra entries:

- `mem_session_start`
- `mem_session_end`
- `mem_session_summary`
- `mem_context`
- `mem_save`
- `mem_update`
- `mem_delete`
- `mem_suggest_topic_key`
- `mem_search`
- `mem_get_observation`
- `mem_timeline`
- `mem_save_prompt`
- `mem_stats`

#### Scenario: Tool inventory returned (happy path)

- GIVEN a running Cerebro MCP service with tool introspection enabled
- WHEN a client requests the available tools
- THEN the response includes exactly the 13 tools listed above
- AND the response contains no additional tool names

#### Scenario: Missing tool name (edge case)

- GIVEN a running Cerebro MCP service
- WHEN tool introspection is called
- THEN the service returns a structured error if any of the 13 tools is not available

### Requirement: Agent Prompt Template Guidance

The Cerebro documentation MUST provide a copy-paste `prompt_template.md` that instructs agents to
use drill-in search patterns and to save structured observations using the What/Why/Where/Learned
format.

#### Scenario: Prompt template available (happy path)

- GIVEN the Cerebro documentation bundle
- WHEN a user searches for agent integration guidance
- THEN `prompt_template.md` is present and contains drill-in usage instructions
- AND the template includes the What/Why/Where/Learned structure

#### Scenario: Missing prompt template (edge case)

- GIVEN the Cerebro documentation bundle
- WHEN `prompt_template.md` is absent or empty
- THEN the documentation build fails with a structured error

### Requirement: Optional TUI Surface

The Cerebro distribution MAY include a TUI; when enabled, it MUST provide the following views:

dashboard, memory explorer, session timeline, live tool-call stream.

#### Scenario: TUI enabled (happy path)

- GIVEN a Cerebro deployment with the TUI enabled
- WHEN the operator opens the TUI
- THEN the dashboard, memory explorer, session timeline, and live tool-call stream views are
  available

#### Scenario: TUI disabled (edge case)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN the operator attempts to open the TUI
- THEN the service starts without a UI and continues to serve MCP requests

## MODIFIED Requirements

### Requirement: Cerebro MCP Tool Surface

The Cerebro module MUST expose the MCP tool set defined in the 13-tool inventory and return
structured, typed errors for invalid requests. Tool contracts MUST align with the Cerebro product
specification and remain agent-agnostic.

(Previously: The tool surface was referenced indirectly via `openspec/changes/cerebro/cerebro.md`
without an explicit inventory.)

#### Scenario: Save and recall through Cerebro (happy path)

- GIVEN a running Cerebro MCP service
- WHEN an agent calls `mem_save` with a valid structured observation
- THEN the service stores the observation and returns a stable memory ID
- AND a subsequent `mem_search` can retrieve a compact summary for that memory

#### Scenario: Invalid tool input (edge case)

- GIVEN a running Cerebro MCP service
- WHEN an agent calls `mem_save` with empty content or missing required fields
- THEN the service rejects the request with a structured validation error

### Requirement: Remove SurrealDB Backend from Runtime

The agent runtime MUST NOT include a SurrealDB memory backend or the `memory-surreal` feature flag.
Embedded SurrealDB is an in-scope deployment mode for the Cerebro service only and MUST NOT be
accessible to the runtime as a local backend.

(Previously: Embedded SurrealDB was explicitly out of scope.)

#### Scenario: Runtime memory backend selection (happy path)

- GIVEN the updated runtime build
- WHEN the runtime loads memory configuration
- THEN SurrealDB is not an available backend option
- AND only local short-term and MCP-backed Cerebro options remain

#### Scenario: Embedded SurrealDB scoped to Cerebro (edge case)

- GIVEN a Cerebro deployment configured for embedded SurrealDB
- WHEN the agent runtime loads memory configuration
- THEN the runtime cannot select embedded SurrealDB as a local backend
- AND all long-term memory requests still route through MCP

### Requirement: Data Hygiene Defaults

Cerebro MUST exclude soft-deleted records from retrieval APIs by default, MUST return a `deleted`
status for direct fetches of soft-deleted IDs, and MUST support deduplication and topic-key upserts
when explicitly requested by the caller.

(Previously: Only soft-delete filtering and deleted status were required.)

#### Scenario: Deleted memory is hidden (happy path)

- GIVEN a memory entry that has been soft-deleted via `mem_delete`
- WHEN an agent calls `mem_search`
- THEN the deleted entry is not included in the results

#### Scenario: Deduplication requested (edge case)

- GIVEN a memory entry saved with a deduplication policy enabled
- WHEN a second `mem_save` is called with identical deduplication inputs
- THEN the service returns a response indicating the entry was deduplicated
- AND no duplicate memory record is created

#### Scenario: Topic-key upsert requested (edge case)

- GIVEN a memory entry exists with `topic_key` set to "alpha"
- WHEN `mem_save` is called with `topic_key` set to "alpha" and upsert requested
- THEN the existing memory entry is updated
- AND the response returns the existing memory ID

## REMOVED Requirements

### Requirement: TUI Out of Scope

(Reason: TUI is now explicitly optional and scoped as a non-blocking surface.)
