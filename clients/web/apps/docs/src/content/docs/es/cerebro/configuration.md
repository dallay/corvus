---
title: Configuración de Cerebro
description: >-
  Configura los ajustes del servidor, backends de almacenamiento,
  autenticación y funcionalidades opcionales de Cerebro.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Configuración

Cerebro se configura mediante un archivo TOML o JSON pasado con
el flag `--config`, con variables de entorno para valores
sensibles. Cuando no se proporciona archivo de configuración,
se aplican todos los valores por defecto.

## Archivo de Configuración

```bash
cerebro serve --config cerebro.toml
```

Formatos soportados: `.toml` y `.json`.

## Referencia Completa de Configuración

```toml
# Servidor
host = "127.0.0.1"       # Dirección de enlace (default: 127.0.0.1)
port = 4040               # Puerto de enlace (default: 4040)
scheme = "http"            # Esquema URL (auto-detectado)

# Autenticación
# auth_token = "..."      # Usar CEREBRO_AUTH_TOKEN env var
# audit_token = "..."     # Usar CEREBRO_AUDIT_TOKEN env var

# Almacenamiento
storage_mode = "embedded_surreal"  # Ver Modos de Almacenamiento
storage_fallback = "none"          # Respaldo si falla el primario
storage_path = "./cerebro-data"    # Ruta para almacenamiento en disco

# SurrealDB (embebido)
[surreal]
namespace = "cerebro"              # Namespace de SurrealDB
database = "cerebro"               # Base de datos de SurrealDB
# storage_path = "..."             # Ruta personalizada de RocksDB
# username = "root"                # Requerido en modo embebido
# password = "..."                 # Requerido en modo embebido
# embedded_bind = "127.0.0.1:0"   # Dirección del motor embebido
# embedded_allow_non_loopback = false

# Worker en segundo plano (experimental)
[worker]
embeddings_enabled = false
enrichment_enabled = false

# Panel TUI
[tui]
enabled = false
event_buffer = 256
refresh_ms = 500
max_payload_bytes = 4096
redact_fields = [
  "password", "secret", "token", "auth",
  "authorization", "api_key", "apikey",
  "cookie", "session", "credential",
]
```

## Ajustes del Servidor

| Campo    | Tipo   | Default       | Descripción                      |
|----------|--------|---------------|----------------------------------|
| `host`   | String | `127.0.0.1`   | Dirección de enlace              |
| `port`   | u16    | `4040`        | Puerto de enlace                 |
| `scheme` | String | auto-detectado| `http` para loopback, sino `https`|

El endpoint MCP está disponible en
`{scheme}://{host}:{port}/mcp`.

## Modos de Almacenamiento

La postura de producción durable soportada actualmente es de nodo único y local-first. SurrealDB
embebido es el modo durable soportado por defecto, `disk` es una alternativa durable local al nodo,
`in_memory` no es durable y es solo para pruebas, y `remote_surreal` no está soportado en esta
versión.

| Modo              | Valor              | Descripción                                        |
|-------------------|--------------------|-----------------------------------------------------|
| SurrealDB embebido| `embedded_surreal` | Modo durable de nodo único soportado por defecto.   |
| En memoria        | `in_memory`        | Sin persistencia. Solo CI/dev/pruebas.              |
| Disco             | `disk`             | Alternativa durable local al nodo.                  |
| SurrealDB remoto  | `remote_surreal`   | No soportado en esta versión.                       |

:::note
SurrealDB embebido requiere que `surreal.username` y
`surreal.password` estén configurados. El motor embebido se enlaza
solo a loopback por defecto.
:::

### Respaldo de Almacenamiento

Si el almacenamiento primario falla al inicializarse, Cerebro
puede usar un respaldo alternativo:

| Respaldo        | Valor            | Descripción                                         |
|-----------------|------------------|------------------------------------------------------|
| Ninguno         | `none`           | Default. Falla si el primario falla.                 |
| En memoria      | `in_memory`      | Respaldo no durable para CI/dev/emergencias.         |
| Disco           | `disk`           | Respaldo durable local al nodo.                      |
| SurrealDB remoto| `remote_surreal` | No soportado en esta versión.                        |

## Variables de Entorno

| Variable              | Sobrescribe        | Notas                    |
|-----------------------|--------------------|--------------------------|
| `CEREBRO_AUTH_TOKEN`  | `auth_token`       | Requerido en producción  |
| `CEREBRO_AUDIT_TOKEN` | `audit_token`      | Auditoría opcional       |
| `CEREBRO_TUI_ENABLED` | `tui.enabled`      | `1`, `true`, `yes`, `on` |
| `RUST_LOG`            | Nivel de log       | ej. `info`, `debug`      |

:::tip
Siempre configura `CEREBRO_AUTH_TOKEN` mediante variable de entorno
en lugar del archivo de configuración para evitar filtrar secretos.
:::

## Autenticación

Cuando `auth_token` está configurado (vía env o config), todas las
peticiones MCP deben incluir un header `Authorization` con el token
correcto. Las peticiones sin token válido son rechazadas.

## Configuración de SurrealDB

La sección `[surreal]` controla el motor SurrealDB embebido:

| Campo                        | Tipo    | Default      | Descripción              |
|------------------------------|---------|--------------|--------------------------|
| `namespace`                  | String  | `cerebro`    | Namespace de SurrealDB   |
| `database`                   | String  | `cerebro`    | Base de datos            |
| `storage_path`               | String  | —            | Ruta de RocksDB          |
| `username`                   | String  | —            | Requerido en embebido    |
| `password`                   | Secret  | —            | Requerido en embebido    |
| `embedded_bind`              | String  | —            | Dirección del motor      |
| `embedded_allow_non_loopback`| bool    | `false`      | Permitir enlace externo  |

## Configuración del Panel TUI

La sección `[tui]` configura el panel de terminal opcional:

| Campo              | Tipo     | Default  | Descripción                    |
|--------------------|----------|----------|--------------------------------|
| `enabled`          | bool     | `false`  | Activar panel TUI              |
| `event_buffer`     | usize    | `256`    | Tamaño del buffer de eventos   |
| `refresh_ms`       | u64      | `500`    | Intervalo de refresco (ms)     |
| `max_payload_bytes`| usize    | `4096`   | Tamaño máximo de payload       |
| `redact_fields`    | [String] | ver arriba| Campos a redactar en pantalla |

:::note
El TUI requiere que el binario se compile con `--features tui`.
Actívalo en runtime con el flag `--tui` o `CEREBRO_TUI_ENABLED=1`.
:::

## Ejemplo Mínimo de Producción

```toml
host = "0.0.0.0"
port = 4040
storage_mode = "embedded_surreal"

[surreal]
namespace = "cerebro"
database = "cerebro"
username = "root"
password = "cambiar-en-produccion"
```

```bash
CEREBRO_AUTH_TOKEN=mi-token-secreto cerebro serve --config cerebro.toml
```
