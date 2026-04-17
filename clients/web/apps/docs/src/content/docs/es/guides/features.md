---
title: Lista de Funcionalidades
description: Checklist de módulos, capacidades, features de build y superficies de integración soportadas en Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: reference
---

Esta página proporciona una lista completa de todas las funcionalidades, módulos y opciones
disponibles en este repositorio.

## Módulos

- [x] **clients/composeApp**: Módulo UI compartido en Compose Kotlin Multiplatform (desktop + iOS + Android).
- [x] **clients/androidApp**: App host nativa de Android para la UI compartida en Compose.
- [x] **clients/web**: Workspace de aplicaciones web (Astro/Vue: docs, dashboard, marketing).
- [x] **clients/agent-runtime**: Runtime del agente en Rust (gateway + daemon + CLI + 22+ proveedores + 14 canales + 32+ herramientas).
- [x] **modules/agent-core-kmp**: Core bootstrap compartido en Kotlin Multiplatform.
- [x] **modules/cerebro**: Servicio de memoria MCP independiente (SurrealDB, 13 herramientas de memoria, TUI opcional).
- [x] **gradle/build-logic**: Plugins de convención centralizados.
- [x] **gradle/aggregation**: Reportes agregados para pruebas y cobertura.
- [x] **gradle/versions**: Gestión de versiones de dependencias y comprobaciones de consistencia.

## Agent Runtime — Proveedores de IA (22+)

- [x] OpenRouter (agregador recomendado)
- [x] Anthropic (modelos Claude, setup-tokens, OAuth)
- [x] OpenAI (modelos GPT)
- [x] OpenAI Codex (OAuth)
- [x] Google Gemini (API key + reutilización de tokens OAuth del CLI)
- [x] Ollama (modelos locales)
- [x] LM Studio (local, `http://localhost:1234`)
- [x] Venice AI, Groq, Mistral, DeepSeek, xAI/Grok
- [x] Together AI, Fireworks AI, Perplexity, Cohere
- [x] GitHub Copilot (suscripción de GitHub)
- [x] Amazon Bedrock, Synthetic, OpenCode Zen, NVIDIA NIM
- [x] Vercel AI, Cloudflare AI, Astrai
- [x] Regionales: Moonshot/Kimi, GLM/Zhipu, MiniMax, Qwen/DashScope, Qianfan/Baidu, Z.AI
- [x] Endpoints personalizados: `custom:<URL>`, `anthropic-custom:<URL>`
- [x] Pools de proveedores (rotación multi-cuenta)
- [x] Enrutamiento de modelos con `[[model_routes]]` y clasificación de consultas
- [x] Wrapper de proveedor confiable (reintentos, backoff, cadenas de fallback)

## Agent Runtime — Canales de Comunicación (14)

- [x] CLI (terminal interactiva)
- [x] Telegram (polling)
- [x] Discord (gateway WebSocket)
- [x] Slack (API Web)
- [x] WhatsApp (webhooks de Meta Cloud API)
- [x] Signal (signal-cli)
- [x] iMessage (AppleScript en macOS)
- [x] Matrix
- [x] DingTalk (modo Stream)
- [x] QQ (SDK de bot de Tencent)
- [x] Lark/Feishu (WebSocket)
- [x] Email (IMAP/SMTP)
- [x] IRC
- [x] Mattermost

## Agent Runtime — Herramientas (32+)

- [x] `shell` — Ejecución de comandos con política de seguridad
- [x] `code_search` — Búsqueda de archivos en workspace (literal + regex)
- [x] `file_read` / `file_write` — Acceso al filesystem del workspace
- [x] `memory_store` / `memory_recall` / `memory_forget` — Memoria a largo plazo
- [x] `web_search` — Búsqueda web (proveedor Brave)
- [x] `http_request` — Llamadas API estructuradas
- [x] `browser` / `browser_open` — Navegación web / computer use
- [x] `screenshot` / `image_info` — Capacidades de visión
- [x] `git_operations` — Gestión de repositorios Git
- [x] `composio` — Integraciones de aplicaciones gestionadas
- [x] `delegate` — Delegación multi-agente
- [x] `pushover` — Notificaciones
- [x] `cron_add` / `cron_list` / `cron_remove` / `cron_update` / `cron_run` / `cron_runs` — Tareas programadas
- [x] `schedule` — Planificación de tareas
- [x] `hardware_board_info` / `hardware_memory_map` / `hardware_memory_read` — Introspección de hardware
- [x] Herramientas MCP (namespace `mcp.<server>.<tool>`, controlado por `mcp.enabled`)

## Agent Runtime — Memoria

- [x] Backend SQLite (búsqueda híbrida vector + FTS5)
- [x] Backend Lucid (SQLite con retrieval mejorado)
- [x] Backend Markdown (basado en archivos)
- [x] Backend None (sin persistencia)
- [x] Generación de embeddings (OpenAI, URL personalizada, noop)
- [x] Caché de respuestas (caché LRU SQLite)
- [x] Snapshots de memoria (exportar/importar)
- [x] Higiene de memoria (retención con throttling)
- [x] Integración Cerebro MCP (memoria a largo plazo vía servicio externo)

## Agent Runtime — Infraestructura

- [x] Gateway API (servidor HTTP Axum con health, pairing, webhooks, streaming SSE)
- [x] Planificador Cron (expresiones, one-shot, intervalo fijo, tareas diferidas)
- [x] Motor Heartbeat (señales de liveness periódicas)
- [x] Diagnóstico Doctor (daemon, scheduler, frescura de canales, validación de config)
- [x] Gestión de servicios OS (systemd en Linux, launchd en macOS)
- [x] Observabilidad (noop, log, prometheus, OpenTelemetry/OTLP, multi-backend)
- [x] Perfiles de auth (OAuth para Codex, setup tokens para Anthropic, gestión de perfiles)
- [x] Seguimiento de costos (precios por modelo, límites sesión/diario/mensual)
- [x] Sistema de actualización (auto-verificación, transacciones de instalación, auditoría)
- [x] Sistema de Skills (catálogo, instalación, lockfile, validación, sandbox)
- [x] Navegador de integraciones (50+ entradas en 9 categorías)
- [x] Migración (importación de memoria OpenClaw)
- [x] Composición de agentes (`agent build`, `agent run`, `agent new`)
- [x] Perfiles de capacidad (`full`, `code`, `lite`)
- [x] Enrutamiento de modelos y clasificación de consultas
- [x] Soporte multimodal (imágenes, límites configurables)
- [x] Soporte de audio (transcripción con Whisper.cpp)
- [x] Sistema de misiones (límites de runtime, presupuestos de pasos/costo)
- [x] Pipeline de evaluación pre-ejecución

## Agent Runtime — Seguridad

- [x] Política de seguridad (niveles de autonomía, allowlists de comandos, clasificación de riesgo)
- [x] Niveles de riesgo de comandos: bajo, medio, alto con rutas prohibidas
- [x] Protección contra path traversal (decodificación URL iterativa, bloqueo de null bytes)
- [x] Rate limiting (configurable, default 20/hora)
- [x] Almacén de secretos (cifrado AEAD con chacha20poly1305)
- [x] Guard de pairing (código de un solo uso de 6 dígitos, intercambio de bearer token)
- [x] Backends de sandbox: Landlock (kernel Linux), Firejail (user-space Linux), Bubblewrap, Docker
- [x] Auto-detección de sandbox con orden específico por plataforma
- [x] Aislamiento de sidecar computer-use con verificación de salud
- [x] Auditoría de todas las operaciones sensibles

## Agent Runtime — Hardware y Periféricos

- [x] Enumeración de dispositivos USB (nusb)
- [x] Soporte STM32/Nucleo (probe-rs, flashing de firmware)
- [x] Raspberry Pi GPIO
- [x] ESP32 bridge
- [x] Arduino Uno Q bridge
- [x] Soporte de dispositivos seriales
- [x] CLI de gestión de periféricos (`list`, `add`, `flash`, `setup`)

## Agent Runtime — Proveedores de Túnel

- [x] Cloudflare
- [x] Tailscale
- [x] Ngrok
- [x] Túneles personalizados

## Build y Calidad

- [x] **Plugins de Convención**: Lógica de construcción modular y reutilizable en `gradle/build-logic/`.
- [x] **Catálogo de Versiones**: Gestión centralizada de dependencias en `gradle/libs.versions.toml`.
- [x] **Análisis de Dependencias**: Herramientas para detectar dependencias no utilizadas o mal configuradas.
- [x] **Construcciones Reproducibles**: Bloqueo de dependencias con lockfiles de Gradle.
- [x] **Soporte Multilenguaje**: Kotlin, Rust, TypeScript/JavaScript.
- [x] **Formateo de Código**: Spotless (Kotlin/Java), Biome (web), rustfmt (Rust).
- [x] **Análisis Estático**: Detekt (Kotlin), Clippy (Rust), Biome (web).
- [x] **Pruebas**: Kotlin (JUnit 5 + Kover), Rust (cargo test), web (Vitest + Playwright).
- [x] **SBOM**: Generación de Software Bill of Materials.
- [x] **Git Hooks**: Comprobaciones automáticas de pre-commit vía `.githooks/`.

## Documentación

- [x] **Sitio Web Estático**: Construido con Astro y Starlight (bilingüe: en/es).
- [x] **Documentación de API**: Generada con Dokka (Kotlin/Java).
- [x] **Documentación en repo**: `AGENTS.md`, `CONTRIBUTING.md`, `README.md`.

## Despliegue y Distribución

- [x] **Shadow JAR**: Jars ejecutables "fat" con dependencias incluidas.
- [x] **Publicación en Maven**: Publicación preconfigurada en repositorios Maven.
- [x] **Soporte para BOM**: Bill of Materials para la alineación de dependencias.
- [x] **Runtime Docker**: Ejecución en contenedores configurables para sandboxing del agente.
- [x] **Auto-actualización**: Verificación de actualizaciones en runtime con transacciones de instalación y auditoría.
