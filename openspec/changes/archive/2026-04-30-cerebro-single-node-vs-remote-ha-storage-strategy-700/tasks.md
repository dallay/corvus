# Tasks: Cerebro Single-Node vs Remote HA Storage Strategy

## Phase 1: Infrastructure

- [x] 1.1 Review `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/proposal.md`, `design.md`, and both spec deltas to extract the approved support-boundary vocabulary and ownership split.
- [x] 1.2 Capture a short evidence checklist from `clients/cerebro/src/config.rs`, `clients/cerebro/src/storage/mod.rs`, `clients/cerebro/src/storage/surreal.rs`, and `.github/workflows/_build-cerebro-binaries.yml` to anchor later wording updates to current behavior.

## Phase 2: Implementation

- [x] 2.1 Update `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/gateway/spec.md` so operator-facing language says durable production is single-node, local-first, and node-local only.
- [x] 2.2 Update `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/gateway/spec.md` so remote/shared SurrealDB, HA multi-node persistence, and CI-only storage modes are described with explicit supported vs unsupported wording.
- [x] 2.3 Update `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/cerebro/spec.md` so `embedded_surreal`, `disk`, `in_memory`, and fallback behavior use the approved local-first support terminology.
- [x] 2.4 Update `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/cerebro/spec.md` so `remote_surreal` and shared persistence are explicitly unsupported in this build and deferred to a separate future change.
- [x] 2.5 Align operator docs in `clients/web/apps/docs/src/content/docs/cerebro/configuration.md` and `clients/web/apps/docs/src/content/docs/cerebro/operations.md` with the new support boundary without adding remote-storage implementation promises.
- [x] 2.6 Update any verification-facing wording in `.github/workflows/_build-cerebro-binaries.yml` comments or adjacent artifact notes, if needed, so `in_memory` smoke startup is clearly test-only scaffolding rather than production support.

## Phase 3: Testing

- [x] 3.1 Verify every scenario in `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/gateway/spec.md` uses RFC 2119 language and matches the approved operator-topology boundary.
- [x] 3.2 Verify every scenario in `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/cerebro/spec.md` matches current runtime evidence for local modes, fallback limits, and remote rejection.
- [x] 3.3 Run a cross-artifact wording audit across the updated spec deltas, docs, and workflow artifact to confirm there are no residual HA, shared-persistence, or remote-supported claims in this build.
