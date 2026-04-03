# Proposal: Cerebro Embedded SurrealDB Migration Phase 1

## Intent

Make embedded SurrealDB the default storage mode for the Cerebro service and deliver full, supported
migration tooling for legacy SurrealDB data (import and validation), enabling existing deployments
to move to the embedded storage path without relying on alias/bridge behavior alone.

## Scope

### In Scope

- Default Cerebro storage mode set to embedded SurrealDB for new deployments.
- Migration tooling beyond alias/bridge, including import of legacy SurrealDB data and validation of
  migrated content.
- Operational guidance for running the migration tooling and verifying results.
- Explicit confirmation that MCP server/tools already exist in `modules/cerebro/` and are reused.

### Out of Scope

- TUI enhancements or implementation (explicitly optional for this phase).
- Changes to the MCP tool surface beyond what is necessary to support migration tooling.
- Any new runtime-local SurrealDB backend (remains removed per current Cerebro spec).

## Assumptions

- The existing MCP server and 13-tool surface in `modules/cerebro/` remain the core interface for
  memory operations.
- Legacy SurrealDB data is accessible in a form that can be exported or connected for migration.
- Embedded SurrealDB can run with acceptable performance for default local deployments.
- Operators need deterministic validation to confirm migrated records and metadata are intact.

## Approach

Introduce a migration tool path within the Cerebro module that can read from legacy SurrealDB
sources, import into embedded SurrealDB, and run validation checks against the expected data model.
Update default configuration so embedded SurrealDB is the default storage mode for Cerebro
deployments while preserving secure configuration defaults and MCP access patterns.

## Affected Areas

| Area                                                     | Impact   | Description                                                          |
|----------------------------------------------------------|----------|----------------------------------------------------------------------|
| `modules/cerebro/`                                       | Modified | Default storage mode selection and migration tooling implementation. |
| `openspec/specs/cerebro/spec.md`                         | Modified | Update deltas to reflect migration tooling now in scope.             |
| `clients/web/apps/docs/src/content/docs/guides/cerebro/` | Modified | Update migration guidance to include import/validate tooling.        |

## Deltas vs Current Implementation

- Current spec explicitly lists SurrealDB migration as out of scope; this change brings migration
  tooling (import/validate) into scope.
- Existing alias/bridge behavior is not sufficient for legacy data access; phase 1 adds supported
  tooling to move data into embedded SurrealDB.
- Embedded SurrealDB becomes the default storage mode for Cerebro rather than just a supported
  option.
- Optional TUI remains out of scope and is explicitly deferred.

## Risks

| Risk                                                  | Likelihood | Mitigation                                                                                     |
|-------------------------------------------------------|------------|------------------------------------------------------------------------------------------------|
| Migration tooling corrupts or partially imports data  | Medium     | Provide validation step and dry-run/verify modes; document backup requirements.                |
| Performance regressions with embedded default storage | Medium     | Include sizing guidance and allow explicit overrides to remote SurrealDB.                      |
| Security regressions from default storage change      | Low        | Preserve secure transport defaults and access controls; validate auth requirements in tooling. |

## Rollback Plan

- Keep the ability to point Cerebro back to a remote SurrealDB instance via configuration.
- Migration tooling is additive; if issues are found, operators can pause migration and continue
  using existing storage until validated.
- Document how to restore from backup and revert configuration defaults if necessary.

## Dependencies

- Access to legacy SurrealDB data sources for migration and validation.
- Updated documentation to guide operators through import and verification.

## Success Criteria

- [ ] New deployments use embedded SurrealDB as the default Cerebro storage mode.
- [ ] Operators can run a migration import from legacy SurrealDB data and validate results.
- [ ] Documentation explains migration steps and verification checks clearly.
- [ ] MCP tool surface remains unchanged for normal runtime operations.
