# Delta for cerebro

## MODIFIED Requirements

### Requirement: Embedded SurrealDB Default Storage Mode

The Cerebro service MUST use embedded SurrealDB as the default storage mode when no storage mode is explicitly configured.

The Cerebro service MUST treat embedded SurrealDB and other supported local modes as a **local-first, single-node storage posture** in the current build.

The Cerebro service MUST allow configuration to override the default storage mode only to other supported local modes, including `disk` and `in_memory`.

The Cerebro service MUST NOT treat `remote_surreal` as a supported storage mode in this build.

(Previously: The Cerebro service MUST use embedded SurrealDB as the default storage mode when no storage mode is explicitly configured. The Cerebro service MUST allow configuration to override the default storage mode to supported non-embedded modes (for example, in-memory or disk-backed storage).)

#### Scenario: Default storage mode uses embedded SurrealDB for single-node durability

- GIVEN a Cerebro deployment with no explicit storage mode configured
- WHEN the service starts
- THEN embedded SurrealDB is selected as the storage mode
- AND the selected mode MUST be treated as the default supported single-node durable production mode

#### Scenario: Explicit override remains limited to supported local modes

- GIVEN a Cerebro deployment with storage mode explicitly set to `disk` or `in_memory`
- WHEN the service starts
- THEN the configured local mode MUST be used
- AND embedded SurrealDB is not initialized
- AND the override MUST NOT imply support for remote/shared persistence

#### Scenario: Remote storage mode is not a supported override

- GIVEN a Cerebro deployment with storage mode explicitly set to `remote_surreal`
- WHEN the service validates startup configuration
- THEN the configuration MUST be rejected as unsupported in this build
- AND the service MUST NOT start as though remote SurrealDB were a supported production mode

### Requirement: Operational Fallback When Embedded SurrealDB Is Unavailable

The Cerebro service MUST support an operational fallback mode for storage when embedded SurrealDB is unavailable at startup and a fallback mode is configured.

Any configured fallback mode MUST be limited to supported local fallback modes in this build.

The Cerebro service MUST NOT accept `remote_surreal` as a supported fallback target in this build.

If no supported local fallback mode is configured and embedded SurrealDB cannot start, the service MUST fail fast and MUST NOT serve MCP requests.

(Previously: The Cerebro service MUST support an operational fallback mode for storage when embedded SurrealDB is unavailable at startup and a fallback mode is configured. If no fallback mode is configured and embedded SurrealDB cannot start, the service MUST fail fast and MUST NOT serve MCP requests.)

#### Scenario: Supported local fallback is used

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND a supported local fallback storage mode is configured
- WHEN embedded SurrealDB fails to start
- THEN the service MUST start using the configured local fallback mode
- AND the service MUST report that it is running in fallback mode

#### Scenario: Unsupported remote fallback is rejected

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND the fallback storage mode is configured as `remote_surreal`
- WHEN the service validates startup configuration
- THEN the configuration MUST be rejected as unsupported in this build
- AND the service MUST NOT treat remote/shared persistence as an available recovery path

#### Scenario: No supported fallback configured

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND no supported local fallback storage mode is configured
- WHEN embedded SurrealDB fails to start
- THEN the service MUST fail to start
- AND no MCP requests are served

## ADDED Requirements

### Requirement: Unsupported Remote Shared Persistence Boundary

The Cerebro specification MUST align with the gateway operational source-of-truth by defining remote/shared SurrealDB and HA multi-node persistence as unsupported in this build.

The Cerebro specification MUST describe `disk` as a node-local durable alternative and `in_memory` as non-durable storage suitable only for CI, development, or emergency fallback scenarios.

The Cerebro specification MUST NOT describe any current storage mode as providing shared remote durability, clustered coordination, or HA multi-node persistence.

#### Scenario: Supported storage modes remain local-first

- GIVEN a reader consults the Cerebro storage behavior specification
- WHEN the reader compares `embedded_surreal`, `disk`, and `in_memory`
- THEN the specification MUST describe `embedded_surreal` and `disk` as local storage modes
- AND the specification MUST describe `in_memory` as non-durable
- AND the specification MUST NOT imply that any of those modes provide shared remote persistence

#### Scenario: Multi-node persistence is not claimed by storage behavior spec

- GIVEN a reader looks for current HA or multi-node persistence guarantees in the Cerebro specification
- WHEN the reader checks the storage behavior requirements
- THEN the specification MUST state that HA multi-node persistence is unsupported in this build
- AND the specification MUST defer any future remote/shared persistence support to a separate follow-on change
