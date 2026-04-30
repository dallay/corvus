# Delta for gateway

## ADDED Requirements

### Requirement: Cerebro Supported Durable Production Topology

The gateway specification MUST treat Cerebro's current durable production posture as **single-node and local-first only**.

The supported durable production topology in this build MUST be exactly one Cerebro node using node-local durable storage.

The gateway specification MUST identify embedded SurrealDB as the default supported durable production mode.

The gateway specification MAY identify `disk` as a supported node-local durable alternative when operators intentionally choose a simpler local storage mode.

The gateway specification MUST NOT describe remote/shared SurrealDB, shared remote persistence, or HA multi-node durable production as supported in this build.

#### Scenario: Single-node durable production is the only supported topology

- GIVEN an operator reads the gateway source-of-truth for Cerebro production deployment posture
- WHEN the operator determines which durable production topology is currently supported
- THEN the specification MUST state that exactly one Cerebro node with node-local durable storage is supported
- AND the specification MUST identify embedded SurrealDB as the default supported durable mode

#### Scenario: Local durable alternative remains bounded to one node

- GIVEN an operator evaluates the `disk` storage mode for production use
- WHEN the operator checks whether that mode changes the supported topology class
- THEN the specification MUST describe `disk` only as a node-local durable alternative
- AND the specification MUST NOT imply that `disk` enables shared persistence or HA multi-node operation

### Requirement: Unsupported Remote and HA Persistence Claims

The gateway specification MUST define remote/shared SurrealDB and HA multi-node persistence as unsupported in this build.

Any gateway-facing operational, release, or deployment guidance MUST NOT present `remote_surreal` as an available production topology switch.

Gateway-facing guidance MUST NOT claim active-active, shared-store, clustered, or multi-node durable persistence for Cerebro until a separate change explicitly specifies and verifies that capability.

#### Scenario: Remote shared storage is described as unsupported

- GIVEN an operator reads gateway-facing deployment guidance for Cerebro storage topology
- WHEN the guidance addresses remote/shared persistence
- THEN the guidance MUST state that remote/shared SurrealDB is unsupported in this build
- AND the guidance MUST NOT present `remote_surreal` as currently available production support

#### Scenario: HA claim is rejected without follow-on specification

- GIVEN a gateway-facing artifact attempts to describe Cerebro as HA or multi-node durable in the current build
- WHEN that claim is compared against the gateway source-of-truth
- THEN the claim MUST be treated as non-compliant with the specification
- AND the source-of-truth MUST require a separate follow-on change before such a claim can be supported

### Requirement: Operational Guidance for Single-Node Local-First Durability

The gateway specification MUST require operator-facing guidance to describe Cerebro durable production as one durable node backed by local persistence and external backup, restore, and replacement procedures.

The gateway specification SHOULD permit CI, development, and bounded smoke validation flows to use an explicit non-durable or non-default storage mode when that mode is chosen only for testability and does not redefine production support.

The gateway specification MUST distinguish such CI-safe startup validation from the supported durable production topology.

#### Scenario: Operator guidance separates production posture from backup strategy

- GIVEN an operator reads the gateway operational guidance for durable Cerebro deployment
- WHEN the guidance describes resilience expectations
- THEN the guidance MUST instruct the operator to treat Cerebro as one durable local-first node
- AND the guidance MUST describe backup, restore, or node replacement procedures rather than HA multi-node persistence as the resilience strategy

#### Scenario: CI-safe storage mode does not redefine production support

- GIVEN a release or CI smoke validation runs Cerebro with an explicit non-embedded storage mode suitable for CI
- WHEN an operator or maintainer interprets that validation posture
- THEN the specification MUST treat the CI-safe mode as test-only operational scaffolding
- AND the specification MUST NOT infer from that validation that non-local or HA durable production is supported
