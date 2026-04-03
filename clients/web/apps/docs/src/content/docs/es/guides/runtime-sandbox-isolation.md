---
title: Aislamiento del Sandbox del Runtime
description: Modelo de seguridad, selección de backend, verificación del sidecar y expectativas de auditoría para el aislamiento del runtime en Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-04-03
appliesTo: main
docType: guide
---

# Aislamiento del Sandbox del Runtime

Corvus usa **dos capas de seguridad** para la ejecución disparada por usuarios:

1. **Política a nivel de aplicación** mediante `SecurityPolicy`
   - allowlists de comandos
   - restricciones de rutas
   - límites de tasa
   - clasificación de riesgo y compuertas de aprobación
2. **Sandboxing a nivel de sistema operativo** mediante los backends `Sandbox` del runtime
   - `landlock`
   - `firejail`
   - `bubblewrap`
   - `docker`

Las dos capas importan. La capa de política decide **si** una acción está permitida. El sandbox del sistema operativo limita **a qué puede acceder la acción incluso si está permitida**.

## Selección del backend de sandbox

Configura el runtime bajo `security.sandbox`:

```toml
[security.sandbox]
enabled = true
backend = "auto"      # auto | landlock | firejail | bubblewrap | docker | none
require = false        # cuando es true, falla cerrado si no hay sandbox disponible
firejail_args = []
```

### `backend = "auto"`

Corvus prueba los backends soportados según la plataforma y usa el primero disponible.

- Linux: `landlock` → `firejail` → `docker`
- macOS: `bubblewrap` → `docker`
- otras plataformas: `docker`
- si ninguno está disponible y `require = false`, Corvus cae a `none`

### `require = true`

Cuando `require = true`, Corvus **falla cerrado**.

Eso significa:
- backend explícito no disponible → error al iniciar
- `backend = "auto"` sin backend encontrado → error al iniciar
- `backend = "none"` → error al iniciar
- `enabled = false` → error al iniciar

Úsalo en despliegues reales orientados a usuarios donde el aislamiento del sistema operativo es obligatorio.

## Contrato de ejecución

### Ejecución shell

Para la herramienta `shell`, Corvus aplica ahora esta secuencia:

1. valida el comando contra `SecurityPolicy`
2. sanitiza variables de entorno
3. envuelve el comando con el backend de sandbox seleccionado
4. ejecuta con timeout
5. audita el resultado con metadata del sandbox

Eso significa que cada comando shell permitido corre dentro del límite de sandbox activo cuando hay uno configurado.

### Comportamiento del sandbox noop

Si no hay backend a nivel de sistema operativo y `require = false`, Corvus usa `none` (`NoopSandbox`).

En ese modo:
- la política a nivel de aplicación sigue aplicando
- los comandos mutables emiten una advertencia de que el sandbox del sistema operativo no está activo
- los logs de auditoría registran `sandbox_backend = "none"`

Esto se permite para desarrollo local, pero es una postura de seguridad más débil.

## Aislamiento del sidecar de computer-use

Las acciones de computer-use (`mouse_move`, `mouse_click`, `mouse_drag`, `key_type`, `screen_capture`) usan un sidecar.

Valores seguros por defecto:
- el endpoint por defecto usa loopback: `http://127.0.0.1:8787/v1/actions`
- los endpoints remotos/públicos se bloquean salvo que `allow_remote_endpoint = true`
- los endpoints públicos remotos deben usar HTTPS
- dominios permitidos, allowlists de ventanas y límites de coordenadas se reenvían como política al sidecar

Corvus hace un **health-check perezoso** contra el sidecar en la primera acción de computer-use usando:

- `GET /v1/health`

Se espera que el sidecar reporte detalles de aislamiento como:

```json
{
  "status": "healthy",
  "isolation": {
    "type": "container",
    "runtime": "docker",
    "version": "24.0.7"
  }
}
```

Corvus registra esto como una entrada de auditoría `SecurityEvent`.

### Cuando falla la verificación del sidecar

- si `security.sandbox.require = false`: Corvus registra una advertencia y continúa
- si `security.sandbox.require = true`: Corvus rechaza la acción de computer-use porque no pudo verificarse el aislamiento del sidecar

## Expectativas de auditoría

Los eventos de auditoría de comandos shell incluyen:
- comando
- nivel de riesgo
- bandera de aprobación
- éxito/fracaso
- `security.sandbox_backend`

La verificación del sidecar de computer-use genera una entrada `SecurityEvent` describiendo:
- estado de salud del sidecar
- tipo de aislamiento reportado
- runtime reportado

## Valores recomendados para operadores

### Desarrollo local

```toml
[security.sandbox]
backend = "auto"
require = false
```

### Workstation endurecida o despliegue en servidor

```toml
[security.sandbox]
backend = "auto"
require = true
```

### Uso de sidecar remoto

Actívalo solo cuando controles el despliegue:

```toml
[browser.computer_use]
endpoint = "https://computer-use.example.com/v1/actions"
allow_remote_endpoint = true
```

Si haces esto, asegúrate de que el sidecar corra en un entorno aislado y exponga `/v1/health` con metadata de aislamiento veraz.

## Lo que esto no garantiza

Este cambio **no** agrega:
- instancias de sandbox por usuario
- containerización por sesión
- sandboxing automático del proceso sidecar por parte de Corvus

Los operadores siguen siendo dueños del límite de despliegue del sidecar de computer-use.
