---
title: Herramientas de Memoria
description: Referencia para herramientas de persistencia y recuperación de memoria a largo plazo en Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-05-13
appliesTo: main
docType: reference
---

Las herramientas de memoria permiten al agente persistir información a través de las conversaciones, construyendo efectivamente un "alma" o base de conocimientos a largo plazo.

## `memory_store`

Almacena un hecho, preferencia o nota en la memoria a largo plazo.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Filtro de Datos Sensibles:** Rechaza automáticamente el contenido que parezca contener contraseñas, claves de API o credenciales.
- **Categorías:**
  - `core`: Hechos permanentes (ej. "El usuario vive en Madrid").
  - `daily`: Notas temporales para la sesión actual.
  - `conversation`: Contexto específico del chat.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `key` | `string` | **Requerido.** Identificador único para la memoria (ej. `user_pref_theme`). |
| `content` | `string` | **Requerido.** La información a recordar. |
| `category` | `string` | Categoría opcional. Por defecto: `core`. |

---

## `memory_recall`

Busca en el sistema de memoria información relevante basada en una consulta semántica.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Modo Plan:** ✅ Seguro para el Modo Plan (`--plan`).
- **Recuperación:** Utiliza búsqueda híbrida (similitud vectorial + BM25 por palabras clave) cuando el backend lo soporta.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `query` | `string` | **Requerido.** Palabras clave o frase a buscar. |
| `limit` | `integer` | Número máximo de resultados a devolver. Por defecto: `5`. |

---

## `memory_forget`

Elimina permanentemente una entrada de memoria mediante su clave.

- **Nivel de Seguridad:** De Acción (Con riesgo).

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `key` | `string` | **Requerido.** La clave de la memoria a eliminar. |
