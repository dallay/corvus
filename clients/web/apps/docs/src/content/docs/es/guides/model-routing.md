---
title: Enrutamiento de Modelos y Clasificación de Consultas
description: Configura enrutamiento multi-modelo con task hints y clasificación automática de consultas en el agent runtime de Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-04-07
appliesTo: main
docType: guide
---

Corvus puede enviar distintos tipos de solicitudes a distintos providers y modelos sin cambiar tu
código de aplicación. Tú defines hints de ruta en TOML, opcionalmente clasificas mensajes hacia
esos hints, y validas la configuración con `corvus doctor`.

Usa esta guía cuando quieras:

- enviar prompts rápidos a un modelo más barato,
- reservar razonamiento profundo para un modelo más fuerte,
- mandar preguntas de código a un modelo especializado,
- dirigir turnos con imágenes solo a rutas que acepten imágenes explícitamente.

## Qué hace el enrutamiento de modelos

`[[model_routes]]` mapea un hint a un provider y a un modelo.

```toml
[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"
```

En tiempo de solicitud Corvus puede recibir un selector como `hint:reasoning`. Ese selector se
resuelve contra la ruta correspondiente y despacha la solicitud al provider y modelo configurados.

Si ninguna ruta coincide, Corvus cae al provider por defecto. El runtime sigue funcionando, pero
registra una advertencia para que puedas corregir la configuración.

## Qué hace la clasificación de consultas

`[query_classification]` es opcional. Cuando está habilitada, examina el mensaje del usuario e
intenta escoger automáticamente el mejor route hint.

```toml
[query_classification]
enabled = true

[[query_classification.rules]]
hint = "code"
keywords = ["bug", "stack trace", "debug"]
patterns = ["fn ", "```", "Exception"]
priority = 20
```

Si ninguna regla coincide, Corvus usa el modelo por defecto. Ese comportamiento es normal y no
requiere advertencia.

## Flujo del hint de extremo a extremo

```text
Mensaje del usuario
  ↓
¿Clasificación habilitada?
  ├─ No  → usar modelo por defecto
  └─ Sí
       ↓
  Reglas evaluadas por prioridad
       ↓
  ¿Se encontró una regla que coincide?
  ├─ No  → usar modelo por defecto
  └─ Sí → emitir hint (ejemplo: "reasoning")
               ↓
          El router resuelve el hint contra [[model_routes]]
               ↓
          ¿Existe la ruta?
          ├─ Sí → despachar a ese provider + modelo
          └─ No  → registrar advertencia y caer al provider por defecto
```

## Referencia de configuración

### `[[model_routes]]`

Cada entrada define una ruta nombrada.

| Campo | Tipo | Requerido | Default | Propósito | Ejemplo |
|---|---|---:|---|---|---|
| `hint` | string | Sí | ninguno | Nombre usado por clasificación o selección directa `hint:<name>`. | `"reasoning"` |
| `provider` | string | Sí | ninguno | Provider al que Corvus debe despachar para esta ruta. | `"openai"` |
| `model` | string | Sí | ninguno | Nombre del modelo que se pasa a ese provider. | `"o1-preview"` |
| `api_key` | string | No | sin definir | Override opcional de credencial para este provider en esta ruta. | `"env:OPENAI_KEY"` o un secreto literal si así gestionas secretos |
| `allow_image_input` | boolean | No | `false` | Puerta de opt-in para turnos con imágenes en esta ruta. | `true` |

#### Notas

- Los valores de `hint` deben ser nombres cortos y estables como `fast`, `reasoning`, `code` o `vision`.
- `provider` debe coincidir con un provider que Corvus pueda inicializar.
- `allow_image_input` es opt-in. Si lo omites, la ruta se trata como solo texto.

### `[query_classification]`

Esta sección controla si Corvus intenta escoger hints automáticamente.

| Campo | Tipo | Requerido | Default | Propósito | Ejemplo |
|---|---|---:|---|---|---|
| `enabled` | boolean | No | `false` | Habilita selección automática de hints a partir del contenido del mensaje. | `true` |
| `rules` | array | No | `[]` | Lista ordenada de reglas de clasificación. | `[[query_classification.rules]]` |

### `[[query_classification.rules]]`

Cada regla puede coincidir por keyword, patrón literal, o ambos.

| Campo | Tipo | Requerido | Default | Propósito | Ejemplo |
|---|---|---:|---|---|---|
| `hint` | string | Sí | ninguno | Hint de ruta que se devuelve cuando esta regla coincide. Debe coincidir con un hint de `[[model_routes]]`. | `"code"` |
| `keywords` | array de strings | No | `[]` | Coincidencias por substring sin distinguir mayúsculas/minúsculas. | `["debug", "bug"]` |
| `patterns` | array de strings | No | `[]` | Coincidencias literales sensibles a mayúsculas/minúsculas. Útil para fragmentos de código. | `["fn ", "```rust"]` |
| `min_length` | integer | No | sin definir | Solo coincide cuando la longitud del mensaje es al menos este valor. | `40` |
| `max_length` | integer | No | sin definir | Solo coincide cuando la longitud del mensaje es como máximo este valor. | `500` |
| `priority` | integer | No | `0` | Los valores más altos se revisan primero. | `20` |

#### Comportamiento de las reglas

- Primero se aplican las restricciones de longitud.
- Después, la regla coincide si **cualquier** keyword coincide o **cualquier** pattern coincide.
- `keywords` no distingue mayúsculas/minúsculas.
- `patterns` sí distingue mayúsculas/minúsculas.
- La primera coincidencia por prioridad descendente gana.

## Ejemplo: división entre fast y reasoning

```toml
default_provider = "openai"
default_model = "gpt-4o-mini"

[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"
```

Usa `hint:fast` para respuestas de baja latencia y `hint:reasoning` para prompts más difíciles.

## Ejemplo: ruta especializada en código

```toml
default_provider = "openai"
default_model = "gpt-4o-mini"

[[model_routes]]
hint = "code"
provider = "groq"
model = "qwen-qwq-32b"

[query_classification]
enabled = true

[[query_classification.rules]]
hint = "code"
keywords = ["debug", "refactor", "stack trace", "compile error"]
patterns = ["fn ", "```", "Exception"]
priority = 20
```

## Ejemplo: ruta vision para entrada de imágenes

```toml
default_provider = "openai"
default_model = "gpt-4o-mini"

[[model_routes]]
hint = "vision"
provider = "openai"
model = "gpt-4o"
allow_image_input = true

[multimodal]
enabled = true
vision_model_hint = "vision"
```

Los turnos con imágenes solo se aceptan cuando `vision_model_hint` apunta a una ruta con
`allow_image_input = true`.

## Ejemplo: enrutamiento multi-provider con clasificación

```toml
default_provider = "openrouter"
default_model = "openai/gpt-4o-mini"

[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"

[[model_routes]]
hint = "code"
provider = "anthropic"
model = "claude-sonnet-4-6"

[query_classification]
enabled = true

[[query_classification.rules]]
hint = "reasoning"
keywords = ["compare", "tradeoff", "strategy"]
priority = 30

[[query_classification.rules]]
hint = "code"
keywords = ["debug", "stack trace", "refactor"]
patterns = ["fn ", "```"]
priority = 20

[[query_classification.rules]]
hint = "fast"
keywords = ["summarize", "quick", "brief"]
priority = 10
```

## Validar la configuración

Ejecuta:

```bash
corvus doctor
```

Para routing y classification, Phase 1 añade estas advertencias:

- clasificación habilitada pero sin reglas configuradas,
- clasificación habilitada pero sin `[[model_routes]]`,
- una regla de clasificación apunta a un hint que no existe en `[[model_routes]]`,
- una regla no tiene keywords ni patterns y nunca podrá coincidir.

Las advertencias **no** bloquean el arranque. Te indican que la configuración puede ejecutarse,
pero todavía no está alineada del todo con el contrato de routing.

## Troubleshooting

| Síntoma | Causa probable | Qué te dice `corvus doctor` o los logs | Resolución |
|---|---|---|---|
| La clasificación nunca cambia el modelo seleccionado. | `enabled = true` pero `rules` está vacío. | Advertencia: la clasificación está habilitada pero no hay reglas configuradas. | Añade al menos una regla o desactiva la clasificación. |
| La clasificación devuelve hints pero el routing igual cae al provider por defecto. | Una regla apunta a un hint que no existe en `[[model_routes]]`. | La advertencia nombra el hint huérfano y los hints de ruta disponibles. | Cambia el hint de la regla para que coincida con una ruta real o añade la ruta faltante. |
| Una regla nunca se dispara aunque la clasificación esté habilitada. | La regla tiene `keywords` y `patterns` vacíos. | La advertencia nombra el hint afectado y dice que la regla nunca va a coincidir. | Añade keywords, patterns o elimina la regla. |
| Los turnos con imágenes son rechazados. | La ruta usada por `vision_model_hint` no tiene `allow_image_input = true`. | El runtime rechaza el turno por una ruta que no acepta imágenes. | Apunta `vision_model_hint` a una ruta con `allow_image_input = true`. |
| Un selector directo como `hint:code` no enruta como esperabas. | El route hint es desconocido. | El runtime registra una advertencia indicando que el hint es desconocido y que Corvus cae al provider por defecto usando el model string bruto. | Corrige el nombre del hint o añade la ruta que falta. |
| Una ruta parece válida en config pero falla en tiempo de solicitud. | Un provider no primario falló durante la inicialización. | El runtime registra una advertencia con el provider que falló y los route hints afectados. | Corrige las credenciales o la configuración de ese provider, luego reinicia y ejecuta `corvus doctor` otra vez. |

## Checklist del operador

- Define al menos una entrada `[[model_routes]]` por cada hint que piensas usar.
- Mantén exactamente iguales los nombres de hint entre rutas y reglas de clasificación.
- Usa `priority` para que la regla más específica gane primero.
- Activa `allow_image_input = true` solo en las rutas que deban aceptar imágenes.
- Ejecuta `corvus doctor` después de cambiar la configuración.

Si quieres empezar simple, crea primero solo las rutas `fast` y `reasoning`, valídalas con
`corvus doctor`, y añade reglas de clasificación después de que el routing base funcione.
