# Gateway and Admin API Specification

**Change**: rook-589-gateway-api
**Issue**: #589
**Parent**: #576 (OpenAI-compatible gateway and admin API for Corvus Rook)
**Phase**: M1 (MVP)
**Domain**: gateway

---

## Purpose

Define the contract for Rook's HTTP control plane in M1: the OpenAI-compatible gateway surface
under `/v1/*` and the operator-facing admin surface under `/api/*`.

The gateway portion covers `POST /v1/chat/completions` and `GET /v1/models`, including
OpenAI-shaped transport types, routing via `RoutingEngine`, upstream proxy behavior, vendor auth
header construction, and health feedback after upstream calls.

The admin portion covers CRUD management for accounts, pools, pool membership, routes, health,
settings, and the placeholder usage endpoint, including redacted response views and coexistence
with the dashboard routes.

---

## Requirements

### R1: `api_key` Field on `ProviderAccount`

The `ProviderAccount` domain struct MUST include an `api_key` field of type `Option<String>`.

The field MUST be nullable — existing accounts without a configured key MUST have
`api_key = None`.

A database migration (`0003_account_api_key.sql`) MUST add an `api_key TEXT` column to the
`provider_accounts` table with `DEFAULT NULL`.

The `db/account.rs` layer MUST include `api_key` in INSERT, SELECT, and row-mapping logic so
the field round-trips through the database.

The in-memory `AccountService` implementation MUST also carry the `api_key` field.

> **Security note (M1 scope)**: The `api_key` is stored as plaintext in SQLite for M1.
> Encryption-at-rest is deferred to #591.

#### Scenario: api_key persists and round-trips through SQLite

- GIVEN a `ProviderAccount` with `api_key = Some("sk-test-123")`
- WHEN the account is inserted via `insert_account` and retrieved via `get_account`
- THEN the retrieved account's `api_key` MUST equal `Some("sk-test-123")`

#### Scenario: api_key defaults to None for existing accounts

- GIVEN an existing database with accounts created before migration `0003`
- WHEN migration `0003` is applied
- THEN all existing accounts MUST have `api_key = NULL`
- AND retrieving them MUST yield `api_key = None`

#### Scenario: Account created without api_key

- GIVEN a `ProviderAccount` with `api_key = None`
- WHEN the account is inserted and retrieved
- THEN the retrieved account's `api_key` MUST be `None`

---

### R2: OpenAI-Compatible Request Types

The system MUST define a `ChatCompletionRequest` type that deserializes from the OpenAI
`POST /v1/chat/completions` request body JSON shape.

Required fields:

| Field      | Type                       | Required | Description                        |
|------------|----------------------------|----------|------------------------------------|
| `model`    | `String`                   | MUST     | Logical model name for routing     |
| `messages` | `Vec<ChatCompletionMessage>` | MUST   | Conversation history               |

Optional fields (MAY be present, MUST be preserved during proxying):

| Field               | Type             | Default  | Description                              |
|---------------------|------------------|----------|------------------------------------------|
| `temperature`       | `Option<f64>`    | `None`   | Sampling temperature (0.0–2.0)           |
| `top_p`             | `Option<f64>`    | `None`   | Nucleus sampling threshold               |
| `n`                 | `Option<u32>`    | `None`   | Number of completions to generate        |
| `stream`            | `Option<bool>`   | `None`   | Whether to stream (MUST be rejected, R11)|
| `stop`              | `Option<Stop>`   | `None`   | Stop sequence(s) — string or array       |
| `max_tokens`        | `Option<u32>`    | `None`   | Maximum tokens to generate               |
| `presence_penalty`  | `Option<f64>`    | `None`   | Presence penalty (-2.0–2.0)              |
| `frequency_penalty` | `Option<f64>`    | `None`   | Frequency penalty (-2.0–2.0)             |
| `user`              | `Option<String>` | `None`   | End-user identifier                      |

The `Stop` type MUST accept either a single string or an array of strings (up to 4), matching
the OpenAI spec's polymorphic `stop` field.

The `ChatCompletionMessage` type MUST have:

| Field     | Type     | Required | Description                                      |
|-----------|----------|----------|--------------------------------------------------|
| `role`    | `String` | MUST     | One of `"system"`, `"user"`, `"assistant"`, `"tool"` |
| `content` | `Option<String>` | SHOULD | Message text content (may be null for assistant) |

Additional fields on `ChatCompletionMessage` (e.g., `name`, `tool_calls`, `tool_call_id`)
MAY be present and MUST be preserved as raw JSON during proxying via `#[serde(flatten)]`
or equivalent.

All types MUST derive `Serialize` and `Deserialize` with `#[serde(rename_all = "snake_case")]`
where appropriate. Unknown fields MUST NOT cause deserialization failure — the gateway acts
as a proxy and MUST forward unrecognized fields transparently.

#### Scenario: Minimal valid request deserializes

- GIVEN the JSON `{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello"}]}`
- WHEN deserialized as `ChatCompletionRequest`
- THEN `model` MUST equal `"gpt-4o"`
- AND `messages` MUST have length 1
- AND `messages[0].role` MUST equal `"user"`
- AND `messages[0].content` MUST equal `Some("Hello")`
- AND all optional fields MUST be `None`

#### Scenario: Request with all optional fields deserializes

- GIVEN a JSON body with `model`, `messages`, `temperature`, `top_p`, `n`, `max_tokens`,
  `stop`, `presence_penalty`, `frequency_penalty`, `user`, and `stream` fields
- WHEN deserialized as `ChatCompletionRequest`
- THEN all fields MUST be populated with the provided values

#### Scenario: Request with unknown extra fields deserializes

- GIVEN a JSON body with `model`, `messages`, and an unknown field `"logprobs": true`
- WHEN deserialized as `ChatCompletionRequest`
- THEN deserialization MUST NOT fail
- AND the unknown field MUST be preserved for upstream forwarding

#### Scenario: Stop field accepts single string

- GIVEN a JSON body with `"stop": "\n"`
- WHEN deserialized
- THEN the `stop` field MUST represent a single stop string `"\n"`

#### Scenario: Stop field accepts array of strings

- GIVEN a JSON body with `"stop": ["\n", "END"]`
- WHEN deserialized
- THEN the `stop` field MUST represent two stop strings `["\n", "END"]`

---

### R3: OpenAI-Compatible Response Types

The system MUST define a `ChatCompletionResponse` type that matches the OpenAI response shape.

`ChatCompletionResponse` fields:

| Field                  | Type                          | Required | Description                       |
|------------------------|-------------------------------|----------|-----------------------------------|
| `id`                   | `String`                      | MUST     | Unique completion identifier      |
| `object`               | `String`                      | MUST     | Always `"chat.completion"`        |
| `created`              | `u64`                         | MUST     | Unix timestamp                    |
| `model`                | `String`                      | MUST     | Model that generated the response |
| `choices`              | `Vec<ChatCompletionChoice>`   | MUST     | List of generated completions     |
| `usage`                | `Option<Usage>`               | SHOULD   | Token usage statistics            |
| `system_fingerprint`   | `Option<String>`              | MAY      | System fingerprint                |

`ChatCompletionChoice` fields:

| Field           | Type                     | Required | Description                        |
|-----------------|--------------------------|----------|------------------------------------|
| `index`         | `u32`                    | MUST     | Choice index                       |
| `message`       | `ChatCompletionMessage`  | MUST     | The generated message              |
| `finish_reason` | `Option<String>`         | MUST     | `"stop"`, `"length"`, `"tool_calls"`, etc. |

`Usage` fields:

| Field               | Type  | Required | Description                 |
|----------------------|------|----------|-----------------------------|
| `prompt_tokens`      | `u32`| MUST     | Tokens in the prompt        |
| `completion_tokens`  | `u32`| MUST     | Tokens in the completion    |
| `total_tokens`       | `u32`| MUST     | Sum of prompt + completion  |

All response types MUST derive `Serialize` and `Deserialize`. Unknown fields from upstream
MUST NOT cause deserialization failure — the gateway returns the upstream response body
verbatim when possible (see R8).

#### Scenario: Response type serde round-trip

- GIVEN a `ChatCompletionResponse` with `id = "chatcmpl-abc"`, `object = "chat.completion"`,
  `created = 1700000000`, `model = "gpt-4o"`, one choice with `finish_reason = "stop"`,
  and usage `{prompt_tokens: 10, completion_tokens: 20, total_tokens: 30}`
- WHEN serialized to JSON and deserialized back
- THEN all fields MUST match the original values

---

### R4: `ModelObject` and `ModelListResponse` Types

The system MUST define a `ModelObject` type matching the OpenAI model object shape:

| Field      | Type     | Required | Description                                    |
|------------|----------|----------|------------------------------------------------|
| `id`       | `String` | MUST     | The logical model name (from `ModelRoute`)     |
| `object`   | `String` | MUST     | Always `"model"`                               |
| `created`  | `u64`    | MUST     | Unix timestamp (MAY use a fixed epoch or now)  |
| `owned_by` | `String` | MUST     | Owner identifier (MAY be `"rook"` or `"system"`)|

The system MUST define a `ModelListResponse` type:

| Field    | Type               | Required | Description              |
|----------|--------------------|----------|--------------------------|
| `object` | `String`           | MUST     | Always `"list"`          |
| `data`   | `Vec<ModelObject>`  | MUST     | List of available models |

#### Scenario: ModelObject serializes to expected shape

- GIVEN a `ModelObject` with `id = "gpt-4o"`, `object = "model"`, `created = 1700000000`,
  `owned_by = "rook"`
- WHEN serialized to JSON
- THEN the output MUST match `{"id":"gpt-4o","object":"model","created":1700000000,"owned_by":"rook"}`

#### Scenario: ModelListResponse with empty data

- GIVEN a `ModelListResponse` with `object = "list"` and `data = []`
- WHEN serialized to JSON
- THEN the output MUST match `{"object":"list","data":[]}`

---

### R5: `GatewayErrorResponse` Type

The system MUST define a `GatewayErrorResponse` type for all gateway error responses:

```json
{
  "error": {
    "message": "human-readable error description",
    "type": "error_category",
    "code": "machine_readable_error_code"
  }
}
```

Fields within the nested `error` object:

| Field     | Type             | Required | Description                                  |
|-----------|------------------|----------|----------------------------------------------|
| `message` | `String`         | MUST     | Human-readable error description             |
| `type`    | `String`         | MUST     | Error category (e.g., `"server_error"`, `"invalid_request_error"`) |
| `code`    | `Option<String>` | MAY      | Machine-readable code (e.g., `"model_not_found"`) |

The structure MUST match the OpenAI API error response shape so that clients expecting
OpenAI errors can parse them.

#### Scenario: Error response shape

- GIVEN a routing failure for model `"unknown-model"`
- WHEN the gateway returns a `GatewayErrorResponse`
- THEN the response body MUST have a top-level `"error"` key
- AND `error.message` MUST contain the model name or a descriptive message
- AND `error.type` MUST be a non-empty string

---

### R6: Vendor Base URL Mapping

The system MUST provide a function that maps `ProviderVendor` to a default API base URL.

Required mappings:

| Vendor         | Default Base URL                    |
|----------------|-------------------------------------|
| `OpenAi`       | `https://api.openai.com`            |
| `Anthropic`    | `https://api.anthropic.com`         |
| `Google`       | `https://generativelanguage.googleapis.com` |
| `OpenRouter`   | `https://openrouter.ai/api`         |
| `DeepSeek`     | `https://api.deepseek.com`          |
| `Other(_)`     | MUST return `None` (no default)     |

When a `ProviderAccount` has `api_base_override = Some(url)`, the override MUST take
precedence over the vendor default.

When the effective base URL is `None` (vendor is `Other` with no override), the gateway
MUST NOT attempt to proxy to that account. This condition SHOULD be treated as if the
account has no valid endpoint.

#### Scenario: OpenAI vendor resolves to default base URL

- GIVEN a `ProviderAccount` with `vendor = OpenAi` and `api_base_override = None`
- WHEN the effective base URL is resolved
- THEN it MUST be `"https://api.openai.com"`

#### Scenario: Override takes precedence over vendor default

- GIVEN a `ProviderAccount` with `vendor = OpenAi` and
  `api_base_override = Some("https://my-proxy.example.com")`
- WHEN the effective base URL is resolved
- THEN it MUST be `"https://my-proxy.example.com"`

#### Scenario: Unknown vendor without override has no base URL

- GIVEN a `ProviderAccount` with `vendor = Other("mistral")` and `api_base_override = None`
- WHEN the effective base URL is resolved
- THEN it MUST be `None`

#### Scenario: Unknown vendor with override uses override

- GIVEN a `ProviderAccount` with `vendor = Other("mistral")` and
  `api_base_override = Some("https://api.mistral.ai")`
- WHEN the effective base URL is resolved
- THEN it MUST be `"https://api.mistral.ai"`

---

### R7: Vendor Auth Header Construction

The system MUST construct vendor-appropriate authentication headers for upstream requests.

| Vendor           | Header                          |
|------------------|---------------------------------|
| `OpenAi`         | `Authorization: Bearer {key}`   |
| `DeepSeek`       | `Authorization: Bearer {key}`   |
| `OpenRouter`     | `Authorization: Bearer {key}`   |
| `Google`         | `Authorization: Bearer {key}`   |
| `Anthropic`      | `x-api-key: {key}`              |
| `Other(_)`       | `Authorization: Bearer {key}`   |

The `{key}` MUST be the account's `api_key` value. If the account has `api_key = None`, the
gateway MUST NOT include an authentication header and SHOULD log a warning.

#### Scenario: OpenAI-compatible vendor sets Bearer auth

- GIVEN a `ProviderAccount` with `vendor = OpenAi` and `api_key = Some("sk-abc")`
- WHEN constructing the upstream request
- THEN the request MUST include header `Authorization: Bearer sk-abc`

#### Scenario: Anthropic vendor sets x-api-key header

- GIVEN a `ProviderAccount` with `vendor = Anthropic` and `api_key = Some("sk-ant-123")`
- WHEN constructing the upstream request
- THEN the request MUST include header `x-api-key: sk-ant-123`

#### Scenario: Account with no api_key omits auth header

- GIVEN a `ProviderAccount` with `api_key = None`
- WHEN constructing the upstream request
- THEN no authentication header MUST be set
- AND a warning MUST be logged indicating the account has no API key

---

### R8: `POST /v1/chat/completions` Endpoint Behavior

The gateway MUST expose a `POST /v1/chat/completions` endpoint that:

1. Accepts a JSON request body conforming to `ChatCompletionRequest` (R2).
2. Extracts the `model` field for routing.
3. Rejects requests where `stream` is `Some(true)` with HTTP 400 (R11).
4. Calls `RoutingEngine::resolve(model)` to obtain a `RoutingDecision`.
5. Constructs the upstream URL as `{effective_base_url}/v1/chat/completions`.
6. Sets vendor-appropriate auth headers (R7).
7. Sets `Content-Type: application/json` on the upstream request.
8. Forwards the **original raw JSON request body** to the upstream provider.
9. Returns the upstream response body and status to the client.
10. After the upstream call, reports health feedback (R10).

**Response mapping**:

| Upstream status | Gateway HTTP status | Body                                |
|-----------------|---------------------|-------------------------------------|
| 2xx             | Same as upstream     | Upstream response body (verbatim)   |
| 4xx             | 502 Bad Gateway      | `GatewayErrorResponse` with upstream details |
| 5xx             | 502 Bad Gateway      | `GatewayErrorResponse` with upstream details |
| Connection error| 502 Bad Gateway      | `GatewayErrorResponse` describing connection failure |
| Timeout         | 504 Gateway Timeout  | `GatewayErrorResponse` describing timeout |

**Error cases before upstream call**:

| Condition                      | HTTP Status              | Error Type                   |
|--------------------------------|--------------------------|------------------------------|
| Missing or invalid JSON body   | 400 Bad Request          | `invalid_request_error`      |
| Missing `model` field          | 400 Bad Request          | `invalid_request_error`      |
| Missing `messages` field       | 400 Bad Request          | `invalid_request_error`      |
| `stream: true`                 | 400 Bad Request          | `invalid_request_error`      |
| No route for model             | 503 Service Unavailable  | `server_error`               |
| All accounts exhausted         | 503 Service Unavailable  | `server_error`               |
| Account has no base URL        | 502 Bad Gateway          | `server_error`               |

The handler MUST return `Content-Type: application/json` for all responses.

The handler MUST log at `INFO` level: the requested model, the resolved account ID, and the
upstream status code.

#### Scenario: Happy path — successful chat completion

- GIVEN a configured route for model `"gpt-4o"` pointing to a pool with a healthy account
  that has `api_key = Some("sk-test")` and `vendor = OpenAi`
- AND the upstream provider returns HTTP 200 with a valid `ChatCompletionResponse` body
- WHEN the client sends `POST /v1/chat/completions` with
  `{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello"}]}`
- THEN the gateway MUST return HTTP 200
- AND the response body MUST be the upstream provider's response (verbatim JSON)
- AND `Content-Type` MUST be `application/json`
- AND the health service MUST have `mark_success` called for the account

#### Scenario: Unknown model returns 503

- GIVEN no route is configured for model `"nonexistent-model"`
- WHEN the client sends `POST /v1/chat/completions` with `{"model": "nonexistent-model",
  "messages": [{"role": "user", "content": "test"}]}`
- THEN the gateway MUST return HTTP 503
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.type` MUST be `"server_error"`
- AND `error.message` MUST contain `"nonexistent-model"` or describe the routing failure

#### Scenario: Upstream returns error → 502

- GIVEN a configured route for model `"gpt-4o"` with a healthy account
- AND the upstream provider returns HTTP 500 with body
  `{"error": {"message": "Internal server error"}}`
- WHEN the client sends a valid chat completion request
- THEN the gateway MUST return HTTP 502
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.type` MUST be `"server_error"`
- AND the health service MUST have `mark_failure` called for the account

#### Scenario: Upstream timeout → 504

- GIVEN a configured route for model `"gpt-4o"` with a healthy account
- AND the upstream provider does not respond within the client timeout
- WHEN the client sends a valid chat completion request
- THEN the gateway MUST return HTTP 504
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.message` MUST describe a timeout
- AND the health service MUST have `mark_failure` called for the account

#### Scenario: Empty request body → 400

- GIVEN any gateway configuration
- WHEN the client sends `POST /v1/chat/completions` with an empty body
- THEN the gateway MUST return HTTP 400
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.type` MUST be `"invalid_request_error"`

#### Scenario: Missing model field → 400

- GIVEN any gateway configuration
- WHEN the client sends `POST /v1/chat/completions` with
  `{"messages": [{"role": "user", "content": "Hello"}]}`
- THEN the gateway MUST return HTTP 400
- AND `error.type` MUST be `"invalid_request_error"`
- AND `error.message` MUST indicate the `model` field is required

#### Scenario: Missing messages field → 400

- GIVEN any gateway configuration
- WHEN the client sends `POST /v1/chat/completions` with `{"model": "gpt-4o"}`
- THEN the gateway MUST return HTTP 400
- AND `error.type` MUST be `"invalid_request_error"`

#### Scenario: Account with no api_key is resolved

- GIVEN a configured route for model `"gpt-4o"` with a single account that has
  `api_key = None`
- WHEN the client sends a valid chat completion request for `"gpt-4o"`
- THEN the gateway MUST forward the request to the upstream without an auth header
- AND a warning MUST be logged indicating the account has no API key
- AND the upstream response (or upstream auth error) MUST be returned to the client

#### Scenario: Upstream request preserves original body

- GIVEN a valid request with extra vendor-specific fields (e.g., `"logprobs": true`)
- WHEN the gateway forwards to the upstream
- THEN the upstream request body MUST contain all original fields, including unknown ones

---

### R9: `GET /v1/models` Endpoint Behavior

The gateway MUST expose a `GET /v1/models` endpoint that:

1. Queries all `ModelRoute`s from the registry via `RouteService::list()`.
2. Maps each route to a `ModelObject` (R4), using `logical_model` as the `id`.
3. Returns a `ModelListResponse` with `object = "list"` and `data` containing all models.
4. Returns `Content-Type: application/json`.
5. MUST NOT require a request body.

#### Scenario: List models with configured routes

- GIVEN the registry contains routes for `"gpt-4o"`, `"claude-3"`, and `"deep-seek-r1"`
- WHEN the client sends `GET /v1/models`
- THEN the gateway MUST return HTTP 200
- AND the response body MUST be a `ModelListResponse`
- AND `data` MUST contain exactly 3 `ModelObject`s
- AND each object's `id` MUST correspond to a `logical_model` from the routes
- AND each object's `object` field MUST be `"model"`

#### Scenario: List models with no routes → empty list

- GIVEN the registry contains no routes
- WHEN the client sends `GET /v1/models`
- THEN the gateway MUST return HTTP 200
- AND the response body MUST be `{"object": "list", "data": []}`

#### Scenario: Model list reflects registry state

- GIVEN the registry initially contains a route for `"gpt-4o"`
- AND a new route for `"claude-3"` is added to the registry
- WHEN the client sends `GET /v1/models`
- THEN the response MUST contain both `"gpt-4o"` and `"claude-3"`

---

### R10: Health Feedback After Upstream Calls

After every upstream proxy call, the gateway MUST update the health service:

- On upstream HTTP 2xx response: call `health.mark_success(account_id)`.
- On upstream HTTP 4xx response: call `health.mark_failure(account_id, cooldown_secs)`.
- On upstream HTTP 5xx response: call `health.mark_failure(account_id, cooldown_secs)`.
- On connection error: call `health.mark_failure(account_id, cooldown_secs)`.
- On timeout: call `health.mark_failure(account_id, cooldown_secs)`.

The `cooldown_secs` value SHOULD default to 60 seconds for M1.

Health feedback MUST happen after the response has been sent to the client (or concurrently)
and MUST NOT block the client response.

#### Scenario: Successful upstream call marks account healthy

- GIVEN an account that was previously marked unhealthy
- WHEN an upstream call to that account returns HTTP 200
- THEN `mark_success(account_id)` MUST be called
- AND the account MUST be available for subsequent routing

#### Scenario: Failed upstream call marks account unhealthy

- GIVEN a healthy account
- WHEN an upstream call to that account returns HTTP 500
- THEN `mark_failure(account_id, cooldown_secs)` MUST be called
- AND the account MUST be excluded from routing for `cooldown_secs`

---

### R11: Stream Rejection

When the `ChatCompletionRequest` contains `stream: true`, the gateway MUST reject the request
with:

- HTTP status: `400 Bad Request`
- Body: `GatewayErrorResponse` with:
  - `error.message`: `"streaming is not yet supported; set stream: false or omit the field"`
  - `error.type`: `"invalid_request_error"`
  - `error.code`: `"unsupported_stream"`

The check MUST happen before routing resolution — no upstream call or routing attempt MUST
occur.

When `stream` is `None` or `Some(false)`, the request MUST proceed normally.

#### Scenario: stream: true → 400

- GIVEN any gateway configuration
- WHEN the client sends `POST /v1/chat/completions` with `"stream": true`
- THEN the gateway MUST return HTTP 400
- AND `error.message` MUST contain `"streaming is not yet supported"`
- AND `error.code` MUST be `"unsupported_stream"`
- AND no routing resolution MUST occur
- AND no upstream call MUST be made

#### Scenario: stream: false → proceeds normally

- GIVEN a valid route configuration
- WHEN the client sends a request with `"stream": false`
- THEN the gateway MUST process the request normally (routing + upstream proxy)

#### Scenario: stream omitted → proceeds normally

- GIVEN a valid route configuration
- WHEN the client sends a request without the `"stream"` field
- THEN the gateway MUST process the request normally

---

### R12: Gateway Router Construction and Relative Routes

The system MUST provide a `gateway::build_router(state) -> axum::Router` function that:

1. Mounts `POST /chat/completions` → `handle_chat_completions` handler.
2. Mounts `GET /models` → `handle_list_models` handler.
3. Accepts shared `GatewayState` as axum `State`.

The `GatewayState` MUST contain:

| Field      | Type               | Description                              |
|------------|--------------------|------------------------------------------|
| `registry` | `RookRegistry`     | Access to accounts, pools, routes, health|
| `engine`   | `RoutingEngine`    | Request-time model → account resolution  |
| `client`   | `reqwest::Client`  | HTTP client for upstream provider calls   |

`GatewayState` MUST be `Clone` (all inner types are `Clone`).

The router MUST be mountable under `Router::nest("/v1", ...)` in the server module without conflicting
with existing routes (`/api/*`, dashboard assets).

#### Scenario: Router mounts both endpoints

- GIVEN a `GatewayState` with a valid registry, engine, and HTTP client
- WHEN `build_router(state)` is called
- THEN the returned `Router` MUST respond to `POST /chat/completions`
- AND the returned `Router` MUST respond to `GET /models`

#### Scenario: Unrelated routes are unaffected

- GIVEN the gateway router is merged into the main server router
- WHEN the client sends `GET /api/health`
- THEN the server MUST return `200 "ok"` (existing admin stub behavior)
- AND no gateway handler MUST be invoked

---

### R13: Server Wiring

The `server::run` function MUST be updated to:

1. Create a `RookRegistry` (or receive one as a parameter).
2. Create a `RoutingEngine` backed by the registry.
3. Create a `reqwest::Client` with a default timeout of 30 seconds.
4. Construct `GatewayState` from the above.
5. Call `gateway::build_router(state)` and mount it with `Router::nest("/v1", ...)` in the main router.

The gateway routes MUST be available alongside existing `/api/*` and dashboard routes.

The `reqwest::Client` MUST use `rustls-tls` (not native-tls) to avoid system OpenSSL
dependencies.

#### Scenario: Server starts with gateway routes available

- GIVEN a default `ServerConfig`
- WHEN `server::run` is called
- THEN both `/v1/chat/completions` and `/v1/models` MUST be reachable
- AND `/api/health` MUST continue to return `200 "ok"`

#### Scenario: reqwest client has 30-second timeout

- GIVEN the server is started with default configuration
- WHEN an upstream provider does not respond within 30 seconds
- THEN the gateway MUST return HTTP 504 Gateway Timeout

---

### R14: Admin Router Composition Under `/api`

The system MUST compose a dedicated admin router under the `/api` prefix in the Rook server hosted
from `clients/rook`.

The composed server MUST preserve all of the following at the same time:

- `GET /api/health`
- all newly defined `/api/*` admin routes in this spec
- existing `/v1/*` gateway routes
- dashboard routes served from `/` and `/assets/*`

The admin router MUST NOT shadow or break existing `/v1/*` behavior.

#### Scenario: router composition preserves existing health and gateway routes

- GIVEN a running Rook server with the composed application router
- WHEN a client requests `GET /api/health`
- THEN the response MUST succeed
- AND the route MUST remain mounted under `/api`
- WHEN a client requests `GET /v1/models`
- THEN the response MUST also succeed
- AND the `/v1/models` behavior MUST remain available alongside `/api/*`

#### Scenario: dashboard routes still coexist with admin routes

- GIVEN a running Rook server with dashboard assets enabled
- WHEN a client requests `GET /`
- THEN the dashboard response MUST still be served
- AND admin router composition MUST NOT replace or intercept the dashboard root route

---

### R15: Preserve `GET /api/health`

The system MUST preserve `GET /api/health` as a valid health endpoint for the admin surface.

For M1, `GET /api/health` SHALL remain a lightweight server-health endpoint and MUST NOT be
redefined to require account-level aggregation semantics.

#### Scenario: base health endpoint remains available

- GIVEN a running Rook server
- WHEN a client requests `GET /api/health`
- THEN the server MUST return a successful response
- AND the route MUST remain available even if no accounts, pools, or routes exist

---

### R16: Account Admin Endpoints

The system MUST expose the following account endpoints:

- `GET /api/accounts`
- `POST /api/accounts`
- `GET /api/accounts/{account_id}`
- `PUT /api/accounts/{account_id}`
- `DELETE /api/accounts/{account_id}`

These endpoints MUST operate on the existing Rook account service via `RookRegistry` and MUST use
the redacted `AccountView` response shape defined in this spec.

The service MUST support create, list, fetch, replace-update, and delete semantics.

#### Scenario: listing accounts returns an empty collection

- GIVEN no accounts have been created
- WHEN a client requests `GET /api/accounts`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: create account happy path

- GIVEN a valid `CreateAccountRequest`
- WHEN a client submits `POST /api/accounts`
- THEN the response status MUST be `201 Created`
- AND the response body MUST be an `AccountView`
- AND the returned `id` MUST be a server-assigned stable identifier
- AND the returned `has_api_key` MUST reflect whether the request included `api_key`

#### Scenario: get account happy path

- GIVEN an existing account id
- WHEN a client requests `GET /api/accounts/{account_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the matching `AccountView`

#### Scenario: update account happy path

- GIVEN an existing account id
- AND a valid `UpdateAccountRequest`
- WHEN a client submits `PUT /api/accounts/{account_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the updated `AccountView`

#### Scenario: delete account happy path

- GIVEN an existing account id that is not referenced by any pool
- WHEN a client submits `DELETE /api/accounts/{account_id}`
- THEN the response status MUST be `204 No Content`
- AND subsequent `GET /api/accounts/{account_id}` MUST return `404 Not Found`

#### Scenario: account fetch for unknown id

- GIVEN no account exists for a requested id
- WHEN a client requests `GET /api/accounts/{account_id}`
- THEN the response status MUST be `404 Not Found`
- AND the response body MUST match the admin error response shape

---

### R17: Account Responses MUST Redact Credentials

The system MUST accept `api_key` in create and update requests as a write-only field.

The system MUST NOT return raw `api_key` values in any admin response body.

Every account response MUST expose `has_api_key: boolean` instead of `api_key`.

This redaction rule MUST apply to list responses, get responses, create responses, update
responses, nested pool member views if any are ever returned, and any error payload metadata.

#### Scenario: account create response redacts api_key

- GIVEN a `CreateAccountRequest` with `api_key = "sk-secret"`
- WHEN the request succeeds
- THEN the response body MUST include `has_api_key: true`
- AND the response body MUST NOT include an `api_key` field
- AND the raw submitted credential MUST NOT be echoed back

#### Scenario: account update response redacts api_key

- GIVEN an existing account without a key
- WHEN a client submits `PUT /api/accounts/{account_id}` with a new `api_key`
- THEN the response body MUST include `has_api_key: true`
- AND the response body MUST NOT include an `api_key` field

#### Scenario: account list response remains redacted

- GIVEN one or more stored accounts with API keys
- WHEN a client requests `GET /api/accounts`
- THEN every item in the response MUST include `has_api_key`
- AND no item in the response MUST expose raw credential material

---

### R18: Pool Admin Endpoints

The system MUST expose the following pool endpoints:

- `GET /api/pools`
- `POST /api/pools`
- `GET /api/pools/{pool_id}`
- `PUT /api/pools/{pool_id}`
- `DELETE /api/pools/{pool_id}`

These endpoints MUST operate on the existing Rook pool service via `RookRegistry`.

Pool responses MUST use the `PoolView` contract defined in this spec.

#### Scenario: listing pools returns an empty collection

- GIVEN no pools have been created
- WHEN a client requests `GET /api/pools`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: create pool happy path

- GIVEN a valid `CreatePoolRequest`
- WHEN a client submits `POST /api/pools`
- THEN the response status MUST be `201 Created`
- AND the response body MUST be a `PoolView`
- AND the new pool MUST initially contain the members specified by the request, if any

#### Scenario: update pool happy path

- GIVEN an existing pool id
- WHEN a client submits `PUT /api/pools/{pool_id}` with updated metadata
- THEN the response status MUST be `200 OK`
- AND the response body MUST reflect the updated pool values

#### Scenario: delete pool happy path

- GIVEN an existing pool id that is not referenced by any route and is not referenced as another
  pool's fallback pool
- WHEN a client submits `DELETE /api/pools/{pool_id}`
- THEN the response status MUST be `204 No Content`

#### Scenario: delete referenced pool fails

- GIVEN a pool is referenced by at least one route or fallback pool reference
- WHEN a client submits `DELETE /api/pools/{pool_id}`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape
- AND the error code MUST identify the resource as still referenced

---

### R19: Pool Membership Endpoints

The system MUST expose pool membership mutation endpoints:

- `POST /api/pools/{pool_id}/accounts`
- `DELETE /api/pools/{pool_id}/accounts/{account_id}`

`POST /api/pools/{pool_id}/accounts` MUST accept `AddPoolMemberRequest`.

Adding an account to a pool MUST be idempotent: if the account is already a member, the operation
MUST still succeed without creating a duplicate membership.

Removing a member MUST fail when the account is not currently a member of the pool.

Adding a member MUST fail when the account does not exist.

#### Scenario: add member happy path

- GIVEN an existing pool and an existing account that is not yet a member
- WHEN a client submits `POST /api/pools/{pool_id}/accounts`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the updated `PoolView`
- AND the `members` list MUST now include that account id exactly once

#### Scenario: add member is idempotent

- GIVEN an existing pool that already contains the requested account id
- WHEN a client submits `POST /api/pools/{pool_id}/accounts` for the same account again
- THEN the response status MUST be `200 OK`
- AND the response body MUST contain the account id exactly once in `members`

#### Scenario: remove member happy path

- GIVEN an existing pool that contains an account id
- WHEN a client submits `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the updated `PoolView`
- AND the removed account id MUST no longer appear in `members`

#### Scenario: remove non-member fails

- GIVEN an existing pool that does not contain the requested account id
- WHEN a client submits `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape

#### Scenario: add nonexistent account to pool fails

- GIVEN an existing pool
- AND no account exists for the requested account id
- WHEN a client submits `POST /api/pools/{pool_id}/accounts`
- THEN the response status MUST be `404 Not Found`
- AND the response body MUST match the admin error response shape

---

### R20: Route Admin Endpoints

The system MUST expose the following route endpoints:

- `GET /api/routes`
- `POST /api/routes`
- `GET /api/routes/{route_id}`
- `PUT /api/routes/{route_id}`
- `DELETE /api/routes/{route_id}`

These endpoints MUST operate on the existing Rook route service via `RookRegistry`.

Route responses MUST use the `RouteView` contract defined in this spec.

#### Scenario: listing routes returns an empty collection

- GIVEN no routes have been created
- WHEN a client requests `GET /api/routes`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: create route happy path

- GIVEN a valid `CreateRouteRequest` referencing an existing pool
- WHEN a client submits `POST /api/routes`
- THEN the response status MUST be `201 Created`
- AND the response body MUST be a `RouteView`

#### Scenario: update route happy path

- GIVEN an existing route id
- AND a valid `UpdateRouteRequest`
- WHEN a client submits `PUT /api/routes/{route_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST reflect the updated route

#### Scenario: delete route happy path

- GIVEN an existing route id that is not referenced as another route's fallback route
- WHEN a client submits `DELETE /api/routes/{route_id}`
- THEN the response status MUST be `204 No Content`

#### Scenario: delete referenced route fails

- GIVEN a route is referenced by another route's `fallback_route_id`
- WHEN a client submits `DELETE /api/routes/{route_id}`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape
- AND the error code MUST identify the route as still referenced

#### Scenario: create route with duplicate logical model fails

- GIVEN a route already exists for a logical model name
- WHEN a client submits `POST /api/routes` with the same `logical_model`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape

---

### R21: Referenced Resource Deletion Safeguards

The system MUST fail closed when a delete operation would violate current resource references.

At minimum, the following failure behavior MUST be defined:

- deleting an account referenced by a pool membership MUST fail
- deleting a pool referenced by a route target MUST fail
- deleting a pool referenced by another pool's `fallback_pool_id` MUST fail
- deleting a route referenced by another route's `fallback_route_id` MUST fail

These failures MUST return `409 Conflict` and MUST use the standard admin error response shape.

#### Scenario: delete account referenced by pool fails

- GIVEN an account is a member of at least one pool
- WHEN a client submits `DELETE /api/accounts/{account_id}`
- THEN the response status MUST be `409 Conflict`
- AND the account MUST remain unchanged

#### Scenario: delete pool referenced by route fails

- GIVEN a route targets a pool
- WHEN a client submits `DELETE /api/pools/{pool_id}`
- THEN the response status MUST be `409 Conflict`
- AND the pool MUST remain unchanged

#### Scenario: delete fallback route target fails

- GIVEN route B references route A as `fallback_route_id`
- WHEN a client submits `DELETE /api/routes/{route_a_id}`
- THEN the response status MUST be `409 Conflict`
- AND route A MUST remain unchanged

---

### R22: Health Account List Endpoint

The system MUST expose `GET /api/health/accounts`.

The response MUST be a JSON array of `HealthAccountView` records representing runtime health state
for known accounts.

For M1, health data is runtime-scoped and in-memory only. It MUST reflect current process state and
MUST NOT imply durable historical health storage.

When an account exists but has never been probed, its health status MUST be `"unknown"`.

#### Scenario: health account list returns empty collection when no accounts exist

- GIVEN no accounts exist
- WHEN a client requests `GET /api/health/accounts`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: health account list reports unknown state for unprobed account

- GIVEN an existing account with no health probes recorded in the current runtime
- WHEN a client requests `GET /api/health/accounts`
- THEN the corresponding item MUST include `status: "unknown"`
- AND `last_checked` MUST be `null`

#### Scenario: health account list reports healthy and unhealthy states

- GIVEN one account has been marked healthy in runtime state
- AND another account has been marked unhealthy in runtime state
- WHEN a client requests `GET /api/health/accounts`
- THEN the response MUST include one item with `status: "healthy"`
- AND one item with `status: "unhealthy"`

---

### R23: Health Summary Endpoint

The system MUST expose `GET /api/health/summary`.

The response MUST be a `HealthSummaryView` object summarizing known account health state for the
current runtime.

The summary MUST include counts for `healthy`, `degraded`, `unhealthy`, `unknown`, and `total`.

#### Scenario: health summary for empty system

- GIVEN no accounts exist
- WHEN a client requests `GET /api/health/summary`
- THEN the response status MUST be `200 OK`
- AND the body MUST report `total: 0`
- AND all status counters MUST be `0`

#### Scenario: health summary counts unknown and healthy states

- GIVEN one existing account has never been probed
- AND one existing account is healthy
- WHEN a client requests `GET /api/health/summary`
- THEN the body MUST report `unknown: 1`
- AND `healthy: 1`
- AND `total: 2`

#### Scenario: health summary counts unhealthy states

- GIVEN one existing account is unhealthy in runtime state
- WHEN a client requests `GET /api/health/summary`
- THEN the body MUST report `unhealthy: 1`
- AND `total` MUST include that account

---

### R24: Settings Endpoints and MVP Update Semantics

The system MUST expose:

- `GET /api/settings`
- `PUT /api/settings`

The system MUST NOT require `PATCH /api/settings` for M1.

For M1, full replace-update semantics via `PUT /api/settings` are sufficient and SHALL be the only
documented write contract. If `PATCH /api/settings` is not implemented, requests to that route MUST
return `404 Not Found` or `405 Method Not Allowed`; it MUST NOT be part of the supported MVP
contract.

The settings endpoints MUST operate on the existing Rook settings service via `RookRegistry`.

`GET /api/settings` MUST return persisted settings if present, otherwise defaults derived from the
current settings service.

#### Scenario: settings read returns defaults before any save

- GIVEN no settings have been persisted yet
- WHEN a client requests `GET /api/settings`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal the service default settings values

#### Scenario: settings update persists replacement values

- GIVEN the server is running
- WHEN a client submits `PUT /api/settings` with a valid `UpdateSettingsRequest`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the persisted `SettingsView`
- AND a subsequent `GET /api/settings` MUST return the same values

#### Scenario: patch settings is not part of MVP

- GIVEN the M1 admin API contract
- WHEN a client submits `PATCH /api/settings`
- THEN the route MUST NOT be treated as a supported MVP requirement
- AND clients MUST rely on `PUT /api/settings` for updates

---

### R25: Usage Placeholder Endpoint

The system MUST expose `GET /api/usage`.

Because no real usage or cost-accounting backend exists in M1, this endpoint MUST return a stable
placeholder response using `UsageStatusView` with `available: false`.

The endpoint MUST NOT invent fake usage totals or provider billing details.

#### Scenario: usage placeholder response

- GIVEN the M1 runtime with no backing usage subsystem
- WHEN a client requests `GET /api/usage`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal the documented placeholder contract
- AND `available` MUST be `false`

---

### R26: Admin Error Response Contract

All non-success admin API responses defined by this spec MUST use a consistent JSON error shape.

The shape MUST distinguish at least:

- not found failures
- validation failures
- conflict/reference failures
- internal server failures

#### Scenario: not found uses admin error response

- GIVEN a request targets a nonexistent admin resource
- WHEN the API returns an error
- THEN the response body MUST match the admin error response shape
- AND the HTTP status MUST be `404 Not Found`

#### Scenario: conflict uses admin error response

- GIVEN a delete operation fails because the resource is still referenced
- WHEN the API returns an error
- THEN the response body MUST match the admin error response shape
- AND the HTTP status MUST be `409 Conflict`

---

### R27: Loopback-First and No-Auth M1 Safety Posture

This change MUST preserve the current M1 safety posture.

The admin API defined here MUST NOT expand exposure beyond the existing loopback/local-admin
assumption.

Authentication and authorization are explicitly out of scope for this spec and belong to #591.

The admin API contract MUST therefore be specified without bearer-token, pairing, or role-based
authorization requirements.

#### Scenario: admin API remains unauthenticated in M1 contract

- GIVEN the M1 admin API defined by this spec
- WHEN a client interacts with `/api/*`
- THEN the contract MUST NOT require auth features from #591
- AND the spec MUST continue to describe this surface as local-admin only

---

## Data Contracts

### ChatCompletionRequest (Input)

```json
{
  "model": "gpt-4o",                              // REQUIRED: string
  "messages": [                                    // REQUIRED: array
    {
      "role": "system",                            // REQUIRED: string
      "content": "You are a helpful assistant."    // OPTIONAL: string | null
    },
    {
      "role": "user",                              // REQUIRED: string
      "content": "Hello, how are you?"             // OPTIONAL: string | null
    }
  ],
  "temperature": 0.7,                             // OPTIONAL: number (0.0–2.0)
  "top_p": 1.0,                                   // OPTIONAL: number (0.0–1.0)
  "n": 1,                                         // OPTIONAL: integer ≥ 1
  "stream": false,                                // OPTIONAL: boolean (true → 400)
  "stop": ["\n"],                                  // OPTIONAL: string | string[]
  "max_tokens": 1000,                             // OPTIONAL: integer ≥ 1
  "presence_penalty": 0.0,                        // OPTIONAL: number (-2.0–2.0)
  "frequency_penalty": 0.0,                       // OPTIONAL: number (-2.0–2.0)
  "user": "user-123"                              // OPTIONAL: string
}
```

### ChatCompletionResponse (Output)

```json
{
  "id": "chatcmpl-abc123",                        // REQUIRED: string
  "object": "chat.completion",                    // REQUIRED: always "chat.completion"
  "created": 1700000000,                          // REQUIRED: unix timestamp
  "model": "gpt-4o",                              // REQUIRED: string
  "choices": [                                    // REQUIRED: array
    {
      "index": 0,                                 // REQUIRED: integer
      "message": {                                // REQUIRED: ChatCompletionMessage
        "role": "assistant",                      // REQUIRED: string
        "content": "Hello! How can I help?"       // OPTIONAL: string | null
      },
      "finish_reason": "stop"                     // REQUIRED: string | null
    }
  ],
  "usage": {                                      // OPTIONAL: object
    "prompt_tokens": 10,                          // REQUIRED within usage: integer
    "completion_tokens": 20,                      // REQUIRED within usage: integer
    "total_tokens": 30                            // REQUIRED within usage: integer
  },
  "system_fingerprint": "fp_abc123"               // OPTIONAL: string | null
}
```

### ChatCompletionMessage

```json
{
  "role": "user",                                 // REQUIRED: "system" | "user" | "assistant" | "tool"
  "content": "Hello"                              // OPTIONAL: string | null
  // Additional fields (name, tool_calls, etc.) MAY be present and are preserved
}
```

### ModelObject

```json
{
  "id": "gpt-4o",                                 // REQUIRED: logical model name
  "object": "model",                              // REQUIRED: always "model"
  "created": 1700000000,                          // REQUIRED: unix timestamp
  "owned_by": "rook"                              // REQUIRED: owner string
}
```

### ModelListResponse

```json
{
  "object": "list",                               // REQUIRED: always "list"
  "data": [                                       // REQUIRED: array of ModelObject
    {"id": "gpt-4o", "object": "model", "created": 1700000000, "owned_by": "rook"},
    {"id": "claude-3", "object": "model", "created": 1700000000, "owned_by": "rook"}
  ]
}
```

### GatewayErrorResponse

```json
{
  "error": {                                      // REQUIRED: object
    "message": "no route configured for model 'xyz'",  // REQUIRED: string
    "type": "server_error",                        // REQUIRED: string
    "code": "model_not_found"                      // OPTIONAL: string | null
  }
}
```

**Error type values used by this spec**:

| `error.type`              | Used when                                           |
|---------------------------|-----------------------------------------------------|
| `invalid_request_error`   | Malformed body, missing fields, stream: true         |
| `server_error`            | Routing failure, upstream error, connection error    |

**Error code values used by this spec** (all optional):

| `error.code`          | Used when                              |
|-----------------------|----------------------------------------|
| `model_not_found`     | No route configured for requested model|
| `unsupported_stream`  | Client requested `stream: true`        |
| `upstream_error`      | Upstream returned 4xx or 5xx           |
| `upstream_timeout`    | Upstream did not respond in time       |
| `upstream_unreachable`| Connection to upstream failed          |

---

### AccountView

```json
{
  "id": "uuid",
  "vendor": "open_ai",
  "display_name": "Primary OpenAI",
  "api_base_override": null,
  "has_api_key": true,
  "enabled": true,
  "weight": 1,
  "priority": 0,
  "tags": ["prod"],
  "capabilities": ["chat", "vision"]
}
```

Rules:

- `api_key` MUST NOT appear.
- `has_api_key` MUST be `true` when a key is stored and `false` otherwise.

### CreateAccountRequest

```json
{
  "vendor": "open_ai",
  "display_name": "Primary OpenAI",
  "api_base_override": null,
  "api_key": "sk-secret",
  "enabled": true,
  "weight": 1,
  "priority": 0,
  "tags": ["prod"],
  "capabilities": ["chat", "vision"]
}
```

Rules:

- `vendor` MUST be required.
- `display_name` MUST be required.
- `api_key` MAY be omitted or `null`.
- `enabled`, `weight`, `priority`, `tags`, and `capabilities` MAY be omitted only if the service
  defines defaults; if omitted, the returned `AccountView` MUST show the effective stored values.

### UpdateAccountRequest

```json
{
  "vendor": "open_ai",
  "display_name": "Primary OpenAI Updated",
  "api_base_override": "http://localhost:4000/v1",
  "api_key": "sk-new-secret",
  "enabled": true,
  "weight": 2,
  "priority": 1,
  "tags": ["prod", "blue"],
  "capabilities": ["chat"]
}
```

Rules:

- `PUT` semantics MUST be full replacement using the path `account_id` as the target identity.
- The request body MUST NOT include `id`.
- `api_key` remains write-only.

---

### PoolView

```json
{
  "id": "uuid",
  "name": "primary",
  "strategy": "round_robin",
  "members": ["account-uuid-1", "account-uuid-2"],
  "fallback_pool_id": null
}
```

### CreatePoolRequest

```json
{
  "name": "primary",
  "strategy": "round_robin",
  "members": ["account-uuid-1"],
  "fallback_pool_id": null
}
```

Rules:

- `name` MUST be required.
- `strategy` MUST be required.
- `members` MAY be omitted and default to `[]`.
- every member id MUST refer to an existing account or the request MUST fail.

### UpdatePoolRequest

```json
{
  "name": "primary-updated",
  "strategy": "priority",
  "members": ["account-uuid-1", "account-uuid-2"],
  "fallback_pool_id": "pool-uuid-2"
}
```

Rules:

- `PUT` semantics MUST be full replacement using the path `pool_id` as the target identity.
- The request body MUST NOT include `id`.

### AddPoolMemberRequest

```json
{
  "account_id": "account-uuid-1"
}
```

Rules:

- `account_id` MUST be required.

---

### RouteView

```json
{
  "id": "uuid",
  "logical_model": "gpt-4o",
  "target_pool_id": "pool-uuid-1",
  "fallback_route_id": null,
  "capability_constraints": ["chat"]
}
```

### CreateRouteRequest

```json
{
  "logical_model": "gpt-4o",
  "target_pool_id": "pool-uuid-1",
  "fallback_route_id": null,
  "capability_constraints": ["chat"]
}
```

Rules:

- `logical_model` MUST be required.
- `target_pool_id` MUST be required and MUST reference an existing pool.

### UpdateRouteRequest

```json
{
  "logical_model": "gpt-4o-mini",
  "target_pool_id": "pool-uuid-2",
  "fallback_route_id": "route-uuid-2",
  "capability_constraints": ["chat", "vision"]
}
```

Rules:

- `PUT` semantics MUST be full replacement using the path `route_id` as the target identity.
- The request body MUST NOT include `id`.

---

### HealthAccountView

```json
{
  "account_id": "account-uuid-1",
  "display_name": "Primary OpenAI",
  "vendor": "open_ai",
  "enabled": true,
  "status": "unknown",
  "last_checked": null,
  "consecutive_failures": 0,
  "cooldown_until": null,
  "is_available": false
}
```

Rules:

- one item MUST correspond to one known account
- `display_name` MUST be a string
- `vendor` MUST be a serialized provider vendor string
- `enabled` MUST be a boolean
- `last_checked` and `cooldown_until` MUST be RFC 3339 timestamps when present
- `is_available` MUST be a boolean derived from current health/cooldown state

### HealthSummaryView

```json
{
  "total": 3,
  "healthy": 1,
  "degraded": 0,
  "unhealthy": 1,
  "unknown": 1
}
```

Rules:

- `total` MUST equal the sum of all four status counters

---

### SettingsView

```json
{
  "gateway_port": 11434,
  "default_routing_policy": {
    "strategy": "priority",
    "max_retries": 3,
    "cooldown_seconds": 60
  },
  "log_json": false,
  "log_level": "info"
}
```

### UpdateSettingsRequest

```json
{
  "gateway_port": 4141,
  "default_routing_policy": {
    "strategy": "round_robin",
    "max_retries": 5,
    "cooldown_seconds": 120
  },
  "log_json": true,
  "log_level": "debug"
}
```

Rules:

- `PUT /api/settings` MUST accept the full settings object shape
- `PATCH /api/settings` is intentionally excluded from the MVP contract

---

### UsageStatusView (placeholder)

```json
{
  "available": false,
  "reason": "usage accounting is not implemented in M1"
}
```

Rules:

- `available` MUST be `false` in M1
- `reason` MUST be a human-readable explanation that usage accounting does not yet exist

---

### Admin Error Response Shape

```json
{
  "error": {
    "code": "resource_in_use",
    "message": "pool 550e8400-e29b-41d4-a716-446655440000 cannot be deleted because it is referenced by one or more routes",
    "details": {
      "resource": "pool",
      "id": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

Rules:

- top-level `error` object MUST exist
- `error.code` MUST be a machine-readable string
- `error.message` MUST be a human-readable string
- `error.details` MAY be omitted; when present it MUST be a JSON object
- error responses MUST NOT include secrets or raw `api_key` values

Suggested error codes include:

- `not_found`
- `validation_error`
- `resource_in_use`
- `conflict`
- `internal_error`

---

## Non-functional Requirements

### NFR-1: Response Time

The gateway MUST NOT add more than 50ms of latency to the upstream response time under normal
conditions (excluding network round-trip to upstream). This includes routing resolution,
header construction, and health feedback.

The default `reqwest::Client` timeout MUST be 30 seconds. This timeout SHOULD be configurable
in a future iteration but MUST be hardcoded for M1.

### NFR-2: Error Handling

All error responses MUST be structured JSON conforming to `GatewayErrorResponse` (R5).

The gateway MUST NOT return HTML error pages or plain-text errors.

The gateway MUST NOT expose internal stack traces, database paths, or implementation details
in error messages.

The gateway MUST NOT panic on any client input — all handler code paths MUST return a proper
HTTP response.

### NFR-3: Logging and Tracing

All upstream proxy calls MUST be logged at `INFO` level with:
- Requested model name
- Resolved account ID (or "none" if routing failed)
- Upstream status code (or error description)
- Request duration in milliseconds

Routing failures MUST be logged at `WARN` level.

Connection errors and timeouts MUST be logged at `ERROR` level.

All log entries MUST use `tracing` structured fields (not string interpolation in the message).

### NFR-4: Backward Compatibility

The gateway router MUST NOT break existing routes:
- `GET /api/health` MUST continue to return `200 "ok"`.
- Dashboard asset routes MUST continue to work.
- No existing domain types or service traits MUST have breaking changes (adding an optional
  field to `ProviderAccount` is non-breaking).

Migration `0003` MUST be backward-compatible: `ALTER TABLE ADD COLUMN ... DEFAULT NULL`
is safe for existing data.

### NFR-5: Dependency Constraints

The `reqwest` dependency MUST use features `["json", "rustls-tls"]` only.

No additional runtime dependencies SHOULD be added beyond `reqwest`. Existing dependencies
(`axum`, `serde`, `serde_json`, `tokio`, `sqlx`, `tracing`, `uuid`, `chrono`) are
already available.

### NFR-6: Concurrency

The gateway handlers MUST be safe to call concurrently from multiple clients.

All shared state (registry, engine, health service) is already `Clone` + `Send` + `Sync`.

The `reqwest::Client` MUST be shared across requests (not created per-request) to enable
connection pooling.
