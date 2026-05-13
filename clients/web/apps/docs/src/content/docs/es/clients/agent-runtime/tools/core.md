---
title: Herramientas Core
description: Referencia para la ejecución de comandos del sistema y herramientas de archivos en Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-05-13
appliesTo: main
docType: reference
---

# Herramientas Core

Las herramientas core proporcionan la base para la autonomía del agente, permitiendo la interacción con el sistema operativo host y el espacio de trabajo local.

## `code_search`

Busca coincidencias literales o regex dentro del workspace y devuelve tanto salida legible para humanos como datos estructurados de coincidencias.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Modo Plan:** ✅ Seguro para el Modo Plan (`--plan`).
- **Ejecución:** Herramienta nativa del runtime con reducción opcional de candidatos por índice para consultas literales y verificación obligatoria en vivo sobre el contenido actual.
- **Limitación actual:** La corrección regex está soportada, pero la reducción de candidatos por índice **no** soporta regex en v1. El planning regex cae en fallback con `query_regex_not_supported` hacia discovery más live verification.
- **Evidencia de rollout:** Consulta la página dedicada de [`code_search`](code-search.md) para resultados medidos shell-vs-native, etiquetas de fallback y guía de recomendación.

### Parámetros

Consulta la página dedicada de [`code_search`](code-search.md) para el contrato completo de parámetros, la metodología del benchmark y la guía de rollout.

---

## `Glob`

Herramienta de paridad estilo Claude para descubrimiento seguro de archivos por patrón dentro del workspace.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Modo Plan:** ✅ Seguro para el Modo Plan (`--plan`).
- **Ejecución:** Herramienta nativa del runtime respaldada por helpers de metadata del discovery del workspace.
- **Contrato:** Requiere `pattern`; opcionalmente limita el recorrido con `path` relativo al workspace.
- **Nota de paridad:** `Glob` es el nombre canónico para superficies de paridad en este slice y se añade sin quitar los nombres nativos existentes.

---

## `Grep`

Herramienta de paridad estilo Claude para búsqueda de contenido, respaldada por los mismos internals de búsqueda que usa `code_search`.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Modo Plan:** ✅ Seguro para el Modo Plan (`--plan`).
- **Ejecución:** Herramienta nativa con salidas determinísticas y rutas relativas al workspace.
- **Contrato:** Soporta `pattern`, `path` opcional, `glob` opcional y `output_mode` con valores `content`, `files_with_matches` o `count`.
- **Nota de paridad:** `Grep` es canónica para documentación de paridad, mientras `code_search` sigue disponible como contrato nativo retenido.

---

## Paridad de gestión de tareas

Estas herramientas proporcionan un ciclo de vida de tareas persistente respaldado por el store de memoria SQLite del runtime (`brain.db`). Solo están disponibles cuando el backend de memoria activo es `sqlite`.

### `TaskCreate`

Crea una tarea persistente del runtime con vinculación opcional a una sesión.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Alias de compatibilidad:** `task_create`

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `title` | `string` | **Requerido.** Breve resumen de la tarea. |
| `description` | `string` | Instrucciones detalladas o contexto opcional. |
| `priority` | `string` | Prioridad de la tarea: `low`, `medium` o `high`. |
| `session_id` | `string` | ID opcional para vincular la tarea a una sesión de chat específica. |

---

### `TaskGet`

Obtiene una tarea persistente del runtime mediante su ID único.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Alias de compatibilidad:** `task_get`

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `id` | `string` | **Requerido.** El UUID de la tarea a obtener. |

---

### `TaskList`

Lista las tareas persistentes del runtime con filtros básicos y paginación.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Alias de compatibilidad:** `task_list`

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `status` | `string` | Filtrar por estado: `pending`, `in_progress`, `completed` o `cancelled`. |
| `priority` | `string` | Filtrar por prioridad: `low`, `medium` o `high`. |
| `session_id` | `string` | Filtrar por ID de sesión de chat. |
| `limit` | `integer` | Número máximo de tareas a devolver (por defecto: 10). |
| `offset` | `integer` | Número de tareas a omitir para la paginación (por defecto: 0). |

---

### `TaskUpdate`

Actualiza los campos mutables de una tarea persistente del runtime.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Alias de compatibilidad:** `task_update`

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `id` | `string` | **Requerido.** El UUID de la tarea a actualizar. |
| `title` | `string` | Nuevo título para la tarea. |
| `description` | `string` | Nueva descripción para la tarea. |
| `priority` | `string` | Nueva prioridad: `low`, `medium` o `high`. |
| `status` | `string` | Nuevo estado: `pending`, `in_progress`, `completed` o `cancelled`. |

---

### `TaskStop`

Cancela una tarea persistente activa del runtime.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Alias de compatibilidad:** `task_stop`

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `id` | `string` | **Requerido.** El UUID de la tarea a cancelar. |

---

## `shell`

Ejecuta un comando de shell arbitrario dentro del directorio del workspace.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Ejecución:** Se ejecuta a través del [Runtime](../architecture.md#runtime) configurado (Nativo o Docker).
- **Restricciones:**
  - Comandos bloqueados: Definidos en `autonomy.forbidden_paths`.
  - Comandos permitidos: Deben estar en `autonomy.allowed_commands` si se configura.
  - Entorno: Solo se pasan variables funcionales seguras (`PATH`, `HOME`, `USER`, etc.). Las claves de API y los secretos se redactan/limpian explícitamente.
  - Tiempo de espera: Por defecto 60 segundos.
  - Límite de salida: Truncado a 1 MB para evitar el agotamiento de la memoria.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `command` | `string` | **Requerido.** El comando de shell a ejecutar. |
| `approved` | `boolean` | Establecer en `true` para aprobar explícitamente comandos de riesgo medio/alto en modo supervisado. |

---

## `file_read`

Lee el contenido de un archivo dentro del workspace.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Modo Plan:** ✅ Seguro para el Modo Plan (`--plan`).
- **Restricciones:**
  - El salto de directorios (path traversal, ej. `../../etc/passwd`) está estrictamente bloqueado.
  - Se rechazan los symlinks que resuelven fuera de los límites del workspace.
  - Tamaño máximo de archivo: 10 MB.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `path` | `string` | **Requerido.** Ruta relativa al archivo dentro del workspace. |

---

## `file_write`

Escribe o sobrescribe un archivo dentro del workspace.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Restricciones:**
  - Crea directorios padres automáticamente si no existen.
  - Se niega a escribir a través de symlinks (protección TOCTOU).
  - Sujeto a las mismas reglas de sandboxing de rutas que `file_read`.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `path` | `string` | **Requerido.** Ruta relativa al archivo dentro del workspace. |
| `content` | `string` | **Requerido.** El contenido a escribir en el archivo. |
