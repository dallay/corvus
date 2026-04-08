# Design: Next-Stage Routing Capabilities

## Technical Approach

This is a decision-only design for archival, not an implementation plan. It records that the v1.0.0
routing surface remains the already shipped request-time model routing and query classification flow
defined by `productize-model-routing`, while embedding routes and managed route updates remain
deferred.

The design follows:

- `openspec/changes/next-stage-routing-capabilities/proposal.md`
- `openspec/changes/next-stage-routing-capabilities/specs/model-routing/spec.md`
- existing routing behavior documented in `openspec/specs/model-routing/spec.md`

## Architecture Decisions

### Decision: Keep request-time routing productized as-is for v1.0.0

**Choice**: Treat the shipped `[[model_routes]]` + `[query_classification]` flow as the complete
v1.0.0 routing product surface.

**Alternatives considered**: Extend v1.0.0 with embedding-specific routing; reopen operator UX/docs
work for request-time routing.

**Rationale**: Request-time routing already has a formal spec, operator docs, examples, and doctor
diagnostics. Reopening it would create duplicate scope without closing a demonstrated product gap.
The existing operator model is already coherent and shippable for v1.0.0.

### Decision: Defer embedding routes

**Choice**: Do not add a first-class embedding routing feature in this change.

**Alternatives considered**: Add `[[embedding_routes]]` now, or reserve schema-only configuration
for future use.

**Rationale**: The current memory stack uses a single embedding provider/model from
`memory.embedding_provider` and `memory.embedding_model`. That matches today's workload better than
route-style dispatch. Embedding flows also have a stronger consistency constraint than request-time
model selection: storing and recalling vectors across mixed models can create incompatible vector
spaces. For v1.0.0, one configured embedding profile is safer and sufficient.

### Decision: Keep `config.toml` as the source of truth

**Choice**: Continue using file-based routing configuration as the approved operating model.

**Alternatives considered**: Managed route mutation through an admin API, dashboard, or agent-facing
update surface.

**Rationale**: The current system already treats `config.toml` as the authoritative operational
contract, validated by `corvus doctor` and applied through explicit operator change management. This
keeps configuration reviewable, restart-bounded, and easy to audit through normal file and
version-control workflows. No additional mutable control plane is needed for v1.0.0.

### Decision: Defer managed route updates

**Choice**: Do not add runtime-managed route updates yet.

**Alternatives considered**: Immediate admin/API support for route edits and route rollout.

**Rationale**: Managed updates introduce a new privileged mutation surface with unanswered
requirements around authentication, authorization, approval, validation parity, rollback, and audit
logging. Operationally, config edits are simpler and safer today: edit TOML, validate, restart,
observe. Security-wise, deferring managed updates avoids expanding attack surface before Corvus has
a broader admin/control-plane model ready to govern it.

## Data Flow

This change adds no runtime flow. It preserves the current operational decision path:

```text
Operator edits config.toml
        ↓
Operator runs `corvus doctor`
        ↓
Operator restarts Corvus
        ↓
Request-time routing uses existing [[model_routes]] / [query_classification]
        ↓
Embedding workloads continue using one memory embedding provider/model
```

## File Changes

| File                                                                           | Action    | Description                                                                                  |
|--------------------------------------------------------------------------------|-----------|----------------------------------------------------------------------------------------------|
| `openspec/changes/next-stage-routing-capabilities/design.md`                   | Create    | Records the final technical rationale for the decision-only change.                          |
| `openspec/changes/next-stage-routing-capabilities/proposal.md`                 | Reference | Defines the decision-only scope and archival intent.                                         |
| `openspec/changes/next-stage-routing-capabilities/specs/model-routing/spec.md` | Reference | Defines the closure and deferral requirements this design supports.                          |
| `openspec/specs/model-routing/spec.md`                                         | Reference | Remains the source of truth for shipped request-time routing behavior.                       |
| `clients/agent-runtime/src/config/schema.rs`                                   | Reference | Confirms that memory embeddings remain single-provider configuration in the current product. |
| `clients/agent-runtime/src/memory/embeddings.rs`                               | Reference | Confirms the current embedding provider factory is single-profile, not routed.               |

## Interfaces / Contracts

No new interfaces, APIs, config schema, or runtime contracts are introduced.

The only contract decision recorded here is scope:

- v1.0.0 request-time routing remains as already shipped
- embedding routes are deferred
- managed route updates are deferred
- `config.toml` remains authoritative for routing configuration

## Testing Strategy

| Layer             | What to Test                                      | Approach                                                                   |
|-------------------|---------------------------------------------------|----------------------------------------------------------------------------|
| Artifact review   | Decision text matches proposal/spec scope         | Manual review of `proposal.md`, delta spec, and this design                |
| Source validation | Existing product state supports the design claims | Confirm references in `schema.rs`, `embeddings.rs`, and routing docs/specs |
| Runtime           | None                                              | No runtime change in scope                                                 |

## Migration / Rollout

No migration required.

No rollout required. This change documents and preserves v1.0.0 scope decisions only.

## Open Questions

- None
