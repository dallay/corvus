---
title: Rook Operational Health
description: Liveness, readiness, and compatibility health endpoints for supervised and containerized Rook deployments.
summary: Liveness, readiness, and compatibility health endpoints for supervised and containerized Rook deployments.
owner: team-platform
status: canonical
lastReviewed: 2026-05-03
appliesTo: main
docType: runbook
---

# Rook Operational Health

Rook exposes operational health endpoints on the admin surface so supervisors and container
orchestrators can distinguish process liveness from service readiness.

## Endpoints

| Purpose | Endpoint | Use |
| --- | --- | --- |
| Compatibility smoke check | `GET /api/health` | Legacy/simple checks that only need `ok`. |
| Liveness | `GET /api/health/live` | Restart the process only if this probe stops succeeding. |
| Readiness | `GET /api/health/ready` | Send traffic only while this probe reports ready. |

## Liveness

`GET /api/health/live` reports whether the Rook process can serve HTTP requests.

Expected healthy response:

```json
{ "status": "ok" }
```

Use this endpoint for liveness probes. It intentionally does not fail because the database,
embedded dashboard assets, provider accounts, or upstream AI providers are unavailable.

## Readiness

`GET /api/health/ready` reports whether Rook has the local dependencies needed to serve traffic.

A ready response returns `200 OK`:

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

Readiness treats these checks as critical:

- `config`
- `database`
- `router`

If a critical check fails, Rook returns `503 Service Unavailable` with `status: "fail"` and a
failing check entry:

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

The `assets` check is non-critical. If embedded dashboard assets are unavailable while critical
checks are ready, readiness returns `200 OK` with `status: "degraded"`.

## Upstream Provider Reachability

Readiness does not actively probe upstream AI providers. Provider availability is dynamic and is
reported through provider account health and routing state, not operational readiness.

## Kubernetes Example

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

Use the port configured for your Rook admin/gateway HTTP listener.

## Compatibility Endpoint

`GET /api/health` remains available for existing smoke checks and returns plain text:

```text
ok
```

Do not use `/api/health` as the primary readiness signal for orchestrated deployments. Use
`/api/health/ready` instead.
