# Task Breakdown: Gateway API Implementation

**Change**: rook-589-gateway-api  
**Issue**: #589  
**Created**: 2026-04-21

---

## Overview

This document breaks down the gateway API implementation into granular, independently testable tasks organized by the 5 phases defined in the proposal:

1. **Infrastructure** — Credential storage + HTTP client
2. **Types + Vendor Mapping** — OpenAI-compatible types + vendor logic
3. **Upstream Proxy** — HTTP forwarding to providers
4. **Handlers + Router** — Axum endpoints + routing
5. **Server Wiring** — Integration into server startup

Each task follows TDD: write test first, implement minimum code to pass, refactor.

---

## Phase 1: Infrastructure

### [x] T1: Add reqwest dependency to Cargo.toml

**Description**: Add `reqwest` with `json` and `rustls-tls` features, plus `bytes` for raw body handling.

**Files**:
- `clients/rook/Cargo.toml`

**Implementation**:
```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
bytes = "1.0"
```

**Tests**: Verify `cargo check` passes after adding dependencies.

**Acceptance**:
- `cargo check` succeeds
- No version conflicts with existing dependencies

**Dependencies**: None

---

### [x] T2: Create migration 0003_account_api_key.sql

**Description**: Create SQL migration to add `api_key` column to `provider_accounts` table.

**Files**:
- `clients/rook/migrations/0003_account_api_key.sql` (new)

**Implementation**:
```sql
-- Add api_key column to provider_accounts for upstream authentication.
-- Nullable: existing accounts get NULL (no key configured yet).
ALTER TABLE provider_accounts ADD COLUMN api_key TEXT DEFAULT NULL;
```

**Tests**: Manual verification that migration applies cleanly to existing database.

**Acceptance**:
- Migration file exists in correct location
- SQL syntax is valid SQLite
- Column is nullable with DEFAULT NULL

**Dependencies**: None

---

### [x] T3: Add api_key field to ProviderAccount domain struct

**Description**: Add `api_key: Option<String>` field to `ProviderAccount` struct with documentation.

**Files**:
- `clients/rook/src/domain/mod.rs`

**Implementation**:
Add field after `api_base_override`:
```rust
/// API key for authenticating with the upstream provider.
/// Stored as plaintext in M1; encryption-at-rest is #591.
pub api_key: Option<String>,
```

**Tests**: Update all existing test fixtures that construct `ProviderAccount` to include `api_key: None`.

**Acceptance**:
- Field added to struct
- All existing tests compile and pass
- Documentation comment present

**Dependencies**: None

---

### [x] T4: Wire migration 0003 into db/mod.rs

**Description**: Add migration constant and apply block to `run_migrations()` function.

**Files**:
- `clients/rook/src/db/mod.rs`

**Implementation**:
1. Add constant:
```rust
const MIGRATION_SQL_0003: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0003_account_api_key.sql"
));
```

2. Add apply block in `run_migrations()` following the 0002 pattern.

**Tests**: 
- Write test that creates in-memory DB, runs migrations, verifies `api_key` column exists
- Test that existing accounts get `NULL` for `api_key`

**Acceptance**:
- Migration applies on fresh database
- Migration applies on database with existing accounts
- `schema_migrations` table records version `0003_account_api_key`

**Dependencies**: T2, T3

---

### [x] T5: Update db/account.rs to persist api_key

**Description**: Add `api_key` to INSERT, SELECT queries and `row_to_account` mapping.

**Files**:
- `clients/rook/src/db/account.rs`

**Implementation**:
1. Update `insert_account` SQL to include `api_key` column and bind
2. Update `get_account` and `list_accounts` SELECT to include `api_key`
3. Update `row_to_account` to extract `api_key` from row

**Tests**:
- Test `insert_account` with `api_key: Some("sk-test-123")` then `get_account` → verify round-trip
- Test `insert_account` with `api_key: None` then `get_account` → verify `None` persists
- Test `list_accounts` includes `api_key` field

**Acceptance**:
- `api_key` round-trips through database correctly
- `None` values persist as `NULL`
- All existing account tests pass

**Dependencies**: T3, T4

---

### [x] T6: Update AccountService test fixtures

**Description**: Update test helper functions in `services/account.rs` to include `api_key: None`.

**Files**:
- `clients/rook/src/services/account.rs`

**Implementation**:
Update `make_account()` or similar test helpers to include `api_key: None` in constructed accounts.

**Tests**: Verify all service-level tests pass.

**Acceptance**:
- All tests in `services/account.rs` compile and pass
- No test regressions

**Dependencies**: T3

---

## Phase 2: Types + Vendor Mapping

### [x] T7: Create gateway/types.rs with ChatCompletionRequest

**Description**: Create types module and implement `ChatCompletionRequest` with required fields.

**Files**:
- `clients/rook/src/gateway/types.rs` (new)

**Implementation**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>,
    #[serde(default)]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
}
```

**Tests**:
- Test minimal valid request deserializes: `{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello"}]}`
- Test `stream` field defaults to `None` when omitted
- Test `stream: true` deserializes correctly
- Test `stream: false` deserializes correctly

**Acceptance**:
- Types compile
- Serde round-trip tests pass
- Unknown fields don't cause deserialization failure

**Dependencies**: T1

---

### [x] T8: Add ChatCompletionResponse and related types

**Description**: Add response types for validation and testing.

**Files**:
- `clients/rook/src/gateway/types.rs`

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

**Tests**:
- Test response serde round-trip with all fields
- Test response with `usage: None` serializes correctly
- Test response with `usage: Some(...)` includes usage object

**Acceptance**:
- Types compile
- Serde round-trip tests pass
- JSON shape matches OpenAI spec

**Dependencies**: T7

---

### [x] T9: Add ModelObject and ModelListResponse types

**Description**: Add types for `/v1/models` endpoint.

**Files**:
- `clients/rook/src/gateway/types.rs`

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelObject>,
}
```

**Tests**:
- Test `ModelObject` serializes to expected JSON shape
- Test `ModelListResponse` with empty data array
- Test `ModelListResponse` with multiple models

**Acceptance**:
- Types compile
- Serde round-trip tests pass
- JSON shape matches OpenAI spec

**Dependencies**: T7

---

### [x] T10: Add GatewayErrorResponse type

**Description**: Add structured error response type matching OpenAI error format.

**Files**:
- `clients/rook/src/gateway/types.rs`

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorResponse {
    pub error: GatewayErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorBody {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: Option<String>,
}
```

**Tests**:
- Test error response serializes with correct nested structure
- Test `code: None` serializes as `null`
- Test `code: Some("model_not_found")` serializes correctly

**Acceptance**:
- Types compile
- JSON shape matches OpenAI error format
- `type` field renames correctly via serde

**Dependencies**: T7

---

### [x] T11: Create gateway/vendor.rs with base URL mapping

**Description**: Implement vendor-to-base-URL mapping function.

**Files**:
- `clients/rook/src/gateway/vendor.rs` (new)

**Implementation**:
```rust
use crate::domain::ProviderVendor;

pub fn default_base_url(vendor: &ProviderVendor) -> &'static str {
    match vendor {
        ProviderVendor::OpenAi => "https://api.openai.com",
        ProviderVendor::Anthropic => "https://api.anthropic.com",
        ProviderVendor::Google => "https://generativelanguage.googleapis.com",
        ProviderVendor::OpenRouter => "https://openrouter.ai/api",
        ProviderVendor::DeepSeek => "https://api.deepseek.com",
        ProviderVendor::Other(_) => "https://api.openai.com",
    }
}
```

**Tests**:
- Test each `ProviderVendor` variant returns correct URL
- Test `Other("custom")` returns OpenAI default

**Acceptance**:
- All vendor variants covered
- URLs match spec table
- Tests pass

**Dependencies**: None

---

### [x] T12: Add effective_base_url function

**Description**: Implement function that resolves effective base URL with override precedence.

**Files**:
- `clients/rook/src/gateway/vendor.rs`

**Implementation**:
```rust
use crate::domain::ProviderAccount;

pub fn effective_base_url(account: &ProviderAccount) -> String {
    let base = account
        .api_base_override
        .as_deref()
        .unwrap_or_else(|| default_base_url(&account.vendor));
    base.trim_end_matches('/').to_string()
}
```

**Tests**:
- Test account with `api_base_override: None` uses vendor default
- Test account with `api_base_override: Some(url)` uses override
- Test trailing slash is stripped: `"https://example.com/"` → `"https://example.com"`

**Acceptance**:
- Override takes precedence over default
- Trailing slashes removed
- Tests pass

**Dependencies**: T11

---

### [x] T13: Add auth_header function

**Description**: Implement vendor-specific auth header construction.

**Files**:
- `clients/rook/src/gateway/vendor.rs`

**Implementation**:
```rust
pub fn auth_header(vendor: &ProviderVendor, api_key: &str) -> (&'static str, String) {
    match vendor {
        ProviderVendor::Anthropic => ("x-api-key", api_key.to_string()),
        _ => ("authorization", format!("Bearer {api_key}")),
    }
}
```

**Tests**:
- Test `Anthropic` returns `("x-api-key", key)`
- Test `OpenAi` returns `("authorization", "Bearer {key}")`
- Test `DeepSeek` returns `("authorization", "Bearer {key}")`
- Test `Other(_)` returns `("authorization", "Bearer {key}")`

**Acceptance**:
- All vendors return correct header format
- Tests pass

**Dependencies**: T11

---

## Phase 3: Upstream Proxy

### [x] T14: Create gateway/upstream.rs module structure

**Description**: Create upstream module with `UpstreamResponse` struct.

**Files**:
- `clients/rook/src/gateway/upstream.rs` (new)

**Implementation**:
```rust
use bytes::Bytes;
use reqwest::StatusCode;

pub struct UpstreamResponse {
    pub status: StatusCode,
    pub body: Bytes,
    pub content_type: Option<String>,
}
```

**Tests**: Verify struct compiles and can be constructed.

**Acceptance**:
- Module compiles
- Struct is public and usable

**Dependencies**: T1

---

### [x] T15: Implement proxy_chat_completion function

**Description**: Implement HTTP proxy logic for chat completions.

**Files**:
- `clients/rook/src/gateway/upstream.rs`

**Implementation**:
```rust
use crate::domain::{ProviderAccount, RookError};
use crate::gateway::vendor;

pub async fn proxy_chat_completion(
    client: &reqwest::Client,
    account: &ProviderAccount,
    api_key: &str,
    raw_body: Bytes,
) -> Result<UpstreamResponse, RookError> {
    let base = vendor::effective_base_url(account);
    let url = format!("{base}/v1/chat/completions");
    let (header_name, header_value) = vendor::auth_header(&account.vendor, api_key);

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .header(header_name, &header_value)
        .body(raw_body)
        .send()
        .await
        .map_err(|e| RookError::Gateway(format!("upstream request to {url} failed: {e}")))?;

    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = response.bytes().await.map_err(|e| {
        RookError::Gateway(format!("failed to read upstream response body: {e}"))
    })?;

    Ok(UpstreamResponse {
        status,
        body,
        content_type,
    })
}
```

**Tests**: Integration test with mock HTTP server (see T16).

**Acceptance**:
- Function compiles
- Constructs correct URL
- Sets correct headers
- Returns response or error

**Dependencies**: T12, T13, T14

---

### [x] T16: Write integration test for proxy_chat_completion

**Description**: Test upstream proxy with mock server.

**Files**:
- `clients/rook/src/gateway/upstream.rs` (test module)

**Implementation**:
Create test helper that spawns a mock axum server, then test:
- Successful 200 response is returned
- Request includes correct auth header
- Request body is forwarded
- Connection error returns `RookError::Gateway`

**Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::post, Json};
    use tokio::net::TcpListener;
    
    async fn mock_upstream() -> (String, tokio::task::JoinHandle<()>) {
        // Spawn mock server returning canned response
        // Return base URL
    }
    
    #[tokio::test]
    async fn test_proxy_success() {
        // Test 200 response
    }
    
    #[tokio::test]
    async fn test_proxy_connection_error() {
        // Test connection failure
    }
}
```

**Acceptance**:
- Mock server helper works
- Success case test passes
- Error case test passes

**Dependencies**: T15

---

## Phase 4: Handlers + Router

### [x] T17: Create gateway/handlers.rs module structure

**Description**: Create handlers module with imports and constants.

**Files**:
- `clients/rook/src/gateway/handlers.rs` (new)

**Implementation**:
```rust
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::gateway::types::*;
use crate::gateway::upstream;
use crate::gateway::GatewayState;
use crate::services::health::HealthService as _;
use crate::services::route::RouteService as _;

const FAILURE_COOLDOWN_SECS: u64 = 60;
```

**Tests**: Verify module compiles.

**Acceptance**:
- Module compiles
- Imports resolve

**Dependencies**: T7-T10, T15

---

### [x] T18: Implement error_response helper

**Description**: Implement helper function to build structured error responses.

**Files**:
- `clients/rook/src/gateway/handlers.rs`

**Implementation**:
```rust
fn error_response(
    status: StatusCode,
    message: &str,
    error_type: &str,
    code: Option<&str>,
) -> Response {
    let body = GatewayErrorResponse {
        error: GatewayErrorBody {
            message: message.to_string(),
            error_type: error_type.to_string(),
            code: code.map(|s| s.to_string()),
        },
    };
    (status, Json(body)).into_response()
}
```

**Tests**:
- Test helper returns correct status code
- Test response body has correct structure
- Test `code: None` serializes as null

**Acceptance**:
- Helper compiles
- Returns valid axum Response
- Tests pass

**Dependencies**: T10, T17

---

### [x] T19: Implement handle_chat_completions handler

**Description**: Implement main chat completions handler with full logic.

**Files**:
- `clients/rook/src/gateway/handlers.rs`

**Implementation**:
Full handler implementation per design doc:
1. Parse request body
2. Check stream flag
3. Resolve routing
4. Extract api_key
5. Proxy to upstream
6. Mark health
7. Return response

**Tests**: Integration tests (see T24).

**Acceptance**:
- Handler compiles
- Signature matches axum handler requirements
- All error paths return structured errors
- Health feedback called

**Dependencies**: T15, T18

---

### [x] T20: Implement handle_list_models handler

**Description**: Implement models list handler.

**Files**:
- `clients/rook/src/gateway/handlers.rs`

**Implementation**:
```rust
pub async fn handle_list_models(
    State(state): State<GatewayState>,
) -> Json<ModelListResponse> {
    let routes = state.registry.routes().list().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let models: Vec<ModelObject> = routes
        .into_iter()
        .map(|route| ModelObject {
            id: route.logical_model,
            object: "model".to_string(),
            created: now,
            owned_by: "rook".to_string(),
        })
        .collect();

    Json(ModelListResponse {
        object: "list".to_string(),
        data: models,
    })
}
```

**Tests**: Integration tests (see T24).

**Acceptance**:
- Handler compiles
- Returns correct JSON structure
- Maps routes to ModelObjects

**Dependencies**: T9, T17

---

### [x] T21: Create GatewayState struct in gateway/mod.rs

**Description**: Define shared state struct for handlers.

**Files**:
- `clients/rook/src/gateway/mod.rs`

**Implementation**:
```rust
use crate::registry::RookRegistry;
use crate::routing::RoutingEngine;

#[derive(Clone)]
pub struct GatewayState {
    pub registry: RookRegistry,
    pub engine: RoutingEngine,
    pub http_client: reqwest::Client,
}
```

**Tests**: Verify struct compiles and is Clone.

**Acceptance**:
- Struct compiles
- Derives Clone
- All fields are public

**Dependencies**: T1

---

### [x] T22: Implement build_router function

**Description**: Implement router construction function.

**Files**:
- `clients/rook/src/gateway/mod.rs`

**Implementation**:
```rust
pub mod handlers;
pub mod types;
pub mod upstream;
pub mod vendor;

use axum::{Router, routing::{get, post}};

pub fn build_router(state: GatewayState) -> Router {
    Router::new()
        .route("/chat/completions", post(handlers::handle_chat_completions))
        .route("/models", get(handlers::handle_list_models))
        .with_state(state)
}
```

**Tests**: Integration tests (see T24).

**Acceptance**:
- Function compiles
- Returns Router
- Routes are mounted correctly

**Dependencies**: T19, T20, T21

---

### [x] T23: Update gateway/mod.rs exports

**Description**: Export all submodules and public API.

**Files**:
- `clients/rook/src/gateway/mod.rs`

**Implementation**:
Ensure all modules are declared and `GatewayState` + `build_router` are public.

**Tests**: Verify `use crate::gateway::*` works from other modules.

**Acceptance**:
- All submodules exported
- Public API accessible
- No compiler warnings

**Dependencies**: T7-T22

---

### [x] T24: Write handler integration tests

**Description**: Comprehensive integration tests for both handlers.

**Files**:
- `clients/rook/src/gateway/handlers.rs` (test module)

**Implementation**:
Test cases:
- `/v1/models` with no routes → empty list
- `/v1/models` with routes → correct list
- `/v1/chat/completions` happy path → 200
- `/v1/chat/completions` unknown model → 503
- `/v1/chat/completions` stream: true → 400
- `/v1/chat/completions` invalid body → 400
- `/v1/chat/completions` upstream error → 502
- `/v1/chat/completions` no api_key → 502
- Health feedback on success
- Health feedback on failure

**Tests**:
Create test helpers:
- `test_app()` — builds Router with in-memory registry
- `mock_upstream(status, body)` — spawns mock server
- `seed_account_pool_route(registry, base_url, api_key)` — seeds test data

**Acceptance**:
- All test cases pass
- Tests use in-memory registry
- Mock server pattern works
- Health service state verified

**Dependencies**: T19, T20, T22

---

## Phase 5: Server Wiring

### [x] T25: Update ServerConfig to include db_path

**Description**: Add optional `db_path` field to `ServerConfig`.

**Files**:
- `clients/rook/src/server/mod.rs`

**Implementation**:
```rust
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_tui: bool,
    /// Path to the SQLite database file. Defaults to `"./rook.db"`.
    pub db_path: Option<String>,
}
```

**Tests**: Verify struct compiles and defaults work.

**Acceptance**:
- Field added
- Existing code compiles
- Default behavior unchanged

**Dependencies**: None

---

### [x] T26: Wire gateway into server::run

**Description**: Create registry, engine, client, and mount gateway router.

**Files**:
- `clients/rook/src/server/mod.rs`

**Implementation**:
```rust
pub async fn run(config: ServerConfig) -> Result<(), RookError> {
    // Create registry, engine, HTTP client
    let db_path = config.db_path.as_deref().unwrap_or("./rook.db");
    let registry = RookRegistry::open(db_path).await?;
    let engine = RoutingEngine::new(registry.clone());
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| RookError::Gateway(format!("failed to build HTTP client: {e}")))?;

    let gateway_state = crate::gateway::GatewayState {
        registry,
        engine,
        http_client,
    };

    // Build router
    let app = Router::new()
        .nest("/api", api_stub_router())
        .nest("/v1", crate::gateway::build_router(gateway_state))
        .merge(dashboard::router());

    // ... rest unchanged
}
```

**Tests**: Manual verification that server starts and routes are accessible.

**Acceptance**:
- Server starts without errors
- `/v1/chat/completions` is reachable
- `/v1/models` is reachable
- `/api/health` still works

**Dependencies**: T22, T25

---

### [x] T27: Write end-to-end smoke test

**Description**: Full integration test from HTTP request to response.

**Files**:
- `clients/rook/tests/gateway_e2e.rs` (new)

**Implementation**:
Test that:
1. Starts server with test config
2. Seeds account + pool + route
3. Sends real HTTP request to `/v1/chat/completions`
4. Verifies response
5. Sends request to `/v1/models`
6. Verifies model list

**Tests**:
```rust
#[tokio::test]
async fn test_gateway_e2e() {
    // Start server
    // Seed data
    // Send requests
    // Verify responses
}
```

**Acceptance**:
- Test passes
- Both endpoints work end-to-end
- Health feedback works

**Dependencies**: T26

---

### [x] T28: Update routing test fixtures

**Description**: Update test fixtures in routing module to include `api_key: None`.

**Files**:
- `clients/rook/src/routing/mod.rs`

**Implementation**:
Find all `ProviderAccount` constructions in tests and add `api_key: None`.

**Tests**: Verify all routing tests pass.

**Acceptance**:
- All routing tests compile and pass
- No test regressions

**Dependencies**: T3

---

## Task Dependencies Graph

```
Phase 1: Infrastructure
T1 (reqwest) ─┬─→ T7 (types)
              └─→ T14 (upstream struct)

T2 (migration) ─→ T4 (wire migration)
T3 (domain) ────→ T4, T5, T6, T28

T4 (wire migration) ─→ T5 (db layer)
T5 (db layer) ───────→ [Phase 2]

Phase 2: Types + Vendor Mapping
T7 (request types) ─→ T8, T9, T10
T8 (response types) ─→ [Phase 4]
T9 (model types) ───→ T20
T10 (error types) ──→ T18

T11 (base URL) ─→ T12, T13
T12 (effective URL) ─→ T15
T13 (auth header) ──→ T15

Phase 3: Upstream Proxy
T14 (upstream struct) ─→ T15
T15 (proxy function) ──→ T16, T19
T16 (proxy tests) ─────→ [Phase 4]

Phase 4: Handlers + Router
T17 (handlers module) ─→ T18, T19, T20
T18 (error helper) ────→ T19
T19 (chat handler) ────→ T22, T24
T20 (models handler) ──→ T22, T24
T21 (GatewayState) ────→ T22
T22 (build_router) ────→ T23, T24, T26
T23 (mod exports) ─────→ T26
T24 (handler tests) ───→ [Phase 5]

Phase 5: Server Wiring
T25 (ServerConfig) ─→ T26
T26 (server wiring) ─→ T27
T27 (e2e test) ─────→ [Done]
T28 (routing fixtures) → [Done]
```

---

## Verification Checklist

After completing all tasks, verify:

- [ ] All unit tests pass: `cargo test --lib`
- [ ] All integration tests pass: `cargo test --test '*'`
- [ ] Clippy passes: `cargo clippy --all-targets -- -D warnings`
- [ ] Format check passes: `cargo fmt --all -- --check`
- [ ] Migration applies cleanly to fresh database
- [ ] Migration applies cleanly to database with existing data
- [ ] `/v1/chat/completions` returns 200 with valid request
- [ ] `/v1/models` returns model list
- [ ] `stream: true` returns 400
- [ ] Unknown model returns 503
- [ ] Upstream error returns 502
- [ ] Health service updates after upstream calls
- [ ] Existing routes (`/api/health`, dashboard) still work
- [ ] No regressions in existing tests

---

## Estimated Effort

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 1: Infrastructure | T1-T6 | 2-3 hours |
| Phase 2: Types + Vendor | T7-T13 | 2-3 hours |
| Phase 3: Upstream Proxy | T14-T16 | 1-2 hours |
| Phase 4: Handlers + Router | T17-T24 | 3-4 hours |
| Phase 5: Server Wiring | T25-T28 | 1-2 hours |
| **Total** | **28 tasks** | **9-14 hours** |

Each task is designed to be completable in 30-60 minutes following TDD.

---

## Notes

- All tasks follow TDD: write failing test, implement minimum code, verify, refactor
- Each task is independently testable where possible
- Integration tests use in-memory registry and mock HTTP servers
- Health feedback is verified in integration tests
- Manual validation with real API keys is recommended after T27
