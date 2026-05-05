---
title: Salud operacional de Rook
description: Endpoints de liveness, readiness y salud de compatibilidad para despliegues supervisados y contenerizados de Rook.
summary: Endpoints de liveness, readiness y salud de compatibilidad para despliegues supervisados y contenerizados de Rook.
owner: team-platform
status: canonical
lastReviewed: 2026-05-03
appliesTo: main
docType: runbook
---

# Salud operacional de Rook

Rook expone endpoints de salud operacional en la superficie administrativa para que supervisores y
orquestadores de contenedores puedan distinguir entre liveness del proceso y readiness del servicio.

## Endpoints

| Propósito | Endpoint | Uso |
| --- | --- | --- |
| Smoke check de compatibilidad | `GET /api/health` | Checks heredados o simples que solo necesitan `ok`. |
| Liveness | `GET /api/health/live` | Reinicia el proceso solo si este probe deja de responder correctamente. |
| Readiness | `GET /api/health/ready` | Envía tráfico solo mientras este probe reporte que el servicio está listo. |

## Liveness

`GET /api/health/live` reporta si el proceso de Rook puede atender requests HTTP.

Respuesta saludable esperada:

```json
{ "status": "ok" }
```

Usa este endpoint para liveness probes. Intencionalmente no falla porque la base de datos, los assets
embebidos del dashboard, las cuentas de proveedores o los proveedores de IA upstream no estén
disponibles.

## Readiness

`GET /api/health/ready` reporta si Rook tiene las dependencias locales necesarias para atender
tráfico.

Una respuesta ready devuelve `200 OK`:

```json
{
  "status": "ok",
  "checks": {
    "config": { "ready": true },
    "database": { "ready": true },
    "router": { "ready": true },
    "assets": { "ready": true }
  }
}
```

Readiness trata estos checks como críticos:

- `config`
- `database`
- `router`

Si falla un check crítico, Rook devuelve `503 Service Unavailable` con `status: "fail"` y una entrada
de check fallida:

```json
{
  "status": "fail",
  "checks": {
    "database": {
      "ready": false,
      "reason": "database connectivity unavailable"
    }
  }
}
```

El check `assets` no es crítico. Si los assets embebidos del dashboard no están disponibles mientras
los checks críticos están listos, readiness devuelve `200 OK` con `status: "degraded"`.

## Alcance de proveedores upstream

Readiness no sondea activamente los proveedores de IA upstream. La disponibilidad de proveedores es
dinámica y se reporta mediante la salud de cuentas de proveedor y el estado de routing, no mediante
readiness operacional.

## Ejemplo de Kubernetes

```yaml
livenessProbe:
  httpGet:
    path: /api/health/live
    port: 4000
readinessProbe:
  httpGet:
    path: /api/health/ready
    port: 4000
```

Usa el puerto configurado para el listener HTTP administrativo/gateway de Rook.

## Endpoint de compatibilidad

`GET /api/health` sigue disponible para smoke checks existentes y devuelve texto plano:

```text
ok
```

No uses `/api/health` como señal primaria de readiness en despliegues orquestados. Usa
`/api/health/ready` en su lugar.
