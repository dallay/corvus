# Design: Cerebro Single-Node vs Remote HA Storage Strategy

## Technical Approach

This change is a **support-boundary codification** change, not a storage implementation change.

The technical strategy is to represent one operational truth consistently across the OpenSpec source-of-truth:

- `gateway` owns the **externally served operational posture** for Cerebro, including what deployment topology operators may claim for the HTTP/MCP service in production.
- `cerebro` owns the **supporting storage behavior details**, including which storage modes are local-only, which are non-durable, and which are explicitly unsupported in this build.
- Runtime code, docs, and CI are treated as **evidence inputs** that justify the spec boundary, not as implementation scope for this change.

This design maps directly to the approved proposal and spec deltas:

- durable production support is **single-node and local-first only**;
- embedded SurrealDB is the **default supported durable production mode**;
- `disk` remains a **node-local durable alternative**;
- `in_memory` remains **non-durable** and acceptable only for CI/dev/emergency fallback contexts;
- `remote_surreal`, shared remote persistence, and HA multi-node durability are **unsupported in this build**.

The change remains intentionally bounded: it clarifies current truth already enforced by `clients/cerebro/src/config.rs`, `clients/cerebro/src/storage/mod.rs`, `clients/cerebro/src/storage/surreal.rs`, startup entrypoints, and release smoke posture. It does **not** introduce remote storage implementation, HA semantics, migration changes, readiness changes, or new operational capabilities.

## Architecture Decisions

### Decision: `gateway` is the primary source-of-truth for operational support posture

**Choice**: Represent the supported production topology primarily in `openspec/specs/gateway/spec.md`, with this change’s `gateway` delta carrying the normative statement that Cerebro durable production is single-node, local-first, and node-local only.

**Alternatives considered**:
- Put the entire support boundary only in `openspec/specs/cerebro/spec.md`.
- Split identical operational wording across both `gateway` and `cerebro` as co-equal owners.
- Treat docs or workflow configuration as the primary operational source-of-truth.

**Rationale**:
- The operator-facing question is fundamentally about the **served Cerebro surface in production**, not just an internal storage adapter.
- `gateway` already owns adjacent operational truth for served posture, startup expectations, bind/auth assumptions, and CI-safe startup validation.
- The repo’s existing source-of-truth already uses the `gateway` domain to describe operational posture around Rook/Cerebro serving behavior, so extending that domain is lower-risk than creating a parallel owner.
- Making `gateway` primary avoids future contradictions where storage internals and operator claims drift apart.

### Decision: `cerebro` remains the supporting owner for storage-mode semantics

**Choice**: Keep `openspec/specs/cerebro/spec.md` responsible for mode-level storage behavior and support boundaries, but subordinate operator topology claims to the `gateway` domain.

**Alternatives considered**:
- Move all storage-mode behavior into `gateway`.
- Remove storage-boundary wording from `cerebro` entirely.

**Rationale**:
- `cerebro` is still the correct place to define the semantics of `embedded_surreal`, `disk`, `in_memory`, fallback behavior, and the explicit unsupported status of `remote_surreal`.
- This preserves separation of concerns: `gateway` answers “what production topology is supported,” while `cerebro` answers “how storage modes behave inside that topology.”
- Keeping supporting storage language in `cerebro` reduces ambiguity for future implementation work when remote storage is later reconsidered.

### Decision: This change updates specification truth, not implementation reality

**Choice**: Limit the change to spec alignment and downstream wording alignment in docs and workflow comments. Do not modify code behavior or workflow execution logic.

**Alternatives considered**:
- Implement `remote_surreal` support now.
- Modify startup validation or readiness behavior to make the unsupported boundary more explicit in code.

**Rationale**:
- Current code already rejects remote storage at validation and construction time:
  - `CerebroConfig::validate_storage()` rejects `StorageMode::RemoteSurreal` and `StorageFallback::RemoteSurreal`.
  - `SurrealStorage::new_remote()` returns `NotImplemented`.
  - `storage_from_config()` and startup entrypoints already reinforce local-first behavior.
- Expanding scope into remote implementation would turn a bounded support-boundary clarification into a larger storage/operability project.
- OpenSpec should first tell the truth about what exists before introducing new capabilities.

### Decision: Unsupported remote/HA capability is expressed as an explicit negative contract

**Choice**: State normatively that remote/shared SurrealDB and HA multi-node persistence are unsupported in this build, rather than merely saying they are “not yet implemented” informally.

**Alternatives considered**:
- Leave unsupported capability implied by missing implementation.
- Use softer language that suggests remote storage may be selectable but incomplete.

**Rationale**:
- The config surface still exposes `remote_surreal` enum values, so omission alone is not sufficient to prevent operator misunderstanding.
- An explicit negative contract prevents accidental overclaiming in specs, docs, release guidance, and future reviews.
- This mirrors the runtime behavior more accurately than vague roadmap language.

### Decision: Terminology must distinguish support class from test posture

**Choice**: Define and reuse a narrow vocabulary:
- **single-node** = exactly one durable Cerebro node is supported;
- **local-first / node-local** = storage is attached to that node, not shared remotely;
- **durable production** = embedded SurrealDB by default, with `disk` as a local durable alternative;
- **non-durable** = `in_memory`, allowed only for CI/dev/emergency fallback;
- **unsupported in this build** = `remote_surreal`, shared persistence, HA multi-node durability.

**Alternatives considered**:
- Continue using mixed wording such as “production,” “default,” “not implemented,” and “fallback” without explicit support-class definitions.

**Rationale**:
- Most ambiguity in the current state comes from terminology drift, not missing code evidence.
- Shared vocabulary makes the support boundary reproducible across specs, docs, CI commentary, and future PR review.
- This also prevents CI smoke choices such as `in_memory` from being misread as production support classes.

## Architecture / Ownership Model

### Ownership boundaries

| Concern | Owning domain | Why |
|---|---|---|
| Supported production topology for Cerebro’s served HTTP/MCP surface | `gateway` | This is an operational contract for what operators may deploy and claim in production. |
| Storage-mode behavior and fallback semantics inside Cerebro | `cerebro` | This is internal service behavior describing how modes behave and which are available. |
| Code enforcement evidence | `clients/cerebro` | Startup validation and storage construction already encode the real boundary. |
| CI smoke posture evidence | `.github/workflows/_build-cerebro-binaries.yml` | Existing smoke startup already uses `in_memory` as test scaffolding, reinforcing the distinction between test posture and production support. |
| Operator documentation alignment | `clients/web/apps/docs/src/content/docs/cerebro/` | Docs must eventually mirror the spec boundary, but are supporting artifacts rather than the normative owner. |

### Operational truth model

The support boundary flows in one direction:

1. **Implementation reality** establishes what actually works.
2. **OpenSpec** formalizes what is supported and what is not.
3. **Docs and CI guidance** must align to the OpenSpec wording.
4. **Future remote/HA work** must be proposed separately before the support boundary changes.

This avoids a failure mode where configuration enums or placeholder docs imply support that the runtime rejects.

## Terminology Rules

The following terminology rules apply to this change and any follow-on artifact updates:

1. `embedded_surreal` MUST be described as the **default supported durable production mode**.
2. `disk` MUST be described only as a **node-local durable alternative**.
3. `in_memory` MUST be described as **non-durable** and suitable only for **CI, development, or emergency fallback**.
4. `remote_surreal` MUST be described as **unsupported in this build**; wording such as “available option” or “production topology switch” MUST NOT be used.
5. “HA,” “multi-node,” “shared persistence,” “clustered,” or “active-active” MUST NOT be used to describe the current build as a supported posture.
6. CI smoke use of a non-default mode MUST be labeled **test-only operational scaffolding** and MUST NOT redefine production support.
7. `gateway` wording MUST describe the **operator claim boundary**; `cerebro` wording MUST describe **storage behavior boundary**.

## Data Flow

This is a truth-propagation change rather than a runtime request-path change.

### Support-boundary propagation

```text
Cerebro runtime reality
  ├─ config validation rejects remote_surreal
  ├─ storage factory only supports local modes
  └─ remote constructor returns NotImplemented
           │
           ▼
OpenSpec source-of-truth
  ├─ gateway: operational/served production posture
  └─ cerebro: supporting storage behavior semantics
           │
           ▼
Downstream alignment artifacts
  ├─ operator docs
  ├─ release/CI guidance
  └─ future reviews / follow-on changes
```

### Sequence diagram

```mermaid
sequenceDiagram
    participant Impl as clients/cerebro implementation
    participant GatewaySpec as gateway spec domain
    participant CerebroSpec as cerebro spec domain
    participant DocsCI as docs / CI guidance
    participant FutureChange as future remote/HA change

    Impl->>GatewaySpec: Evidence: served posture is single-node/local-first
    Impl->>CerebroSpec: Evidence: supported modes are embedded/disk/in_memory; remote unsupported
    GatewaySpec->>DocsCI: Publish operator support boundary
    CerebroSpec->>DocsCI: Publish storage-mode terminology and fallback boundary
    DocsCI-->>DocsCI: Align wording without adding capability
    FutureChange->>GatewaySpec: Propose topology change only after implementation exists
    FutureChange->>CerebroSpec: Propose storage behavior change only after backend semantics are specified
```

### Runtime evidence inputs referenced by the design

- `clients/cerebro/src/config.rs`
  - rejects `storage_mode = remote_surreal`
  - rejects `storage_fallback = remote_surreal`
  - validates embedded bind and credentials for supported durable operation
- `clients/cerebro/src/storage/surreal.rs`
  - implements embedded storage only
  - returns `NotImplemented` for remote construction
- `clients/cerebro/src/storage/mod.rs`
  - factory supports `embedded_surreal`, `disk`, and `in_memory`
  - fallback orchestration remains local-first in practice
- `clients/cerebro/src/main.rs` and `clients/cerebro/src/bin/cerebro.rs`
  - startup validates requirements before serving MCP
- `.github/workflows/_build-cerebro-binaries.yml`
  - smoke test uses `storage_mode = "in_memory"` for CI-safe startup, demonstrating test posture rather than production support

## Artifact Update Strategy

This change should update the support boundary in descending order of authority.

1. **Primary normative artifact**: `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/gateway/spec.md`
   - define the supported durable production topology;
   - define unsupported remote/shared/HA claims;
   - define how CI-safe modes relate to production posture.

2. **Supporting normative artifact**: `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/cerebro/spec.md`
   - align storage-mode behavior with the same support boundary;
   - describe local durable vs non-durable vs unsupported remote modes;
   - constrain fallback wording to supported local modes.

3. **Design artifact**: `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/design.md`
   - document the ownership rationale, terminology rules, verification approach, and rollout/rollback posture.

4. **Contextual follow-on alignment targets**: not part of implementation scope for this change, but explicitly referenced for later consistency checks:
   - `clients/web/apps/docs/src/content/docs/cerebro/configuration.md`
   - `clients/web/apps/docs/src/content/docs/cerebro/operations.md`
   - `.github/workflows/_build-cerebro-binaries.yml`

The design intentionally keeps docs and CI as downstream alignment artifacts rather than normative owners.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/design.md` | Create | Technical design describing ownership, terminology, verification, and rollout of the support-boundary clarification. |
| `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/gateway/spec.md` | Modify | Main spec delta for operator-facing production topology and unsupported HA/remote claims. |
| `openspec/changes/cerebro-single-node-vs-remote-ha-storage-strategy-700/specs/cerebro/spec.md` | Modify | Supporting spec delta for storage-mode semantics and unsupported remote/shared persistence behavior. |
| `openspec/specs/gateway/spec.md` | Modify | Main source-of-truth updated to codify operator-facing single-node/local-first production posture and unsupported remote/HA claims. |
| `openspec/specs/cerebro/spec.md` | Modify | Supporting source-of-truth updated to align storage-mode semantics, fallback limits, and unsupported remote/shared persistence wording. |
| `clients/web/apps/docs/src/content/docs/cerebro/configuration.md` | Modify | Configuration guidance aligned to describe supported local modes, unsupported `remote_surreal`, and supported local fallback behavior. |
| `clients/web/apps/docs/src/content/docs/cerebro/operations.md` | Modify | Operations guidance aligned to describe single-node local-first production, node-local durability, and unsupported remote/HA claims. |
| `.github/workflows/_build-cerebro-binaries.yml` | Modify | Smoke-test wording updated so `in_memory` startup is explicitly test-only scaffolding, not a production support signal. |

## Interfaces / Contracts

No new runtime interfaces, APIs, or data structures are introduced.

The contract change is **normative support language** across spec domains.

### Contract statements established by this design

```text
Supported durable production in this build:
- exactly one Cerebro node
- node-local durable storage
- embedded_surreal as default
- disk as a supported local durable alternative

Supported non-production/test posture in this build:
- in_memory for CI, development, or emergency fallback

Unsupported in this build:
- remote_surreal
- shared remote persistence
- HA / multi-node durable production
- clustered or active-active durability claims
```

### Cross-domain contract boundary

```text
gateway:
  owns operator-facing production topology and support claims

cerebro:
  owns mode-level storage semantics and fallback constraints
```

## Verification Strategy

Verification for this change is primarily **spec-consistency verification** against current implementation evidence.

| Layer | What to Verify | Approach |
|-------|----------------|----------|
| Spec delta | `gateway` states the supported topology as single-node/local-first only | Review delta wording against proposal and exploration; confirm scenarios use RFC 2119 language and reject remote/HA claims. |
| Spec delta | `cerebro` states storage modes and fallback semantics consistently | Review delta wording against implementation evidence in `config.rs`, `storage/mod.rs`, and `surreal.rs`. |
| Cross-spec consistency | `gateway` and `cerebro` do not duplicate or contradict ownership | Verify `gateway` owns operator posture, while `cerebro` owns storage behavior detail. |
| Implementation evidence | Runtime truly rejects remote primary and remote fallback | Confirm `validate_storage()` rejects both remote primary and fallback, and `new_remote()` returns `NotImplemented`. |
| CI evidence | CI-safe startup mode is clearly test-only | Confirm workflow smoke test uses `in_memory` and the design/spec wording does not reinterpret that as production support. |
| Downstream wording audit | Docs do not overclaim capability | As a follow-on check, compare current docs against the spec boundary and flag any wording still implying remote availability. |

### Verification notes

- No repository-wide build/test execution is required to justify the support boundary because the change does not alter code.
- The key verification activity is **evidence traceability**: every support claim in the delta must map back to an already-enforced implementation fact.
- If future work changes runtime support before the spec changes, this design becomes invalid and must be revisited with a new change.

## Migration / Rollout

No data migration required.

### Rollout

This change rolls out as a source-of-truth clarification:

1. land the design and spec deltas;
2. use `gateway` as the canonical reference in future operational reviews;
3. align downstream docs wording in a follow-on update if needed;
4. reject future PRs that describe remote/shared or HA Cerebro production support without a new approved change.

### Rollback

Rollback is documentation/spec-only:

1. revert the `gateway` and `cerebro` deltas for this change;
2. remove the explicit unsupported-boundary wording if product direction changes;
3. reopen exploration before any attempt to advertise remote/shared durability;
4. if runtime support is later implemented, create a separate change rather than weakening this change in place.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `gateway` and `cerebro` wording drift or duplicate each other | Conflicting source-of-truth and reviewer confusion | Keep `gateway` focused on production topology claims and `cerebro` focused on storage behavior semantics. |
| Existing docs continue to say “not yet implemented” in a way that sounds selectable | Operators may infer remote support exists but is immature | Treat docs as follow-on alignment targets and use the stricter spec wording as the normative source. |
| CI smoke posture is misread as supported production configuration | Incorrect conclusions about supported storage modes | Explicitly label `in_memory` startup in CI as test-only scaffolding in `gateway`. |
| Future remote work tries to reuse this change instead of creating a new one | Scope creep and diluted acceptance criteria | State clearly that remote/shared persistence and HA semantics require a separate follow-on change. |
| Single-node clarification is perceived as a capability removal | Stakeholder confusion | Frame the change as a documentation/spec truth alignment to already-enforced runtime behavior. |

## Open Questions

- [ ] None blocking. The current design is intentionally bounded to support-boundary representation and does not require remote storage design decisions.
