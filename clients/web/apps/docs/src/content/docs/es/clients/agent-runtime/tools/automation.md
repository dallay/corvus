---
title: Herramientas de Automatización y Utilidades
description: Referencia para las herramientas de Git, Cron, Programación y Notificaciones en Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

Estas herramientas permiten al agente realizar la gestión de repositorios, programar acciones futuras y notificar al usuario.

## `git_operations`

Una interfaz estructurada para tareas comunes de Git.

- **Nivel de Seguridad:** Mixto (Las operaciones de escritura como `commit` son De Acción).
- **Operaciones Soportadas:** `status`, `diff`, `log`, `branch`, `commit`, `add`, `checkout`, `stash`.
- **Seguridad:** Desinfecta automáticamente los argumentos para evitar la inyección de shell (bloquea `--exec`, `-c`, etc.).

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `operation` | `string` | **Requerido.** Uno de los comandos de git soportados. |
| `message` | `string` | Mensaje de commit (para `commit`). |
| `paths` | `string` | Rutas de archivos para añadir al stage (para `add`). |

---

## `cron_add` / `schedule`

Herramientas para gestionar la ejecución autónoma basada en el tiempo.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Capacidad:** Permite al agente programarse a sí mismo (Agent Job) o un script de shell (Shell Job) para ejecutarse en el futuro.
- **Programaciones:**
  - `cron`: Tareas recurrentes (ej. `0 9 * * *`).
  - `at`: Tareas únicas en un timestamp RFC3339 específico.
  - `every`: Intervalos fijos en milisegundos.

### Acciones de `schedule`

`create`, `list`, `get`, `cancel`, `pause`, `resume`.

---

## `pushover`

Envía una notificación push al dispositivo móvil del usuario.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Requisitos:** Requiere `PUSHOVER_TOKEN` y `PUSHOVER_USER_KEY` en el archivo `.env` del workspace.
- **Uso:** Ideal para notificar al usuario cuando una misión de larga duración se completa o requiere intervención manual.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `message` | `string` | **Requerido.** El texto de la notificación. |
| `priority` | `integer` | Prioridad de -2 (silencioso) a 2 (emergencia). |
| `sound` | `string` | Sobrescritura de sonido opcional (ej. `bugle`, `bike`). |
