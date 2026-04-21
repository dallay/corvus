# Gateway API Specification

**Change**: rook-589-gateway-api
**Issue**: #589
**Parent**: #576 (OpenAI-compatible gateway and admin API for Corvus Rook)
**Phase**: M1 (MVP)
**Domain**: gateway

---

## Purpose

Define the contract for Rook's OpenAI-compatible HTTP gateway surface — two endpoints
(`POST /v1/chat/completions` and `GET /v1/models`) that accept OpenAI-shaped requests,
resolve routing via the existing `RoutingEngine`, proxy upstream provider calls, and return
OpenAI-shaped responses. This spec also covers the prerequisite `api_key` credential storage
on `ProviderAccount`, vendor base URL mapping, vendor-specific auth header construction, and
health feedback after upstream calls.

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
