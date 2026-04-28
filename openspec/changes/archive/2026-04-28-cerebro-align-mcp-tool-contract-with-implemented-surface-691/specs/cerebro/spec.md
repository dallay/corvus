# Delta for Cerebro

## MODIFIED Requirements

### Requirement: MCP Tool Inventory

The Cerebro MCP service MUST publish an 8-tool implemented inventory as the canonical callable surface for normal operation.

The published callable inventory MUST include exactly:

- `mem_save`
- `mem_search`
- `mem_delete`
- `mem_get_observation`
- `mem_update`
- `mem_suggest_topic_key`
- `mem_timeline`
- `mem_stats`

The service MUST treat the following 5 tools as deferred and not currently implemented:

- `mem_save_prompt`
- `mem_session_start`
- `mem_session_end`
- `mem_session_summary`
- `mem_context`

The published inventory MUST NOT advertise deferred tools as callable or implemented.

(Previously: The specification declared a canonical 13-tool surface and required tool introspection to return all 13 names.)

#### Scenario: Implemented callable inventory is published

- GIVEN a running Cerebro MCP service
- WHEN a client requests the currently callable tool inventory through the supported discovery path
- THEN the response MUST include exactly the 8 implemented tools listed above
- AND the response MUST NOT include any of the 5 deferred tools as callable entries

#### Scenario: Deferred tool is excluded from callable inventory

- GIVEN a running Cerebro MCP service
- WHEN a client evaluates whether `mem_context` is currently callable
- THEN the service's published callable inventory MUST exclude `mem_context`
- AND clients MUST be able to distinguish that exclusion from successful tool availability

### Requirement: Cerebro MCP Tool Surface

The Cerebro module MUST align its published contract with the implemented 8-tool surface while preserving structured unavailable behavior for deferred tools.

Calls to `mem_save`, `mem_search`, `mem_delete`, `mem_get_observation`, `mem_update`, `mem_suggest_topic_key`, `mem_timeline`, and `mem_stats` MUST remain supported.

Calls to `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and `mem_context` MUST return a structured `NotImplemented` outcome rather than being represented as successful or generally available tools.

The published Cerebro contract MUST describe current guarantees in terms of the supported callable surface and MUST NOT claim a broader implemented MCP surface than the service currently provides.

(Previously: The specification stated that the module exposed the 13-tool inventory as the MCP tool set.)

#### Scenario: Implemented tool call succeeds under published contract

- GIVEN a running Cerebro MCP service
- WHEN an agent calls `mem_save` with a valid structured observation
- THEN the service MUST store the observation and return a stable memory ID
- AND that success MUST remain consistent with the published callable contract

#### Scenario: Deferred tool call returns structured NotImplemented

- GIVEN a running Cerebro MCP service
- WHEN an agent calls `mem_session_summary` or `mem_context`
- THEN the service MUST reject the call with a structured `NotImplemented` outcome
- AND the response MUST NOT imply that the tool is implemented or generally available

## ADDED Requirements

### Requirement: Contract Verification for Implemented and Deferred Tools

Cerebro contract verification MUST distinguish implemented tools from deferred tools to prevent future inventory drift.

Verification MUST assert both of the following:

- the published callable inventory contains exactly the 8 implemented tools, and
- each deferred tool returns a structured `NotImplemented` outcome when invoked through the supported call path.

#### Scenario: Verification passes for implemented and deferred split

- GIVEN contract verification is executed against the current Cerebro service
- WHEN verification checks inventory publication and deferred-tool behavior
- THEN verification MUST pass only if the 8 implemented tools are published as callable
- AND each of the 5 deferred tools returns structured `NotImplemented`

#### Scenario: Verification fails on overstated inventory

- GIVEN a future change republishes `mem_save_prompt` or `mem_context` as implemented without backend support
- WHEN contract verification runs
- THEN verification MUST fail
- AND the failure MUST identify the mismatch between published availability and observed behavior
