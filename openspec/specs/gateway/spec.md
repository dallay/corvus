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

> See also: `openspec/specs/gateway/rook-acceptance-regression-matrix.md` for the consolidated
> acceptance/regression traceability matrix covering shipped Rook slices #592-#599. This matrix is
> a verification artifact only; the behavioral source-of-truth remains this spec plus the dedicated
> `dashboard` and `rook-tui` specs for their surfaces.

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
> Encryption-at-rest is not covered by this requirement and would require a separate follow-up change if adopted.

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

For this slice, health data SHALL remain runtime-scoped and in-memory only. It MUST reflect current
process state and MUST NOT imply durable historical health storage, automatic health snapshots, or
persisted health history.

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

#### Scenario: health account list remains runtime-only after audit slice

- GIVEN the audit slice has been added
- WHEN a client requests `GET /api/health/accounts`
- THEN the response MUST still represent current runtime health state only
- AND the response MUST NOT claim or require durable health history

---

### R23: Health Summary Endpoint

The system MUST expose `GET /api/health/summary`.

The response MUST be a `HealthSummaryView` object summarizing known account health state for the
current runtime.

The summary MUST include counts for `healthy`, `degraded`, `unhealthy`, `unknown`, and `total`.

For this slice, the summary MUST remain a current-state runtime view and MUST NOT be reinterpreted
as historical health reporting.

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

#### Scenario: health summary does not become historical reporting

- GIVEN persisted admin audit events exist
- WHEN a client requests `GET /api/health/summary`
- THEN the response MUST still summarize current runtime health only
- AND the response MUST NOT include or imply persisted health history

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

The endpoint MUST NOT invent fake usage totals, provider billing details, quota consumption, token
accounting, or analytics summaries.

This audit slice MUST preserve that placeholder behavior unchanged unless a separate change adds a
real usage ledger and corresponding specification updates.

#### Scenario: usage placeholder response

- GIVEN the M1 runtime with no backing usage subsystem
- WHEN a client requests `GET /api/usage`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal the documented placeholder contract
- AND `available` MUST be `false`

#### Scenario: usage endpoint remains placeholder after audit slice

- GIVEN persisted admin audit events exist in the system
- WHEN a client requests `GET /api/usage`
- THEN the response MUST still equal the documented placeholder contract
- AND `available` MUST be `false`
- AND the response MUST NOT claim real usage analytics or accounting

---

### Requirement: Persisted Append-Only Admin Audit Events

The system MUST persist an append-only admin audit record whenever a supported admin mutation
successfully changes gateway control-plane state.

Supported mutation categories for this slice MUST include:

- account create, update, and delete operations
- pool create, update, and delete operations
- pool membership add and remove operations
- route create, update, and delete operations
- settings update operations

Each audit record MUST be durably stored so that it survives process restart.

Each audit record MUST be append-only: once written, the record MUST NOT be updated in place to alter
its action, subject, actor context, or payload.

Failed validation, authorization, or conflict attempts MAY be excluded from persistence for this
slice; the required scope is successful persisted mutations only.

#### Scenario: successful account mutation writes audit record

- GIVEN an admin request successfully creates or updates an account
- WHEN the mutation is committed
- THEN the system MUST append exactly one persisted audit record for that mutation
- AND the record MUST identify the resource category as `account`
- AND the record MUST identify the mutation action that occurred

#### Scenario: successful pool membership change writes audit record

- GIVEN an admin request successfully adds or removes an account from a pool
- WHEN the membership change is committed
- THEN the system MUST append exactly one persisted audit record for that mutation
- AND the record MUST identify the resource category as `pool_membership`

#### Scenario: failed mutation does not require persisted audit record

- GIVEN an admin mutation request is rejected before any state change is committed
- WHEN the API returns a validation, not-found, or conflict error
- THEN this slice MUST NOT require a persisted audit record for that rejected attempt

---

### Requirement: Minimal Redacted Audit Payload Semantics

The system MUST store only a minimal admin-safe audit payload for this slice.

Each persisted audit record MUST include enough metadata to answer who acted within the available
request context, what mutation occurred, what resource category was affected, which resource identity
was targeted, and when the change was committed.

The stored payload MUST be redacted and bounded. It MUST NOT persist raw secrets, credentials,
authorization headers, API keys, bearer tokens, session cookies, or other sensitive values from the
request or resulting resource state.

When a mutation involves fields that are secret-bearing or operationally sensitive, the audit payload
MUST either omit those fields entirely or persist only an explicit redacted marker rather than the raw
value.

The audit payload SHOULD avoid storing full before/after resource snapshots when narrower changed-field
or identifier-oriented metadata is sufficient.

#### Scenario: account secret fields are excluded from audit payload

- GIVEN an admin request creates or updates an account with an `api_key` or other credential material
- WHEN the audit record is persisted
- THEN the persisted payload MUST NOT contain the raw credential value
- AND the record MUST preserve only redacted or non-secret mutation metadata

#### Scenario: auth transport secrets are excluded from audit payload

- GIVEN an authenticated admin mutation request includes authorization headers or cookies
- WHEN the audit record is persisted
- THEN the persisted payload MUST NOT contain those raw header or cookie values

#### Scenario: settings audit payload remains bounded

- GIVEN an admin request updates settings
- WHEN the audit record is persisted
- THEN the payload MUST capture only the minimal settings mutation metadata needed for auditability
- AND the payload MUST NOT expand into an unbounded full-observability document

---

### Requirement: Admin Audit Retrieval Endpoint

The system MUST provide a bounded admin read surface for retrieving recent persisted audit events.

If this slice exposes audit retrieval, the endpoint MUST be read-only and admin-scoped.

Audit retrieval MUST return persisted audit events in reverse chronological order, newest first.

Each returned audit item MUST preserve the same redaction guarantees as the stored record.

The retrieval contract MAY be limited to recent events only and MAY omit advanced filtering, full-text
search, historical analytics, or retention management.

#### Scenario: audit retrieval returns newest records first

- GIVEN multiple persisted audit records exist for prior admin mutations
- WHEN an admin client requests the audit trail
- THEN the response status MUST be `200 OK`
- AND the response body MUST contain recent audit records ordered newest first

#### Scenario: audit retrieval returns redacted records only

- GIVEN persisted audit records exist for secret-adjacent mutations
- WHEN an admin client requests the audit trail
- THEN the response MUST NOT reveal raw secrets or credentials
- AND each returned item MUST match the redacted audit payload contract

---

### Requirement: Gateway-Published Cerebro Tool Contract

The gateway specification SHALL treat the implemented Cerebro MCP surface as an 8-tool callable
contract for normal operation.

The gateway-published callable tool surface MUST be limited to:

- `mem_save`
- `mem_search`
- `mem_delete`
- `mem_get_observation`
- `mem_update`
- `mem_suggest_topic_key`
- `mem_timeline`
- `mem_stats`

The gateway specification MUST NOT publish `mem_save_prompt`, `mem_session_start`, `mem_session_end`,
`mem_session_summary`, or `mem_context` as currently callable capabilities.

The gateway specification MAY describe those 5 tools only as deferred Cerebro capabilities that
currently return structured `NotImplemented` outcomes.

#### Scenario: Gateway-facing contract lists only implemented callable tools

- GIVEN a gateway-facing contract, runtime integration, or published capability summary derived from
  the source-of-truth spec
- WHEN that contract enumerates Cerebro tools that are callable in normal operation
- THEN the enumeration MUST include exactly the 8 implemented tools listed above
- AND the enumeration MUST NOT include any deferred tool as callable

#### Scenario: Deferred tool remains documented without being advertised as callable

- GIVEN gateway documentation or runtime-facing capability guidance references a deferred Cerebro
  tool such as `mem_context`
- WHEN the tool's current availability is described
- THEN the tool MUST be identified as deferred or unavailable
- AND the description MUST state that current calls receive a structured `NotImplemented` outcome
  rather than normal success

### Requirement: Gateway Verification of Deferred Cerebro Availability Claims

Gateway verification artifacts MUST assert that downstream gateway-facing surfaces do not advertise
deferred Cerebro tools as available for normal use.

Verification MUST cover `mem_context` explicitly because prior downstream drift treated it as available.

#### Scenario: Verification catches mem_context capability drift

- GIVEN a downstream runtime, dashboard, or docs surface that consumes gateway-published Cerebro
  capability data
- WHEN verification checks the published callable capability set
- THEN `mem_context` MUST NOT appear as available for normal use
- AND verification MUST fail if `mem_context` is advertised as callable without a deferred or
  unavailable designation

### Requirement: Cerebro Supported Durable Production Topology

The gateway specification MUST treat Cerebro's current durable production posture as single-node and
local-first only.

The supported durable production topology in this build MUST be exactly one Cerebro node using
node-local durable storage.

The gateway specification MUST identify embedded SurrealDB as the default supported durable
production mode.

The gateway specification MAY identify `disk` as a supported node-local durable alternative when
operators intentionally choose a simpler local storage mode.

The gateway specification MUST NOT describe remote/shared SurrealDB, shared remote persistence, or
HA multi-node durable production as supported in this build.

#### Scenario: Single-node durable production is the only supported topology

- GIVEN an operator reads the gateway source-of-truth for Cerebro production deployment posture
- WHEN the operator determines which durable production topology is currently supported
- THEN the specification MUST state that exactly one Cerebro node with node-local durable storage is supported
- AND the specification MUST identify embedded SurrealDB as the default supported durable mode

#### Scenario: Local durable alternative remains bounded to one node

- GIVEN an operator evaluates the `disk` storage mode for production use
- WHEN the operator checks whether that mode changes the supported topology class
- THEN the specification MUST describe `disk` only as a node-local durable alternative
- AND the specification MUST NOT imply that `disk` enables shared persistence or HA multi-node operation

### Requirement: Unsupported Remote and HA Persistence Claims

The gateway specification MUST define remote/shared SurrealDB and HA multi-node persistence as unsupported in this build.

Any gateway-facing operational, release, or deployment guidance MUST NOT present `remote_surreal` as an available production topology switch.

Gateway-facing guidance MUST NOT claim active-active, shared-store, clustered, or multi-node durable persistence for Cerebro until a separate change explicitly specifies and verifies that capability.

#### Scenario: Remote shared storage is described as unsupported

- GIVEN an operator reads gateway-facing deployment guidance for Cerebro storage topology
- WHEN the guidance addresses remote/shared persistence
- THEN the guidance MUST state that remote/shared SurrealDB is unsupported in this build
- AND the guidance MUST NOT present `remote_surreal` as currently available production support

#### Scenario: HA claim is rejected without follow-on specification

- GIVEN a gateway-facing artifact attempts to describe Cerebro as HA or multi-node durable in the current build
- WHEN that claim is compared against the gateway source-of-truth
- THEN the claim MUST be treated as non-compliant with the specification
- AND the source-of-truth MUST require a separate follow-on change before such a claim can be supported

### Requirement: Operational Guidance for Single-Node Local-First Durability

The gateway specification MUST require operator-facing guidance to describe Cerebro durable production as one durable node backed by local persistence and external backup, restore, and replacement procedures.

The gateway specification SHOULD permit CI, development, and bounded smoke validation flows to use an explicit non-durable or non-default storage mode when that mode is chosen only for testability and does not redefine production support.

The gateway specification MUST distinguish such CI-safe startup validation from the supported durable production topology.

#### Scenario: Operator guidance separates production posture from backup strategy

- GIVEN an operator reads the gateway operational guidance for durable Cerebro deployment
- WHEN the guidance describes resilience expectations
- THEN the guidance MUST instruct the operator to treat Cerebro as one durable local-first node
- AND the guidance MUST describe backup, restore, or node replacement procedures rather than HA multi-node persistence as the resilience strategy

#### Scenario: CI-safe storage mode does not redefine production support

- GIVEN a release or CI smoke validation runs Cerebro with an explicit non-embedded storage mode suitable for CI
- WHEN an operator or maintainer interprets that validation posture
- THEN the specification MUST treat the CI-safe mode as test-only operational scaffolding
- AND the specification MUST NOT infer from that validation that non-local or HA durable production is supported

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

Rook MUST preserve the current loopback-first deployment posture for protected `/api/*` and `/v1/*`
entrypoints, and the safe default bind target for this slice MUST remain `127.0.0.1:4141`.

Non-loopback exposure MUST require an explicit operator bind override through the existing host or
address configuration path. The system MUST NOT treat non-loopback exposure as an implicit or
accidental default.

Dashboard routes outside `/api/*` and `/v1/*` remain outside this slice unless a later change says
otherwise.

Inbound auth for protected Rook routes MUST remain independent from runtime trust flows, pairing
state, webhook secrets, and outbound provider auth.

Operator-visible reporting of the effective bind target MUST identify the effective host and port
without implying that loopback posture, pairing state, or local-network placement is itself an
authentication mechanism.

#### Scenario: protected surfaces no longer use unauthenticated M1 contract

- GIVEN the gateway domain after applying change `rook-591-inbound-auth-boundary`
- WHEN a client interacts with `/api/*` or `/v1/*`
- THEN the contract MUST require the inbound auth behavior defined by this delta spec
- AND the spec MUST NOT describe those protected surfaces as unauthenticated

#### Scenario: runtime trust flows remain out of scope for Rook inbound auth

- GIVEN the inbound auth boundary for protected Rook routes
- WHEN the design reuses ideas from `clients/agent-runtime/src/gateway/utils.rs`
- THEN it MUST adapt only general patterns such as bearer extraction or defensive request filtering
- AND it MUST NOT import runtime-specific pairing or onboarding trust requirements into this contract

#### Scenario: default serve startup remains local-only

- GIVEN an operator starts Rook without overriding the existing bind host or port inputs
- WHEN the server derives its effective listen address
- THEN the effective bind target MUST be `127.0.0.1:4141`
- AND protected route security semantics MUST still remain governed separately by the inbound auth
  contract for this spec

#### Scenario: non-loopback binding requires explicit operator intent

- GIVEN an operator explicitly supplies a non-loopback host such as `0.0.0.0`
- WHEN the server starts successfully
- THEN the effective bind target MUST use that explicit override rather than silently reverting to
  loopback
- AND the system MUST NOT describe that non-loopback exposure as secured solely because Rook is
  local-first or paired elsewhere

---

### R28: Inbound Auth Protected Surfaces

Rook MUST enforce an inbound authentication boundary for the HTTP entrypoints mounted under
`/api/*` and `/v1/*`.

For this slice, every request whose effective route is under `/api/*` or `/v1/*` MUST be treated as
protected unless a later requirement in this spec explicitly marks it public.

This slice MUST cover at least the following already-documented surfaces:

- `GET /api/health`
- all admin routes under `/api/*`
- `POST /v1/chat/completions`
- `GET /v1/models`

Dashboard routes outside those prefixes, including `/` and dashboard asset routes, MUST remain out
of scope for this inbound auth boundary.

Inbound route authentication MUST be enforced before the matched admin or gateway handler performs
its business logic.

#### Scenario: authenticated request reaches protected gateway route

- GIVEN the server is configured with inbound auth enabled for this slice
- AND a client sends `Authorization: Bearer valid-inbound-token`
- WHEN the client requests `GET /v1/models`
- THEN the request MUST be evaluated against the inbound auth boundary before the route handler runs
- AND the request MAY proceed to normal route handling only after the token is accepted

#### Scenario: authenticated request reaches protected admin route

- GIVEN the server is configured with inbound auth enabled for this slice
- AND a client sends `Authorization: Bearer valid-inbound-token`
- WHEN the client requests `GET /api/health`
- THEN the request MUST be evaluated against the inbound auth boundary before the route handler runs
- AND the request MAY proceed to normal route handling only after the token is accepted

#### Scenario: dashboard route remains outside inbound auth scope

- GIVEN the server hosts dashboard routes at `/` alongside `/api/*` and `/v1/*`
- WHEN a client requests `/`
- THEN this inbound auth boundary spec MUST NOT require bearer-token enforcement for that route

---

### R29: Inbound Bearer-Token Contract

Protected inbound requests MUST present credentials using the HTTP `Authorization` header with the
exact scheme `Bearer` followed by a single configured token value.

The inbound auth boundary MUST treat this credential as a Rook client-to-Rook transport credential.
It MUST remain distinct from provider account credentials, outbound vendor authentication, and any
pairing-issued or onboarding-issued credentials unless a later implemented change explicitly wires
those sources into Rook inbound auth.

Validation for this slice MUST compare the presented bearer token against Rook inbound auth
configuration and MUST produce a deterministic allow/deny outcome.

Requests to protected routes MUST be rejected when:

- the `Authorization` header is missing
- the auth scheme is not `Bearer`
- the bearer token value is empty after parsing
- more than one bearer credential is presented in a way the server cannot interpret deterministically
- the bearer token does not match the configured inbound credential

Rook MUST NOT forward the accepted inbound bearer token to upstream providers as vendor auth and
MUST NOT substitute it for a provider account `api_key` when constructing outbound requests.

#### Scenario: valid bearer token is accepted

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `Authorization: Bearer rook-inbound-secret` to `GET /v1/models`
- THEN the inbound auth boundary MUST accept the credential
- AND the request MUST continue to normal route handling

#### Scenario: missing authorization header is rejected

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `GET /v1/models` without an `Authorization` header
- THEN the inbound auth boundary MUST reject the request

#### Scenario: non-bearer authorization scheme is rejected

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `Authorization: Basic abc123` to `GET /api/health`
- THEN the inbound auth boundary MUST reject the request

#### Scenario: wrong bearer token is rejected

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `Authorization: Bearer wrong-token` to `POST /v1/chat/completions`
- THEN the inbound auth boundary MUST reject the request

#### Scenario: accepted inbound token is not reused for outbound provider auth

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- AND a routed provider account uses `api_key = Some("sk-provider")`
- WHEN a protected request is accepted and Rook constructs the outbound provider request
- THEN the outbound authentication header MUST be derived from the provider account credential
- AND it MUST NOT forward `rook-inbound-secret` as the provider auth value

#### Scenario: missing provider credential does not fall back to inbound auth token

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- AND the selected provider account has no usable outbound `api_key`
- WHEN Rook constructs the outbound provider request after authenticating the inbound client
- THEN Rook MUST NOT treat `rook-inbound-secret` as the provider credential source
- AND the outbound behavior MUST remain governed by the existing vendor-auth requirements for that
  account state

---

### R30: Unauthorized and Forbidden Error Semantics

When a protected request fails inbound authentication because credentials are missing, malformed, or
invalid, Rook MUST return `401 Unauthorized`.

`401 Unauthorized` responses produced by the inbound auth boundary MUST be returned before admin or
gateway business logic executes.

For protected `/v1/*` routes, the response body MUST use the documented gateway error response
shape, with:

- `error.type` set to `invalid_request_error`
- `error.code` set to `unauthorized`
- `error.message` describing that a valid inbound bearer token is required

For protected `/api/*` routes, the response body MUST use the standard admin error response shape
defined by the gateway domain, and the body MUST clearly indicate that authentication failed.

When a request presents a valid inbound bearer token but the route is disallowed by an explicit
server-side policy added by this slice or a compatible follow-on slice, the server MUST return
`403 Forbidden` instead of `401 Unauthorized`.

This slice SHOULD NOT introduce `403 Forbidden` behavior unless a concrete policy beyond token
validity is configured.

#### Scenario: gateway route missing token returns 401 gateway error

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `GET /v1/models` without credentials
- THEN the server MUST return `401 Unauthorized`
- AND the response body MUST use the gateway error response shape
- AND `error.code` MUST be `unauthorized`

#### Scenario: admin route invalid token returns 401 admin error

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `GET /api/health` with `Authorization: Bearer wrong-token`
- THEN the server MUST return `401 Unauthorized`
- AND the response body MUST use the standard admin error response shape

#### Scenario: explicit deny policy returns 403

- GIVEN inbound auth accepts the presented bearer token
- AND an explicit server-side authorization policy denies access to the requested protected route
- WHEN the client sends the request
- THEN the server MUST return `403 Forbidden`
- AND the response MUST identify that the request was authenticated but not permitted

---

### R31: Inbound Auth Configuration Contract

This slice MUST define explicit configuration required for inbound auth enforcement.

At minimum, Rook configuration for this slice MUST provide:

- a boolean or equivalent explicit switch that determines whether inbound auth enforcement is active
- a bearer-token value or secret reference used to validate inbound client credentials

When inbound auth enforcement is active, startup or config loading MUST fail closed if the inbound
bearer token is absent, empty, or not resolvable.

When inbound auth enforcement is inactive, the server MAY retain the existing loopback-first M1
behavior until a stricter default is adopted by a later slice.

The configuration contract for inbound auth MUST remain separate from provider account credentials,
vendor API keys, outbound header construction in `clients/rook/src/gateway/vendor.rs`, and shared
onboarding or pairing state.

If an existing operator-visible config or status surface reports inbound auth configuration, that
surface MUST report only enabled, disabled, configured, or absent state and MUST NOT expose the raw
inbound bearer token.

#### Scenario: enabled auth without token fails closed

- GIVEN server configuration enables inbound auth enforcement
- AND the inbound bearer token value is missing or empty
- WHEN the server loads configuration for startup
- THEN startup or configuration initialization MUST fail
- AND the server MUST NOT start in a partially protected state

#### Scenario: enabled auth with token is valid configuration

- GIVEN server configuration enables inbound auth enforcement
- AND the inbound bearer token value is present and non-empty
- WHEN the server loads configuration for startup
- THEN configuration validation MUST succeed for this slice

#### Scenario: operator-visible auth configuration remains redacted

- GIVEN server configuration enables inbound auth enforcement with a non-empty bearer token
- WHEN an existing operator-visible config or status surface reports that configuration state
- THEN the surface MUST indicate only enabled or configured state
- AND it MUST NOT include the raw bearer token value

---

### Requirement: Operator-Visible Secret Protection

Rook MUST protect secret material across operator-visible gateway surfaces, not only in account CRUD
responses.

Any operator-visible admin response, status view, config export, startup report, or structured log
field that indicates inbound auth state or provider credential state MUST use presence-only or
redacted semantics.

These outputs MUST NOT expose raw inbound bearer tokens, provider `api_key` values, pairing codes,
cookies, `Authorization` header values, or equivalent secret-bearing material.

When an existing surface only needs to communicate whether a secret is configured, it MUST use an
existing boolean or equivalent presence indicator rather than echoing the secret value.

#### Scenario: admin account responses remain presence-only for provider credentials

- GIVEN a stored provider account includes `api_key = Some("sk-secret")`
- WHEN an operator reads account state through an existing admin response body
- THEN the response MUST expose only presence information such as `has_api_key: true`
- AND the response MUST NOT include the raw `api_key` value

#### Scenario: inbound auth status outputs do not expose the inbound token

- GIVEN inbound auth is enabled with a configured bearer token
- WHEN an operator-visible config or status output reports inbound auth state
- THEN that output MAY report enabled or configured state
- AND it MUST NOT expose the raw inbound bearer token value

#### Scenario: logs remain redacted when secret-bearing state is present

- GIVEN Rook starts or handles requests while inbound auth and provider credentials are configured
- WHEN operator-visible logs or structured observability fields are emitted for this slice
- THEN the emitted fields MUST NOT contain raw inbound bearer tokens or provider `api_key` values
- AND only redacted, omitted, or presence-only representations MAY appear

---

### Requirement: Onboarding Terminology Alignment Without Pairing Reuse

Rook MUST align with the shared onboarding terminology without claiming pairing integration that is
not evidenced in Rook code.

For this slice, Rook inbound auth for protected `/api/*` and `/v1/*` routes MUST remain a
client-to-Rook transport credential boundary and MUST NOT be described as a pairing flow,
pairing-code exchange, or pairing-issued credential unless a later implemented change proves that
integration.

This slice MAY reuse shared terms such as `bearer token` or `connect to gateway` only when those
terms preserve the trust-boundary meanings defined in `openspec/specs/onboarding/spec.md`.

#### Scenario: Rook inbound auth is not described as pairing by default

- GIVEN operator-facing spec, docs, or product copy for protected Rook routes
- WHEN the credential boundary for `/api/*` or `/v1/*` is described
- THEN the text MUST describe that boundary as Rook inbound auth or an inbound bearer token
- AND it MUST NOT describe that boundary as a pairing code or completed pairing flow unless such a
  flow is implemented for Rook

#### Scenario: onboarding pairing state does not satisfy protected Rook routes by itself

- GIVEN a Corvus environment may also support onboarding or pairing flows on other HTTP surfaces
- AND Rook inbound auth is not configured with an accepted inbound token for a protected route
- WHEN a client requests `GET /v1/models` or `GET /api/health`
- THEN the request MUST NOT be treated as authenticated solely because some other pairing or trust
  state exists elsewhere in the product

#### Scenario: inbound config is separate from vendor auth config

- GIVEN a provider account has an outbound `api_key`
- AND inbound auth is configured with a different bearer token
- WHEN the server validates inbound auth configuration
- THEN it MUST NOT treat the provider account `api_key` as the inbound credential source

---

### R32: Coexistence with Loopback-First Posture

This slice MUST preserve Rook's loopback-first posture as a deployment default while making clear
that loopback binding is not a substitute for inbound authentication on protected routes.

The spec MUST treat loopback binding as an exposure-reduction measure and inbound bearer validation
as the transport authentication control for `/api/*` and `/v1/*`.

If the server is bound only to loopback, protected routes MUST still honor the same inbound auth
contract whenever inbound auth enforcement is active.

This slice MUST NOT rely on browser-origin checks, local-network assumptions, pairing state, or
runtime onboarding trust flows as the primary authenticator for protected Rook routes.

#### Scenario: loopback binding does not bypass active auth

- GIVEN the server is bound only to `127.0.0.1` or equivalent loopback interfaces
- AND inbound auth enforcement is active
- WHEN the client requests `GET /api/health` without credentials
- THEN the server MUST still return `401 Unauthorized`

#### Scenario: loopback posture remains an additional safety layer

- GIVEN the server is configured for loopback-first binding
- WHEN inbound auth for this slice is enabled
- THEN the effective protection model MUST combine loopback exposure reduction with inbound bearer validation
- AND the spec MUST NOT describe loopback binding as sufficient authentication by itself

---

### R33: Non-Goals and Deferred Security Concerns

This slice MUST remain narrow.

The inbound auth boundary defined here MUST NOT require or imply implementation of:

- outbound provider authentication changes in `clients/rook/src/gateway/vendor.rs`
- shared trust state with `clients/agent-runtime`
- pairing-code or onboarding recovery flows
- TLS termination or reverse-proxy certificate policy
- RBAC, scopes, multi-tenant authorization, or per-route permission models
- rate limiting, quotas, abuse prevention, IP allowlists, or WAF controls
- secret storage redesign beyond the minimal inbound token configuration this slice requires

These concerns MAY be specified in later changes, but MUST NOT be prerequisites for satisfying this
slice.

#### Scenario: slice acceptance does not require outbound auth changes

- GIVEN this inbound auth slice is implemented
- WHEN `clients/rook/src/gateway/vendor.rs` constructs outbound provider headers
- THEN its outbound auth behavior MUST remain governed by the existing vendor auth requirements
- AND compliance with this slice MUST NOT depend on changing that behavior

---

### R34: Transport Middleware Covered Surfaces

Rook MUST apply this transport middleware baseline to every inbound HTTP request whose effective
route is mounted under `/api/*` or `/v1/*` before the matched admin or gateway handler executes
business logic.

This slice MUST cover at least the following transport surfaces:

- `GET /api/health`
- all other admin routes under `/api/*`
- `GET /v1/models`
- `POST /v1/chat/completions`

The baseline defined by this slice MUST be limited to request ID handling, tracing/logging hooks,
header sanitation, and forwarded-header trust policy.

Routes outside `/api/*` and `/v1/*`, including dashboard routes at `/` and dashboard asset routes,
MUST remain out of scope for this slice.

This slice MUST remain distinct from the archived `rook-591-inbound-auth-boundary` change.
Meeting this slice MUST NOT require changing inbound bearer-auth semantics.

#### Scenario: middleware baseline applies to protected gateway route

- GIVEN the server hosts routes under `/v1/*`
- WHEN a client sends `GET /v1/models`
- THEN the transport middleware baseline MUST execute before the matched handler's business logic
- AND request ID, sanitation, and transport observability behavior MUST be available to the route

#### Scenario: middleware baseline applies to protected admin route

- GIVEN the server hosts routes under `/api/*`
- WHEN a client sends `GET /api/health`
- THEN the transport middleware baseline MUST execute before the matched handler's business logic

#### Scenario: dashboard routes remain out of scope

- GIVEN the server also hosts dashboard routes outside `/api/*` and `/v1/*`
- WHEN a client requests `/`
- THEN this slice MUST NOT require the transport middleware baseline defined here to govern that route

---

### R35: Request ID Generation and Propagation Contract

Every inbound request covered by this slice MUST have exactly one transport request identifier for
the lifetime of that request.

If the inbound request already contains a syntactically valid request ID in the configured inbound
request ID header, Rook MUST adopt that value as the request's transport request ID.

If the inbound request does not contain a valid request ID in the configured inbound request ID
header, Rook MUST generate a new request ID before invoking downstream handlers.

Rook MUST make the effective request ID available to downstream middleware, handlers, and transport
observability hooks as request-scoped metadata.

Rook MUST return the effective request ID to the client in the configured response request ID
header on both success and error responses for covered routes.

Request ID handling in this slice MUST be transport-scoped only. The request ID MUST NOT be used
as an authentication credential, authorization decision input, or substitute for provider account
identity.

#### Scenario: server generates request ID when absent

- GIVEN a covered inbound request without the configured request ID header
- WHEN the request enters the transport middleware baseline
- THEN Rook MUST generate a request ID before handler execution
- AND the same request ID MUST be exposed to downstream request context
- AND the response MUST include that request ID in the configured response header

#### Scenario: server propagates valid inbound request ID

- GIVEN a covered inbound request with a syntactically valid request ID in the configured inbound header
- WHEN the request enters the transport middleware baseline
- THEN Rook MUST reuse that inbound request ID as the effective request ID
- AND the response MUST include the same request ID value

#### Scenario: invalid inbound request ID is replaced deterministically

- GIVEN a covered inbound request with a malformed or empty value in the configured inbound request ID header
- WHEN the request enters the transport middleware baseline
- THEN Rook MUST reject that value for transport correlation purposes
- AND Rook MUST generate a new effective request ID
- AND the response MUST include the generated request ID instead of the malformed inbound value

---

### R36: Transport Tracing and Logging Hooks

Rook MUST emit transport-level tracing or structured logging hooks for every request covered by
this slice.

At minimum, transport observability hooks for a covered request MUST be able to record:

- the effective request ID
- the matched route or route template when available
- the HTTP method
- the response status code
- request handling duration
- whether forwarded metadata was ignored or trusted under the configured policy

Transport observability hooks MUST use structured fields rather than relying only on interpolated
message strings.

Transport observability hooks MUST execute for both successful and error responses on covered
routes.

Transport observability hooks MUST NOT log or attach raw secret-bearing header values. At minimum,
values for `Authorization`, `Proxy-Authorization`, `Cookie`, `Set-Cookie`, and provider or bearer
token-like credentials MUST be redacted or omitted.

If header metadata is logged for diagnostics, the implementation MUST log only sanitized header
views consistent with this slice's header sanitation rules.

#### Scenario: successful request emits correlated transport fields

- GIVEN a covered request with an effective request ID
- WHEN the request completes successfully
- THEN transport tracing or logging MUST include the request ID, method, route metadata, status code, and duration

#### Scenario: error response still emits transport correlation data

- GIVEN a covered request that terminates with an error response
- WHEN the response is produced
- THEN transport tracing or logging MUST still include the effective request ID and response status code

#### Scenario: secret-bearing headers are redacted from observability output

- GIVEN a covered request containing `Authorization` and `Cookie` headers
- WHEN transport tracing or logging hooks capture header-related diagnostics
- THEN the raw values of those headers MUST NOT appear in logs or spans
- AND only redacted or omitted representations MAY be emitted

---

### R37: Inbound Header Sanitation Rules

Before downstream handlers rely on inbound transport metadata, Rook MUST sanitize inbound
transport-layer and proxy-related headers for every request covered by this slice.

For this slice, sanitation MUST apply at least to:

- the configured request ID header when used for request correlation
- `X-Forwarded-For`
- `X-Forwarded-Host`
- `X-Forwarded-Proto`
- `X-Forwarded-Port`
- `X-Real-IP`
- `Via` (diagnostic-only; never trusted as canonical client/host/proto metadata)

Sanitation for these headers MUST reject empty values and syntactically malformed values from
security-sensitive interpretation.

When a covered header is rejected for security-sensitive interpretation, Rook MUST prevent
downstream transport consumers from treating the rejected value as trusted transport metadata.

Header sanitation in this slice MUST NOT rewrite or remove unrelated application headers outside
the transport/proxy concerns listed here unless another requirement explicitly defines that
behavior.

#### Scenario: empty forwarded header value is sanitized out of trusted view

- GIVEN a covered request with `X-Forwarded-Proto: ` as an empty value
- WHEN header sanitation runs
- THEN the empty value MUST be rejected for trusted transport interpretation
- AND downstream transport context MUST NOT expose it as trusted forwarded metadata

#### Scenario: malformed request ID header does not survive as effective correlation ID

- GIVEN a covered request with a malformed configured request ID header value
- WHEN header sanitation runs
- THEN that value MUST be rejected for request ID adoption
- AND the effective request ID MUST come from generated server-side correlation data instead

#### Scenario: unrelated application headers remain outside this sanitation contract

- GIVEN a covered request with both `X-Forwarded-For` and a domain-specific application header
- WHEN header sanitation runs
- THEN this slice MUST govern the `X-Forwarded-For` handling
- AND it MUST NOT require rewriting the unrelated application header

---

### R38: Strict-by-Default Forwarded Header Trust Policy

Rook MUST treat inbound forwarded metadata as untrusted by default.

Unless explicit trusted-proxy configuration is enabled for this slice, Rook MUST NOT trust
`X-Forwarded-*`, `X-Real-IP`, or similar proxy-provided metadata for security-sensitive or
canonical transport interpretation.

When trusted-proxy configuration is not enabled, Rook MUST derive canonical transport context from
the direct connection context and server-local request properties instead of forwarded headers.

When forwarded metadata is ignored under the default strict posture, observability hooks SHOULD be
able to indicate that forwarded metadata was present but not trusted.

This strict default MUST apply equally to `/api/*` and `/v1/*` surfaces covered by this slice.

#### Scenario: default policy ignores untrusted forwarded host and proto

- GIVEN trusted-proxy configuration is not enabled
- AND a client sends `X-Forwarded-Host: public.example.com` and `X-Forwarded-Proto: https`
- WHEN a covered request is processed
- THEN Rook MUST NOT treat those headers as canonical host or scheme metadata
- AND downstream transport context MUST rely on direct connection or server-local request metadata

#### Scenario: default policy ignores untrusted client IP metadata

- GIVEN trusted-proxy configuration is not enabled
- AND a client sends `X-Forwarded-For: 203.0.113.9` and `X-Real-IP: 203.0.113.9`
- WHEN a covered request is processed
- THEN Rook MUST NOT treat those values as trusted client address metadata for this slice

---

### R39: Explicit Trusted-Proxy Opt-In Behavior

Rook MAY honor supported forwarded metadata only when an explicit trusted-proxy policy is
configured for this slice.

The trusted-proxy policy MUST be explicit enough to distinguish trusted proxy paths from untrusted
clients; a bare assumption that the deployment is "behind a proxy" is insufficient.

When trusted-proxy behavior is enabled, Rook MUST honor only the supported forwarded header
families for this slice (`X-Forwarded-*` and `X-Real-IP`) and only for proxy sources covered by the
configured policy.

If a covered request arrives from a source that does not satisfy the trusted-proxy policy, Rook
MUST fall back to the strict default behavior and ignore forwarded metadata for canonical transport
interpretation.

The standard `Forwarded` header is explicitly out of scope for this slice and MAY be specified in a
later change.

Trusted-proxy opt-in for this slice MUST affect only inbound transport interpretation. It MUST NOT,
by itself, change auth policy, rate limiting, TLS policy, or outbound provider authentication.

#### Scenario: trusted proxy policy allows configured forwarded metadata

- GIVEN trusted-proxy configuration is enabled for a covered request path
- AND the connection source satisfies the configured trusted-proxy policy
- AND the request includes allowed forwarded metadata
- WHEN the request is processed
- THEN Rook MAY use that forwarded metadata for canonical transport interpretation within the configured scope

#### Scenario: opt-in policy does not trust headers from non-trusted source

- GIVEN trusted-proxy configuration is enabled
- AND a covered request includes forwarded headers
- AND the connection source does not satisfy the trusted-proxy policy
- WHEN the request is processed
- THEN Rook MUST ignore the forwarded headers for canonical transport interpretation
- AND the request MUST fall back to the strict default behavior

#### Scenario: trusted-proxy opt-in does not widen unrelated security behavior

- GIVEN trusted-proxy configuration is enabled
- WHEN a covered request is processed
- THEN this slice MUST NOT treat that opt-in as enabling rate limiting, TLS termination policy, or outbound provider auth changes

---

### R40: Transport Middleware Configuration Contract

This slice MUST define explicit configuration for transport middleware behavior on covered Rook
HTTP entrypoints.

At minimum, the configuration contract for this slice MUST provide:

- whether the transport middleware baseline is enabled for covered `/api/*` and `/v1/*` surfaces if the implementation makes it configurable
- the inbound request ID header name used for request ID adoption checks
- the response request ID header name used to return the effective request ID
- the strict forwarded-header trust posture as the default behavior when no trusted-proxy policy is configured
- an explicit trusted-proxy policy shape or equivalent configuration entry required before forwarded metadata MAY be honored
- any validation constraints necessary so invalid trusted-proxy configuration cannot silently weaken the strict default posture

If trusted-proxy behavior is enabled but the trusted-proxy policy is missing, malformed, or not
resolvable, configuration loading or startup MUST fail closed, or the server MUST deterministically
fall back to the strict default behavior without partially trusting forwarded metadata.

Configuration for this slice MUST remain separate from inbound bearer-auth secrets, provider
account API keys, and outbound vendor authentication settings.

#### Scenario: strict default requires no proxy trust configuration

- GIVEN no trusted-proxy policy is configured
- WHEN the server loads configuration for this slice
- THEN the effective behavior MUST remain strict by default
- AND forwarded metadata MUST remain untrusted

#### Scenario: malformed trusted-proxy configuration cannot enable partial trust

- GIVEN trusted-proxy behavior is configured with an invalid or incomplete policy
- WHEN the server loads configuration for this slice
- THEN the server MUST fail closed or deterministically revert to strict default behavior
- AND it MUST NOT start in a partially trusted forwarded-header state

#### Scenario: transport configuration is separate from auth and provider credentials

- GIVEN inbound auth and provider account credentials are also configured
- WHEN transport middleware configuration is loaded
- THEN request ID and trusted-proxy settings MUST be validated independently from bearer-auth and provider API key settings

---

### R41: Non-Goals and Deferred Concerns for Transport Middleware Baseline

This slice MUST remain narrow and MUST NOT require or imply implementation of:

- rate limiting, quotas, or abuse controls
- idempotency keys or replay protection
- streaming request or streaming response transport behavior
- TLS termination, certificate handling, or mTLS policy
- RBAC, scopes, or multi-tenant authorization models
- outbound provider authentication changes
- changes to the archived `rook-591-inbound-auth-boundary` scope

These concerns MAY be specified later, but compliance with this slice MUST NOT depend on them.

#### Scenario: baseline acceptance does not require rate limiting or TLS work

- GIVEN this transport middleware baseline slice is implemented
- WHEN acceptance is evaluated
- THEN the slice MUST be satisfiable without adding rate limiting, idempotency, streaming, or TLS policy changes

---

### R42: Global Surface Rate-Limit Coverage and Scope

Rook MUST define this slice as a transport-boundary, global-by-surface admission-control policy for
the following HTTP entrypoints only:

- all routes whose effective path is under `/api/*`
- `GET /v1/models`
- `POST /v1/chat/completions`

For this slice, each covered surface MUST consume from its own independent global budget. The
`/api/*` surface MUST NOT share a budget with `/v1/models`, and `/v1/models` MUST NOT share a
budget with `/v1/chat/completions`.

Routes outside those surfaces, including dashboard routes and assets outside `/api/*` and `/v1/*`,
MUST remain out of scope for this slice.

This slice MUST remain limited to transport-level surface protection. It MUST NOT define or imply
per-client, per-IP, per-identity, per-token, or per-session limit partitioning.

#### Scenario: covered surfaces are limited independently

- GIVEN Rook is configured with a global rate-limit policy for `/api/*`, `/v1/models`, and `/v1/chat/completions`
- WHEN traffic reaches each covered surface
- THEN Rook MUST evaluate each request against the budget for that exact covered surface
- AND exhausting one covered surface MUST NOT, by itself, exhaust either of the other covered surfaces

#### Scenario: out-of-scope routes remain unaffected by this slice

- GIVEN Rook also serves routes outside `/api/*`, `/v1/models`, and `/v1/chat/completions`
- WHEN a client sends a request to an out-of-scope route
- THEN this slice MUST NOT require global surface rate-limit evaluation for that route

---

### R43: Global Rate-Limit Contract by Surface

For each covered surface, Rook MUST support an explicit operator-controlled policy with a bounded
request budget and a bounded time window.

When a covered request arrives, Rook MUST evaluate that request against the configured global budget
for the covered surface before auth middleware or business handler execution proceeds.

If the covered surface still has capacity within the active window, the request MUST be admitted and
allowed to proceed normally.

If the covered surface has exhausted its budget for the active window, Rook MUST reject the request
at the transport boundary without invoking the downstream admin or gateway business handler.

This slice MAY use an in-memory, process-local implementation for the global-by-surface budget.

#### Scenario: request within surface budget proceeds

- GIVEN a covered surface still has remaining capacity in the active window
- WHEN a request reaches that covered surface
- THEN Rook MUST admit the request
- AND downstream auth and business handler execution MAY continue

#### Scenario: request over surface budget is rejected at the boundary

- GIVEN a covered surface has exhausted its configured budget for the active window
- WHEN another request reaches that covered surface
- THEN Rook MUST reject the request before downstream auth or business handler execution

---

### R44: Surface Rate-Limit Rejection Semantics

When a covered request is rejected because its surface budget is exhausted, Rook MUST return:

- HTTP status `429 Too Many Requests`
- a `Retry-After` response header

The rejection body MUST preserve the existing error-envelope style for the affected surface:

- `/api/*` rejections MUST use the admin/API error response contract
- `/v1/*` rejections MUST use the gateway/OpenAI-style error response contract

`Retry-After` MUST reflect the remaining wait time until the next admission opportunity for the
covered surface according to the configured budget window.

#### Scenario: admin surface rejection uses admin envelope plus Retry-After

- GIVEN the `/api/*` surface has exhausted its budget
- WHEN another request reaches `/api/health` or another `/api/*` route
- THEN Rook MUST return `429`
- AND the response MUST include `Retry-After`
- AND the response body MUST follow the admin/API error envelope

#### Scenario: gateway surface rejection uses gateway envelope plus Retry-After

- GIVEN the `GET /v1/models` or `POST /v1/chat/completions` surface has exhausted its budget
- WHEN another request reaches that covered route
- THEN Rook MUST return `429`
- AND the response MUST include `Retry-After`
- AND the response body MUST follow the gateway/OpenAI-style error envelope

---

### R45: Surface Rate-Limit Startup and Configuration Contract

Rook MUST expose startup/config-driven inputs for all three covered surface policies in this slice:

- `/api/*`
- `GET /v1/models`
- `POST /v1/chat/completions`

For each covered surface, the operator-facing configuration MUST provide, at minimum:

- a request budget value
- a time-window value

The startup/config path for this slice MUST fail closed when any required covered-surface policy is
missing, malformed, zero-valued where prohibited, or otherwise invalid for safe enforcement.

This configuration contract MUST remain separate from:

- inbound auth token configuration
- transport request-ID / trusted-proxy configuration
- outbound provider credential configuration

#### Scenario: startup fails closed on incomplete surface configuration

- GIVEN Rook is started with a missing or invalid rate-limit policy for any required covered surface
- WHEN startup/config validation runs
- THEN startup/config initialization MUST fail closed
- AND Rook MUST NOT proceed with partially-applied surface rate limiting

#### Scenario: explicit startup configuration controls each covered surface independently

- GIVEN the operator provides explicit startup/config values for `/api/*`, `/v1/models`, and `/v1/chat/completions`
- WHEN Rook initializes successfully
- THEN each covered surface MUST use the policy configured for that exact surface
- AND one covered surface's policy MUST NOT silently overwrite or inherit another's

---

### R46: Composition Boundaries with Existing Transport Slices

This slice MUST remain separate from the archived inbound-auth-boundary and transport-middleware-baseline slices.

For covered routes, the rate-limit boundary MUST compose at the transport/router layer without
changing the contract of:

- inbound auth enforcement
- request ID propagation
- tracing/logging hooks
- trusted-proxy / header sanitation behavior

This slice MUST NOT require implementing or altering:

- per-client or per-IP throttling
- idempotency
- streaming support
- TLS configuration
- RBAC
- outbound provider authentication behavior

#### Scenario: rate-limit rejection occurs without replacing auth and middleware responsibilities

- GIVEN a covered request reaches a surface whose budget is exhausted
- WHEN Rook rejects the request with `429`
- THEN the rejection MUST come from the rate-limit boundary for that surface
- AND this slice MUST NOT require changing the already-defined auth or transport-middleware contracts

#### Scenario: acceptance does not require streaming or idempotency work

- GIVEN this slice is implemented as specified
- WHEN rate-limit behavior is verified for the covered surfaces
- THEN acceptance for this slice MUST NOT depend on adding streaming or idempotency functionality

---

### R47: Chat Completions Idempotency Surface

The system MUST apply this idempotency slice only to `POST /v1/chat/completions`.

The system MUST NOT apply this slice to `/api/*`, `GET /v1/models`, or any other route.

The system MUST scope idempotency records by the authenticated inbound principal established by the
existing inbound-auth boundary, so the same raw idempotency key used by different authenticated
principals SHALL NOT collide.

#### Scenario: Idempotency applies only to chat completions create

- GIVEN a valid authenticated request to `POST /v1/chat/completions`
- WHEN the request includes a valid idempotency header
- THEN the gateway MUST evaluate the request under this idempotency slice

#### Scenario: Admin API remains out of scope

- GIVEN a valid authenticated request to `/api/accounts` with an `Idempotency-Key` header
- WHEN the request is handled
- THEN this slice MUST NOT create, read, or reject against a chat-completions idempotency record

#### Scenario: Model listing remains out of scope

- GIVEN a valid authenticated request to `GET /v1/models` with an `Idempotency-Key` header
- WHEN the request is handled
- THEN this slice MUST NOT create, read, or reject against a chat-completions idempotency record

---

### R48: Idempotency Request and Replay Contract

The system MUST use `Idempotency-Key` as the request contract for this slice.

When a valid idempotency key is present on `POST /v1/chat/completions`, the system MUST evaluate
whether a prior keyed request for the same authenticated principal and equivalent canonical request
body already exists within the replay window.

If a keyed equivalent completed request exists, the system MUST return the stored terminal response
deterministically and mark the replay response with `Idempotency-Replayed: true`.

If the same keyed logical request is already in progress, the system MUST reject the duplicate with
a conflict response and MUST NOT invoke the downstream handler a second time.

If the same key is reused with materially different canonical request content, the system MUST
reject the request as a key reuse mismatch.

#### Scenario: completed equivalent request is replayed deterministically

- GIVEN a valid keyed `POST /v1/chat/completions` request has already completed
- WHEN an equivalent keyed request is retried within the replay window
- THEN the system MUST return the stored terminal response
- AND the response MUST include `Idempotency-Replayed: true`

#### Scenario: in-progress duplicate is rejected without second execution

- GIVEN a valid keyed `POST /v1/chat/completions` request is already in progress
- WHEN an equivalent keyed request is retried before the first completes
- THEN the system MUST reject the duplicate request
- AND it MUST NOT invoke the downstream handler a second time

#### Scenario: mismatched keyed request is rejected

- GIVEN a previously seen idempotency key for `POST /v1/chat/completions`
- WHEN the same key is reused with materially different canonical request content
- THEN the system MUST reject the request as an idempotency mismatch

---

### R49: Idempotency Availability, Retention, and Boundaries

This slice MUST define a bounded replay-retention window sufficient for meaningful client retries.

Requests without `Idempotency-Key` MUST continue to behave as ordinary non-idempotent chat
completion requests.

If replay state is unavailable for a keyed request, the system MUST fail closed with an
idempotency-unavailable server error rather than silently executing without replay protection.

This slice MUST remain separate from rate limiting, streaming, TLS, RBAC, and outbound provider
authentication behavior.

#### Scenario: missing key does not enable replay protection

- GIVEN a `POST /v1/chat/completions` request without `Idempotency-Key`
- WHEN the request is handled
- THEN the request MUST proceed without replay protection from this slice

#### Scenario: acceptance does not require streaming or unrelated route idempotency

- GIVEN this slice is implemented as specified
- WHEN idempotency behavior is verified
- THEN acceptance for this slice MUST NOT depend on adding streaming support or idempotency to `/api/*` or `GET /v1/models`

#### Scenario: baseline acceptance remains separate from archived inbound auth work

- GIVEN the archived `rook-591-inbound-auth-boundary` change already defines inbound bearer-auth behavior
- WHEN this slice is accepted
- THEN it MUST remain valid without changing that archived auth contract

---

### R50: Chat Completions Streaming Surface and Request Contract

The system MUST apply this streaming slice only to `POST /v1/chat/completions` when the request body
sets `stream: true`.

The system MUST NOT apply this slice to `/api/*`, `GET /v1/models`, or non-streaming
`POST /v1/chat/completions` requests.

#### Scenario: streaming applies only to chat completions with stream true

- GIVEN a request to `POST /v1/chat/completions`
- WHEN the request body sets `stream: true`
- THEN the gateway MUST use the streaming transport path for this slice

#### Scenario: non-streaming chat completions remain on buffered path

- GIVEN a request to `POST /v1/chat/completions`
- WHEN the request body does not set `stream: true`
- THEN this slice MUST NOT require streaming transport behavior

---

### R51: OpenAI-Compatible SSE Response Contract

For covered streaming requests, the gateway MUST respond using OpenAI-compatible server-sent events
(SSE).

The response MUST use the SSE content type and emit ordered `data:` frames compatible with OpenAI
chat-completions streaming clients.

Successful stream completion MUST terminate with a single `[DONE]` sentinel frame.

#### Scenario: streaming response emits SSE frames and done sentinel

- GIVEN a valid covered streaming request
- WHEN the upstream stream completes normally
- THEN the gateway MUST emit ordered SSE `data:` frames
- AND it MUST emit exactly one `[DONE]` sentinel on normal completion

---

### R52: Streaming Failure and Composition Boundaries

If streaming setup fails before the response stream begins, the gateway MUST return the normal JSON
gateway error response rather than an SSE stream.

If a mid-stream transport failure occurs after streaming has started, the gateway MAY terminate the
SSE stream without emitting `[DONE]`.

This slice MUST remain separate from existing auth, transport middleware, rate limiting, and
buffered idempotency behavior. Streaming requests MUST NOT be forced through buffered idempotency
capture/replay semantics.

#### Scenario: setup failure returns JSON error before stream starts

- GIVEN a covered streaming request
- WHEN the gateway cannot establish streaming before response emission starts
- THEN the gateway MUST return a normal JSON gateway error response

#### Scenario: mid-stream failure omits done sentinel

- GIVEN a covered streaming request has already begun emitting SSE frames
- WHEN a mid-stream transport failure occurs
- THEN the gateway MAY terminate the stream abnormally
- AND it MUST NOT emit `[DONE]` for that abnormal termination

#### Scenario: streaming bypasses buffered idempotency replay path

- GIVEN a covered streaming request with `stream: true`
- WHEN the request is processed
- THEN this slice MUST NOT require buffered idempotency capture/replay behavior for that response path

---

### Requirement: Dream Eligibility for Completed Sessions

The runtime MUST treat Dream as a post-session consolidation capability that evaluates completed
sessions for Dream eligibility.

A Dream run MUST target a specific completed session identity. Dream MUST NOT consolidate sessions
that have not been recorded as completed.

The eligibility decision MUST be deterministic for the same completed session inputs and runtime
configuration.

#### Scenario: Completed session becomes a Dream candidate

- GIVEN a session `sess-123` has been recorded as completed
- AND the runtime can access the relevant session history for `sess-123`
- WHEN Dream eligibility is evaluated for `sess-123`
- THEN the runtime MUST treat `sess-123` as a Dream candidate
- AND the eligibility result MUST be derived from the completed session inputs rather than gateway transport details.

#### Scenario: Active session is not Dream-eligible

- GIVEN a session `sess-123` is still active and has not been recorded as completed
- WHEN Dream eligibility is evaluated for `sess-123`
- THEN the runtime MUST reject Dream consolidation for that session
- AND no Dream artifact MUST be persisted.

### Requirement: Dream Consolidation Output Contract

For an eligible completed session, Dream MUST synthesize durable long-term memory artifacts that
capture stable high-value knowledge from the completed session.

The Dream output MUST be additive. Dream MUST NOT require preserving the full session transcript as
the durable long-term memory artifact itself.

The runtime SHOULD favor stable summaries, facts, or other high-value distilled outputs over
verbatim transcript retention.

#### Scenario: Eligible completed session produces durable distilled memory

- GIVEN a completed session `sess-123` is Dream-eligible
- WHEN Dream consolidates the session
- THEN the runtime MUST produce one or more durable long-term memory artifacts for `sess-123`
- AND those artifacts MUST represent distilled high-value information from the session
- AND the artifacts MUST be persisted independently of the original request/response transport flow.

#### Scenario: Dream does not require verbatim transcript persistence as output

- GIVEN a completed session `sess-123` contains a multi-turn transcript
- WHEN Dream completes consolidation for `sess-123`
- THEN the durable Dream output MUST be allowed to omit verbatim transcript reproduction
- AND the Dream contract MUST remain satisfied so long as stable high-value information is preserved.

### Requirement: Dream Persistence Across Supported Backends

Dream artifacts MUST survive restart, export, and reload flows across supported memory backends.

For this change, supported backends are the runtime backends that already participate in Corvus
memory persistence and snapshot hydration/export behavior, including SQLite, markdown, and runtime
snapshot flows where supported.

A backend that claims Dream support MUST persist both Dream artifacts and enough Dream state to
avoid ambiguous replay for the same completed session.

#### Scenario: Dream artifacts survive runtime restart and reload

- GIVEN Dream has successfully consolidated completed session `sess-123`
- AND the runtime persists Dream through a supported backend
- WHEN the runtime is restarted and its persisted state is reloaded
- THEN the Dream artifacts for `sess-123` MUST still be available
- AND the runtime MUST preserve enough Dream state to avoid treating `sess-123` as unconsolidated solely because of the restart.

#### Scenario: Snapshot export and hydration preserve Dream state

- GIVEN Dream artifacts and Dream replay state exist for completed session `sess-123`
- WHEN the runtime exports its persisted state and later hydrates from that exported state
- THEN the hydrated runtime MUST restore the Dream artifacts for `sess-123`
- AND it MUST restore enough Dream state to keep consolidation behavior unambiguous for that session.

### Requirement: Dream Idempotency per Completed Session

Dream consolidation for a completed session MUST be idempotent.

The runtime MUST record enough Dream state to determine whether a completed session has already
been consolidated or is otherwise no longer eligible for duplicate consolidation.

Repeated Dream triggering for the same completed session MUST NOT create duplicate or ambiguous
consolidation results.

#### Scenario: Duplicate Dream trigger for completed session is suppressed

- GIVEN completed session `sess-123` has already been successfully consolidated by Dream
- WHEN Dream is triggered again for `sess-123`
- THEN the runtime MUST detect that `sess-123` has already been consolidated
- AND it MUST NOT create a second ambiguous consolidation result for the same completion event.

#### Scenario: Repeated trigger after restore remains idempotent

- GIVEN completed session `sess-123` was consolidated before runtime export or shutdown
- AND the persisted Dream state is later restored
- WHEN Dream is triggered again for `sess-123` after restore
- THEN the runtime MUST preserve idempotent behavior for that session
- AND it MUST NOT treat the restored runtime as permission to reconsolidate the same completion ambiguously.

### Requirement: Gateway Dream Integration Is Trigger-Only

Gateway behavior for Dream MUST be limited to invoking the runtime-defined session completion and
Dream trigger integration points.

The gateway MUST NOT become the behavioral source-of-truth for Dream eligibility, consolidation
content, or persistence semantics.

#### Scenario: Gateway delegates Dream semantics to runtime

- GIVEN the gateway completes a request flow that reaches the runtime session-completion path
- WHEN the gateway invokes the completion and Dream trigger integration points
- THEN the gateway MUST rely on the runtime to determine Dream eligibility and consolidation behavior
- AND the gateway MUST NOT define independent Dream eligibility rules.

#### Scenario: Gateway acceptance does not require Dream-specific transport contract

- GIVEN the runtime exposes Dream only through existing completion-trigger integration points
- WHEN a gateway-served request completes successfully
- THEN the gateway MUST remain valid without adding a new Dream-specific public HTTP contract
- AND Dream behavior MUST remain an internal runtime concern unless another spec adds a public surface.

### Requirement: Gateway Completion Hooks MUST Preserve Runtime Ordering and Idempotency

When the gateway participates in a session flow that records completion and triggers Dream, it MUST
invoke those runtime integration points in the runtime-defined order.

The gateway MUST preserve the runtime contract that completion recording happens before Dream
trigger evaluation, and repeated gateway completion handling for the same session MUST NOT require
gateway-defined duplicate-consolidation logic.

#### Scenario: Gateway calls completion recording before Dream trigger

- GIVEN a gateway-served session `sess-123` reaches its completion path
- WHEN the gateway invokes runtime integration for that completion
- THEN the completion-recording hook MUST run before the Dream-trigger hook
- AND Dream evaluation MUST consume the runtime-recorded completion state.

#### Scenario: Replayed gateway completion path stays safe through runtime idempotency

- GIVEN the gateway re-enters the same completion path for session `sess-123`
- WHEN it invokes the runtime completion and Dream hooks again
- THEN the gateway MUST rely on runtime idempotency for the completed session
- AND the repeated gateway path MUST NOT require a second independent Dream result.

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

### Requirement: Shared Effective Rook Configuration Assembly

The system MUST define a first-class `RookConfig` model as the effective runtime configuration for
Rook within the gateway domain.

The system MUST assemble the effective configuration through one shared resolution path that is used
by `serve`, `rook doctor`, and `rook config export`.

That shared resolution path MUST apply configuration sources in this precedence order:

1. built-in defaults
2. configuration file values
3. `ROOK_*` environment overrides
4. CLI flag overrides

The shared resolution path MUST validate the final effective configuration before it is used for
server startup, doctor diagnostics, or config export.

`rook doctor` MUST evaluate effective configuration through that same runtime-startup path rather
than through a parallel or reduced validation path.

The system MUST NOT allow `serve`, `rook doctor`, and `rook config export` to diverge in
effective-value resolution, precedence behavior, or validation outcome.

Operator-visible reporting derived from the effective configuration MUST identify the effective bind
target consistently with the gateway domain's loopback-first posture and MUST NOT imply that
loopback binding alone is an authentication mechanism.

#### Scenario: serve and doctor use the same effective configuration validation path

- GIVEN the same built-in defaults, config file inputs, `ROOK_*` environment values, and CLI flag
  values
- WHEN Rook resolves configuration for `serve`
- AND Rook resolves configuration for `rook doctor`
- THEN both commands MUST produce the same effective configuration values
- AND both commands MUST apply the same precedence order
- AND both commands MUST apply the same validation rules

#### Scenario: doctor reports the effective bind target from startup-equivalent configuration

- GIVEN the effective configuration resolves a bind host and port
- WHEN an operator runs `rook doctor`
- THEN the diagnostics output MUST report the same effective bind target that `serve` would use
- AND the output MUST describe that bind target without treating loopback posture as sufficient
  authentication

---

### Requirement: Rook Doctor Deterministic Diagnostics

The system MUST provide a `rook doctor` command for operator diagnostics in the gateway domain.

Default `rook doctor` execution MUST remain deterministic and local-first.

Default `rook doctor` execution MUST evaluate startup-readiness checks against the same effective
configuration and local prerequisites used by runtime startup.

The default doctor command MUST NOT require live upstream provider reachability, remote account
verification, or other network-dependent checks in order to determine overall success.

Doctor coverage MUST include at minimum:

- effective configuration load and validation through the runtime-startup path
- database open and migration readiness sufficient for local service startup
- inbound auth configuration validation when inbound auth is enabled
- embedded dashboard asset availability required for the local admin/dashboard surface

The doctor command MUST classify each check as `pass`, `warn`, or `fail`.

Each reported check MUST include at minimum:

- a stable check name
- a machine-readable status
- a human-readable explanation
- actionable operator guidance when the status is `warn` or `fail`

A `fail` result MUST mean the checked condition prevents or invalidates correct local startup for the
default operational contract.

A `warn` result MUST mean the checked condition is advisory, degraded, or noteworthy but does not by
itself block correct local startup.

A `pass` result MUST mean the checked condition satisfied the local startup expectation being
validated.

The command MUST return a non-zero exit status when one or more required checks report `fail`.

The command MUST return a zero exit status when all required checks report only `pass` or `warn`.

The doctor command MUST keep secrets redacted in status output and MUST NOT expose raw inbound
bearer tokens, provider API keys, or equivalent secret-bearing values.

#### Scenario: doctor succeeds with passes and warnings only

- GIVEN effective configuration is valid through the runtime-startup validation path
- AND startup-equivalent database open and migration readiness succeeds
- AND required embedded dashboard assets are available
- AND inbound auth is either disabled or enabled with valid configuration
- AND one or more advisory conditions produce `warn` results only
- WHEN an operator runs `rook doctor`
- THEN every required blocking check MUST report `pass` or `warn`
- AND the command MUST return a zero exit status

#### Scenario: doctor fails when startup-equivalent configuration validation fails

- GIVEN effective configuration inputs resolve to a configuration that runtime startup would reject
- WHEN an operator runs `rook doctor`
- THEN the output MUST include a configuration-related check with status `fail`
- AND that check MUST explain what configuration area is invalid
- AND the command MUST return a non-zero exit status

#### Scenario: doctor fails when database readiness would block startup

- GIVEN effective configuration is otherwise valid
- AND the configured database cannot be opened, initialized, or migrated to the state required for
  local startup
- WHEN an operator runs `rook doctor`
- THEN the output MUST include a database-related check with status `fail`
- AND the explanation MUST identify the database readiness problem in operator-actionable terms
- AND the command MUST return a non-zero exit status

#### Scenario: doctor validates inbound auth only when enabled

- GIVEN inbound auth enforcement is disabled in the effective configuration
- WHEN an operator runs `rook doctor`
- THEN the inbound auth diagnostic MUST NOT fail solely because no inbound bearer token is configured
- AND the command MAY report the auth check as `pass` or `warn` according to the disabled state

#### Scenario: doctor fails enabled inbound auth that startup would reject

- GIVEN inbound auth enforcement is enabled in the effective configuration
- AND the inbound bearer credential is missing, empty, or otherwise invalid for startup
- WHEN an operator runs `rook doctor`
- THEN the output MUST include an inbound-auth-related check with status `fail`
- AND the explanation MUST state that inbound auth is enabled but not correctly configured
- AND the output MUST NOT reveal the raw bearer token value
- AND the command MUST return a non-zero exit status

#### Scenario: doctor fails when required dashboard assets are unavailable

- GIVEN effective configuration is otherwise valid
- AND the required embedded dashboard assets for the local admin/dashboard surface are unavailable
- WHEN an operator runs `rook doctor`
- THEN the output MUST include an asset-related check with status `fail`
- AND the explanation MUST identify that the dashboard/admin surface would be broken locally
- AND the command MUST return a non-zero exit status

#### Scenario: default doctor remains local and deterministic when remote providers are unreachable

- GIVEN effective configuration is valid for local startup
- AND one or more configured upstream providers are unreachable over the network
- WHEN an operator runs the default `rook doctor`
- THEN the command MUST complete using only deterministic local checks
- AND upstream reachability MUST NOT be required for a successful overall result

---

### Requirement: Optional Advisory Upstream Probe Mode

The system MAY provide an explicitly opt-in `rook doctor` mode that probes configured upstream
providers or other remote dependencies.

If such a mode is provided, it MUST be disabled by default.

Any remote or upstream probe performed by `rook doctor` MUST be clearly identified as advisory and
MUST remain separate from the default deterministic local readiness result.

Remote probe results MUST NOT change a successful default local readiness result into a required
failure solely because an upstream dependency is unreachable, slow, or otherwise unavailable.

Remote probe execution, if provided, SHOULD be bounded by explicit timeouts or equivalent limits so
that the command remains operationally predictable.

Remote probe output MUST communicate that the probe reflects optional connectivity or upstream state
rather than the baseline local startup contract.

#### Scenario: default doctor omits remote probes

- GIVEN Rook is configured with one or more upstream provider accounts
- WHEN an operator runs `rook doctor` without an explicit remote-probe opt-in
- THEN the command MUST NOT perform upstream reachability checks as part of the default run
- AND the overall result MUST be derived only from local deterministic diagnostics

#### Scenario: opt-in remote probe remains advisory

- GIVEN Rook provides an explicit opt-in mode for remote or upstream probing
- AND local deterministic doctor checks all report `pass`
- AND an opt-in upstream probe cannot reach a configured provider
- WHEN an operator runs `rook doctor` with that explicit opt-in enabled
- THEN the output MUST identify the upstream probe result as advisory
- AND the command MUST continue to distinguish the local readiness result from the remote probe
  outcome
- AND the unreachable upstream probe MUST NOT by itself redefine the default local readiness
  contract as failed

---

### Requirement: `ROOK_*` Environment Override Contract

The system MUST support operator-facing configuration overrides through documented `ROOK_*`
environment variables.

Each supported `ROOK_*` environment variable MUST map deterministically onto the corresponding
`RookConfig` field or sub-field it overrides.

Environment override behavior MUST be documented for operators, including the variable naming
scheme and its position in the precedence order.

Verification for this change MUST include automated coverage that demonstrates supported
`ROOK_*` overrides affect the effective configuration as documented.

#### Scenario: documented environment override is applied to effective configuration

- GIVEN a supported `ROOK_*` environment variable is documented for a specific configuration field
- AND the config file provides a different value for that field
- WHEN Rook resolves the effective configuration
- THEN the effective configuration MUST use the environment-provided value for that field

#### Scenario: unsupported environment variable does not create ambiguous configuration

- GIVEN an environment variable outside the documented supported `ROOK_*` override contract
- WHEN Rook resolves the effective configuration
- THEN the system MUST NOT treat that variable as a valid override for an unrelated config field
- AND operator-facing documentation MUST remain the source of truth for supported overrides

---

### Requirement: Redacted Effective Config Export

The system MUST provide `rook config export` as an operator-visible command that outputs the
validated effective configuration.

`rook config export` MUST render the effective configuration derived from the same shared assembly
path used by `serve`.

Config export MUST protect secret-bearing values using redacted or presence-only semantics.

Config export MUST NOT expose raw inbound bearer tokens, provider API keys, authorization header
values, cookies, or equivalent secret-bearing material.

When config export needs to communicate secret state, it MUST report only configured, enabled,
present, absent, or an equivalent redacted state rather than the raw secret value.

#### Scenario: config export shows effective non-secret values and redacts secrets

- GIVEN effective configuration contains non-secret runtime settings and one or more configured
  secret-bearing values
- WHEN an operator runs `rook config export`
- THEN the output MUST include the effective non-secret values
- AND the output MUST represent secret-bearing fields only with redacted or presence-only state
- AND the output MUST NOT reveal the raw secret values

#### Scenario: config export preserves gateway bind posture reporting without leaking secrets

- GIVEN the effective configuration resolves the gateway bind host and port
- AND inbound auth or provider credentials are also configured
- WHEN an operator runs `rook config export`
- THEN the output MUST report the effective bind target consistently with the gateway domain
- AND the output MUST continue to redact secret-bearing fields

---

### Requirement: Invalid Configuration Fails Closed With Operator-Facing Messages

The system MUST fail closed when the effective configuration is invalid after applying defaults,
file inputs, environment overrides, and CLI overrides.

Validation failure MUST prevent server startup and MUST prevent successful config export.

Validation failure output MUST be operator-facing and clear enough to identify the invalid
configuration area and the reason the configuration cannot be used.

The system MUST NOT continue with partially applied or partially validated configuration.

#### Scenario: invalid effective configuration blocks startup

- GIVEN effective configuration inputs resolve to an invalid state required for gateway startup
- WHEN Rook loads configuration for `serve`
- THEN configuration validation MUST fail before the server starts
- AND the command MUST return a non-success result
- AND the operator-facing error MUST identify the invalid configuration area

#### Scenario: invalid effective configuration blocks config export

- GIVEN effective configuration inputs resolve to an invalid state
- WHEN an operator runs `rook config export`
- THEN configuration validation MUST fail before export output is produced as a successful result
- AND the command MUST return a non-success result
- AND the operator-facing error MUST clearly explain why the configuration is invalid

---

### Requirement: Explicit Precedence Verification and Documentation

The system MUST make configuration precedence explicit in operator-facing documentation for this
change.

The documented precedence order MUST be defaults < file < environment < CLI.

Verification for this change MUST include automated tests that assert precedence behavior across at
least defaults, file inputs, `ROOK_*` environment overrides, and CLI flags.

Verification for this change MUST include coverage that confirms config export redaction behavior
and invalid-configuration fail-closed behavior.

#### Scenario: precedence documentation matches implemented behavior

- GIVEN operator-facing documentation for Rook configuration inputs
- WHEN the documentation describes configuration precedence
- THEN it MUST state the order defaults < file < environment < CLI
- AND that documented order MUST match the behavior verified by automated tests

#### Scenario: automated verification catches precedence regressions

- GIVEN automated tests for layered configuration resolution
- WHEN a lower-precedence source would incorrectly override a higher-precedence source
- THEN the relevant precedence verification MUST fail
- AND the failure MUST identify that implemented precedence drifted from the documented contract

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

### NFR-7: Release Decoupling Rollout Must Be Incremental and Reversible

The release architecture transition from a repo-wide release train to component-scoped releases
MUST be executed in explicit phases with rollback criteria for each phase.

#### Scenario: Operators prepare the pilot rollout

- GIVEN the repository has completed component inventory and impact mapping
- WHEN operators prepare the first component-scoped release pilot
- THEN they have a documented rollout phase sequence
- AND they have explicit success and failure signals for the pilot
- AND they know how to return to the last known-good release configuration

#### Scenario: A rollout phase fails

- GIVEN a release-decoupling phase causes tag drift, changelog confusion, or incorrect publish
  scope
- WHEN operators execute rollback
- THEN they restore the last known-good release config, manifest state, and workflow contract for
  the affected channel
- AND they do not continue to the next phase until the failure is explained

---

## Production Observability and Metrics (Issue #680)

The following requirements were added as part of the production observability and metrics slice.

### Requirement: Production Request Metrics for Gateway Surfaces

The system MUST expose bounded, operator-visible request metrics for the gateway domain across both
`/api/*` and `/v1/*` HTTP surfaces.

At minimum, this slice MUST make the following request-level signals observable for covered routes:

- total requests
- terminal responses by outcome class
- request duration distributions

This requirement MUST cover successful and error responses for representative admin and
OpenAI-compatible routes under `/api/*` and `/v1/*`.

The emitted metric dimensions for this slice MUST remain bounded to stable transport or route
attributes such as surface, normalized endpoint or route template, method, and coarse outcome or
status class.

The system MUST NOT require operators to infer covered request volume, error rate, or latency only
from structured logs when this metrics surface is available.

#### Scenario: `/api/*` and `/v1/*` requests emit bounded request and latency metrics

- GIVEN Rook is serving covered admin and gateway routes under `/api/*` and `/v1/*`
- WHEN a request completes on a covered route
- THEN the metrics surface MUST reflect one additional request for that route family
- AND the emitted metrics MUST include a bounded route or endpoint dimension for the covered route
- AND the emitted metrics MUST include request duration data for that completed request

#### Scenario: error responses remain observable in the same request metric families

- GIVEN a covered `/api/*` or `/v1/*` request terminates with an error response
- WHEN the response is produced
- THEN the metrics surface MUST record the request in the same bounded request metric families
- AND the emitted dimensions MUST indicate an error outcome or status class without requiring raw logs

#### Scenario: uncovered request payload details are not promoted into labels

- GIVEN a covered request contains request-specific values such as raw model prompts, user text, or
  arbitrary header values
- WHEN request metrics are emitted
- THEN those request-specific values MUST NOT appear as metric labels
- AND only bounded route and outcome dimensions MAY be emitted for this slice

### Requirement: Upstream Failure Metrics for Routed Gateway Calls

The system MUST expose bounded metrics for upstream gateway call failures on covered `/v1/*` request
paths that perform routed provider work.

At minimum, this slice MUST make upstream failure outcomes observable by failure class so operators
can distinguish upstream-related failures from local request-handling failures.

The metrics MAY include routing-context dimensions such as vendor, account, and logical model only
when those values are already available at the routing boundary and are safe and bounded to emit.

If vendor, account, or model dimensions are not already available in a safe bounded form for a given
failure path, the system MUST still emit the upstream failure metric without those optional
identifiers.

This slice MUST NOT emit raw provider credentials, upstream authorization material, full upstream
URLs with secret-bearing query strings, or unbounded provider error body content as metric labels.

#### Scenario: upstream provider failure increments a bounded failure metric

- GIVEN a covered `/v1/*` request is routed to an upstream provider
- AND the upstream interaction fails by timeout, transport error, or non-success upstream outcome
- WHEN the gateway returns the terminal client response for that request
- THEN the metrics surface MUST record an upstream failure outcome for that routed request
- AND the failure metric MUST use bounded failure classification labels

#### Scenario: safe routing identifiers are included only when already available and bounded

- GIVEN a covered upstream failure path already has routing context for vendor, account, and logical
  model in a bounded safe form
- WHEN the upstream failure metric is emitted
- THEN the metric MAY include those routing identifiers as labels
- AND the labels MUST NOT expose secrets or unbounded free-form upstream data

#### Scenario: upstream failures remain observable when optional routing labels are unavailable

- GIVEN a covered upstream failure occurs before safe bounded vendor, account, or model labels are
  available
- WHEN the failure metric is emitted
- THEN the gateway MUST still emit an upstream failure metric
- AND omission of optional routing identifiers MUST NOT suppress the failure signal

### Requirement: Rate-Limit and Idempotency Outcome Metrics

The system MUST expose bounded outcome metrics for the existing rate-limit and idempotency slices on
covered gateway surfaces.

For the global surface rate-limit slice, the metrics MUST make both admitted and rejected outcomes
observable for covered `/api/*` and `/v1/*` surfaces at a bounded surface granularity.

For the chat-completions idempotency slice, the metrics MUST make normal keyed execution, replay,
conflict, mismatch, and unavailable outcomes observable for `POST /v1/chat/completions` without
requiring operators to parse logs.

Outcome metrics for this slice MUST use bounded outcome classes and covered-surface identifiers
rather than raw idempotency keys, principal tokens, or request body fingerprints.

#### Scenario: rate-limit saturation is observable for a covered surface

- GIVEN a covered `/api/*` or `/v1/*` surface exhausts its configured rate-limit budget
- WHEN an additional request is rejected with the existing rate-limit behavior
- THEN the metrics surface MUST record a rate-limit rejection outcome for that covered surface
- AND the emitted dimensions MUST remain bounded to the covered surface and coarse outcome class

#### Scenario: idempotency replay and conflict outcomes are observable

- GIVEN `POST /v1/chat/completions` receives keyed requests that exercise replay and in-progress
  conflict behavior
- WHEN the gateway returns those terminal outcomes
- THEN the metrics surface MUST record the corresponding idempotency replay and conflict outcomes
- AND the emitted metrics MUST NOT include the raw idempotency key value

#### Scenario: idempotency mismatch or unavailable outcomes remain bounded and secret-safe

- GIVEN `POST /v1/chat/completions` returns an idempotency mismatch or idempotency-unavailable
  outcome
- WHEN the metrics surface is updated
- THEN the corresponding outcome MUST be observable in idempotency metrics
- AND the emitted labels MUST NOT include request body fingerprints, bearer tokens, or principal secrets

### Requirement: Operator Metrics Collection Contract

The system MUST expose the metrics surface through an explicit operator-scrapable contract suitable
for collection by external operators or platform scrapers.

The metrics surface MUST use a stable text exposition format and a stable content type suitable for
standard scraping-based collection.

The gateway specification MUST define operator expectations for scraping or collecting the metrics
surface, including that collection is external to this change and that the service is only required
to expose a scrapeable endpoint or equivalent bounded metrics surface.

This slice MUST support operator collection expectations for the request, error, latency, upstream
failure, rate-limit, and idempotency metrics defined here.

This change MUST NOT require shipping dashboards, alert rules, tracing pipelines, or long-term
analytics storage as part of compliance with the metrics contract.

#### Scenario: operator scraper can collect the bounded metrics surface

- GIVEN Rook is running with this observability slice enabled
- WHEN an operator-managed scraper or collector reads the metrics surface
- THEN the service MUST return the bounded metrics exposition in the documented scrapeable format
- AND the exposition MUST include the metric families required by this change

#### Scenario: collection expectations do not imply bundled observability infrastructure

- GIVEN an operator evaluates this slice for deployment
- WHEN they read the gateway observability contract for collection expectations
- THEN the contract MUST require Rook to expose a scrapeable metrics surface
- AND it MUST NOT require Rook to bundle dashboards, alerts, or a specific collector deployment

### Requirement: Metric Label Safety and Cardinality Boundaries

All metrics introduced by this slice MUST use secret-safe, bounded label sets appropriate for
production operation.

Allowed label dimensions for this slice MUST be limited to stable low-cardinality identifiers such
as covered surface, normalized endpoint or route template, HTTP method, coarse status or outcome
class, and safe bounded routing identifiers when explicitly permitted by this change.

Metrics in this slice MUST NOT use raw paths, request IDs, idempotency keys, bearer tokens, API
keys, cookies, request bodies, upstream response bodies, arbitrary user identifiers, or equivalent
high-cardinality or secret-bearing values as labels.

If a potentially useful dimension cannot be emitted in a bounded and secret-safe form, the system
MUST omit that dimension rather than emitting an unsafe label.

#### Scenario: secret-bearing values are excluded from metrics labels

- GIVEN a covered request or upstream failure path includes bearer tokens, API keys, cookies, or
  other secret-bearing values
- WHEN metrics for this slice are emitted
- THEN those values MUST NOT appear in metric labels or metric names
- AND the metrics MUST remain observable through bounded non-secret dimensions

#### Scenario: unbounded identifiers are omitted instead of emitted

- GIVEN a candidate metric dimension would vary with raw request path fragments, request IDs,
  idempotency keys, or arbitrary user-supplied values
- WHEN the implementation evaluates whether to label the metric with that dimension
- THEN the system MUST omit that dimension from the metric
- AND compliance with this slice MUST prefer lower-cardinality observability over unsafe label growth

---

### Requirement: Effective Rook Configuration Assembly and Export

The system MUST provide a single effective configuration assembly path for Rook runtime startup,
operator diagnostics, and operator-visible config export within the gateway domain.

The effective configuration MUST apply sources in this precedence order:

1. built-in defaults
2. configuration file values
3. `ROOK_*` environment overrides
4. CLI flag overrides

The system MUST support `rook config export` as an operator-visible command that returns the
effective configuration after precedence resolution and validation.

The export output MUST be deterministic for the same effective inputs and MUST be safe for
operator visibility.

The export output MUST include Phase 1 runtime concerns needed by startup and diagnostics,
including at minimum server bind configuration, database path, inbound auth configuration state,
transport configuration, rate-limit configuration, and idempotency configuration.

The system MUST validate the effective configuration before using it for server startup, doctor
checks, or config export. Invalid effective configuration MUST fail closed with a non-success
result and a readable validation error.

The system MUST use the same effective configuration assembly behavior for `serve`, `rook doctor`,
and `rook config export`; these commands MUST NOT diverge in precedence, validation, or redaction
behavior.

#### Scenario: config export reflects precedence across defaults, file, environment, and CLI

- GIVEN built-in defaults define a server bind port
- AND a config file sets a different server bind port
- AND `ROOK_*` environment overrides set a third server bind port
- AND CLI flags set a fourth server bind port
- WHEN an operator runs `rook config export`
- THEN the exported effective configuration MUST report the CLI-provided port
- AND the result MUST reflect the precedence order defaults < file < environment < CLI

#### Scenario: config export uses environment overrides when CLI does not override them

- GIVEN a config file sets a database path
- AND `ROOK_*` environment overrides set a different database path
- AND no CLI flag overrides the database path
- WHEN an operator runs `rook config export`
- THEN the exported effective configuration MUST report the environment-provided database path

#### Scenario: serve and config export share the same effective configuration

- GIVEN a config file and `ROOK_*` environment overrides together define the effective runtime
  configuration
- WHEN the server starts with `serve`
- AND an operator runs `rook config export` under the same inputs
- THEN both code paths MUST resolve the same effective configuration values
- AND neither path MUST apply a different precedence or validation rule

#### Scenario: invalid effective configuration fails closed before startup or export

- GIVEN effective configuration inputs contain an invalid value for a required Phase 1 runtime
  setting
- WHEN the server loads configuration for `serve` or `rook config export`
- THEN configuration validation MUST fail with a readable error
- AND the command MUST return a non-success result
- AND the server MUST NOT start with partially applied configuration

---

### Requirement: Operator-Visible Config Export Redaction

The system MUST treat `rook config export` as an operator-visible gateway surface and MUST redact
or reduce secret-bearing fields to presence-only state.

Config export MUST NOT expose raw inbound bearer tokens, provider API keys, authorization header
values, cookies, pairing codes, or equivalent secret-bearing material.

When a secret is configured, config export SHOULD indicate presence or enabled state rather than
echoing the raw value.

#### Scenario: config export redacts inbound auth secrets

- GIVEN inbound auth is enabled with a configured non-empty bearer token
- WHEN an operator runs `rook config export`
- THEN the export MAY report that inbound auth is enabled or configured
- AND it MUST NOT expose the raw bearer token value

#### Scenario: config export redacts provider credentials

- GIVEN effective configuration contains one or more provider credentials needed for runtime
- WHEN an operator runs `rook config export`
- THEN the export MAY report credential presence or enabled state
- AND it MUST NOT expose raw provider credential values

---

### Requirement: Rook Doctor Deterministic Diagnostics

The system MUST provide a `rook doctor` command for operator diagnostics in the gateway domain.

`rook doctor` MUST evaluate deterministic local checks against the same effective configuration
used by runtime startup.

Phase 1 doctor coverage MUST include at minimum:

- effective configuration load and validation
- database path usability and migration/open readiness
- embedded dashboard or admin asset availability required by the process
- inbound auth configuration consistency

The doctor command MUST classify each check as `pass`, `warn`, or `fail` and MUST include for each
check a machine-readable status, a short check name, and a human-readable explanation.

The doctor command MUST return a non-zero exit status when any required check fails.

The first Phase 1 doctor version MUST remain fast and deterministic and MUST NOT require live
upstream provider probing.

#### Scenario: doctor succeeds when all required local checks pass

- GIVEN effective configuration is valid
- AND the configured database path can be opened and any required migrations can run
- AND required embedded assets are available
- AND inbound auth configuration is internally consistent
- WHEN an operator runs `rook doctor`
- THEN the command MUST report all required checks with status `pass` or `warn`
- AND the command MUST return a zero exit status

#### Scenario: doctor fails when configuration is invalid

- GIVEN effective configuration fails validation
- WHEN an operator runs `rook doctor`
- THEN the output MUST include at least one check with status `fail`
- AND that failed check MUST identify configuration as the failing area
- AND the command MUST return a non-zero exit status

#### Scenario: doctor fails when database path is unusable

- GIVEN effective configuration is otherwise valid
- AND the configured database path cannot be opened or prepared for runtime use
- WHEN an operator runs `rook doctor`
- THEN the output MUST include a database-related check with status `fail`
- AND the explanation MUST indicate that the database path is not usable
- AND the command MUST return a non-zero exit status

#### Scenario: doctor does not depend on live upstream network health

- GIVEN effective configuration is valid for local startup
- AND external upstream AI providers are unreachable
- WHEN an operator runs the default Phase 1 `rook doctor`
- THEN the command MUST still complete using deterministic local checks
- AND upstream reachability MUST NOT be required for overall success in this phase

---

### Requirement: Readiness and Liveness Health Endpoints

The system MUST expose distinct liveness and readiness health semantics for the admin surface.

The system MUST expose a liveness endpoint and a readiness endpoint under the `/api/health/*`
namespace.

For Phase 1, the liveness endpoint MUST report whether the Rook process is running and capable of
serving the event loop. Liveness MUST NOT depend on database reachability, provider reachability,
or account-level routing state.

For Phase 1, the readiness endpoint MUST report whether critical local dependencies required to
serve traffic are available.

Readiness MUST evaluate at minimum:

- effective configuration validation success
- database open or initialization success required for serving
- router availability
- embedded assets or other local runtime resources required by the process

Readiness MUST NOT require all upstream AI providers to be reachable in Phase 1.

Both health endpoints MUST return structured JSON responses with stable semantics suitable for
orchestration.

#### Scenario: liveness is healthy while process is running

- GIVEN the Rook process is running
- WHEN a client requests the liveness endpoint
- THEN the response status MUST be successful
- AND the JSON body MUST indicate a live state
- AND the result MUST NOT depend on database or upstream provider reachability

#### Scenario: readiness is healthy after valid startup

- GIVEN the Rook process has completed startup with valid effective configuration
- AND the required database and local runtime resources are available
- WHEN a client requests the readiness endpoint
- THEN the response status MUST be successful
- AND the JSON body MUST indicate a ready state

#### Scenario: readiness fails when a critical local dependency is unavailable

- GIVEN the Rook process cannot satisfy a critical local serving dependency such as configuration
  validation or database initialization
- WHEN a client requests the readiness endpoint
- THEN the response status MUST be non-success
- AND the JSON body MUST indicate not-ready state
- AND the response MUST identify at least one failing readiness dependency

#### Scenario: readiness does not fail solely because upstream providers are unreachable

- GIVEN the Rook process has valid local startup state
- AND one or more upstream AI providers are unreachable
- WHEN a client requests the readiness endpoint
- THEN readiness MUST continue to report ready for Phase 1
- AND upstream provider reachability MUST NOT be required by this requirement

---

### Requirement: Existing Base Health Endpoint Compatibility

The existing `GET /api/health` admin route MUST remain available for compatibility during Phase 1.

If distinct readiness and liveness routes are added, `GET /api/health` MUST continue to return a
successful lightweight health response or a documented compatibility view, and it MUST NOT be
removed by this change.

#### Scenario: existing base health endpoint remains available after readiness/liveness are added

- GIVEN Phase 1 readiness and liveness endpoints are implemented
- WHEN a client requests `GET /api/health`
- THEN the route MUST still exist
- AND the response MUST remain successful for a healthy running process

---

### Requirement: Baseline Metrics Exposure for Gateway Operations

The system MUST expose a production metrics surface for the gateway domain in Phase 1.

The metrics surface MUST be reachable through one explicit operator-facing endpoint suitable for
scraping or inspection.

The Phase 1 metrics baseline MUST include at minimum:

- total requests partitioned by route surface, endpoint, or status class
- request duration metrics for core gateway and admin request paths
- rate-limit rejection counts
- idempotency replay, conflict, and pass counts
- upstream request outcome counts

The metrics surface SHOULD be scrape-friendly for operators.

The metrics surface MUST use Prometheus/OpenMetrics text exposition with a stable `Content-Type`
header so downstream scrapers can rely on the format contract.

Instrumentation MUST be attached through stable middleware, transport hooks, or gateway helper
boundaries where available; it MUST NOT require one-off per-handler duplication to satisfy the
baseline contract.

The metrics surface MUST be observable without requiring operator access to application logs.

#### Scenario: metrics endpoint is available for operators

- GIVEN a running Rook server with Phase 1 observability enabled
- WHEN an operator requests the metrics endpoint
- THEN the server MUST return a successful metrics response
- AND the response MUST advertise `application/openmetrics-text; version=1.0.0; charset=utf-8`
- AND the response body MUST conform to Prometheus/OpenMetrics text exposition
- AND the response MUST include metric families for the Phase 1 baseline

#### Scenario: request metrics increment for core routed traffic

- GIVEN a running Rook server
- WHEN a client successfully calls a core route such as `/api/*`, `/v1/models`, or
  `/v1/chat/completions`
- THEN the metrics surface MUST reflect an incremented request count
- AND the emitted metrics MUST include latency or duration data for the request class

#### Scenario: rate-limit and idempotency outcomes are observable in metrics

- GIVEN a request is rejected by rate limiting
- AND another request exercises idempotency replay or conflict behavior
- WHEN an operator inspects the metrics surface
- THEN the metrics MUST include a counter increment for the rate-limit rejection
- AND the metrics MUST include the corresponding idempotency outcome increments

#### Scenario: upstream outcomes are observable without reading logs

- GIVEN the gateway performs upstream requests that result in success and failure outcomes
- WHEN an operator inspects the metrics surface
- THEN the metrics MUST expose upstream outcome counts by result type
- AND the operator MUST NOT need to infer these counts exclusively from logs

---

### Requirement: Linux Release Binary Startup Smoke Validation

The release workflow for the Linux Cerebro binary MUST execute a startup smoke validation that proves the built release artifact can start the real HTTP and MCP service surface, not merely parse CLI arguments or print help text.

This validation MUST run the produced release binary with a temporary CI-specific configuration that defines explicit loopback binding, an explicit CI-safe non-production storage mode, and a deterministic inbound bearer token for MCP authentication.

When the workflow uses a non-durable test mode (for example, `in_memory`) for smoke startup, that choice MUST be treated as test-only operational scaffolding and MUST NOT redefine the supported durable production posture.

The smoke validation MUST start the service as a background process, poll for startup within a bounded timeout, capture service logs for diagnostics, and terminate the process before the workflow step exits on both success and failure.

The smoke validation scope for this change MUST apply at least to the Linux release build path.

#### Scenario: Linux release artifact starts real service surface

- GIVEN the Linux release workflow has built a Cerebro release binary
- AND the workflow prepares a temporary configuration with explicit loopback binding, explicit CI-safe storage mode, and a known bearer token
- WHEN the workflow launches the built binary for the smoke validation
- THEN the binary MUST start the real HTTP and MCP service surface
- AND the workflow MUST treat startup as successful only after the service responds within the configured timeout
- AND the workflow MUST terminate the background process before the workflow step exits

#### Scenario: Startup failure surfaces diagnostics and cleanup

- GIVEN the Linux release workflow has launched the Cerebro release binary for smoke validation
- WHEN the service fails to start or respond within the configured timeout
- THEN the workflow MUST capture and surface the service logs for diagnostics
- AND the workflow MUST terminate the background process before the workflow step exits
- AND the workflow step MUST fail with a clear error message

#### Scenario: Health endpoint responds after startup

- GIVEN the Linux release workflow has successfully started the Cerebro release binary
- WHEN the workflow polls the health endpoint
- THEN the endpoint MUST respond with HTTP 200
- AND the response MUST indicate the service is healthy

#### Scenario: Readiness endpoint responds after startup

- GIVEN the Linux release workflow has successfully started the Cerebro release binary
- WHEN the workflow queries the readiness endpoint
- THEN the endpoint MUST respond with HTTP 200
- AND the response MUST indicate the service is ready to accept requests

#### Scenario: Unauthenticated MCP request is rejected

- GIVEN the Linux release workflow has successfully started the Cerebro release binary with a configured bearer token
- WHEN the workflow sends an MCP request without authentication
- THEN the service MUST reject the request
- AND the response MUST indicate authentication is required

#### Scenario: Authenticated MCP request succeeds

- GIVEN the Linux release workflow has successfully started the Cerebro release binary with a configured bearer token
- WHEN the workflow sends an authenticated MCP tools/list request with the correct bearer token
- THEN the service MUST accept the request
- AND the response MUST be a valid JSON-RPC 2.0 success response
- AND the response MUST include a tools array in the result
