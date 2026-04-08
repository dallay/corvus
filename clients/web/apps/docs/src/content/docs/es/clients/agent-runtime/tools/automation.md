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

## `delegate`

Delega una subtarea a un agente especializado.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Modos de Ejecución:**
  - **OneShot:** Una única llamada de LLM a un subagente.
  - **Session:** Lanza un agente hijo delimitado con un bucle completo de herramientas (Sesión de Código).
- **Límite de Profundidad:** Impone una profundidad máxima de recursión para evitar bucles de delegación infinitos.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `agent` | `string` | **Requerido.** Nombre del subagente configurado (ej. `researcher`, `coder`). |
| `prompt` | `string` | **Requerido.** La tarea o instrucción para enviar al subagente. |
| `context` | `string` | Contexto opcional para anteponer a la tarea. |

---

## `composio`

Ejecuta acciones en más de 1000 aplicaciones gestionadas a través de la plataforma Composio.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Integraciones:** Gmail, Notion, GitHub, Slack, Linear y más.
- **Requisitos:** Requiere `COMPOSIO_API_KEY` en el entorno del workspace.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `action` | `string` | **Requerido.** Operación a realizar: `list`, `execute` o `connect`. |
| `app` | `string` | Slug de la aplicación/toolkit (ej. `gmail`). |
| `tool_slug` | `string` | El identificador específico de la herramienta a ejecutar. |
| `params` | `object` | Parámetros JSON para la acción. |

---

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

## `cron_*` / `schedule`

Herramientas para gestionar la ejecución autónoma basada en el tiempo. Corvus proporciona tanto un conjunto de herramientas granulares `cron_*` como una herramienta unificada `schedule`.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Tipos de Trabajo:**
  - **Agent Job:** El agente se ejecuta a sí mismo con un prompt específico.
  - **Shell Job:** Ejecuta un comando de shell.
- **Programaciones:**
  - `cron`: Tareas recurrentes (ej. `0 9 * * *`).
  - `at`: Tareas únicas en un timestamp RFC3339 específico.
  - `every`: Intervalos fijos en milisegundos.

### Herramientas

| Herramienta | Descripción |
| :--- | :--- |
| `cron_add` | Crea un nuevo trabajo programado. |
| `cron_list` | Lista todos los trabajos cron configurados. |
| `cron_remove` | Elimina un trabajo cron por ID. |
| `cron_run` | Fuerza la ejecución inmediata de un trabajo. |
| `cron_runs` | Ver el historial de ejecuciones recientes de un trabajo. |
| `cron_update` | Parchea la programación o configuración de un trabajo existente. |
| `schedule` | Herramienta unificada para `create`, `list`, `get`, `cancel`, `pause` y `resume`. |

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
