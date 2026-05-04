# Exploration: cerebro-single-node-vs-remote-ha-storage-strategy-700

## Current State
Cerebro is currently local-first and effectively single-node for durable production use. The default `storage_mode` is `embedded_surreal`, and startup validation hard-rejects both `storage_mode = remote_surreal` and `storage_fallback = remote_surreal` with `NotImplemented` errors (`clients/cerebro/src/config.rs`). The storage factory only supports embedded SurrealDB, disk, and in-memory at runtime; `SurrealStorage::new_remote()` is a stub that always returns `NotImplemented` (`clients/cerebro/src/storage/surreal.rs`).

Operationally, this means:
- Durable production today means one Cerebro instance with local embedded SurrealDB/RocksDB storage.
- `disk` is a simpler durable fallback, but still local to one node.
- `in_memory` is non-durable and suitable only for CI/dev or emergency fallback.
- Remote/HA shared storage is not available in this build, so true multi-node Cerebro behind shared persistence is not currently supported.

This is already reflected in the source-of-truth and docs:
- `openspec/specs/cerebro/spec.md` explicitly says remote SurrealDB is unavailable in this build and defines embedded SurrealDB as the default mode.
- Public docs mark `remote_surreal` as “not yet implemented”.
- CI smoke validation in `openspec/specs/gateway/spec.md` and `.github/workflows/_build-cerebro-binaries.yml` already assumes an explicit non-embedded CI-safe mode (`in_memory`), which shows the gateway domain already owns the operational startup posture for the served HTTP/MCP surface.

## Affected Areas
- `clients/cerebro/src/config.rs` — storage enums, startup validation, embedded-only security constraints, and production posture.
- `clients/cerebro/src/storage/mod.rs` — storage factory and fallback orchestration.
- `clients/cerebro/src/storage/surreal.rs` — embedded SurrealDB implementation and the unimplemented remote constructor.
- `clients/cerebro/src/server.rs` — readiness semantics; would need to reflect remote backend availability/failure modes if remote storage is added.
- `clients/cerebro/src/main.rs`
- `clients/cerebro/src/bin/cerebro.rs` — startup path enforces current storage constraints before serving HTTP/MCP.
- `clients/cerebro/src/migration/mod.rs` — migration tooling currently targets embedded storage only.
- `clients/web/apps/docs/src/content/docs/cerebro/configuration.md` — documents `remote_surreal` as not implemented.
- `clients/web/apps/docs/src/content/docs/cerebro/operations.md` — production guidance currently implies embedded/local durability and warns against remote mode.
- `openspec/specs/cerebro/spec.md` — current behavioral source for storage defaults, fallback, migration, and “remote unavailable in this build”.
- `openspec/specs/gateway/spec.md` — should own the operational/serving implications because Cerebro’s externally served HTTP/MCP production posture, startup smoke expectations, and bind/auth assumptions already live here.

## Approaches
1. **Document and formalize single-node/local-first as the supported production story** — keep remote storage out of scope for this change, clarify that production HA is not yet supported, and tighten specs/docs around acceptable deployment patterns.
   - Pros: Lowest risk; aligns with real code; avoids overcommitting to unfinished HA behavior; easy to verify.
   - Cons: Does not deliver remote/shared persistence; production scaling story remains limited to one durable node plus external failover/restore procedures.
   - Effort: Low

2. **Add remote SurrealDB as a supported optional storage backend** — implement `remote_surreal` for primary and fallback use, preserving embedded as default but enabling shared persistence for HA/topology flexibility.
   - Pros: Enables multi-node/shared-store deployments; cleaner HA story; preserves current local-first default while opening a remote option.
   - Cons: Higher implementation and operational complexity; requires connection/auth/TLS/readiness/retry semantics; migration and docs expand materially; likely needs careful compatibility testing against SurrealDB remote engine behavior.
   - Effort: High

3. **Introduce an explicit “single-node now, HA later” dual-mode spec split** — keep code unchanged for now, but define two operating classes: supported local-first/embedded production and future remote/HA production as a separately gated capability.
   - Pros: Honest product messaging; lets proposal work separate immediate operational truth from future architecture; creates a clean path for phased delivery.
   - Cons: Still no HA in this release; may require touching both `cerebro` and `gateway` specs to separate current vs future guarantees.
   - Effort: Medium

## Recommendation
Recommend **Approach 3**, with an immediate proposal that codifies **single-node/local-first as the only supported durable production mode today** while explicitly reserving **remote/HA storage as a future capability**.

Why this approach:
- It matches the actual implementation and existing docs/spec evidence.
- It avoids pretending that `remote_surreal` is a near-ready switch when the code currently rejects it at validation and construction time.
- It creates space for a later implementation change without blocking the current change on a large storage/backend project.
- It lets the proposal use **`gateway` as the owning/main spec domain** for served-surface operational guarantees, while updating `cerebro` as the domain that describes internal storage behavior.

Owning-domain rationale:
- `gateway` already contains the source-of-truth for Cerebro’s HTTP/MCP startup posture, release smoke validation, explicit non-embedded CI-safe storage behavior, and bind/auth operational contract.
- This change is not just about an internal storage adapter; it changes what operators may claim about production deployment topology for the served service.
- Therefore `gateway` should be the main spec domain, with linked `cerebro` deltas for storage-mode behavior and wording.

If remote storage is later brought in scope, likely implementation areas are:
- `clients/cerebro/src/storage/surreal.rs` — implement remote SurrealDB client construction, auth, namespace/database selection, TLS/URL handling, and readiness.
- `clients/cerebro/src/config.rs` — validate `remote_url`, credentials, and any TLS/auth requirements; revisit fallback validation.
- `clients/cerebro/src/storage/mod.rs` — remote primary/fallback orchestration and error classification.
- `clients/cerebro/src/server.rs` — readiness and degraded-state reporting for remote dependency outages.
- `clients/cerebro/src/migration/mod.rs` — import/validate against remote targets or define unsupported migration boundaries.
- Tests/CI/release workflows — add remote-mode integration coverage and decide whether CI keeps `in_memory` for smoke while adding separate remote integration jobs.
- Docs/specs — deployment topology, latency/failure semantics, backup/restore, secrets/TLS handling, and HA claims.

### Risks
- Existing docs/config surface may over-signal that `remote_surreal` is a selectable mode rather than an unavailable placeholder.
- Operators may infer HA from “production default” wording even though embedded and disk modes are still node-local.
- If remote support is proposed too broadly now, the change scope can balloon into storage client work, operability, migrations, and release verification.
- Splitting ownership poorly between `cerebro` and `gateway` could duplicate or conflict on operational truth.

### Ready for Proposal
Yes — propose a change that makes the current support boundary explicit:
- Today: single-node/local-first durable production using embedded SurrealDB; disk as local durable alternative; in-memory for CI/dev/emergency fallback.
- Not supported in this build: remote/shared SurrealDB and HA multi-node persistence.
- Main spec domain: `gateway`, because the operative question is the production/served-surface posture of Cerebro, with supporting updates in `cerebro` for storage behavior details.
