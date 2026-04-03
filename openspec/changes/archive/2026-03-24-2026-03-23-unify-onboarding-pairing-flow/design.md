# Design: Unify onboarding and pairing flow across Corvus clients

## Technical Approach

This change introduces a single product-level onboarding model that all Corvus surfaces map onto,
while preserving the existing transport invariants from
`openspec/specs/client-surfaces/spec.md` and the narrower operator-dashboard activation scope in
`openspec/specs/dashboard/spec.md`. The design does not change pairing protocol, bearer token
semantics, bridge mechanics, or gateway auth rules. Instead, it defines a canonical state machine,
surface adapters, recovery taxonomy, and source-of-truth boundaries so later per-surface specs and
implementation issues can stay aligned.

The shared flow is:

1. Choose surface and intent.
2. Confirm runtime path is available.
3. Establish trust for the chosen surface.
4. Validate the active transport connection.
5. Confirm ready state and capabilities.
6. Create or resume a session when the surface is a chat surface.

The core design decision is to treat onboarding as a product state machine with transport-specific
adapters:

- CLI/runtime uses direct host trust.
- Web dashboard and web chat use HTTP pairing code exchange, then bearer token auth.
- composeApp mobile uses bridge linking, not HTTP pairing language.

## Architecture Decisions

### Decision: Canonical onboarding is a product state machine, not four independent flows

**Choice**: Define one shared state machine for onboarding and pairing outcomes, with each surface
mapping its own UI and transport behavior onto the same states.

**Alternatives considered**:

- Keep separate per-surface onboarding narratives
- Use CLI/dashboard activation as the de facto canonical flow
- Define only terminology without a formal state model

**Rationale**: The repository already shows drift between `wizard.rs`, dashboard quick pair, stubbed
web chat, and mobile gateway-centric scaffolding. A shared state machine creates durable alignment
for terminology, recovery, and future issue slicing without forcing the same UI everywhere.

### Decision: Trust establishment is the shared concept; pairing and linking are adapter-specific

**Choice**: Model `trust_established` as the canonical outcome, with three adapter modes:

- `host_trusted` for CLI/runtime
- `http_paired` for dashboard/web chat
- `bridge_linked` for composeApp mobile

**Alternatives considered**:

- Use “pairing” as the umbrella term for all surfaces
- Keep separate unrelated notions of pairing, activation, and linking

**Rationale**: Current code makes pairing explicitly HTTP-specific (`POST /pair`, bearer token),
while
the mobile contract explicitly forbids HTTP as the primary path. A shared trust outcome plus
surface-specific naming preserves security semantics without leaking gateway terminology into mobile
UX.

### Decision: Ready state is separate from trust state

**Choice**: Split `trust_established` from `transport_connected` and `ready`.

**Alternatives considered**:

- Collapse pairing/linking and readiness into one “connected” step
- Treat bearer token presence as synonymous with readiness

**Rationale**: In the current system a client can be paired but still not able to proceed: gateway
may be unreachable, admin endpoints may reject missing auth, mobile may be linked but lack a
session-capable bridge, and CLI may be configured but runtime services may not be available. The
extra separation keeps recovery states precise and cross-surface comparable.

### Decision: Recovery taxonomy is normalized at the product layer

**Choice**: Use one shared recovery taxonomy with surface-specific triggers, labels, and actions.

**Alternatives considered**:

- Keep dashboard `DASH-*` codes as the only formal taxonomy
- Let each surface define its own retry model independently

**Rationale**: Dashboard already has deterministic readiness states, but web chat and mobile do not.
Normalizing recovery semantics now prevents later drift and makes follow-up specs/test plans
comparable across surfaces.

### Decision: Source of truth stays layered rather than consolidated into one spec

**Choice**:

- `openspec/specs/client-surfaces/spec.md` remains the authority for role, capability, and
  transport.
- `openspec/specs/dashboard/spec.md` remains the authority for the operator-only dashboard
  activation slice inside CLI onboarding.
- This change becomes the authority for cross-surface onboarding sequence, terminology, state model,
  and recovery mapping.

**Alternatives considered**:

- Move dashboard activation into this change entirely
- Add onboarding rules directly into the client-surfaces capability matrix
- Let each surface contract redefine onboarding locally

**Rationale**: The capability matrix and the dashboard activation spec already exist and cover real
scope. Replacing them would create avoidable churn. A layered ownership model keeps this design
focused on cross-surface cohesion.

## Canonical Flow Model

### Canonical states

The onboarding model uses the following product states:

| State                    | Meaning                                              | Shared Exit Condition                                       |
|--------------------------|------------------------------------------------------|-------------------------------------------------------------|
| `intent_selected`        | Surface and user goal are known                      | Surface adapter chosen                                      |
| `runtime_path_confirmed` | A usable Corvus backend path exists for this surface | Runtime or bridge/gateway endpoint verified                 |
| `trust_pending`          | Surface is not yet trusted                           | Surface-specific trust action available                     |
| `trust_established`      | Surface now has the right to continue                | Pairing/token, link, or host trust completed                |
| `transport_connecting`   | Surface validates its active communication path      | Transport health/auth/session preconditions checked         |
| `ready`                  | Surface can start its primary work                   | Capability-appropriate ready UI shown                       |
| `session_pending`        | Chat surfaces need a new or resumed session          | Session is created/resumed or skipped for operator surfaces |
| `session_ready`          | Chat session is active or resumable                  | Conversation workspace shown                                |
| `blocked`                | User cannot progress without recovery                | Recovery action surfaced                                    |

### Canonical state machine

```text
start
  -> intent_selected
  -> runtime_path_confirmed
  -> trust_pending
      -> trust_established
      -> transport_connecting
      -> ready
          -> session_pending
              -> session_ready

At any step:
  -> blocked(recovery_kind)
  -> retry same state OR backtrack to prior prerequisite
```

### Surface mapping

| Surface           | Intent archetype            | Trust mode      | Ready outcome                                      |
|-------------------|-----------------------------|-----------------|----------------------------------------------------|
| CLI/runtime       | Operator setup / direct use | `host_trusted`  | CLI ready; may offer dashboard activation guidance |
| Web dashboard     | Operator web management     | `http_paired`   | Gateway-authenticated admin access                 |
| Web chat          | End-user browser chat       | `http_paired`   | Gateway-authenticated chat access                  |
| composeApp mobile | End-user mobile chat        | `bridge_linked` | Bridge-linked chat access                          |

## Shared UX Concepts vs Adapters

### Shared UX concepts

These concepts MUST remain product-consistent across surfaces:

- **Connect to Corvus**: umbrella phrase for reaching a usable backend
- **Runtime available**: the chosen surface can reach its required backend path
- **Trust this surface**: one-time authorization step before authenticated use
- **Ready**: the user can now perform the surface’s primary task
- **Resume**: continue from a previously successful trust or session state
- **Recover**: show the problem class, why it happened, and the next safe action

### Adapter-specific UX labels

| Product concept   | CLI/runtime                     | Web dashboard / web chat            | composeApp mobile                   |
|-------------------|---------------------------------|-------------------------------------|-------------------------------------|
| Trust action      | Already trusted on host         | Pair with code                      | Link app to local Corvus            |
| Trust credential  | None                            | Bearer token                        | Bridge link state / platform secret |
| Transport check   | Local runtime/gateway readiness | Gateway reachable + authenticated   | Bridge reachable + session-capable  |
| Primary next step | Continue setup or management    | Open/manage dashboard or start chat | Start/resume mobile session         |

### Adapter boundary rules

- The product model MUST NOT rename HTTP pairing into mobile linking.
- Mobile UX MUST NOT ask for pairing codes as its primary flow.
- Web surfaces MUST NOT imply direct runtime or CLI-bridge trust.
- CLI/runtime MAY expose dashboard activation as a follow-on path, but that is still a web adapter
  step rather than a CLI transport change.

## Trust, Pairing, Token, and Linking Relationship

### Shared conceptual ladder

```text
runtime trust model
  -> choose trust adapter
      -> HTTP pairing code exchange OR mobile bridge linking OR host trust
          -> transport credential/authorization established
              -> transport connection validated
                  -> ready
```

### HTTP flow sequence (dashboard and web chat)

```text
User
  -> Web Surface
  -> Local Gateway / Runtime

Web Surface -> GET /health
Gateway -> { status: ok, paired: bool }

If not trusted:
  Runtime displays pairing code
  Web Surface -> POST /pair (X-Pairing-Code)
  Gateway -> { paired: true, token }
  Web Surface stores bearer token

Web Surface -> authenticated gateway endpoint
Gateway -> authorized response
Web Surface -> ready
```

### Mobile flow sequence (composeApp)

```text
User
  -> composeApp
  -> MobileBridgeContract / RustCliBridge / companion path
  -> Runtime

composeApp -> verify bridge/runtime path
composeApp -> initiate link flow
Bridge adapter -> establish local trusted relationship
composeApp -> validate session-capable access
composeApp -> ready
```

### CLI/runtime flow sequence

```text
User -> corvus onboard / corvus agent
CLI -> local runtime configuration/readiness checks
CLI -> ready
CLI -> optionally direct user toward dashboard activation
```

### Design constraints for trust chain

- Runtime trust is the root security concept.
- HTTP pairing is a one-time trust bootstrap only for HTTP clients.
- Bearer token acquisition is the persisted result of successful HTTP trust bootstrap.
- Mobile linking is the bridge-specific trust bootstrap and MUST remain distinct from bearer-token
  auth.
- Dashboard activation guidance in CLI is a handoff into the HTTP adapter, not a new trust model.

## Recovery and Retry Model

### Normalized recovery kinds

| Recovery kind                  | Meaning                                                   | Typical retry anchor           |
|--------------------------------|-----------------------------------------------------------|--------------------------------|
| `runtime_unavailable`          | Required local runtime path is missing or down            | Re-check runtime availability  |
| `transport_unavailable`        | Selected transport cannot currently communicate           | Retry transport validation     |
| `trust_input_invalid`          | Pairing code or linking input is invalid                  | Re-enter trust input           |
| `trust_input_expired`          | Pairing/link bootstrap has expired                        | Regenerate/restart trust flow  |
| `credential_missing`           | Expected bearer token or link secret is absent            | Return to trust step           |
| `credential_invalid`           | Credential exists but is rejected/revoked                 | Clear and re-establish trust   |
| `paired_but_not_connected`     | Trust exists, but active transport validation fails       | Retry transport connection     |
| `linked_but_not_session_ready` | Bridge link exists, but session operations cannot proceed | Retry session capability check |
| `session_unavailable`          | No active/resumable session exists                        | Create a new session           |
| `environment_unsupported`      | Surface cannot run in this environment                    | Redirect to supported path     |

### Surface-specific mapping

| Recovery kind                  | CLI/runtime                 | Web dashboard                           | Web chat                        | composeApp mobile                       |
|--------------------------------|-----------------------------|-----------------------------------------|---------------------------------|-----------------------------------------|
| `runtime_unavailable`          | Gateway/runtime not started | Local gateway missing                   | Local gateway missing           | CLI/companion not found                 |
| `transport_unavailable`        | Local status check fails    | `/health` unreachable or UI path broken | `/health` unreachable           | Bridge handshake fails                  |
| `trust_input_invalid`          | N/A                         | Pairing code rejected                   | Pairing code rejected           | Link code/path invalid                  |
| `trust_input_expired`          | N/A                         | Pairing code expired                    | Pairing code expired            | Link invitation expired                 |
| `credential_missing`           | N/A                         | Bearer token absent                     | Bearer token absent             | Stored link state absent                |
| `credential_invalid`           | N/A                         | Bearer token revoked                    | Bearer token revoked            | Link state no longer trusted            |
| `paired_but_not_connected`     | N/A                         | Paired but admin fetch fails            | Paired but chat transport fails | N/A                                     |
| `linked_but_not_session_ready` | N/A                         | N/A                                     | N/A                             | Bridge reachable but session calls fail |
| `session_unavailable`          | Resume target missing       | Session monitoring scope only           | No resumable chat session       | No resumable mobile session             |
| `environment_unsupported`      | Browser open unsupported    | Unsafe local origin                     | Unsafe local origin             | iOS/desktop bridge path unavailable     |

### Retry semantics

- Retries SHOULD target the nearest failed prerequisite, not restart the whole flow by default.
- Surfaces SHOULD preserve successful prior states (`trust_established`, stored bearer token,
  stored link state) unless the failure is credential-related.
- Credential-related failures SHOULD explicitly clear stale trust state before asking the user to
  retry.
- Chat surfaces SHOULD distinguish “ready but no session” from “not ready to chat”.

## Source-of-Truth Boundaries

### Boundary model

| Artifact                                             | Owns                                                                           | Does not own                                       |
|------------------------------------------------------|--------------------------------------------------------------------------------|----------------------------------------------------|
| `openspec/specs/client-surfaces/spec.md`             | Transport invariants, capability matrix, parity requirements                   | Product onboarding wording, retry taxonomy details |
| `openspec/specs/client-surfaces/surface-contracts/*` | Surface capability obligations and local platform rules                        | Cross-surface canonical journey                    |
| `openspec/specs/dashboard/spec.md`                   | Optional CLI-to-dashboard activation flow and `DASH-*` operator diagnostics    | End-user web chat or mobile onboarding             |
| This change design/spec chain                        | Shared onboarding sequence, terminology, state machine, recovery normalization | Low-level protocol/storage mechanics               |

### Relationship to existing dashboard activation spec

The dashboard activation spec remains valid as a narrower operator slice. This change reframes it
as:

- a concrete CLI/operator adapter instance of the canonical model,
- using the same `runtime_path_confirmed -> trust_pending -> trust_established -> ready` flow,
- with `DASH-*` states treated as dashboard-specific renderings of the broader recovery taxonomy.

This design does **not** supersede dashboard-specific diagnosis labels. It defines how those labels
fit into the shared product model so later web chat and mobile work can adopt equivalent semantics
without copying `DASH-*` verbatim.

## Data Flow

### Cross-surface onboarding orchestration

```text
Surface entrypoint
  -> surface intent resolver
  -> canonical onboarding controller
      -> runtime path check adapter
      -> trust adapter
      -> transport validation adapter
      -> ready-state presenter
      -> session adapter (chat surfaces only)
```

### Logical contract sketch

```text
OnboardingFlowModel
  surface_id
  intent
  trust_mode
  transport_mode
  state
  recovery_kind?
  capabilities_after_ready

SurfaceAdapter
  confirmRuntimePath()
  establishTrust()
  validateTransport()
  enterReadyState()
  createOrResumeSession()
```

### Suggested normalized state payload

```text
OnboardingState {
  state: intent_selected | runtime_path_confirmed | trust_pending | trust_established |
         transport_connecting | ready | session_pending | session_ready | blocked
  trust_mode: host_trusted | http_paired | bridge_linked
  transport_mode: direct | http_gateway | cli_bridge
  recovery_kind?: runtime_unavailable | transport_unavailable | trust_input_invalid |
                  trust_input_expired | credential_missing | credential_invalid |
                  paired_but_not_connected | linked_but_not_session_ready |
                  session_unavailable | environment_unsupported
  can_retry: bool
  can_resume: bool
}
```

## File Changes

| File                                                                                                   | Action               | Description                                                                                   |
|--------------------------------------------------------------------------------------------------------|----------------------|-----------------------------------------------------------------------------------------------|
| `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/design.md`                                  | Create               | Canonical cross-surface onboarding design artifact                                            |
| `openspec/changes/2026-03-23-unify-onboarding-pairing-flow/specs/...`                                  | Future Modify        | Follow-up delta specs should encode the canonical state model and terminology per surface     |
| `clients/agent-runtime/src/onboard/wizard.rs`                                                          | Future Modify        | Align CLI onboarding states and dashboard handoff language to the canonical model             |
| `clients/agent-runtime/src/gateway/mod.rs`                                                             | Future Modify        | Preserve pairing/token mechanics while aligning user-facing trust/readiness wording           |
| `clients/web/apps/dashboard/src/composables/useConfig.ts`                                              | Future Modify        | Map quick-pair, token, and connected states onto canonical trust/readiness states             |
| `clients/web/apps/chat/src/composables/useGateway.ts`                                                  | Future Create/Modify | Implement web-chat onboarding and retry behavior using the HTTP adapter model                 |
| `clients/web/apps/chat/src/composables/useChat.ts`                                                     | Future Create/Modify | Separate session lifecycle from trust/readiness state                                         |
| `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt` | Future Modify        | Replace gateway-centric step copy with mobile linking stages                                  |
| `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`          | Future Modify        | Remove HTTP gateway config assumptions and model bridge-linked/session-ready states           |
| `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ConfigPanel.kt`            | Future Modify/Delete | Replace HTTP config panel with bridge/linking diagnostics if still needed                     |
| `modules/agent-core-kmp/src/commonMain/kotlin/com/profiletailors/agent/core/CoreContracts.kt`          | Future Modify        | Add canonical bridge/link/session status contracts only if needed for mobile/shared alignment |
| `modules/agent-core-kmp/src/jvmMain/kotlin/com/profiletailors/agent/core/RustCliBridge.kt`             | Future Modify        | Support link/session capability checks required by the mobile adapter                         |

## Interfaces / Contracts

The design intentionally stays implementation-guiding, but later work should converge on these
logical contracts.

### Product-level onboarding contract

```text
enum SurfaceId {
  CLI_RUNTIME,
  WEB_DASHBOARD,
  WEB_CHAT,
  COMPOSEAPP_MOBILE,
}

enum TrustMode {
  HOST_TRUSTED,
  HTTP_PAIRED,
  BRIDGE_LINKED,
}

enum RecoveryKind {
  RUNTIME_UNAVAILABLE,
  TRANSPORT_UNAVAILABLE,
  TRUST_INPUT_INVALID,
  TRUST_INPUT_EXPIRED,
  CREDENTIAL_MISSING,
  CREDENTIAL_INVALID,
  PAIRED_BUT_NOT_CONNECTED,
  LINKED_BUT_NOT_SESSION_READY,
  SESSION_UNAVAILABLE,
  ENVIRONMENT_UNSUPPORTED,
}
```

### Dashboard/web adapter contract expectations

- Input: gateway base URL, pairing code entry or quick-pair link, persisted bearer token
- Output: canonical trust/readiness/recovery states consumable by dashboard and web chat UI
- Security invariant: pairing code is ephemeral and MUST NOT be persisted; bearer token is the only
  persisted HTTP trust credential

### Mobile adapter contract expectations

- Input: bridge availability, local CLI/companion presence, optional stored link state, session ID
- Output: canonical linking/readiness/session states consumable by composeApp shared/mobile UI
- Security invariant: mobile MUST NOT depend on HTTP pairing terminology or bearer-token-first flow

### CLI/runtime adapter contract expectations

- Input: local runtime configuration and gateway readiness checks
- Output: direct-ready state plus optional handoff instructions for the dashboard adapter
- Security invariant: CLI remains the host surface and does not acquire a second trust credential
  for
  itself

## Suggested Implementation Slicing

### Slice 1: Shared product/state-model specs

Deliverables:

- Delta spec for canonical onboarding states, terms, and recovery taxonomy
- Cross-reference updates into relevant surface contracts

Why first: every surface depends on the same model and terminology.

### Slice 2: CLI/runtime alignment

Deliverables:

- Map `wizard.rs` post-summary/dashboard activation states to canonical states
- Keep existing `DASH-*` diagnostics but document their normalized recovery mapping

Why second: it is the only implemented onboarding flow and anchors operator wording.

### Slice 3: Shared HTTP onboarding model for dashboard + web chat

Deliverables:

- Shared HTTP adapter state vocabulary (`trust_pending`, `http_paired`, `paired_but_not_connected`)
- Dashboard keeps admin focus; web chat gets the same trust/recovery model without admin semantics

Why third: dashboard already has code and web chat is currently empty.

### Slice 4: composeApp mobile linking model

Deliverables:

- Replace gateway-centric onboarding and config assumptions
- Introduce bridge-link/session-ready mapping consistent with the canonical model

Why fourth: mobile transport mechanics are less complete and should build on the shared model.

### Slice 5: Shared observability and resume semantics

Deliverables:

- Standardized analytics/log labels for onboarding state transitions and recovery kinds
- Consistent session resume vs re-trust rules across chat surfaces

Why last: this depends on the prior surface models being agreed.

## Testing Strategy

| Layer       | What to Test                                   | Approach                                                                                                         |
|-------------|------------------------------------------------|------------------------------------------------------------------------------------------------------------------|
| Unit        | State-machine transitions and recovery mapping | Pure mapping tests for canonical state/recovery logic in each surface adapter                                    |
| Unit        | Terminology boundaries                         | Assertions that mobile adapters never emit HTTP pairing labels and web adapters never emit bridge labels         |
| Integration | HTTP trust flow                                | Exercise `/health` -> `/pair` -> authenticated access mapping without changing gateway semantics                 |
| Integration | CLI/dashboard handoff                          | Verify dashboard activation outputs map to canonical states and recovery kinds                                   |
| Integration | Mobile linking flow                            | Verify bridge presence/link/session capability states map to canonical ready and blocked states                  |
| E2E         | Cross-surface first-run stories                | Scenario tests for operator CLI, operator web, end-user web, and mobile paths using the same normalized outcomes |

## Migration / Rollout

No migration required.

This change is design-only and preserves existing runtime pairing, token persistence, origin-guard,
and bridge constraints. Rollout for later implementation work should be surface-by-surface behind
existing application boundaries rather than a repo-wide flag.

Recommended rollout order:

1. Spec the canonical model.
2. Align CLI/runtime dashboard handoff.
3. Align dashboard and implement web chat HTTP flow.
4. Align composeApp mobile linking flow.

## Open Questions

- [ ] Should later web chat work reuse the dashboard quick-pair hash-link pattern directly, or
  define
  a chat-specific but equivalent entry flow on top of the same HTTP trust adapter?
- [ ] Should canonical product docs standardize one local user-facing web entrypoint now, or should
  that remain a separate follow-up so this change stays focused on flow/state alignment?
- [ ] For iOS, should the first mobile-linking spec target companion-daemon linking explicitly, or
  keep the linking adapter abstract until platform mechanics are ready?
