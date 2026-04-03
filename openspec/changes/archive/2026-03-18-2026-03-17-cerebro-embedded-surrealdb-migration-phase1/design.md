# Design: Cerebro Embedded SurrealDB Migration Phase 1

## Technical Approach

Move Cerebro storage to an embedded SurrealDB engine by default while keeping existing in-memory and
file-backed modes available. Add a migration CLI within `modules/cerebro/` that imports legacy
SurrealDB data into the embedded store and validates the results using deterministic record
counts and checksums. All changes are additive, keep the MCP tool surface unchanged, and preserve
secure configuration defaults. The TUI remains explicitly out of scope for this phase.

## Architecture Decisions

### Decision: Default storage mode is embedded SurrealDB

**Choice**: Change the default `StorageMode` in `modules/cerebro/src/config.rs` from `InMemory` to
`EmbeddedSurreal`, with explicit configuration to opt into `InMemory` or `Disk`.
**Alternatives considered**: Keep `InMemory` default; default to `Disk` JSON storage; require an
explicit `storage_mode` for all deployments.
**Rationale**: The proposal requires embedded SurrealDB as the default for new deployments while
preserving explicit overrides. `InMemory` is not durable and `Disk` JSON is a stopgap. A default
that matches the long-term persistence model avoids silent data loss.

### Decision: No silent fallback on storage init

**Choice**: Fail startup if embedded SurrealDB initialization fails unless an explicit
`storage_fallback` policy is provided.
**Alternatives considered**: Always fall back to `InMemory`; auto-fallback to `Disk`.
**Rationale**: Silent fallback can cause unintentional data loss. Explicit fallback keeps operators
in control and aligns with security-first and data integrity requirements.

### Decision: Migration tooling via CLI subcommands

**Choice**: Add a `cerebro` CLI with `serve`, `migrate import`, and `migrate validate` subcommands
implemented with `clap` under `modules/cerebro/src/bin/`.
**Alternatives considered**: Separate `cerebro-migrate` binary; embed migration in MCP tools.
**Rationale**: A single CLI keeps operational workflows consistent and avoids expanding the MCP
surface. `clap` is already in use elsewhere in the repo, so it matches established patterns.

### Decision: Migration data format is file-based, SurrealDB export compatible

**Choice**: Accept legacy exports as files (e.g., SurrealDB export output) and provide a mapping to
Cerebro memory records. Validation checks operate on normalized record representations.
**Alternatives considered**: Direct database-to-database copy over network; raw SurrealDB query
reads.
**Rationale**: File-based imports are deterministic, easier to audit, and reduce security risk by
avoiding direct network credentials inside the tool. This supports offline validation and rollback.

### Decision: Validation uses record counts + content checksums

**Choice**: Compute a per-collection count and stable checksum (SHA-256 of canonical JSON) for
legacy export and for imported embedded data.
**Alternatives considered**: Count-only validation; random sampling.
**Rationale**: Count-only validation can miss corruption. Deterministic checksums provide a
practical integrity guarantee without requiring full cryptographic provenance.

## Data Flow

### Embedded storage initialization and fallback

```text
Cerebro::main
  └─ CerebroConfig::load
        └─ storage_from_config
              ├─ init EmbeddedSurrealStore
              │     └─ open/create storage_path
              │           └─ run migrations / schema checks
              └─ if init error
                    ├─ fallback policy? -> init fallback store
                    └─ else -> error, fail startup
```

### Migration import and validation

```text
Operator
  └─ cerebro migrate import --source export.surreal --target ./cerebro.db
        ├─ parse legacy export
        ├─ map legacy records -> MemoryRecord/Session/Prompt
        ├─ write to embedded SurrealDB (transactional batches)
        └─ emit import report (counts + checksums)

Operator
  └─ cerebro migrate validate --source export.surreal --target ./cerebro.db
        ├─ recompute legacy checksums
        ├─ compute embedded checksums
        └─ compare + report discrepancies
```

### Sequence diagram: startup with explicit fallback

```text
Client/Operator -> Cerebro binary: start
Cerebro binary -> CerebroConfig: load defaults + overrides
CerebroConfig -> storage_from_config: init EmbeddedSurrealStore
storage_from_config -> EmbeddedSurrealStore: open storage_path
EmbeddedSurrealStore --> storage_from_config: ok | error
storage_from_config -> storage_fallback: init fallback store (if configured)
storage_from_config --> Cerebro binary: storage handle | error
Cerebro binary -> MCP server: serve requests
```

### Sequence diagram: migration import + validation

```text
Operator -> CLI: migrate import
CLI -> LegacyReader: parse export
LegacyReader --> CLI: normalized records + checksums
CLI -> EmbeddedSurrealStore: begin batch
EmbeddedSurrealStore --> CLI: write success
CLI -> EmbeddedSurrealStore: commit
CLI -> ReportWriter: output import report

Operator -> CLI: migrate validate
CLI -> LegacyReader: recompute legacy checksums
CLI -> EmbeddedSurrealStore: compute embedded checksums
CLI -> Comparator: compare counts + checksums
Comparator --> CLI: validation status
```

## File Changes

| File                                                                 | Action | Description                                                                          |
|----------------------------------------------------------------------|--------|--------------------------------------------------------------------------------------|
| `modules/cerebro/src/config.rs`                                      | Modify | Add embedded/remote storage config fields, default to embedded, add fallback policy. |
| `modules/cerebro/src/storage/mod.rs`                                 | Modify | Add embedded SurrealDB storage implementation and fallback selection logic.          |
| `modules/cerebro/src/storage/surreal.rs`                             | Create | Embedded SurrealDB storage adapter implementing `Storage`.                           |
| `modules/cerebro/src/migration/mod.rs`                               | Create | Migration orchestration: import + validate workflows.                                |
| `modules/cerebro/src/migration/legacy.rs`                            | Create | Legacy export reader + normalization + checksum generation.                          |
| `modules/cerebro/src/migration/report.rs`                            | Create | Import/validation report output (JSON + human-readable).                             |
| `modules/cerebro/src/bin/cerebro.rs`                                 | Create | CLI entrypoint with `serve` and `migrate` subcommands.                               |
| `modules/cerebro/src/main.rs`                                        | Modify | Delegate to CLI or keep as thin `serve` wrapper.                                     |
| `modules/cerebro/Cargo.toml`                                         | Modify | Add SurrealDB + clap + checksum dependencies.                                        |
| `modules/cerebro/tests/`                                             | Modify | Add migration validation tests; update storage default tests.                        |
| `clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md` | Modify | Document import/validate tooling and embedded default.                               |
| `openspec/specs/cerebro/spec.md`                                     | Modify | Update delta spec references to include migration tooling scope.                     |

## Interfaces / Contracts

### Config additions

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageMode {
  EmbeddedSurreal,
  RemoteSurreal,
  InMemory,
  Disk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageFallback {
  None,
  InMemory,
  Disk,
  RemoteSurreal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealConfig {
  pub namespace: String,
  pub database: String,
  pub storage_path: Option<String>,
  pub remote_url: Option<String>,
  pub username: Option<String>,
  pub password: Option<SecretString>,
}
```

### CLI surface (draft)

```text
cerebro serve [--config <path>]

cerebro migrate import \
  --source <legacy_export> \
  --target <embedded_path> \
  [--namespace <ns>] [--database <db>] \
  [--dry-run]

cerebro migrate validate \
  --source <legacy_export> \
  --target <embedded_path> \
  [--namespace <ns>] [--database <db>]
```

### Migration report schema (summary)

```json
{
  "source": "legacy_export.surreal",
  "target": "./cerebro.db",
  "collections": {
    "memory": { "count": 1234, "checksum": "sha256:..." },
    "session": { "count": 22, "checksum": "sha256:..." },
    "prompt": { "count": 57, "checksum": "sha256:..." }
  },
  "status": "ok" | "mismatch" | "error"
}
```

## Testing Strategy

| Layer       | What to Test                            | Approach                                                     |
|-------------|-----------------------------------------|--------------------------------------------------------------|
| Unit        | Config defaults and fallback policy     | Assert default storage mode and explicit fallback behaviors. |
| Unit        | Legacy parsing + checksum normalization | Golden fixtures for export parsing and stable hash results.  |
| Integration | Embedded SurrealDB storage              | Write/read/search/delete against embedded store.             |
| Integration | Migration import + validate             | Import fixture export, verify counts + checksums match.      |
| E2E         | CLI workflows                           | Run `cerebro migrate import/validate` against temp dirs.     |

## Migration / Rollout

- No in-place runtime migration. Operators run explicit import tooling and verify results.
- Default storage mode becomes embedded for new deployments; existing deployments can override via
  config to `RemoteSurreal`, `Disk`, or `InMemory`.
- Rollout steps:
    1. Export legacy SurrealDB data to file.
    2. Run `cerebro migrate import` targeting the embedded path.
    3. Run `cerebro migrate validate` and review report.
    4. Switch Cerebro config to embedded default and restart.
- Backward compatibility:
    - Existing `storage_mode` values remain supported.
    - `storage_path` continues to work for `Disk` or embedded path.
    - MCP tool behavior remains unchanged.

## Security Constraints

- Never log auth tokens or database credentials.
- Require explicit configuration for remote SurrealDB endpoints.
- Reject insecure remote endpoints by default (loopback only for dev).
- Ensure embedded storage paths are created with restrictive permissions where possible.
- Migration tooling must treat legacy exports as untrusted input and validate schema/fields.

## Data Integrity and Error Handling

- All import operations must be transactional per batch; on error, abort and leave target
  consistent.
- Validation must compare both counts and checksums; mismatches return non-zero exit codes.
- Provide dry-run mode that parses and reports counts/checksums without writes.
- On storage init errors, fail startup unless a fallback policy is explicitly configured.

## Open Questions

- [ ] Which legacy export formats need to be supported in phase 1 (SurrealDB SQL export, JSON
  export, or both)?
- [ ] Should the migration tool support direct remote SurrealDB reads in phase 1, or only file
  exports?
- [ ] Do we need to persist migration reports to disk by default for audit trails?
