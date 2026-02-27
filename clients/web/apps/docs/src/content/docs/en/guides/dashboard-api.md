---
title: Dashboard API
---

The Corvus Gateway provides a REST API for the operator dashboard and external integrations. This API allows for configuration management, client pairing, and message handling via webhooks.

## Base URL

By default, the gateway listens on:
`http://127.0.0.1:3000`

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

- **Format**: `text/plain`
