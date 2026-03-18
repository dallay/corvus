# Delta for Cerebro

## ADDED Requirements

### Requirement: Embedded SurrealDB Default Storage Mode

The Cerebro service MUST use embedded SurrealDB as the default storage mode when no storage mode is
explicitly configured.

The Cerebro service MUST allow configuration to override the default storage mode to supported
non-embedded modes (for example, in-memory or disk-backed storage).

#### Scenario: Default storage mode uses embedded SurrealDB (happy path)

- GIVEN a Cerebro deployment with no explicit storage mode configured
- WHEN the service starts
- THEN embedded SurrealDB is selected as the storage mode
- AND the service is ready to serve MCP requests

#### Scenario: Explicit storage override bypasses embedded SurrealDB (edge case)

- GIVEN a Cerebro deployment with storage mode explicitly set to a non-embedded mode
- WHEN the service starts
- THEN the configured storage mode is used
- AND embedded SurrealDB is not initialized

### Requirement: Embedded SurrealDB Loopback Binding

The embedded SurrealDB endpoint MUST bind only to loopback addresses by default.

The Cerebro service MUST reject any configuration that attempts to bind embedded SurrealDB to a
non-loopback address unless an explicit security override is provided.

#### Scenario: Loopback-only binding enforced (happy path)

- GIVEN a Cerebro deployment with embedded SurrealDB enabled and no binding override configured
- WHEN the service starts
- THEN embedded SurrealDB binds only to loopback interfaces

#### Scenario: Non-loopback binding rejected (edge case)

- GIVEN a Cerebro deployment configured to bind embedded SurrealDB to a non-loopback address
- WHEN the service starts
- THEN startup fails with a security validation error

### Requirement: Embedded SurrealDB Authentication

The embedded SurrealDB endpoint MUST require authentication for all direct access.

The Cerebro service MUST reject empty or missing embedded SurrealDB credentials at startup.

#### Scenario: Authentication enforced for embedded SurrealDB (happy path)

- GIVEN a Cerebro deployment with embedded SurrealDB credentials configured
- WHEN a client attempts to access embedded SurrealDB without credentials
- THEN the request is rejected with an unauthorized error

#### Scenario: Missing credentials prevent startup (edge case)

- GIVEN a Cerebro deployment with embedded SurrealDB enabled and empty credentials
- WHEN the service starts
- THEN startup fails with a configuration error

### Requirement: Migration Tooling for Legacy SurrealDB Data

The system MUST provide migration tooling that can import legacy SurrealDB data into the embedded
SurrealDB store.

The migration tooling MUST provide validation that verifies import completeness (at minimum, record
counts and schema compatibility).

#### Scenario: Import and validation succeed (happy path)

- GIVEN a legacy SurrealDB export and a target embedded SurrealDB store
- WHEN the migration tooling runs in import mode
- THEN the data is imported into the embedded store
- AND validation reports matching record counts and compatible schemas

#### Scenario: Validation failure halts migration (edge case)

- GIVEN a legacy SurrealDB export with incompatible schema or missing records
- WHEN the migration tooling runs with validation enabled
- THEN the migration reports a validation failure
- AND the tooling exits without marking the migration as successful

### Requirement: Operational Fallback When Embedded SurrealDB Is Unavailable

The Cerebro service MUST support an operational fallback mode for storage when embedded SurrealDB is
unavailable at startup and a fallback mode is configured.

If no fallback mode is configured and embedded SurrealDB cannot start, the service MUST fail fast
and MUST NOT serve MCP requests.

#### Scenario: Fallback configured and used (happy path)

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND a fallback storage mode is configured
- WHEN embedded SurrealDB fails to start
- THEN the service starts using the fallback storage mode
- AND the service reports that it is running in fallback mode

#### Scenario: No fallback configured (edge case)

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND no fallback storage mode is configured
- WHEN embedded SurrealDB fails to start
- THEN the service fails to start
- AND no MCP requests are served

### Requirement: TUI Out of Scope for Phase 1

This change SHALL NOT introduce or modify TUI requirements or behavior.

Migration tooling MUST be operable without any TUI dependency.

#### Scenario: Migration tooling operates without TUI (happy path)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN an operator runs the migration tooling
- THEN the tooling completes without requiring a TUI

#### Scenario: TUI remains optional (edge case)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN the Cerebro service starts
- THEN MCP requests are still served without a UI
