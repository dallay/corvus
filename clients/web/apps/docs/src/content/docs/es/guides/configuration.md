---
title: Opciones de Configuración
description: Superficies de configuración canónicas para el runtime del agente Corvus, propiedades Gradle, catálogos de versiones y variables de entorno.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: reference
---

Corvus tiene múltiples superficies de configuración. Esta página cubre las más utilizadas.

## Configuración del Agent Runtime (`~/.corvus/config.toml`)

El runtime del agente se configura a través de `~/.corvus/config.toml`, creado durante `corvus onboard`.

### Configuración Base

| Clave | Default | Descripción |
|---|---|---|
| `default_provider` | `openrouter` | Proveedor de IA por defecto |
| `default_model` | — | Nombre del modelo por defecto |
| `default_temperature` | — | Temperatura por defecto (0.0–2.0) |

### Autonomía

```toml
[autonomy]
level = "supervised"          # readonly | supervised | full
workspace_only = true         # Restringir al directorio workspace
max_actions_per_hour = 20
require_approval_for_medium_risk = false
block_high_risk_commands = true
```

### Seguridad y Sandbox

```toml
[security]
sandbox = "auto"              # auto | landlock | firejail | bubblewrap | docker | none
```

Ver la [guía de Aislamiento de Sandbox](./runtime-sandbox-isolation.md) para detalles.

### Runtime

```toml
[runtime]
kind = "native"               # native | docker
# docker.image = "..."
# docker.network = "..."
# docker.memory_limit_mb = 1024
# docker.cpu_limit = 1.0
```

### Gateway

```toml
[gateway]
port = 3000
host = "127.0.0.1"
require_pairing = true
# trust_forwarded_headers = false
# webhook_dispatcher_enabled = false
```

### Memoria

```toml
[memory]
backend = "sqlite"            # sqlite | lucid | markdown | none
auto_save = true
# embedding_provider = "openai"
# embedding_model = "..."
# vector_weight = 0.7
# keyword_weight = 0.3
```

### Perfiles del Agente

```toml
[agent]
compact_context = false
profile = "full"              # full | code | lite
max_tool_iterations = 10
max_history_messages = 50
# parallel_tools = false
# tool_dispatcher = "auto"    # auto | native | xml
```

### Enrutamiento de Modelos

```toml
[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"

[query_classification]
enabled = true
```

Ver la [guía de Enrutamiento de Modelos](./model-routing.md) para detalles completos.

### Multimodal y Audio

```toml
[multimodal]
enabled = false
# vision_model_hint = "vision"
# max_image_bytes = 10485760

[audio]
enabled = false
# transcription_model = "whisper"
# whisper_binary = "/usr/local/bin/whisper"
```

### Planificador y Cron

```toml
[scheduler]
enabled = true
max_tasks = 100
max_concurrent = 10
```

Las tareas Cron se almacenan en `~/.corvus/workspace/cron/`. Usa `corvus cron` para gestionarlas.

### Servidores MCP

```toml
[mcp]
enabled = false

[[mcp.servers]]
name = "docs"
enabled = true
command = "mcp-docs"
args = ["serve"]
startup_timeout_ms = 5000
call_timeout_ms = 30000
output_limit_bytes = 65536
```

- Las herramientas MCP tienen namespace `mcp.<server>.<tool>`.
- Las llamadas MCP son denegadas por defecto en flujos supervisados.
- Si un servidor MCP falla, los servidores sanos se registran igualmente.

### Observabilidad

```toml
[observability]
backend = "none"              # none | log | prometheus | otel
# otel_endpoint = "http://localhost:4318/v1/traces"
# otel_service_name = "corvus"
```

### Seguimiento de Costos

```toml
[cost]
enabled = false
# session_limit_usd = 10.0
# daily_limit_usd = 50.0
# monthly_limit_usd = 200.0
# warn_at_percent = 80
```

### Actualizaciones

```toml
[updates]
enabled = false
# auto_install = false
# channel_visibility_enabled = true
```

### Skills

```toml
[skills]
catalog_repo_url = "https://github.com/dallay/corvus-skills"
verify_integrity = true
# scan_threshold = 50
```

### Overrides de Variables de Entorno

| Variable | Override |
|---|---|
| `CORVUS_API_KEY` | `api_key` |
| `CORVUS_PROVIDER` | `default_provider` |
| `CORVUS_MODEL` | `default_model` |
| `CORVUS_TEMPERATURE` | `default_temperature` |
| `CORVUS_MEMORY_BACKEND` | `memory.backend` |
| `CORVUS_WORKSPACE` | `workspace_dir` |
| `CORVUS_GATEWAY_PORT` | `gateway.port` |
| `CORVUS_GATEWAY_HOST` | `gateway.host` |
| `RUST_LOG` | Nivel de logging (ej. `info`, `debug`) |

Variables de proveedor: `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
`GEMINI_API_KEY`, `OLLAMA_API_KEY`, y muchas más. Ver la [referencia de Proveedores](../clients/agent-runtime/providers/) para la lista completa.

---

## Propiedades de Gradle

Configuración de build en `gradle.properties`. Valores clave:

- **Java target**: 21
- **Kotlin**: Gestionado vía catálogo de versiones
- **Builds paralelos**: Habilitados
- **Configuration cache**: Habilitado

## Catálogo de Versiones (`libs.versions.toml`)

Gestión centralizada de dependencias:

- `versions`: Fuente única de verdad para números de versión.
- `libraries`: Definiciones individuales de dependencias.
- `bundles`: Grupos de dependencias de uso común.
- `plugins`: Plugins de Gradle.
