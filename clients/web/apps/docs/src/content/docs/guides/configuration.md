---
title: Configuration Options
description: Canonical configuration surfaces for the Corvus agent runtime, Gradle properties, version catalogs, and environment variables.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: reference
---

Corvus has multiple configuration surfaces. This page covers the most commonly needed settings.

## Agent Runtime Configuration (`~/.corvus/config.toml`)

The agent runtime is configured through `~/.corvus/config.toml`, created during `corvus onboard`.

### Core Settings

| Key | Default | Description |
|---|---|---|
| `default_provider` | `openrouter` | Default AI provider |
| `default_model` | — | Default model name |
| `default_temperature` | — | Default temperature (0.0–2.0) |

### Autonomy

```toml
[autonomy]
level = "supervised"          # readonly | supervised | full
workspace_only = true         # Restrict to workspace directory
max_actions_per_hour = 20
require_approval_for_medium_risk = false
block_high_risk_commands = true
```

### Security & Sandbox

```toml
[security]
sandbox = "auto"              # auto | landlock | firejail | bubblewrap | docker | none
```

See the [Sandbox Isolation guide](./runtime-sandbox-isolation.md) for details.

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

### Memory

```toml
[memory]
backend = "sqlite"            # sqlite | lucid | markdown | none
auto_save = true
# embedding_provider = "openai"
# embedding_model = "..."
# vector_weight = 0.7
# keyword_weight = 0.3
```

### Agent Profiles

```toml
[agent]
compact_context = false
profile = "full"              # full | code | lite
max_tool_iterations = 10
max_history_messages = 50
# parallel_tools = false
# tool_dispatcher = "auto"    # auto | native | xml
```

### Model Routing

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

See the [Model Routing guide](./model-routing.md) for full details.

### Multimodal & Audio

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

### Scheduler & Cron

```toml
[scheduler]
enabled = true
max_tasks = 100
max_concurrent = 10
```

Cron tasks are stored in `~/.corvus/workspace/cron/`. Use `corvus cron` to manage them.

### MCP Servers

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

- MCP tools are namespaced as `mcp.<server>.<tool>`.
- MCP calls are deny-by-default in supervised flows.
- If one MCP server fails, healthy servers still register.

### Observability

```toml
[observability]
backend = "none"              # none | log | prometheus | otel
# otel_endpoint = "http://localhost:4318/v1/traces"
# otel_service_name = "corvus"
```

### Cost Tracking

```toml
[cost]
enabled = false
# session_limit_usd = 10.0
# daily_limit_usd = 50.0
# monthly_limit_usd = 200.0
# warn_at_percent = 80
```

### Updates

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

### Environment Variable Overrides

| Variable | Overrides |
|---|---|
| `CORVUS_API_KEY` | `api_key` |
| `CORVUS_PROVIDER` | `default_provider` |
| `CORVUS_MODEL` | `default_model` |
| `CORVUS_TEMPERATURE` | `default_temperature` |
| `CORVUS_MEMORY_BACKEND` | `memory.backend` |
| `CORVUS_WORKSPACE` | `workspace_dir` |
| `CORVUS_GATEWAY_PORT` | `gateway.port` |
| `CORVUS_GATEWAY_HOST` | `gateway.host` |
| `RUST_LOG` | Logging level (e.g., `info`, `debug`) |

Provider-specific env vars: `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
`GEMINI_API_KEY`, `OLLAMA_API_KEY`, and many more. See the [Providers reference](../clients/agent-runtime/providers/) for the full list.

---

## Gradle Properties

Build settings are in `gradle.properties`. Key values:

- **Java target**: 21
- **Kotlin**: Managed via version catalog
- **Parallel builds**: Enabled
- **Configuration cache**: Enabled

## Version Catalog (`libs.versions.toml`)

Central dependency management:

- `versions`: Single source of truth for version numbers.
- `libraries`: Individual dependency definitions.
- `bundles`: Groups of commonly-used dependencies.
- `plugins`: Gradle plugins.
