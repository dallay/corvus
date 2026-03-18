## Exploration: multi-account-provider-pool

### Current State
- Provider construction is centralized in `clients/agent-runtime/src/providers/mod.rs` via `create_provider_*`, `create_resilient_provider_with_options`, and `create_routed_provider`.
- `ReliableProvider` (`clients/agent-runtime/src/providers/reliable.rs`) wraps a list of providers and handles retry/backoff, fallback, model failover, and **rate-limit** handling. It has `api_keys` and `rotate_key`, but the rotated key is not applied to any provider instance today (rotation only logs).
- Routing is handled by `RouterProvider` (`clients/agent-runtime/src/providers/router.rs`) using model hints (`hint:<name>`) mapped to provider+model pairs. Each route is backed by its own resilient provider instance created in `create_routed_provider`.
- Config parsing lives in `clients/agent-runtime/src/config/schema.rs`, including `ReliabilityConfig` (fallback providers, `api_keys`, model fallbacks) and `ModelRouteConfig` (per-route provider/model, optional `api_key`). Config is loaded via `Config::load_or_init`, decrypted via `SecretStore`, then env overrides are applied.
- Admin HTTP config API (`clients/agent-runtime/src/gateway/admin.rs`) supports reading and patching a limited set of fields (default provider/model, api_url, memory backend, provider.api_key, etc.). It does **not** expose reliability or model routing updates today.
- Agent bootstrap uses routed provider for chat flows (`clients/agent-runtime/src/agent/agent.rs`) and resilient provider for the gateway HTTP runtime (`clients/agent-runtime/src/gateway/mod.rs`).

### Affected Areas
- `clients/agent-runtime/src/providers/reliable.rs` — reliability wrapper would need to actually select and apply account credentials per request.
- `clients/agent-runtime/src/providers/mod.rs` — provider factory and resilient/routed construction logic; currently only one provider instance per provider name and api_key.
- `clients/agent-runtime/src/config/schema.rs` — config shape, parsing, encryption/decryption, validation, and env overrides for any new account pool settings.
- `clients/agent-runtime/src/gateway/admin.rs` — admin API schema/patching if pool config is mutable at runtime.
- `clients/agent-runtime/src/bootstrap/mod.rs` — selection of provider/runtime options if multiple accounts are supported.
- `clients/web/apps/dashboard/src/types/admin-config.ts` + related composables/tests — admin UI types/patch payloads if exposed.
- `clients/agent-runtime/tests/admin_config_api_integration.rs` — admin API contract tests that may need updates when config expands.

### Approaches
1. **Account Pool in ReliabilityConfig** — add structured provider account list (provider name + api_key + optional api_url/weight) and have `ReliableProvider` choose an account per request (round-robin, least-failure, or weighted).
   - Pros: Centralized reliability surface; minimal change to routing semantics; can share retry/backoff logic.
   - Cons: Requires refactor so `ReliableProvider` can construct/use per-account provider instances; must ensure secrets encryption and admin update paths.
   - Effort: High

2. **Treat accounts as routed providers** — represent each account as a distinct provider instance inside `create_routed_provider`, using `ModelRouteConfig` (or a new `AccountRouteConfig`) to select account by hint.
   - Pros: Reuses existing router semantics; avoids changing `ReliableProvider` internals.
   - Cons: Manual hint routing for account selection; no automatic balancing across accounts without new classifier logic.
   - Effort: Medium

3. **Enhance current `api_keys` rotation** — keep config as-is but make `ReliableProvider` actually rotate credentials per request (e.g., by having providers accept a per-call key or by wrapping a credential-aware provider).
   - Pros: Smallest config/API surface change; preserves existing `ReliabilityConfig::api_keys` intent.
   - Cons: Requires provider trait changes or a credential-injection layer; riskier to retrofit without breaking API compatibility.
   - Effort: Medium-High

### Recommendation
Start with **Approach 1** if the goal is truly “multi-account provider pool” with automatic balancing and failover. It provides a clear config model and makes account selection explicit in the reliability layer. If the goal is simply to allow manual account selection, **Approach 2** is safer and leverages existing routing without modifying provider traits.

### Risks
- **Credential rotation is currently ineffective**: `ReliableProvider::rotate_key` does not apply the rotated key to provider instances, so any pool behavior must change that.
- **Admin API scope**: admin HTTP endpoints only patch a small subset of config fields. Exposing pool config may require new admin patch types and validation.
- **Secret handling**: new credential lists must flow through `SecretStore` encryption/decryption and redact handling to avoid leakage.

### Ready for Proposal
Yes — propose a target approach (pool vs routed accounts), define the new config shape, and specify whether admin HTTP should support updates for account pools.
