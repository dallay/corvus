## Exploration: Unify onboarding and pairing flow across Corvus clients

### Current State
- `clients/agent-runtime/src/onboard/wizard.rs` is the only implemented end-to-end onboarding flow today. It configures the runtime, then offers optional web dashboard activation with bounded diagnosis states (`DASH-001`..`DASH-999`), browser-open behavior, and resume-later commands.
- `clients/agent-runtime/src/gateway/mod.rs` and `clients/agent-runtime/src/gateway/utils.rs` define the current pairing/auth contract for HTTP clients: public `GET /health`, one-time `POST /pair` with `X-Pairing-Code`, then bearer token auth for admin/gateway endpoints. Pairing codes are ephemeral; bearer tokens are persisted as paired tokens.
- `clients/web/apps/dashboard/src/composables/useConfig.ts` is the only client that fully implements token acquisition today. It supports manual pairing code entry and a quick-pair magic-link hash flow, then connects to `/web/admin/*` with a bearer token.
- `clients/web/apps/chat` is still scaffolded. `src/composables/useGateway.ts` and `src/composables/useChat.ts` are empty, so the web end-user surface has no real onboarding, pairing, session, or recovery story yet.
- `clients/composeApp` currently presents a three-step welcome/connect/talk mobile onboarding, but the shared chat UI still uses `AgentGatewayConfig` (`baseUrl`, `pairingCode`, `bearerToken`, `webhookSecret`) and a local fake reply stub. This conflicts with the active surface contract, which says mobile MUST use `RustCliBridge` and should onboard users to install/use the Corvus CLI rather than pair to HTTP as its primary path.
- Existing specs describe parts of the story, but not one product-level sequence across surfaces. CLI/dashboard activation is specified; web chat and mobile contracts mention required capabilities but not a single canonical first-run flow.
- There is also product-copy drift in local entrypoints: CLI onboarding/runtime code still points users to `http://localhost:4324`, while prior dashboard artifacts and web monorepo docs also reference portless/local-domain entrypoints (`dashboard.localhost:1355`, `corvus.localhost`). Proposal work needs one canonical explanation.

### Affected Areas
- `clients/agent-runtime/src/onboard/wizard.rs` — current canonical onboarding flow and dashboard activation copy/diagnostics.
- `clients/agent-runtime/src/gateway/mod.rs` — current `/health` and `/pair` semantics, printed pairing instructions, magic link behavior.
- `clients/agent-runtime/src/gateway/utils.rs` — bearer token extraction and local-origin auth guard constraints that shape safe recovery guidance.
- `clients/web/apps/dashboard/src/composables/useConfig.ts` — current web operator pairing/token acquisition and quick-pair behavior.
- `clients/web/apps/chat/src/composables/useGateway.ts` — currently empty, likely future home of shared web end-user pairing/session connection behavior.
- `clients/web/apps/chat/src/composables/useChat.ts` — currently empty, likely future home of session/retry state handling.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/onboarding/OnboardingScreen.kt` — current mobile onboarding copy/sequence.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt` — current mobile gateway-centric config model that conflicts with the mobile transport contract.
- `openspec/specs/client-surfaces/spec.md` — source of truth for transport and capability boundaries across CLI, web, and mobile.
- `openspec/specs/client-surfaces/surface-contracts/web-chat.md` — web chat onboarding/pairing expectations exist only as checklist items.
- `openspec/specs/client-surfaces/surface-contracts/web-dashboard.md` — dashboard already owns pairing/token management for operator web.
- `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` — explicitly says mobile should install/use CLI bridge, not HTTP gateway.
- `openspec/specs/dashboard/spec.md` — existing first-run dashboard activation spec covers only one slice of the broader story.
- `openspec/specs/client-surfaces/migrations.md` — follow-up implementation work is already naturally separable into M1/M2/M3/M4-style surface issues.

### Approaches
1. **Surface-by-surface alignment** — patch each client contract independently and treat onboarding as a collection of per-surface flows.
   - Pros: Low immediate scope; maps neatly to current file ownership and migration tickets.
   - Cons: Likely preserves product drift; hard to keep pairing/token/recovery language identical; proposal would still need a hidden shared model.
   - Effort: Medium

2. **Canonical product journey with transport-specific variants** — define one shared onboarding sequence first, then attach explicit CLI/web/mobile variants at the step level.
   - Pros: Best fit for the problem statement; makes shared vs client-specific steps explicit; creates clean follow-up issues per surface; keeps security semantics centralized while allowing transport differences.
   - Cons: Requires proposal/spec work to resolve terminology and lifecycle boundaries before implementation starts.
   - Effort: Medium

3. **Operator-first canonical flow anchored on current CLI onboarding** — elevate the existing CLI/dashboard activation path as the default story, then adapt chat/mobile later.
   - Pros: Fastest path because CLI/runtime flow already exists and is tested.
   - Cons: Risks overfitting the product story to operator setup; does not naturally explain end-user web chat or mobile bridge setup; leaves mobile inconsistencies unresolved.
   - Effort: Low

### Recommendation
Use approach 2. The proposal should define a single product-level onboarding model with these likely layers:
- Shared steps: choose surface/use case, verify local Corvus runtime availability, establish trust/identity for the surface, connect to runtime transport, confirm readiness, then create or resume a first session.
- HTTP-client steps (dashboard, future web chat): obtain one-time pairing code from the local runtime/gateway, exchange it for a bearer token, store it safely per surface, then validate gateway health/session access.
- Mobile-specific steps: replace HTTP pairing language with CLI-bridge or companion-daemon linking language, but keep the same conceptual outcomes (`runtime discovered`, `surface linked`, `ready to start/resume session`).
- Recovery/retry states should be normalized across all clients even if the triggers differ: runtime unavailable, pairing/link code invalid or expired, token missing/invalid, gateway reachable but not paired, client UI unavailable, session expired, and local environment unsupported.

### Risks
- The current repository mixes at least three local entrypoint conventions (`localhost:4324`, `dashboard.localhost:1355`, historical `corvus.localhost`), so proposal language can easily become inconsistent unless it chooses one canonical user-facing entrypoint.
- Mobile transport is not implemented yet, so there is a risk of defining a product story that assumes bridge capabilities (session-aware linking, retries, secure storage) that do not yet exist.
- Web chat has no real runtime integration yet, so some recovery states will be product-defined before code exists.
- Pairing is currently HTTP-specific in code; proposal must decide whether “pairing” remains the umbrella term for all surfaces or whether mobile uses a separate “link/connect” term with shared semantics.
- Existing dashboard activation spec may overlap this change; proposal should decide whether to supersede, extend, or reference that narrower spec to avoid conflicting source-of-truth documents.

### Ready for Proposal
Yes — but the proposal must settle three decisions before implementation work is split out: (1) the canonical user-facing sequence and terminology (`pair`, `link`, `connect`, `activate`), (2) the canonical local entrypoint/runtime-discovery story for operator web vs end-user web vs mobile, and (3) the normalized recovery/retry taxonomy that every surface must expose even when transport details differ.
