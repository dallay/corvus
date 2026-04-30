---
title: Operaciones de Cerebro
description: >-
  Operaciones del día a día para Cerebro: modos de almacenamiento,
  monitoreo, estrategias de respaldo y panel TUI.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Operaciones

Esta página cubre las operaciones del día a día para ejecutar
Cerebro en producción: gestión de almacenamiento, monitoreo,
respaldos y resolución de problemas.

## Modos de Almacenamiento

La postura de producción durable soportada en esta versión es de nodo único y local-first.
SurrealDB embebido es el modo durable soportado por defecto. `disk` es una alternativa
durable local al nodo. `in_memory` no es durable y solo es apropiado para CI/dev/emergencias.

| Modo              | Persistencia | Rendimiento | Caso de Uso                         |
|-------------------|--------------|-------------|-------------------------------------|
| SurrealDB embebido | Durable      | Alto        | Producción (default, nodo único)    |
| Disco             | Durable      | Moderado    | Alternativa durable local al nodo   |
| En memoria        | Ninguna      | Máximo      | Solo CI/dev/pruebas                 |

`remote_surreal`, la persistencia remota compartida y la durabilidad HA multi-nodo no están
soportadas en esta versión. No configures `remote_surreal` como modo primario ni como
fallback.

### SurrealDB Embebido (Default)

Usa RocksDB como motor de almacenamiento. Los datos persisten
entre reinicios.

```toml
storage_mode = "embedded_surreal"

[surreal]
namespace = "cerebro"
database = "cerebro"
username = "root"
password = "contraseña-segura"
```

:::caution
Estas son credenciales de ejemplo. En producción, usa contraseñas
seguras y carga valores sensibles mediante variables de entorno.
:::

La ruta de almacenamiento por defecto es el directorio de trabajo.
Sobrescríbela con `surreal.storage_path`:

```toml
[surreal]
storage_path = "/var/lib/cerebro/data"
```

### Modo En Memoria

Sin persistencia. Todos los datos se pierden al reiniciar. Usa
solo para pruebas y desarrollo.

```toml
storage_mode = "in_memory"
```

### Modo Disco

Persistencia basada en archivos. Menos rendimiento que SurrealDB
pero más simple de gestionar.

```toml
storage_mode = "disk"
storage_path = "/var/lib/cerebro/disk-data"
```

## Fallback de Almacenamiento

Configura un backend alternativo si el primario falla al
inicializarse:

```toml
storage_mode = "embedded_surreal"
storage_fallback = "in_memory"
```

Esto puede mantener Cerebro en ejecución aunque el backend
primario no esté disponible. La pérdida de persistencia ocurre
solo si el backend de fallback no ofrece persistencia (por
ejemplo, `in_memory`). `remote_surreal`
no está soportado en esta versión y no es una opción de recuperación en producción.

## Panel TUI

Cerebro incluye un panel de terminal opcional para monitoreo en
tiempo real de llamadas a herramientas y exploración de memorias.

### Activar el TUI

El TUI requiere que el binario se compile con la feature `tui`:

```bash
cargo build --features tui
```

Luego inicia con el flag `--tui` o variable de entorno:

```bash
cerebro serve --tui

# O mediante variable de entorno
CEREBRO_TUI_ENABLED=1 cerebro serve
```

### Configuración del TUI

```toml
[tui]
enabled = true
event_buffer = 256      # Tamaño del buffer de eventos
refresh_ms = 500         # Intervalo de refresco de pantalla
max_payload_bytes = 4096 # Tamaño máximo de payload mostrado
redact_fields = [        # Campos ocultos en el TUI
  "password", "secret", "token", "auth",
  "authorization", "api_key", "apikey",
  "cookie", "session", "credential",
]
```

### Qué Muestra el TUI

- Feed en vivo de llamadas a herramientas con tiempos
- Payloads de petición/respuesta (redactados para campos sensibles)
- Estadísticas de memoria
- Estado del almacenamiento

:::note
El TUI valida que no haya otros listeners de red en conflicto
antes de iniciar. Si la validación falla, Cerebro inicia sin el
TUI y registra una advertencia.
:::

## Monitoreo

### Logging

Cerebro usa `tracing` para logging estructurado. Controla los
niveles de log con `RUST_LOG`:

```bash
# Producción (default)
RUST_LOG=info cerebro serve

# Debug de módulos específicos
RUST_LOG=cerebro=debug,surrealdb=warn cerebro serve

# Nivel trace para resolución de problemas
RUST_LOG=cerebro=trace cerebro serve
```

### Verificación de Salud

Envía una llamada `mem_stats` para verificar que el servicio
responde:

```bash
curl -s -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{}}}' \
  | jq .result
```

:::note
Si la autenticación está habilitada, incluye el header de auth:
```bash
curl -s -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <TOKEN>" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{}}}' \
  | jq .result
```
:::

Úsalo en health checks de contenedores o sondas de monitoreo.

## Respaldo y Restauración

### SurrealDB Embebido

Los datos de SurrealDB embebido están en el directorio de trabajo
o la ruta especificada por `surreal.storage_path`. Para respaldar:

```bash
# Detener Cerebro primero para consistencia
docker stop cerebro

# Copiar el directorio de datos
cp -r /ruta/a/cerebro-data /ruta/a/respaldo/

# Reiniciar
docker start cerebro
```

### Volúmenes Docker

```bash
# Respaldar un volumen Docker
docker run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/cerebro-backup.tar.gz -C /data .

# Restaurar
docker run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar xzf /backup/cerebro-backup.tar.gz -C /data
```

## Resolución de Problemas

### Problemas Comunes

| Síntoma | Causa | Solución |
|---------|-------|----------|
| Conexión rechazada en :4040 | Cerebro no está corriendo | Iniciar con `cerebro serve` |
| Error de auth en llamadas MCP | Token no coincide | Verificar `CEREBRO_AUTH_TOKEN` |
| "embedded surrealdb credentials are required" | Falta auth de surreal | Configurar `surreal.username` y `surreal.password` |
| "embedded surrealdb must bind to loopback only" | Validación de seguridad | Preferir loopback con proxy inverso. Solo usar `surreal.embedded_allow_non_loopback = true` en redes privadas de confianza. |
| TUI no inicia | Feature faltante | Recompilar con `--features tui` |

### Modo Debug

Activa logging detallado para diagnosticar problemas:

```bash
RUST_LOG=cerebro=debug,tower_http=debug cerebro serve
```

## Páginas Relacionadas

- [Configuración](configuration.md) — Referencia completa
- [Ejecución](running.md) — Iniciar y detener el servicio
- [Referencia CLI](cli-reference.md) — Opciones de línea de
  comandos
