# Design: OpenAI-Compatible Gateway API (`/v1/chat/completions` & `/v1/models`)

## Technical Approach

Implement a raw HTTP proxy gateway that accepts OpenAI-shaped requests, resolves routing via the
existing `RoutingEngine`, forwards requests to upstream providers with vendor-specific auth headers,
and returns upstream responses verbatim. This maps directly to the proposal's "Raw HTTP Proxy"
architecture — no `corvus-traits::Provider` consumption, no per-vendor adapters.

The gateway adds five new files under `gateway/`, one migration, and surgical modifications to four
existing files (`domain/mod.rs`, `db/account.rs`, `db/mod.rs`, `server/mod.rs`).

## Architecture Decisions

### Decision: Raw HTTP Proxy over Provider Trait Consumption

**Choice**: Forward raw JSON bodies to upstream providers via `reqwest`, returning responses
verbatim.
**Alternatives considered**: Deserialize requests into `corvus-traits::Provider` calls, then
re-serialize responses.
**Rationale**: Proxy approach is simpler (no per-vendor adapter code), preserves fidelity
(vendor-specific extensions pass through), and any OpenAI-compatible provider works out of the box.
The `Provider` trait is designed for the Rust runtime's internal dispatch, not for gateway proxying.

### Decision: `reqwest` with `rustls-tls` for Upstream HTTP

**Choice**: Add `reqwest` with `json` + `rustls-tls` features.
**Alternatives considered**: `hyper` directly, `ureq` (blocking).
**Rationale**: `reqwest` provides the right abstraction level for a proxy — connection pooling,
timeouts, TLS — without the boilerplate of raw `hyper`. `rustls-tls` avoids OpenSSL system
dependency, consistent with the project's `runtime-tokio-rustls` choice for sqlx.

### Decision: Shared `reqwest::Client` via `GatewayState`

**Choice**: Single `reqwest::Client` instance created at startup, shared across all handler
invocations via axum's `State` extractor.
**Alternatives considered**: Create a new client per request.
**Rationale**: `reqwest::Client` manages an internal connection pool. Creating one per request wastes
connections and prevents keep-alive. A shared client is the documented best practice.

### Decision: `api_key` Stored as Plaintext in M1

**Choice**: `api_key TEXT` column in SQLite, no encryption at rest.
**Alternatives considered**: Encrypted column with a master key, external secrets manager.
**Rationale**: M1 scope — local SQLite file permissions are the primary control. Encryption is
explicitly scoped to #591 (transport hardening). The column is nullable, so existing rows get
`NULL` without data migration issues.

### Decision: `stream: true` Returns 400, Not Silent Ignore

**Choice**: Parse the `stream` field; if `true`, return `400 Bad Request` with a clear message.
**Alternatives considered**: Silently ignore the field and return non-streaming, attempt streaming.
**Rationale**: Honest failure prevents clients from assuming streaming works. Designing the handler
return type as `axum::response::Response` (not `Json<T>`) means streaming can be added later without
signature changes.

### Decision: Gateway Router Nested at `/v1`

**Choice**: `Router::nest("/v1", gateway::build_router(state))` in `server::run`.
**Alternatives considered**: Mount at root with full paths in handlers, merge without nesting.
**Rationale**: Nesting at `/v1` keeps the gateway isolated from `/api` (admin) and `/` (dashboard).
Handlers define relative paths (`/chat/completions`, `/models`), matching the OpenAI URL structure.

## Data Flow

### Chat Completions Request Flow

```
Client
  │
  ├── POST /v1/chat/completions
  │   Content-Type: application/json
  │   Body: { "model": "gpt-4o", "messages": [...] }
  │
  ▼
┌─────────────────────────────────────────────────────────┐
│  axum Router (/v1)                                      │
│  └── POST /chat/completions → handle_chat_completions() │
└─────────────────────────────────────────────────────────┘
  │
  │ 1. Parse JSON body → ChatCompletionRequest
  │ 2. Reject if stream == Some(true) → 400
  │ 3. Extract model name
  │
  ▼
┌───────────────────────────┐
│  RoutingEngine::resolve() │
│  (registry → pool → acct) │
└───────────────────────────┘
  │
  │ Returns RoutingDecision { account, pool_id, route_id }
  │ — or — RookError::Routing → 503
  │
  ▼
┌────────────────────────────────────────────────┐
│  upstream::proxy_chat_completion()             │
│  1. Resolve base URL (override or vendor map)  │
│  2. Build URL: {base}/v1/chat/completions      │
│  3. Set auth header per vendor                 │
│  4. POST raw JSON body via reqwest             │
└────────────────────────────────────────────────┘
  │
  │ HTTP response from upstream provider
  │
  ▼
┌─────────────────────────────────────────────────┐
│  Handler post-processing                        │
│  1. 2xx → mark_success(account_id) → return body│
│  2. 4xx/5xx → mark_failure(account_id, 60)      │
│     → return GatewayErrorResponse with 502      │
│  3. Connection error → mark_failure → 502       │
└─────────────────────────────────────────────────┘
  │
  ▼
Client receives response
```

### Models List Request Flow

```
Client
  │
  ├── GET /v1/models
  │
  ▼
┌──────────────────────────────────────────────┐
│  axum Router (/v1)                           │
│  └── GET /models → handle_list_models()      │
└──────────────────────────────────────────────┘
  │
  │ 1. registry.routes().list() → Vec<ModelRoute>
  │ 2. Map each route to ModelObject { id: logical_model, ... }
  │ 3. Return ModelListResponse { object: "list", data: [...] }
  │
  ▼
Client receives JSON response
```

## Module Structure

### File Layout

```
clients/rook/src/gateway/
├── mod.rs          — module exports + GatewayState + build_router()
├── types.rs        — OpenAI-compatible serde types
├── vendor.rs       — vendor URL mapping + auth header construction
├── upstream.rs     — HTTP proxy logic for upstream provider calls
└── handlers.rs     — axum handler functions
```

### Module Details

#### `gateway/mod.rs`

Replaces the current 10-line stub. Exports submodules and provides the composition entry point.

```rust
pub mod handlers;
pub mod types;
pub mod upstream;
pub mod vendor;

use crate::registry::RookRegistry;
use crate::routing::RoutingEngine;
use axum::{Router, routing::{get, post}};

/// Shared state for all gateway handlers.
///
/// Cheap to clone — all inner types are `Arc`-backed or use connection pools.
#[derive(Clone)]
pub struct GatewayState {
    pub registry: RookRegistry,
    pub engine: RoutingEngine,
    pub http_client: reqwest::Client,
}

/// Build the gateway router with OpenAI-compatible endpoints.
///
/// Mount at `/v1` in the server:
/// ```ignore
/// Router::new().nest("/v1", gateway::build_router(state))
/// ```
pub fn build_router(state: GatewayState) -> Router {
    Router::new()
        .route("/chat/completions", post(handlers::handle_chat_completions))
        .route("/models", get(handlers::handle_list_models))
        .with_state(state)
}
```

**Public API**:
- `GatewayState` struct
- `build_router(GatewayState) -> Router`

**Dependencies**: `crate::registry`, `crate::routing`, `axum`, `reqwest`

---

#### `gateway/types.rs`

All OpenAI-compatible serde types. These are data-only — no business logic.

```rust
use serde::{Deserialize, Serialize};

// ── Chat Completions Request ──────────────────────────────────────────────

/// OpenAI-compatible chat completion request.
///
/// Only fields needed for routing and validation are parsed. The full body is
/// forwarded to the upstream provider as-is, so vendor-specific extensions pass
/// through transparently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// The model to use (maps to a logical model name in the route table).
    pub model: String,
    /// The messages to generate a completion for.
    pub messages: Vec<ChatCompletionMessage>,
    /// If `true`, the response will be streamed via SSE.
    /// M1: not supported — returns 400 if set to `true`.
    #[serde(default)]
    pub stream: Option<bool>,
    // All other fields (temperature, max_tokens, etc.) are forwarded
    // as raw JSON — we don't need to parse them for routing.
}

/// A single message in a chat completion conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    /// One of: "system", "user", "assistant", "tool"
    pub role: String,
    /// The message content. Optional for assistant messages with tool_calls.
    pub content: Option<serde_json::Value>,
    // Other fields (name, tool_calls, tool_call_id) pass through via
    // the raw JSON forwarding — no need to model them here.
}

// ── Chat Completions Response ─────────────────────────────────────────────
// Note: these types are defined for test validation and /v1/models.
// The actual chat completion response is returned verbatim from upstream.

/// OpenAI-compatible chat completion response.
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

/// A single completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Models ────────────────────────────────────────────────────────────────

/// OpenAI-compatible model object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    /// Model identifier — corresponds to `ModelRoute.logical_model`.
    pub id: String,
    /// Always `"model"` per the OpenAI spec.
    pub object: String,
    /// Unix timestamp of model creation. We use the current time since
    /// we don't track route creation timestamps.
    pub created: u64,
    /// Owner — we use `"rook"` as a synthetic owner.
    pub owned_by: String,
}

/// OpenAI-compatible model list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    /// Always `"list"`.
    pub object: String,
    pub data: Vec<ModelObject>,
}

// ── Error Response ────────────────────────────────────────────────────────

/// Structured error response matching OpenAI's error format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorResponse {
    pub error: GatewayErrorBody,
}

/// Inner error body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorBody {
    pub message: String,
    /// Error classification: "routing_error", "upstream_error",
    /// "invalid_request_error", "gateway_error"
    #[serde(rename = "type")]
    pub error_type: String,
    /// Additional info (e.g., upstream status code). Null when not applicable.
    pub code: Option<String>,
}
```

**Public API**: All types are `pub`.
**Dependencies**: `serde`, `serde_json` (for `serde_json::Value` in message content)

---

#### `gateway/vendor.rs`

Maps `ProviderVendor` to base URLs and constructs auth headers.

```rust
use crate::domain::{ProviderAccount, ProviderVendor};

/// Default API base URLs per vendor.
///
/// These are the production endpoints. Accounts may override via
/// `api_base_override`.
pub fn default_base_url(vendor: &ProviderVendor) -> &'static str {
    match vendor {
        ProviderVendor::OpenAi      => "https://api.openai.com",
        ProviderVendor::Anthropic   => "https://api.anthropic.com",
        ProviderVendor::Google      => "https://generativelanguage.googleapis.com",
        ProviderVendor::OpenRouter  => "https://openrouter.ai/api",
        ProviderVendor::DeepSeek    => "https://api.deepseek.com",
        // Unknown vendors: use OpenAI-compatible endpoint as a reasonable
        // default. Accounts should set api_base_override for non-standard vendors.
        ProviderVendor::Other(_)    => "https://api.openai.com",
    }
}

/// Resolve the effective base URL for an account.
///
/// Precedence: `api_base_override` > `default_base_url(vendor)`.
/// Trailing slashes are stripped for consistent URL joining.
pub fn effective_base_url(account: &ProviderAccount) -> String {
    let base = account
        .api_base_override
        .as_deref()
        .unwrap_or_else(|| default_base_url(&account.vendor));
    base.trim_end_matches('/').to_string()
}

/// Auth header name and value for a vendor.
///
/// Most vendors use `Authorization: Bearer {key}`.
/// Anthropic uses `x-api-key: {key}`.
pub fn auth_header(vendor: &ProviderVendor, api_key: &str) -> (&'static str, String) {
    match vendor {
        ProviderVendor::Anthropic => ("x-api-key", api_key.to_string()),
        // OpenAI, DeepSeek, OpenRouter, Google, and unknown vendors all use Bearer.
        _ => ("authorization", format!("Bearer {api_key}")),
    }
}
```

**Public API**:
- `default_base_url(&ProviderVendor) -> &'static str`
- `effective_base_url(&ProviderAccount) -> String`
- `auth_header(&ProviderVendor, &str) -> (&'static str, String)`

**Dependencies**: `crate::domain`

**Vendor → Base URL mapping table**:

| ProviderVendor  | Default Base URL                                  |
|-----------------|---------------------------------------------------|
| `OpenAi`        | `https://api.openai.com`                          |
| `Anthropic`     | `https://api.anthropic.com`                       |
| `Google`        | `https://generativelanguage.googleapis.com`       |
| `OpenRouter`    | `https://openrouter.ai/api`                       |
| `DeepSeek`      | `https://api.deepseek.com`                        |
| `Other(_)`      | `https://api.openai.com` (override recommended)   |

**Auth header mapping**:

| ProviderVendor  | Header Name     | Header Value          |
|-----------------|----------------|-----------------------|
| `Anthropic`     | `x-api-key`    | `{api_key}`           |
| All others      | `authorization`| `Bearer {api_key}`    |

---

#### `gateway/upstream.rs`

HTTP proxy logic — constructs upstream requests and handles responses.

```rust
use crate::domain::{ProviderAccount, RookError};
use crate::gateway::types::GatewayErrorResponse;
use crate::gateway::vendor;

/// Result of an upstream proxy call.
pub struct UpstreamResponse {
    /// HTTP status code from upstream.
    pub status: reqwest::StatusCode,
    /// Response body bytes.
    pub body: bytes::Bytes,
    /// Content-Type header from upstream (for passthrough).
    pub content_type: Option<String>,
}

/// Forward a chat completion request body to the upstream provider.
///
/// Constructs the URL, sets auth headers, and sends the raw JSON body.
/// Returns the raw upstream response or a `RookError::Gateway` on connection
/// failure / timeout.
pub async fn proxy_chat_completion(
    client: &reqwest::Client,
    account: &ProviderAccount,
    api_key: &str,
    raw_body: bytes::Bytes,
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
        .map_err(|e| RookError::Gateway(format!(
            "upstream request to {url} failed: {e}"
        )))?;

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

**Public API**:
- `UpstreamResponse` struct
- `proxy_chat_completion(client, account, api_key, body) -> Result<UpstreamResponse, RookError>`

**Dependencies**: `crate::domain`, `crate::gateway::vendor`, `reqwest`, `bytes`

---

#### `gateway/handlers.rs`

Axum handler functions. Each handler extracts state, processes the request, and constructs a response.

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

/// Default cooldown in seconds applied when an upstream call fails.
const FAILURE_COOLDOWN_SECS: u64 = 60;

/// POST /v1/chat/completions
///
/// 1. Read raw body bytes (kept for upstream forwarding).
/// 2. Parse into ChatCompletionRequest (extract `model` + validate `stream`).
/// 3. Resolve routing via RoutingEngine.
/// 4. Extract api_key from the resolved account.
/// 5. Forward raw body to upstream.
/// 6. Mark health success/failure.
/// 7. Return upstream response or structured error.
pub async fn handle_chat_completions(
    State(state): State<GatewayState>,
    body: Bytes,
) -> Response {
    // 1. Parse the request to extract model name and check stream flag
    let request: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid request body: {e}"),
                "invalid_request_error",
                None,
            );
        }
    };

    // 2. Reject streaming requests (M1 scope)
    if request.stream == Some(true) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "streaming is not yet supported; \
             set stream: false or omit the field",
            "invalid_request_error",
            Some("streaming_not_supported"),
        );
    }

    // 3. Resolve routing
    let decision = match state.engine.resolve(&request.model).await {
        Ok(d) => d,
        Err(e) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("routing failed: {e}"),
                "routing_error",
                None,
            );
        }
    };

    // 4. Extract API key
    let api_key = match &decision.account.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!(
                    "account '{}' has no API key configured",
                    decision.account.display_name
                ),
                "gateway_error",
                Some("missing_api_key"),
            );
        }
    };

    // 5. Forward to upstream
    let account_id = decision.account.id;
    match upstream::proxy_chat_completion(
        &state.http_client,
        &decision.account,
        &api_key,
        body,
    )
    .await
    {
        Ok(upstream_resp) => {
            if upstream_resp.status.is_success() {
                // 6a. Success — mark healthy
                state.registry.health().mark_success(account_id).await;

                // Return upstream response verbatim
                let mut builder = Response::builder()
                    .status(upstream_resp.status.as_u16());
                if let Some(ct) = &upstream_resp.content_type {
                    builder = builder.header("content-type", ct.as_str());
                }
                builder
                    .body(axum::body::Body::from(upstream_resp.body))
                    .unwrap_or_else(|_| {
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    })
            } else {
                // 6b. Upstream error — mark failure
                state
                    .registry
                    .health()
                    .mark_failure(account_id, FAILURE_COOLDOWN_SECS)
                    .await;

                error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!(
                        "upstream returned HTTP {}",
                        upstream_resp.status.as_u16()
                    ),
                    "upstream_error",
                    Some(
                        &upstream_resp.status.as_u16().to_string()
                    ),
                )
            }
        }
        Err(e) => {
            // 6c. Connection error — mark failure
            state
                .registry
                .health()
                .mark_failure(account_id, FAILURE_COOLDOWN_SECS)
                .await;

            error_response(
                StatusCode::BAD_GATEWAY,
                &format!("upstream connection failed: {e}"),
                "gateway_error",
                Some("connection_error"),
            )
        }
    }
}

/// GET /v1/models
///
/// Returns all configured logical models in OpenAI ModelObject format.
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

// ── Helpers ───────────────────────────────────────────────────────────────

/// Build a structured error response matching OpenAI's error format.
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

**Public API**:
- `handle_chat_completions(State, Bytes) -> Response`
- `handle_list_models(State) -> Json<ModelListResponse>`

**Dependencies**: `crate::gateway::{types, upstream, GatewayState}`, `crate::services::health`,
`crate::services::route`, `axum`, `serde_json`

## State Management

### `GatewayState` Definition

```rust
#[derive(Clone)]
pub struct GatewayState {
    pub registry: RookRegistry,
    pub engine: RoutingEngine,
    pub http_client: reqwest::Client,
}
```

**Construction** (in `server::run`):
```rust
let registry = RookRegistry::open("./rook.db").await?;
let engine = RoutingEngine::new(registry.clone());
let http_client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(30))
    .build()
    .map_err(|e| RookError::Gateway(format!("failed to build HTTP client: {e}")))?;

let gateway_state = GatewayState {
    registry: registry.clone(),
    engine,
    http_client,
};
```

**How it reaches handlers**: Axum's `State` extractor. `GatewayState` implements `Clone` (required
by axum). All inner types are already cheap to clone:
- `RookRegistry` — all fields are `Arc`-backed services
- `RoutingEngine` — holds `RookRegistry` + `Arc<Mutex<_>>` counters
- `reqwest::Client` — internally `Arc<_>`

## Database Changes

### Migration `0003_account_api_key.sql`

```sql
-- Add api_key column to provider_accounts for upstream authentication.
-- Nullable: existing accounts get NULL (no key configured yet).
ALTER TABLE provider_accounts ADD COLUMN api_key TEXT DEFAULT NULL;
```

This is a backward-compatible schema change. `ALTER TABLE ADD COLUMN` with `DEFAULT NULL` works on
existing databases without data migration.

### Changes to `db/mod.rs`

Add the new migration constant and apply it in `run_migrations`:

```rust
const MIGRATION_SQL_0003: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0003_account_api_key.sql"
));
```

Then in `run_migrations()`, add a block mirroring the existing `0002_settings` pattern:

```rust
// ── Migration 0003: account_api_key ───────────────────────────────────
let version_0003 = "0003_account_api_key";
let row_0003: Option<(String,)> = sqlx::query_as(
    "SELECT version FROM schema_migrations WHERE version = ?"
)
.bind(version_0003)
.fetch_optional(pool)
.await
.map_err(|e| RookError::Registry(
    format!("failed to check migration 0003 status: {e}")
))?;

if row_0003.is_none() {
    sqlx::raw_sql(MIGRATION_SQL_0003)
        .execute(pool)
        .await
        .map_err(|e| RookError::Registry(
            format!("migration 0003 failed: {e}")
        ))?;

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)"
    )
    .bind(version_0003)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| RookError::Registry(
        format!("failed to record migration 0003: {e}")
    ))?;
}
```

### Changes to `domain/mod.rs`

Add `api_key` field to `ProviderAccount`:

```rust
pub struct ProviderAccount {
    pub id: AccountId,
    pub vendor: ProviderVendor,
    pub display_name: String,
    pub api_base_override: Option<String>,
    /// API key for authenticating with the upstream provider.
    /// Stored as plaintext in M1; encryption-at-rest is #591.
    pub api_key: Option<String>,
    pub enabled: bool,
    pub weight: u32,
    pub priority: u32,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
}
```

**Impact on existing code**: Every `ProviderAccount` literal in tests and production code must add
the `api_key` field. The field is `Option<String>`, so existing test fixtures add
`api_key: None` — mechanical, no logic changes.

### Changes to `db/account.rs`

**`row_to_account`**: Add extraction of the `api_key` column:

```rust
let api_key: Option<String> = row
    .try_get("api_key")
    .map_err(|e| RookError::Registry(format!("missing api_key: {e}")))?;
```

And include it in the returned `ProviderAccount`.

**`insert_account`**: Add `api_key` to the INSERT statement:

```rust
sqlx::query(
    "INSERT INTO provider_accounts \
     (id, display_name, vendor, api_base, api_key, enabled, weight, priority, \
      tags, capabilities, created_at, updated_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
)
// ... existing binds ...
.bind(&account.api_key)  // new bind after api_base
// ... remaining binds ...
```

**`get_account` and `list_accounts`**: Add `api_key` to the SELECT column lists:

```sql
SELECT id, display_name, vendor, api_base, api_key, enabled, weight, priority,
       tags, capabilities, created_at, updated_at
FROM provider_accounts ...
```

### Changes to `services/account.rs`

No trait signature changes needed. The `AccountService` trait methods (`create`, `get`, `list`,
`update`, `delete`) already operate on `ProviderAccount` values, which now include `api_key`.
The `InMemoryAccountService` stores the whole struct — it automatically picks up the new field.
Only test fixtures in `services/account.rs` need `api_key: None` added to `make_account()`.

## Error Handling Strategy

### RookError → HTTP Status Code Mapping

| RookError Variant     | HTTP Status | Response Body Type     | When                                    |
|-----------------------|-------------|------------------------|-----------------------------------------|
| `Routing(msg)`        | 503         | `GatewayErrorResponse` | No route, all pools exhausted, cycle    |
| `Gateway(msg)`        | 502         | `GatewayErrorResponse` | Upstream connection failure / timeout   |
| N/A (parse failure)   | 400         | `GatewayErrorResponse` | Invalid JSON body                       |
| N/A (`stream: true`)  | 400         | `GatewayErrorResponse` | Streaming not supported                 |
| N/A (no api_key)      | 502         | `GatewayErrorResponse` | Account has no key configured           |

### Upstream HTTP Status → Gateway Response Mapping

| Upstream Status | Gateway HTTP Status | Error Type       | Notes                           |
|-----------------|---------------------|------------------|---------------------------------|
| 2xx             | Same as upstream    | N/A              | Body passed through verbatim    |
| 4xx             | 502                 | `upstream_error` | Client issue at upstream level  |
| 5xx             | 502                 | `upstream_error` | Server error at upstream level  |
| Connection fail | 502                 | `gateway_error`  | DNS, timeout, TLS failure, etc. |

**Design rationale**: All upstream non-2xx responses map to 502 (Bad Gateway) because from the
client's perspective, Rook is a gateway and the upstream is a backend. The original upstream status
is included in the error body's `code` field for debugging.

### GatewayErrorResponse Format

```json
{
  "error": {
    "message": "routing failed: no route configured for model 'gpt-5'",
    "type": "routing_error",
    "code": null
  }
}
```

This matches OpenAI's error response shape, so clients with OpenAI error handling logic can parse
it naturally.

## Server Wiring Changes

### Exact Changes to `server::run()`

The function signature changes to accept a database path (or the config can be extended):

```rust
pub async fn run(config: ServerConfig) -> Result<(), RookError> {
    // ── New: create registry, engine, and HTTP client ──────────────────
    let db_path = config.db_path.as_deref().unwrap_or("./rook.db");
    let registry = RookRegistry::open(db_path).await?;
    let engine = RoutingEngine::new(registry.clone());
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| RookError::Gateway(format!(
            "failed to build HTTP client: {e}"
        )))?;

    let gateway_state = crate::gateway::GatewayState {
        registry,
        engine,
        http_client,
    };

    // ── Router assembly ───────────────────────────────────────────────
    let app = Router::new()
        .nest("/api", api_stub_router())
        .nest("/v1", crate::gateway::build_router(gateway_state))  // ← new
        .merge(dashboard::router());

    // ... rest unchanged ...
}
```

### ServerConfig Extension

Add optional `db_path` to `ServerConfig`:

```rust
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_tui: bool,
    /// Path to the SQLite database file. Defaults to `"./rook.db"`.
    pub db_path: Option<String>,
}
```

Default: `db_path: None` (falls back to `"./rook.db"`).

### Route Nesting Strategy

```
/api/*          → admin stub (existing, unchanged)
/v1/chat/completions  → gateway::handle_chat_completions
/v1/models            → gateway::handle_list_models
/*              → dashboard assets (existing, unchanged)
```

Order matters: `nest("/api", ...)` and `nest("/v1", ...)` are checked before the dashboard's
catch-all `merge`. This is already correct because axum routes are matched by specificity, and
`nest` creates a prefix match before the dashboard's fallback.

## Dependency Changes

### Cargo.toml Additions

```toml
# HTTP client for upstream provider calls
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# Byte buffer for raw body forwarding
bytes = "1.0"
```

**Note**: `bytes` may already be a transitive dependency via `axum`/`hyper`, but adding it
explicitly ensures the API is available. `reqwest` 0.12 aligns with the project's async ecosystem
(tokio 1.x, hyper 1.x).

## Testing Architecture

### Unit Test Strategy

| Module         | What to Test                                          | Approach                                |
|----------------|-------------------------------------------------------|----------------------------------------|
| `types.rs`     | Serde round-trip for all types                        | `#[cfg(test)]` inline: serialize → deserialize → assert equality |
| `types.rs`     | `ChatCompletionRequest` with `stream: true/false/omitted` | Deserialize variants, check `stream` field |
| `types.rs`     | `GatewayErrorResponse` JSON shape matches OpenAI format | Serialize and compare against expected JSON string |
| `vendor.rs`    | `default_base_url` returns correct URL per vendor     | Exhaustive match test for all `ProviderVendor` variants |
| `vendor.rs`    | `effective_base_url` respects override precedence     | Test with `api_base_override: Some(...)` and `None` |
| `vendor.rs`    | `effective_base_url` strips trailing slashes          | Test `"https://example.com/"` → `"https://example.com"` |
| `vendor.rs`    | `auth_header` returns correct header per vendor       | Anthropic → `x-api-key`, others → `Authorization: Bearer` |
| `db/account.rs`| `api_key` round-trip through insert/get               | Existing test pattern + `api_key: Some("sk-test".into())` |
| `db/account.rs`| `api_key` as `None` round-trips correctly             | Verify `NULL` → `None` |
| `domain/mod.rs`| Existing tests pass with new field                    | Add `api_key: None` to test fixtures (mechanical) |

### Integration Test Strategy

Integration tests use `axum::test` (via `tower::ServiceExt` / `axum::body::to_bytes`) and a mock
upstream server.

**Mock Upstream Server Pattern**:

```rust
/// Start a mock upstream server that returns a canned response.
/// Returns the base URL (e.g., "http://127.0.0.1:{port}").
async fn mock_upstream(
    status: StatusCode,
    body: serde_json::Value,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/v1/chat/completions", post(move || async move {
            (status, Json(body.clone()))
        }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), handle)
}
```

**Test Helper: Build Test App**:

```rust
/// Create a fully wired axum app backed by an in-memory registry.
async fn test_app() -> (Router, RookRegistry) {
    let registry = RookRegistry::open_in_memory().await.unwrap();
    let engine = RoutingEngine::new(registry.clone());
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let state = GatewayState { registry: registry.clone(), engine, http_client };
    let app = Router::new()
        .nest("/v1", crate::gateway::build_router(state));
    (app, registry)
}
```

**Integration test cases**:

| Test                              | Setup                                       | Action                                   | Assert                                             |
|-----------------------------------|---------------------------------------------|------------------------------------------|----------------------------------------------------|
| `/v1/models` empty                | No routes seeded                            | `GET /v1/models`                         | 200, `data: []`                                    |
| `/v1/models` with routes          | Seed 2 routes                               | `GET /v1/models`                         | 200, `data` has 2 entries, correct `id` values     |
| Chat completion happy path        | Mock upstream (200), seed account+pool+route| `POST /v1/chat/completions`              | 200, body matches mock response                    |
| Chat completion unknown model     | No routes                                   | `POST /v1/chat/completions`              | 503, `routing_error`                               |
| Chat completion upstream error    | Mock upstream (500)                         | `POST /v1/chat/completions`              | 502, `upstream_error`                              |
| Chat completion stream rejected   | N/A                                         | `POST` with `"stream": true`             | 400, `streaming_not_supported`                     |
| Chat completion invalid body      | N/A                                         | `POST` with `"not json"`                 | 400, `invalid_request_error`                       |
| Chat completion no api_key        | Seed account without api_key                | `POST /v1/chat/completions`              | 502, `missing_api_key`                             |
| Health feedback on success        | Mock upstream (200)                         | `POST`, then check `health.get()`        | Status is `Healthy`                                |
| Health feedback on failure        | Mock upstream (500)                         | `POST`, then check `health.get()`        | Status is `Unhealthy`, cooldown set                |

### Test Helpers Needed

1. **`mock_upstream(status, body)`** — Spawns a temporary axum server returning canned responses.
2. **`test_app()`** — Creates a fully wired `Router` with in-memory registry.
3. **`seed_account_pool_route(registry, mock_base_url)`** — Seeds an account (with `api_key` and
   `api_base_override` pointing to the mock), a pool, and a route for a test model name.
4. **`send_chat_request(app, model, messages)`** — Constructs and sends a `POST /v1/chat/completions`
   request via `tower::ServiceExt::oneshot`.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/rook/Cargo.toml` | Modify | Add `reqwest` and `bytes` dependencies |
| `clients/rook/migrations/0003_account_api_key.sql` | Create | `ALTER TABLE provider_accounts ADD COLUMN api_key TEXT` |
| `clients/rook/src/db/mod.rs` | Modify | Add migration 0003 constant and apply block |
| `clients/rook/src/db/account.rs` | Modify | Add `api_key` to INSERT/SELECT/row_to_account |
| `clients/rook/src/domain/mod.rs` | Modify | Add `api_key: Option<String>` to `ProviderAccount`; update test fixtures |
| `clients/rook/src/gateway/mod.rs` | Modify | Replace stub with module exports + `GatewayState` + `build_router()` |
| `clients/rook/src/gateway/types.rs` | Create | OpenAI-compatible serde types |
| `clients/rook/src/gateway/vendor.rs` | Create | Vendor base URL mapping + auth header construction |
| `clients/rook/src/gateway/upstream.rs` | Create | HTTP proxy logic |
| `clients/rook/src/gateway/handlers.rs` | Create | `handle_chat_completions` + `handle_list_models` |
| `clients/rook/src/server/mod.rs` | Modify | Wire gateway router + create registry/engine/client |
| `clients/rook/src/services/account.rs` | Modify | Update test fixtures to include `api_key: None` |
| `clients/rook/src/routing/mod.rs` | Modify | Update test fixtures to include `api_key: None` |

## Migration / Rollout

No feature flags or phased rollout needed. All changes are additive:

1. Migration 0003 adds a nullable column — no data transformation.
2. New gateway files are self-contained.
3. Server wiring adds a `nest("/v1", ...)` call — existing routes unchanged.
4. Rollback: revert merge commit, drop `api_key` column (or leave it — nullable columns with no
   readers are harmless).

## Open Questions

- [x] ~~Should `api_key` be encrypted at rest?~~ No — explicitly deferred to #591 per proposal.
- [x] ~~Should we support Anthropic's native `/v1/messages` format?~~ No — deferred per proposal.
      Anthropic works via OpenAI-compatible proxy or `api_base_override`.
- [ ] Should the `reqwest::Client` timeout (30s) be configurable via `RookSettings`? Low priority
      for M1 — hardcoded 30s is reasonable. Can be made configurable as a fast follow-up.
- [ ] Should the `/v1/chat/completions` handler log the `model` and `account_id` at `info` level
      for observability? Not blocking but useful. Recommendation: add a `tracing::info!` span
      with `model` and `account_id` fields.
