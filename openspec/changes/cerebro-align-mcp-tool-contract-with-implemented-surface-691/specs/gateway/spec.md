# Delta for Gateway

## ADDED Requirements

### Requirement: Gateway-Published Cerebro Tool Contract

The gateway specification SHALL treat the implemented Cerebro MCP surface as an 8-tool callable contract for normal operation.

The gateway-published callable tool surface MUST be limited to:

- `mem_save`
- `mem_search`
- `mem_delete`
- `mem_get_observation`
- `mem_update`
- `mem_suggest_topic_key`
- `mem_timeline`
- `mem_stats`

The gateway specification MUST NOT publish `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, or `mem_context` as currently callable capabilities.

The gateway specification MAY describe those 5 tools only as deferred Cerebro capabilities that currently return structured `NotImplemented` outcomes.

#### Scenario: Gateway-facing contract lists only implemented callable tools

- GIVEN a gateway-facing contract, runtime integration, or published capability summary derived from the source-of-truth spec
- WHEN that contract enumerates Cerebro tools that are callable in normal operation
- THEN the enumeration MUST include exactly the 8 implemented tools listed above
- AND the enumeration MUST NOT include any deferred tool as callable

#### Scenario: Deferred tool remains documented without being advertised as callable

- GIVEN gateway documentation or runtime-facing capability guidance references a deferred Cerebro tool such as `mem_context`
- WHEN the tool's current availability is described
- THEN the tool MUST be identified as deferred or unavailable
- AND the description MUST state that current calls receive a structured `NotImplemented` outcome rather than normal success

### Requirement: Gateway Verification of Deferred Cerebro Availability Claims

Gateway verification artifacts MUST assert that downstream gateway-facing surfaces do not advertise deferred Cerebro tools as available for normal use.

Verification MUST cover `mem_context` explicitly because prior downstream drift treated it as available.

#### Scenario: Verification catches mem_context capability drift

- GIVEN a downstream runtime, dashboard, or docs surface that consumes gateway-published Cerebro capability data
- WHEN verification checks the published callable capability set
- THEN `mem_context` MUST NOT appear as available for normal use
- AND verification MUST fail if `mem_context` is advertised as callable without a deferred or unavailable designation
