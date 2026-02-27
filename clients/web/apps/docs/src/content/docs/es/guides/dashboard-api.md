---
title: API del Dashboard
---

El Gateway de Corvus proporciona una API REST para el dashboard del operador e integraciones externas. Esta API permite la gestión de la configuración, el emparejamiento de clientes y el manejo de mensajes a través de webhooks.

## URL Base

Por defecto, el gateway escucha en:
`http://127.0.0.1:3000`

## Autenticación

La API utiliza dos mecanismos principales de autenticación:

### 1. Código de Emparejamiento (Pairing Code)
Se utiliza una vez para intercambiar un código de un solo uso por un token de portador (bearer token) persistente.
- **Cabecera (Header)**: `X-Pairing-Code: <CÓDIGO>`

### 2. Token de Portador (Bearer Token)
Se utiliza para todas las solicitudes administrativas y de webhook una vez que el cliente está emparejado.
- **Cabecera (Header)**: `Authorization: Bearer <TOKEN>`

---

## Endpoints

### Emparejamiento

#### `POST /pair`
Intercambia un código de emparejamiento de un solo uso por un token de portador persistente.

- **Cabeceras**:
  - `X-Pairing-Code`: El código de un solo uso que se muestra en la consola del gateway.
- **Respuesta Exitosa** (200 OK):
  ```json
  {
    "paired": true,
    "persisted": true,
    "token": "...",
    "message": "Save this token — use it as Authorization: Bearer <token>"
  }
  ```

---

### Gestión de Configuración

#### `GET /web/admin/config`
Devuelve una vista editada de la configuración actual del gateway.

- **Autenticación**: Requiere Token de Portador.
- **Respuesta Exitosa** (200 OK):
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
Actualiza campos seleccionados de la configuración y los persiste en `config.toml`.

- **Autenticación**: Requiere Token de Portador.
- **Cuerpo de la Solicitud**: Un objeto JSON que contiene los campos a actualizar (se admiten actualizaciones parciales).
- **Códigos de Estado**:
  - `200 OK`: Configuración actualizada correctamente.
  - `409 Conflict`: Uno o más cambios solicitados requieren un reinicio del gateway para surtir efecto.
  ```json
  {
    "error": "One or more requested config changes require a gateway restart to take effect.",
    "restart_required": true,
    "fields": ["default_model", "gateway.port"]
  }
  ```

#### `GET /web/admin/options`
Devuelve las opciones disponibles para los campos de configuración (por ejemplo, backends soportados, niveles de autonomía).

- **Autenticación**: Requiere Token de Portador.
- **Respuesta Exitosa** (200 OK):
  ```json
  {
    "memory_backends": ["sqlite", "lucid", "surreal-graphs", "markdown", "surreal", "none"],
    "observability_backends": ["none", "log", "prometheus", "otel"],
    "runtime_kinds": ["native", "docker"],
    "autonomy_levels": ["readonly", "supervised", "full"]
  }
  ```

---

### Mensajería y Webhooks

#### `POST /webhook`
El endpoint principal para enviar mensajes al agente.

- **Autenticación**: Requiere Token de Portador.
- **Cabecera Opcional**: `X-Webhook-Secret` (si está configurado en `config.toml`).
- **Cuerpo de la Solicitud**:
  ```json
  {
    "message": "tu mensaje aquí"
  }
  ```
- **Respuesta Exitosa** (200 OK):
  ```json
  {
    "response": "respuesta del agente...",
    "model": "..."
  }
  ```

#### `POST /whatsapp` & `GET /whatsapp`
Maneja los webhooks y la verificación de la API de WhatsApp Business.

---

### Observabilidad del Sistema

#### `GET /health`
Devuelve el estado actual del gateway y sus componentes. Accesible al público.

- **Respuesta Exitosa** (200 OK):
  ```json
  {
    "status": "ok",
    "paired": true,
    "runtime": { ... }
  }
  ```

#### `GET /metrics`
Devuelve métricas en formato Prometheus para su monitorización.

- **Formato**: `text/plain`
