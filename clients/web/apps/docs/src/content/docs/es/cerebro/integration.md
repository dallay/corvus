---
title: Integración de Cerebro con Corvus
description: >-
  Conecta Cerebro como backend de memoria a largo plazo para el
  runtime de agentes Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Integración con Corvus

Cerebro se integra con el runtime de agentes Corvus como su
backend de memoria a largo plazo. El runtime se conecta a Cerebro
mediante MCP (JSON-RPC sobre HTTP) usando la sección de
configuración `[memory.cerebro]`.

## Cómo Funciona

```
Runtime Corvus ──MCP/HTTP──▶ Servicio Cerebro ──▶ SurrealDB
  (corvus)                    (cerebro serve)      (embebido)
```

El runtime envía operaciones de memoria (guardar, buscar,
recuperar) a Cerebro mediante llamadas MCP. Cerebro maneja la
persistencia, indexación y organización por temas de forma
independiente.

## Configuración del Runtime

Añade la sección `[memory.cerebro]` a tu archivo de configuración
de Corvus:

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
request_timeout_ms = 30000
allow_insecure_loopback = true
```

### Campos de Configuración

| Campo                     | Tipo   | Default | Descripción                      |
|---------------------------|--------|---------|----------------------------------|
| `endpoint`                | String | —       | URL del endpoint MCP de Cerebro  |
| `auth_token`              | String | —       | Token de autenticación           |
| `request_timeout_ms`      | u64    | `30000` | Timeout de petición en ms        |
| `allow_insecure_loopback` | bool   | `false` | Permitir HTTP plano en loopback  |

:::note
Cuando `endpoint` no está configurado, la integración con Cerebro
está desactivada. El runtime opera sin memoria a largo plazo.
:::

## Variables de Entorno

| Variable                                 | Sobrescribe                         |
|------------------------------------------|-------------------------------------|
| `CORVUS_CEREBRO_ENDPOINT`                | `memory.cerebro.endpoint`           |
| `CORVUS_CEREBRO_AUTH_TOKEN`              | `memory.cerebro.auth_token`         |
| `CORVUS_CEREBRO_TIMEOUT_MS`             | `memory.cerebro.request_timeout_ms` |
| `CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK`| `memory.cerebro.allow_insecure_loopback` |

:::tip
Usa variables de entorno para `auth_token` y evita almacenar
secretos en archivos de configuración:

```bash
CORVUS_CEREBRO_AUTH_TOKEN=mi-secreto corvus
```
:::

## Inicio Rápido

### 1. Iniciar Cerebro

```bash
CEREBRO_AUTH_TOKEN=secreto-compartido cerebro serve
```

### 2. Configurar Corvus

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
allow_insecure_loopback = true
```

```bash
CORVUS_CEREBRO_AUTH_TOKEN=secreto-compartido corvus
```

### 3. Verificar Conexión

Los logs del runtime mostrarán Cerebro como configurado:

```
INFO cerebro_configured=true endpoint="http://127.0.0.1:4040/mcp"
```

## Ejemplo con Docker Compose

```yaml
services:
  cerebro:
    image: dallay/cerebro:latest
    volumes:
      - cerebro-data:/cerebro-data
    environment:
      - CEREBRO_AUTH_TOKEN=secreto-compartido
    ports:
      - "4040:4040"

  corvus:
    image: dallay/corvus:latest
    environment:
      - CORVUS_CEREBRO_ENDPOINT=http://cerebro:4040/mcp
      - CORVUS_CEREBRO_AUTH_TOKEN=secreto-compartido
      - CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK=true
    depends_on:
      - cerebro

volumes:
  cerebro-data:
```

## Consideraciones de Seguridad

- **Los tokens deben coincidir** entre Corvus y Cerebro.
- **HTTPS se aplica** por defecto para endpoints no loopback.
  Usa `allow_insecure_loopback = true` solo para desarrollo local
  o redes internas de Docker.
- **El token se redacta** en la salida de debug y los logs.

## Páginas Relacionadas

- [Configuración](configuration.md) — Configuración del servidor
- [Ejecución](running.md) — Iniciar el servicio Cerebro
- [Migración](migration.md) — Migrar desde memoria heredada
