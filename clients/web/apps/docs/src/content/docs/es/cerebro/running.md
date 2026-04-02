---
title: Ejecutar Cerebro
description: >-
  Inicia el servicio de memoria MCP Cerebro, verifica que está
  funcionando y comprende el comportamiento de apagado.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Ejecutar Cerebro

Cerebro funciona como un servicio HTTP independiente que expone
herramientas MCP vía JSON-RPC. Esta página cubre cómo iniciar,
verificar y detener el servicio.

## Inicio Rápido

### Usando un binario

```bash
cerebro serve
```

Esto inicia el servidor en `127.0.0.1:4040` con la configuración
por defecto.

### Con archivo de configuración

```bash
cerebro serve --config cerebro.toml
```

Consulta [Configuración](configuration.md) para todas las opciones.

### Con panel TUI

```bash
cerebro serve --tui
```

O mediante variable de entorno:

```bash
CEREBRO_TUI_ENABLED=1 cerebro serve
```

:::note
El TUI requiere que el binario se compile con `--features tui`.
:::

## Docker

```bash
docker run -d \
  --name cerebro \
  -v cerebro-data:/cerebro-data \
  -p 4040:4040 \
  dallay/cerebro:latest
```

Para pasar una configuración personalizada:

```bash
docker run -d \
  --name cerebro \
  -v cerebro-data:/cerebro-data \
  -v ./cerebro.toml:/etc/cerebro/cerebro.toml \
  -p 4040:4040 \
  -e CEREBRO_AUTH_TOKEN=mi-token-secreto \
  dallay/cerebro:latest \
  cerebro serve --config /etc/cerebro/cerebro.toml
```

## Verificar el Servicio

Una vez que Cerebro está corriendo, envía una petición MCP de
prueba:

```bash
curl -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "mem_stats",
      "arguments": {}
    }
  }'
```

Una respuesta exitosa devuelve estadísticas de memoria:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "memory_count": 0,
    "session_count": 0,
    "prompt_count": 0
  }
}
```

## Endpoint MCP

El endpoint MCP siempre está en:

```
POST http://{host}:{port}/mcp
```

Por defecto: `http://127.0.0.1:4040/mcp`

## Logging

Cerebro usa `tracing` con `RUST_LOG` para controlar el nivel de
log:

```bash
# Por defecto (nivel info)
cerebro serve

# Logging de depuración
RUST_LOG=debug cerebro serve

# Debug específico de Cerebro
RUST_LOG=cerebro=debug cerebro serve
```

## Apagado Graceful

Cerebro maneja las señales de apagado de forma limpia:

- **Ctrl+C** — envía SIGINT
- **SIGTERM** — señal estándar de contenedores/systemd

Al apagarse, Cerebro:

1. Deja de aceptar nuevas conexiones
2. Completa las peticiones en curso
3. Vacía el almacenamiento
4. Sale limpiamente

```bash
# Detener un contenedor Docker de forma limpia
docker stop cerebro
```

## Enlace de Red

| Escenario       | Host          | Notas                          |
|-----------------|---------------|--------------------------------|
| Desarrollo local| `127.0.0.1`   | Default. Solo loopback.        |
| Docker          | `0.0.0.0`     | Requerido para puerto del contenedor.|
| Producción      | `0.0.0.0`     | Enlace a todas las interfaces. |

:::caution
Al enlazar a `0.0.0.0`, siempre configura `CEREBRO_AUTH_TOKEN`
para prevenir acceso sin autenticación.
:::
