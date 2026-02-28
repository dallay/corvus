---
title: Dashboard API
---

The Corvus Gateway provides a REST API for the operator dashboard and external integrations. This API allows for configuration management, client pairing, and message handling via webhooks.

## Base URL

By default, the gateway listens on:
`http://127.0.0.1:3000`

:::caution[Production Security]
The default URL is for local development only. In production, you **must** enable TLS (HTTPS) and ensure authentication tokens are never exposed in URLs, logs, or client-side code. Use secure storage like environment variables or a secrets manager for all tokens.
:::

## Authentication

The API uses two main authentication mechanisms:

### 1. Pairing Code
Used once to exchange a one-time code for a persistent bearer token.
- **Header**: `X-Pairing-Code: <CODE>`

### 2. Bearer Token
Used for all administrative and webhook requests once the client is paired.
- **Header**: `Authorization: Bearer <TOKEN>`

---

## Endpoints

### Pairing

#### `POST /pair`
Exchanges a one-time pairing code for a persistent bearer token.

- **Headers**:
  - `X-Pairing-Code`: The one-time code displayed in the gateway console.
- **Success Response** (200 OK):
  ```json
  {
    "paired": true,
    "persisted": true,
    "token": "...",
    "message": "Save this token — use it as Authorization: Bearer <token>"
  }
  ```

---

### Configuration Management

#### `GET /web/admin/config`
Returns a redacted view of the current gateway configuration.

- **Authentication**: Bearer Token required.
- **Success Response** (200 OK):
  ```json
  {
    "config": {
      "default_provider": "openrouter",
      "default_model": "anthropic/claude-3-5-sonnet",
      "default_temperature": 0.7,
      "memory_backend": "sqlite",
      "observability": { ... },
      "runtime": { "kind": "native" },
      "autonomy": { ... },
      "scheduler": { ... },
      "gateway": { ... },
      "channels": { ... }
    }
  }
  ```

#### `PUT /web/admin/config`
Updates selected configuration fields and persists them to `config.toml`.

- **Authentication**: Bearer Token required.
- **Request Body**: A JSON object containing the fields to update (partial updates supported).
- **Status Codes**:
  - `200 OK`: Configuration updated successfully.
  - `409 Conflict`: One or more requested changes require a gateway restart to take effect.
  ```json
  {
    "error": "One or more requested config changes require a gateway restart to take effect.",
    "restart_required": true,
    "fields": ["default_model", "gateway.port"]
  }
  ```

#### `GET /web/admin/options`
Returns the available options for configuration fields (e.g., supported backends, autonomy levels).

- **Authentication**: Bearer Token required.
- **Success Response** (200 OK):
  ```json
  {
    "memory_backends": ["sqlite", "lucid", "surreal-graphs", "markdown", "surreal", "none"],
    "observability_backends": ["none", "log", "prometheus", "otel"],
    "runtime_kinds": ["native", "docker"],
    "autonomy_levels": ["readonly", "supervised", "full"]
  }
  ```

---

### Messaging & Webhooks

#### `POST /webhook`
The primary endpoint for sending messages to the agent.

- **Authentication**: Bearer Token required.
- **Status Codes**:
  - `200 OK`: Message processed successfully.
  - `401 Unauthorized`: Missing or invalid bearer token or webhook secret.
  - `403 Forbidden`: Origin not allowed.
  - `429 Too Many Requests`: Rate limit exceeded.
  - `5xx Server Error`: Internal gateway or provider error.
- **Optional Header**: `X-Webhook-Secret` (if configured in `config.toml`).
- **Request Body**:
  ```json
  {
    "message": "your prompt here"
  }
  ```
- **Success Response** (200 OK):
  ```json
  {
    "response": "agent reply...",
    "model": "..."
  }
  ```

#### `POST /whatsapp` & `GET /whatsapp`
Handles WhatsApp Business API webhooks and verification.

- **Authentication**:
  - `GET`: Uses `hub.verify_token` for Meta verification.
  - `POST`: Requires `X-Hub-Signature-256` HMAC-SHA256 verification if `app_secret` is configured.
- **Status Codes**:
  - `200 OK`: Webhook processed or verification successful.
  - `401 Unauthorized`: Missing or invalid signature.
  - `403 Forbidden`: Verification token mismatch.
  - `404 Not Found`: WhatsApp channel not configured.
  - `5xx Server Error`: Internal gateway error.

---

### System Observability

#### `GET /health`
Returns the current status of the gateway and its components. Publicly accessible.

- **Success Response** (200 OK):
  ```json
  {
    "status": "ok",
    "paired": true,
    "runtime": { ... }
  }
  ```

#### `GET /metrics`
Returns Prometheus-formatted metrics for monitoring.

- **Authentication**: Bearer Token required.
- **Format**: `text/plain`
- **Status Codes**:
  - `200 OK`: Metrics returned successfully.
  - `401 Unauthorized`: Missing or invalid bearer token.
  - `403 Forbidden`: Origin not allowed.
  - `5xx Server Error`: Internal gateway error.
