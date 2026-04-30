# Proposal: Cerebro Single-Node vs Remote HA Storage Strategy

## Intent

Make the supported Cerebro production storage posture explicit in OpenSpec so operators, release validation, and future implementation work all align with the current build reality.

Today, **embedded/local-first single-node durable production is the only supported mode**. Cerebro can run durably with embedded SurrealDB as the default production mode, with `disk` as a simpler node-local durable alternative. **Remote/shared SurrealDB and HA multi-node persistence are not supported in this build** and MUST NOT be presented as available production topologies.

This change is needed because the current implementation, startup validation, and runtime storage factory already reject remote SurrealDB, while surrounding docs and configuration surface can still be misread as implying a broader HA/storage story than actually exists.

## Scope

### In Scope
- Formalize `gateway` as the main spec domain for Cerebro’s externally served operational posture and supported deployment topology.
- Clarify that single-node, local-first durable production is the only supported production mode in the current build.
- Clarify that remote/shared SurrealDB and HA multi-node persistence are unsupported in the current build.
- Document the product and architecture decision, including operator-facing tradeoffs and constraints.
- Define high-level future boundaries for a later remote-storage/HA change without committing this proposal to implementation.
- Align supporting `cerebro` spec language with the same storage support boundary.

### Out of Scope
- Implementing `remote_surreal` support in Cerebro.
- Adding shared persistence, clustering, failover coordination, or multi-node write/read semantics.
- Changing startup validation, storage factories, migration code, readiness behavior, or release workflows in this proposal.
- Defining a detailed technical design for remote SurrealDB connectivity, auth, TLS, retries, migration, or HA orchestration.
- Making any code, documentation, or spec edits beyond this proposal artifact.

## Approach

Adopt a **single-node now, HA later** product and architecture position.

The proposal will treat the current codebase as the source of truth: durable production support is local-first and node-local, with embedded SurrealDB as the default supported mode. This matches current validation behavior, storage construction, migration assumptions, and release smoke posture.

The main domain for the change is `gateway` because the primary question is not only internal storage capability, but also what operators may claim about the served Cerebro HTTP/MCP surface in production. The `gateway` spec already owns startup posture, bind/auth assumptions, and operational acceptance expectations for the externally served system. The `cerebro` domain remains the supporting spec area for storage-mode behavior and internal constraints.

This proposal intentionally does **not** promise remote persistence in the current release. If remote/shared storage is brought in scope later, it should land as a separately scoped change that introduces explicit backend support, readiness semantics, security requirements, migration boundaries, and topology acceptance criteria before any HA claim is made.

## Product / Architecture Decision

### Decision
- Cerebro durable production support is **single-node and local-first only** in the current build.
- Embedded SurrealDB is the default supported durable production mode.
- `disk` remains a node-local durable alternative with simpler operational characteristics.
- `in_memory` remains non-durable and suitable only for CI, development, or emergency fallback scenarios.
- `remote_surreal`, shared remote persistence, and HA multi-node production claims are **not supported** in this build.

### Rationale
- The current implementation already enforces this boundary at configuration validation and storage construction time.
- The migration/import story is currently built around embedded/local operation.
- Readiness, outage handling, retries, TLS/auth, and consistency semantics for remote storage are not yet implemented.
- A spec that advertises remote/HA availability before those guarantees exist would mislead operators and weaken release truthfulness.

### Tradeoffs and Operational Constraints
- The supported production story is simpler and lower risk, but horizontal durability through shared persistence is unavailable.
- Operators must treat a Cerebro durable deployment as one durable node with local storage, backed by external backup/restore and replacement procedures rather than active-active persistence.
- Node-local durability reduces distributed-system complexity, but it limits HA posture, maintenance flexibility, and scale-out options.
- Multi-node fronting, shared storage failover, and remote database dependency management all remain future work and MUST NOT be assumed by deployment guidance in this build.

## Future Direction Boundaries

If remote/shared storage is later brought in scope, it SHOULD be handled as a dedicated follow-on change with clearly bounded acceptance goals rather than implied by this proposal.

High-level future direction:
- Add explicit remote SurrealDB backend construction and validation.
- Define configuration requirements for URL, credentials, namespace/database selection, TLS, and secret handling.
- Add readiness and degraded-state semantics for remote dependency outages.
- Define migration support boundaries for remote targets.
- Add separate verification coverage for remote-mode startup and integration behavior.
- Only claim HA or multi-node support after shared persistence semantics, failure handling, and operator guidance are fully specified and verified.

This future direction is informational only and does not authorize implementation under the current proposal.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/gateway/spec.md` | Modified | Main source-of-truth for Cerebro served-surface production posture, supported topology claims, and operational constraints. |
| `openspec/specs/cerebro/spec.md` | Modified | Supporting source-of-truth for storage-mode behavior, local-first durability, and remote-storage unsupported status. |
| `clients/cerebro` | Affected context only | Owning runtime/storage package whose current implementation already enforces the support boundary; no implementation changes are part of this proposal. |
| `clients/web/apps/docs/src/content/docs/cerebro/` | Affected context only | Operator documentation area that will need alignment in a later doc/spec update phase. |
| `.github/workflows/_build-cerebro-binaries.yml` | Affected context only | Release/smoke workflow whose current assumptions reinforce non-remote startup posture. |

## Affected Modules / Packages

- `clients/cerebro`
- `clients/web/apps/docs`
- `openspec/specs/gateway`
- `openspec/specs/cerebro`
- `.github/workflows`

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Operators continue to infer HA support from config surface or partial docs | Medium | Make the supported/unsupported boundary explicit in the main `gateway` spec and supporting `cerebro` wording. |
| Future work may accidentally overpromise remote storage before implementation is complete | Medium | Reserve remote/shared persistence for a separate follow-on change with explicit acceptance gates. |
| Ownership between `gateway` and `cerebro` may become duplicated or contradictory | Medium | Keep `gateway` as the operational source-of-truth and use `cerebro` only for storage-behavior details. |
| The single-node decision may be perceived as a regression rather than a clarification | Low | Frame the change as an alignment of product messaging to existing enforced behavior, not a removal of shipped HA support. |

## Rollback Plan

If this proposal proves directionally incorrect, revert the subsequent spec changes that formalize `gateway` as the main operational domain for this storage posture and restore the prior wording while reassessing ownership and deployment claims.

Because this proposal introduces no implementation changes, rollback is documentation/spec-only:
- revert the resulting `gateway` and `cerebro` spec deltas,
- remove any follow-on wording that declares remote/HA unsupported if the product decision changes, and
- reopen exploration to choose a different support boundary before implementation begins.

## Dependencies

- Existing exploration at `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/exploration.md`
- Current source-of-truth behavior in `clients/cerebro` configuration and storage initialization
- Existing `gateway` and `cerebro` OpenSpec domains for operational and storage behavior ownership

## Success Criteria

- [ ] The proposal states unambiguously that embedded/local-first single-node durable production is the only supported mode today.
- [ ] The proposal states unambiguously that remote/shared SurrealDB and HA multi-node persistence are not supported in this build.
- [ ] The proposal identifies `gateway` as the main spec domain and `cerebro` as a supporting domain for this change.
- [ ] The proposal documents tradeoffs and operational constraints without implying current HA support.
- [ ] The proposal provides a high-level future direction for remote/shared storage without committing implementation scope in this change.
- [ ] The proposal includes affected modules/packages, risks, and a rollback plan consistent with `openspec/config.yaml`.
