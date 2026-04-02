---
title: Referencia CLI de Cerebro
description: >-
  Referencia completa de los comandos, subcomandos y opciones
  del CLI de Cerebro.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: reference
---

# Referencia CLI

El binario `cerebro` proporciona dos comandos principales: `serve`
para ejecutar el servicio de memoria MCP y `migrate` para la
migración de datos heredados.

```bash
cerebro <COMANDO>
```

## Opciones Globales

| Flag        | Descripción                |
|-------------|----------------------------|
| `--version` | Muestra la versión y sale  |
| `--help`    | Muestra la ayuda y sale    |

---

## `cerebro serve`

Inicia el servicio de memoria MCP de Cerebro.

```bash
cerebro serve [OPCIONES]
```

### Opciones

| Flag              | Tipo   | Default | Descripción                               |
|-------------------|--------|---------|-------------------------------------------|
| `--config <RUTA>` | Path   | —       | Ruta al archivo de config (.toml o .json) |
| `--tui`           | bool   | `false` | Activa el panel TUI                       |

### Ejemplos

```bash
# Iniciar con valores por defecto (127.0.0.1:4040)
cerebro serve

# Iniciar con archivo de configuración
cerebro serve --config /etc/cerebro/cerebro.toml

# Iniciar con panel TUI
cerebro serve --tui

# Iniciar con auth y logging de depuración
CEREBRO_AUTH_TOKEN=secreto RUST_LOG=debug cerebro serve
```

### Comportamiento

- Carga la configuración del archivo (si se proporciona
  `--config`), luego aplica las variables de entorno.
- Se enlaza a `{host}:{port}` y sirve MCP en `POST /mcp`.
- Maneja apagado graceful en `SIGINT` (Ctrl+C) y `SIGTERM`.
- Si se pasa `--tui` o `CEREBRO_TUI_ENABLED=1`, inicia el panel
  de terminal (requiere feature `tui`).

---

## `cerebro migrate`

Herramientas de migración de datos heredados. Contiene subcomandos
para importar y validar exportaciones de memoria.

```bash
cerebro migrate <COMANDO>
```

---

### `cerebro migrate import`

Importa una exportación de memoria heredada a un destino SurrealDB.

```bash
cerebro migrate import [OPCIONES]
```

#### Opciones

| Flag                   | Tipo   | Requerido | Default   | Descripción               |
|------------------------|--------|-----------|-----------|---------------------------|
| `--source <RUTA>`      | Path   | Sí        | —         | Archivo de exportación    |
| `--target <RUTA>`      | Path   | Sí        | —         | Ruta de SurrealDB destino |
| `--namespace <NOMBRE>` | String | No        | `cerebro` | Namespace de SurrealDB    |
| `--database <NOMBRE>`  | String | No        | `cerebro` | Base de datos             |
| `--dry-run`            | bool   | No        | `false`   | Vista previa sin escribir |

#### Ejemplos

```bash
# Importar con valores por defecto
cerebro migrate import \
  --source ./exportacion-legacy.json \
  --target ./cerebro.db

# Vista previa sin escribir
cerebro migrate import \
  --source ./exportacion-legacy.json \
  --target ./cerebro.db \
  --dry-run
```

#### Salida

Imprime un reporte de migración JSON en stdout:

```json
{
  "status": "success",
  "imported": 42,
  "skipped": 0,
  "errors": []
}
```

---

### `cerebro migrate validate`

Valida una exportación heredada contra un destino SurrealDB para
verificar la integridad de la migración.

```bash
cerebro migrate validate [OPCIONES]
```

#### Opciones

| Flag                   | Tipo   | Requerido | Default   | Descripción               |
|------------------------|--------|-----------|-----------|---------------------------|
| `--source <RUTA>`      | Path   | Sí        | —         | Archivo de exportación    |
| `--target <RUTA>`      | Path   | Sí        | —         | Ruta de SurrealDB destino |
| `--namespace <NOMBRE>` | String | No        | `cerebro` | Namespace de SurrealDB    |
| `--database <NOMBRE>`  | String | No        | `cerebro` | Base de datos             |

#### Ejemplos

```bash
cerebro migrate validate \
  --source ./exportacion-legacy.json \
  --target ./cerebro.db
```

#### Salida

Imprime un reporte de validación JSON. Sale con código `2` si
se encuentran discrepancias:

```json
{
  "status": "match",
  "total_source": 42,
  "total_target": 42,
  "mismatches": []
}
```

:::tip
Ejecuta `validate` después de cada `import` para confirmar la
integridad de los datos.
:::
