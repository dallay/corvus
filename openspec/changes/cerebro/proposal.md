# Proposal: Cerebro Module Extraction

## Intent

Introduce a new `cerebro` module that centralizes long-term, agent-agnostic memory behind an MCP
service while keeping per-agent local memory short-term and private. Remove the SurrealDB backend
from the agent runtime so memory persistence flows through Cerebro via MCP.

## Scope

### In Scope

- Create a new Rust module/binary `modules/cerebro` implementing the MCP memory service defined in
  `openspec/changes/cerebro/cerebro.md`.
- Shift agent runtime memory integration to the MCP protocol (Cerebro), replacing the SurrealDB
  backend in `clients/agent-runtime`.
- Update memory configuration and tool wiring in the runtime to align with the Cerebro tool surface
  (with aliasing for legacy tool names).
- Define separation between local short-term memory and centralized Cerebro memory scopes.

### Out of Scope

- Building the optional TUI for Cerebro.
- Full data migration tooling beyond the initial alias/bridge strategy.
- Embedding SurrealDB as a default deployment mode for Cerebro.

The **initial alias/bridge strategy** is limited to runtime tool aliasing and a lightweight
in-process bridge in `clients/agent-runtime` that forwards reads/writes to the Cerebro MCP service
in `modules/cerebro`, as defined in `openspec/changes/cerebro/cerebro.md`. It explicitly excludes
bulk migration/ETL/import/export; historical data must be migrated manually or via future tooling.

## Approach

Implement a new `cerebro` Rust module that exposes the MCP tools API described in
`openspec/changes/cerebro/cerebro.md`, backed by the existing storage model with a clear distinction
between
short-term local memory (runtime-owned) and long-term shared memory (Cerebro). In the agent
runtime, remove the SurrealDB backend and re-route memory tool calls through MCP adapters to
Cerebro, retaining legacy tool names as aliases during a transition period.

## Affected Areas

| Area                                          | Impact           | Description                                                    |
|-----------------------------------------------|------------------|----------------------------------------------------------------|
| `modules/cerebro`                             | New              | New Rust crate/binary providing MCP memory service.            |
| `clients/agent-runtime/src/memory/`           | Modified/Removed | Remove SurrealDB backend and re-wire memory factory to MCP.    |
| `clients/agent-runtime/src/config/schema.rs`  | Modified         | Adjust `MemoryConfig` for Cerebro/MCP selection and defaults.  |
| `clients/agent-runtime/Cargo.toml`            | Modified         | Remove `memory-surreal` feature and update dependencies.       |
| `clients/agent-runtime/src/tools/memory_*.rs` | Modified         | Map legacy tools to new Cerebro tool names and adapters.       |
| `clients/agent-runtime/src/tools/mcp/`        | Modified         | Add/extend MCP tool adapters for Cerebro memory operations.    |
| `openspec/changes/cerebro/cerebro.md`         | Reference        | Source specification for tool contracts and security defaults. |

## Risks

| Risk                                          | Likelihood | Mitigation                                                            |
|-----------------------------------------------|------------|-----------------------------------------------------------------------|
| Memory regressions from backend removal       | Medium     | Maintain legacy tool aliases; validate via integration tests.         |
| Data loss or missing history                  | Medium     | Preserve existing SQLite/local memory; plan migration in later phase. |
| MCP connectivity failures                     | Medium     | Return structured errors, log/alert on failures, and enforce no
|                                               |            | fallback to local memory (legacy-only opt-in is out of scope).         |
| Security misconfiguration of remote endpoints | Low        | Enforce secure defaults and explicit opt-in for insecure modes.       |

## Rollback Plan

Re-enable the SurrealDB backend and restore the `memory-surreal` feature flag in
`clients/agent-runtime`, reverting configuration defaults to the previous local/Surreal path.
Restore legacy memory tool wiring if MCP Cerebro integration introduces regressions.

## Dependencies

- MCP runtime tooling spec (`openspec/specs/mcp-runtime/spec.md`).
- Cerebro tool contract and security model (`openspec/changes/cerebro/cerebro.md`).

## Success Criteria

- [ ] Agent runtime uses MCP to access Cerebro for long-term memory operations.
- [ ] SurrealDB backend is removed from `clients/agent-runtime` and memory remains functional.
- [ ] Legacy memory tool names continue to work via aliases/bridge to Cerebro tools.
- [ ] Configuration clearly separates local short-term memory from shared Cerebro memory.
