---
title: Referencia de Herramientas MCP de Cerebro
description: >-
  Referencia de las 13 herramientas de memoria de Cerebro
  expuestas mediante el Model Context Protocol.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: reference
---

# Referencia de Herramientas MCP

Cerebro expone 13 herramientas de memoria vía JSON-RPC sobre HTTP
en `POST /mcp`. Todas las peticiones usan el protocolo MCP
(JSON-RPC 2.0).

## Formato de Petición

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "<nombre_herramienta>",
    "arguments": { ... }
  }
}
```

## Estado de las Herramientas

| Estado        | Significado                             |
|---------------|-----------------------------------------|
| Implementada  | Disponible y funcional                  |
| Planificada   | Definida pero retorna `NotImplemented`  |

---

## Herramientas Implementadas

### `mem_save`

Guarda una nueva observación en memoria.

**Parámetros:**

| Campo       | Tipo   | Requerido | Descripción                   |
|-------------|--------|-----------|-------------------------------|
| `content`   | String | Sí        | Contenido de la observación   |
| `what`      | String | No        | Qué se observó                |
| `why`       | String | No        | Por qué es importante         |
| `where`     | String | No        | Contexto o fuente             |
| `scope`     | String | No        | Identificador de ámbito       |
| `topic_key` | String | No        | Tema para organización        |

**Retorna:** `memory_id`, `status`

---

### `mem_search`

Busca memorias almacenadas por consulta.

**Parámetros:**

| Campo             | Tipo   | Requerido | Descripción                   |
|-------------------|--------|-----------|-------------------------------|
| `query`           | String | Sí        | Consulta de búsqueda          |
| `limit`           | Number | No        | Máximo de resultados          |
| `scope`           | String | No        | Filtrar por ámbito            |
| `topic_key`       | String | No        | Filtrar por tema              |
| `include_deleted` | bool   | No        | Incluir eliminados suavemente |

**Retorna:** `results_count`, `truncated`, lista de memorias

---

### `mem_delete`

Elimina una observación de memoria.

**Parámetros:**

| Campo         | Tipo   | Requerido | Descripción                       |
|---------------|--------|-----------|-----------------------------------|
| `memory_id`   | String | Sí        | ID de la memoria a eliminar       |
| `topic_key`   | String | No        | Filtro por tema                   |
| `hard_delete` | bool   | No        | Eliminar permanentemente (vs suave)|

**Retorna:** `memory_id`, `status`, `deleted`

---

### `mem_get_observation`

Recupera una memoria específica por ID.

**Parámetros:**

| Campo             | Tipo   | Requerido | Descripción                   |
|-------------------|--------|-----------|-------------------------------|
| `memory_id`       | String | Sí        | ID de la memoria a recuperar  |
| `include_deleted` | bool   | No        | Incluir si fue eliminada      |

**Retorna:** `memory_id`, `status`, datos completos de la
observación

---

### `mem_update`

Actualiza una observación de memoria existente.

**Parámetros:**

| Campo       | Tipo   | Requerido | Descripción                   |
|-------------|--------|-----------|-------------------------------|
| `memory_id` | String | Sí        | ID de la memoria a actualizar |
| `content`   | String | No        | Contenido actualizado         |
| `what`      | String | No        | Qué actualizado               |
| `why`       | String | No        | Por qué actualizado           |
| `where`     | String | No        | Contexto actualizado          |

**Retorna:** `memory_id`, `status`

---

### `mem_suggest_topic_key`

Sugiere una clave de tema para organizar memorias.

**Parámetros:**

| Campo   | Tipo   | Requerido | Descripción                   |
|---------|--------|-----------|-------------------------------|
| `scope` | String | No        | Ámbito para la sugerencia     |
| `input` | String | No        | Texto de entrada              |

**Retorna:** `topic_key`, `candidates_count`

---

### `mem_stats`

Obtiene estadísticas del almacenamiento de memoria.

**Parámetros:** Ninguno

**Retorna:**

| Campo               | Tipo   | Descripción                     |
|---------------------|--------|---------------------------------|
| `memory_count`      | Number | Total de memorias almacenadas   |
| `session_count`     | Number | Total de sesiones               |
| `prompt_count`      | Number | Total de prompts guardados      |
| `worker_enabled`    | bool   | Estado del worker en segundo plano|
| `worker_queue_depth`| Number | Tareas pendientes del worker    |

---

### `mem_timeline`

Obtiene una línea temporal cronológica de memorias.

**Parámetros:**

| Campo             | Tipo   | Requerido | Descripción                   |
|-------------------|--------|-----------|-------------------------------|
| `memory_id`       | String | No        | Centrar la línea en este ID   |
| `before`          | String | No        | Entradas antes de esta fecha  |
| `after`           | String | No        | Entradas después de esta fecha|
| `include_deleted` | bool   | No        | Incluir eliminados suavemente |

**Retorna:** `items_count`, lista de entradas de la línea temporal

---

## Herramientas Planificadas

:::caution
Estas herramientas están definidas en el esquema pero aún no están
implementadas. Llamarlas retorna un error `NotImplemented`.
:::

### `mem_save_prompt`

Guarda una plantilla de prompt para reutilización.

### `mem_session_start`

Inicia una nueva sesión de memoria para agrupar observaciones.

### `mem_session_end`

Finaliza una sesión de memoria activa.

### `mem_session_summary`

Genera un resumen de una sesión completada.

### `mem_context`

Recupera memoria contextual para la conversación actual.

---

## Esquemas JSON

Las definiciones de esquemas JSON legibles por máquina para
todas las herramientas están disponibles en el repositorio en
[`guides/cerebro/mcp-schema/`](../guides/cerebro/mcp-schema/).

## Ejemplo: Petición/Respuesta Completa

```bash
curl -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "mem_save",
      "arguments": {
        "content": "El usuario prefiere modo oscuro",
        "topic_key": "preferencias",
        "what": "Preferencia de interfaz",
        "why": "Personalización"
      }
    }
  }'
```

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "memory_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "saved"
  }
}
```
