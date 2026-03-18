# Design: Multi-Account Provider Pool

## Technical Approach

Introduce a provider account pool in the reliability configuration and route provider
construction through a pooled wrapper that selects a credentialed account per call.
The pool will be provider-agnostic, cache per-account provider instances, and apply
weighted/round-robin selection. Existing reliability behaviors (retry, model failover,
fallback providers) remain unchanged, but when a provider has a configured pool, each
provider call uses a selected account from that pool. This aligns with the proposal’s
approach: explicit pooling at the reliability layer without changing routing semantics.

## Architecture Decisions

### Decision: Represent pools as provider-keyed config map

**Choice**: Add `reliability.account_pools: HashMap<String, ProviderAccountPoolConfig>` with
provider names as keys.
**Alternatives considered**: A flat `Vec<AccountConfig>` with provider field; a top-level
`[[account_pools]]` list unrelated to reliability.
**Rationale**: Provider-keyed maps avoid repeating provider names per account and allow
strategy settings to live with the pool. It also keeps the pool scoped to reliability, which
is the current home for retry/fallback behaviors.

### Decision: Implement pooling as a provider wrapper (AccountPoolProvider)

**Choice**: Add `AccountPoolProvider` implementing `Provider` that selects an account per
call and delegates to a cached provider instance created with that account’s credentials.
**Alternatives considered**: Embedding account selection directly inside `ReliableProvider`;
changing the `Provider` trait to accept per-request credentials.
**Rationale**: A wrapper isolates pooling logic and keeps `Provider` trait stable while
minimizing changes to reliability code. It also enables reuse by any caller that wants
multi-account pooling without altering routing semantics.

### Decision: Rate-limit aware selection with cooldown hints

**Choice**: Track per-account cooldown timestamps in `AccountPoolProvider` and skip accounts
temporarily when rate-limit errors are detected; retries by `ReliableProvider` naturally
advance the account selection index.
**Alternatives considered**: No cooldown (pure round-robin); rate-limit handling solely in
`ReliableProvider`.
**Rationale**: The pool has local context about which account was used, so it can apply a
cooldown without changing `ReliableProvider`’s error classification logic. This yields a
low-cost, effective strategy for avoiding repeated 429s.

## Data Flow

Primary request flow with account selection:

    Caller
      │
      │  chat_with_system
      ▼
  ReliableProvider
      │  (retry/fallback loops)
      ▼
  AccountPoolProvider (per-provider)
      │  select account (round-robin/weighted)
      │  get/create provider for account
      ▼
  Concrete Provider (OpenAI/Anthropic/etc)
      │
      ▼
  Response / Error

If a rate-limit error is detected inside `AccountPoolProvider`, the account is put on
cooldown, so subsequent calls skip it until the cooldown expires.

## Sequence Diagram

Request with account selection + rate-limit cooldown:

    Client              ReliableProvider        AccountPoolProvider      Provider
      │                        │                        │                 │
      │ chat_with_system       │                        │                 │
      │──────────────────────▶ │                        │                 │
      │                        │ call provider          │                 │
      │                        │──────────────────────▶ │                 │
      │                        │                        │ select account  │
      │                        │                        │───────────────▶│
      │                        │                        │ call provider   │
      │                        │                        │────────────────▶│
      │                        │                        │ 429 error       │
      │                        │                        │◀────────────────│
      │                        │                        │ mark cooldown   │
      │                        │ retry (same provider)  │                 │
      │                        │──────────────────────▶ │                 │
      │                        │                        │ select next     │
      │                        │                        │───────────────▶│
      │                        │                        │ call provider   │
      │                        │                        │────────────────▶│
      │                        │                        │ success         │
      │                        │                        │◀────────────────│
      │                        │◀────────────────────── │                 │
      │◀────────────────────── │                        │                 │

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/config/schema.rs` | Modify | Add pool config structs; load/save encryption for pooled secrets; validation rules. |
| `clients/agent-runtime/src/providers/pool.rs` | Create | New `AccountPoolProvider` and selection logic. |
| `clients/agent-runtime/src/providers/mod.rs` | Modify | Construct pooled providers when pool config exists; wire into resilient provider creation. |
| `clients/agent-runtime/src/providers/reliable.rs` | Modify | Minor integration if needed (e.g., removing unused api_key rotation when pool present). |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Ensure runtime options pass through pooled provider creation (no behavior change). |
| `clients/agent-runtime/tests/admin_config_api_integration.rs` | Modify | Only if admin config includes pool exposure. |
| `clients/web/apps/dashboard/src/types/admin-config.ts` | Modify | Only if admin config includes pool exposure. |

## Interfaces / Contracts

### Config shape

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountPoolStrategy {
  RoundRobin,
  WeightedRoundRobin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAccountConfig {
  pub id: String,
  pub api_key: String,
  #[serde(default)]
  pub api_url: Option<String>,
  #[serde(default = "default_account_weight")]
  pub weight: u32,
  #[serde(default = "default_true")]
  pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAccountPoolConfig {
  #[serde(default)]
  pub strategy: AccountPoolStrategy,
  pub accounts: Vec<ProviderAccountConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
  // existing fields...
  #[serde(default)]
  pub account_pools: std::collections::HashMap<String, ProviderAccountPoolConfig>,
}
```

TOML example:

```toml
[reliability.account_pools.openrouter]
strategy = "round_robin"

[[reliability.account_pools.openrouter.accounts]]
id = "acct-1"
api_key = "enc:..."
weight = 1

[[reliability.account_pools.openrouter.accounts]]
id = "acct-2"
api_key = "enc:..."
weight = 2
```

### Provider wrapper

```rust
pub struct AccountPoolProvider {
  provider_name: String,
  strategy: AccountPoolStrategy,
  accounts: Vec<ProviderAccountConfig>,
  index: std::sync::atomic::AtomicUsize,
  cooldown_until: parking_lot::Mutex<HashMap<String, std::time::Instant>>,
  cache: parking_lot::Mutex<HashMap<String, Box<dyn Provider>>>,
  runtime: ProviderRuntimeOptions,
}
```

Key behaviors:
- `select_account()` skips disabled accounts and those with active cooldowns.
- Provider instance is created lazily per account and cached by `id`.
- Errors are inspected for rate-limits; if detected, the account enters cooldown
  (using Retry-After when available, otherwise a small backoff window).

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Pool selection (round-robin/weighted), cooldown skip | New tests in `clients/agent-runtime/src/providers/pool.rs` with mock provider. |
| Unit | Config load/save encrypts pooled api_key | Extend config save/load tests in `clients/agent-runtime/src/config/schema.rs`. |
| Unit | Validation rejects empty ids, duplicate ids, zero weights | Add validation tests near `validate_for_runtime`. |
| Integration | Provider creation uses pool when configured | Tests in `clients/agent-runtime/src/providers/mod.rs` using a synthetic provider. |
| Integration | Admin config API (only if exposed) | Update `clients/agent-runtime/tests/admin_config_api_integration.rs`. |

## Migration / Rollout

No migration required. If `reliability.account_pools` is empty, behavior remains unchanged.
Existing `reliability.api_keys` rotation remains supported for non-pooled providers.

## Open Questions

- [ ] Should admin config API expose pool read/patch in this phase, or defer to reduce
      secret-handling scope?
- [ ] Should pooled accounts allow `api_key` omission (fall back to `config.api_key`), or
      require explicit keys for clarity and safety?
