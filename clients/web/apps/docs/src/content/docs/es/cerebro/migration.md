---
title: Guía de Migración a Cerebro
description: Mueve la memoria a largo plazo al servicio Cerebro basado en MCP.
slug: es/cerebro/migration
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: runbook
---

Esta guía cubre la migración desde la memoria SurrealDB local del runtime al servicio Cerebro basado en MCP. Para contexto narrativo e intención de diseño, consulta la especificación de Cerebro en
https://github.com/dallay/corvus/blob/main/openspec/specs/cerebro/spec.md.

## Resumen

- La memoria a largo plazo está ahora centralizada en Cerebro y se accede a través de MCP (JSON-RPC).
- La memoria local del runtime sigue siendo a corto plazo y privada, a menos que se guarde mediante herramientas de Cerebro.
- Las herramientas heredadas (`memory_store`, `memory_recall`, `memory_forget`) son alias de las herramientas MCP.
- Cerebro utiliza por defecto el almacenamiento SurrealDB embebido, a menos que se configure explícitamente lo contrario.

## Aviso de paridad

Cerebro no es un reemplazo directo de SurrealDB. No hay migración automática y el comportamiento de búsqueda y ordenamiento puede variar. Planifica un paso deliberado de exportación/importación y una ruta de retorno (rollback) antes del cambio definitivo.

## Valores seguros por defecto (requerido)

Cerebro y el runtime imponen el transporte seguro por defecto:

- Los endpoints `https` y `wss` se aceptan sin flags adicionales.
- Los endpoints `http` y `ws` se rechazan a menos que se permitan explícitamente para loopback.
- Mantén los tokens limitados a las herramientas de memoria y rótalos regularmente.

## Configuración de Cerebro MCP

Configura el endpoint MCP y el token de autenticación para Cerebro. El runtime rechaza endpoints inseguros a menos que permitas explícitamente el desarrollo en loopback.

```toml
[memory]
backend = "sqlite"              # memoria local a corto plazo

[memory.cerebro]
endpoint = "https://cerebro.example.com/mcp"
# el auth_token se lee de CORVUS_CEREBRO_AUTH_TOKEN
request_timeout_ms = 30000
allow_insecure_loopback = false
```

Configura `CORVUS_CEREBRO_AUTH_TOKEN` en tu entorno; evita incluir tokens en archivos de configuración.

Ejemplo de desarrollo solo en loopback:

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
# el auth_token se lee de CORVUS_CEREBRO_AUTH_TOKEN
allow_insecure_loopback = true
```

## Alias de herramientas heredadas

El runtime preserva los nombres de las herramientas heredadas durante la migración:

- `memory_store` -> `mem_save`
- `memory_recall` -> `mem_search`
- `memory_forget` -> `mem_delete`

Si Cerebro no está configurado o no es accesible, las llamadas a herramientas heredadas devuelven un error estructurado. Dependiendo de la configuración de `storage_fallback`, Cerebro puede intentar usar un almacenamiento de respaldo (como `InMemory`, `Disk` o `RemoteSurreal`) si el almacenamiento principal falla.

## Esquemas de herramientas MCP

Los esquemas JSON legibles por máquina para las 13 herramientas están disponibles en:

- [`mcp-schema/`](./mcp-schema/)

Usa estos esquemas para validar las llamadas a herramientas y las respuestas en agentes e integraciones.

## Lista de verificación para la migración

1. Exporta cualquier memoria de SurrealDB que necesites conservar (espera pérdida de datos si omites este paso).
2. Elimina las referencias al backend de memoria SurrealDB de las configuraciones del runtime.
3. Configura `memory.cerebro.endpoint` y `memory.cerebro.auth_token`.
4. Confirma el transporte seguro (https/wss) o habilita `allow_insecure_loopback` solo para loopback.
5. Importa o rehidrata memorias críticas en Cerebro a través de `mem_save`.
6. Actualiza el uso de herramientas personalizadas para preferir los nombres `mem_*`; los alias heredados siguen siendo compatibles.
7. Valida las integraciones contra los esquemas MCP antes del despliegue.
8. Prepara un plan de rollback (restaurar la configuración antigua + desactivar Cerebro) y mantén una captura de exportación para recuperación.
9. Realiza una prueba piloto (`mem_save` -> `mem_search` -> `mem_get_observation`) antes del cambio total.

## Valores por defecto de almacenamiento en Cerebro (embebido)

Los nuevos despliegues de Cerebro utilizan por defecto el almacenamiento SurrealDB embebido. Para sobrescribir este valor, configura el modo de almacenamiento explícitamente en la configuración de Cerebro (no en la del runtime).

Modos de almacenamiento soportados:

- `embedded_surreal` (por defecto)
- `remote_surreal`
- `disk`
- `in_memory`

Usa `storage_fallback` solo cuando aceptes explícitamente la semántica de fallback para fallos en el arranque.

## TUI opcional (solo para operadores)

Cerebro incluye una interfaz de terminal (TUI) opcional para obtener información operativa en vivo. Está desactivada por defecto y no expone ningún puerto de red.

Habilitar vía CLI (comando serve):

```bash
cerebro serve --tui
```

Habilitar vía entorno para el binario `cerebro-serve`:

```bash
export CEREBRO_TUI_ENABLED=1
```

Claves de configuración:

- `tui.enabled` (booleano, por defecto false)
- `tui.event_buffer` (tamaño del búfer de eventos acotado)
- `tui.refresh_ms` (intervalo de refresco de la UI)
- `tui.redact_fields` (lista de denegación para claves sensibles)
- `tui.max_payload_bytes` (límite de carga para datos redactados)

Notas de seguridad:

- Los eventos de llamadas a herramientas se redactan antes de llegar a la TUI.
- La contrapresión (backpressure) descarta eventos en lugar de bloquear el rendimiento de MCP.
- La interfaz se ejecuta en el mismo proceso y no crea puertos de red adicionales.

## CLI de migración

Usa la CLI incluida para importar exportaciones heredadas y validar los resultados:

```bash
cerebro migrate import \
  --source legacy_export.json \
  --target ./cerebro.db

cerebro migrate validate \
  --source legacy_export.json \
  --target ./cerebro.db
```

Flags opcionales:

- `--namespace` / `--database` para apuntar a un espacio de nombres embebido específico.
- `--dry-run` para calcular recuentos/checksums sin realizar escrituras.

## Notas operativas

- Si la inicialización embebida falla y no hay un `storage_fallback` configurado, Cerebro se cierra con un error para evitar la pérdida silenciosa de datos.
- Códigos de salida de validación de migración:
  - `0` = ok
  - `2` = desajuste (los recuentos/checksums divergieron)
  - `1` = error
