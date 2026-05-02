---
title: Runbook de despliegue de Cerebro
description: >-
  Runbook operativo para despliegues de producción de Cerebro,
  incidentes, rotación de tokens, diagnóstico de readiness y recuperación de almacenamiento.
owner: team-platform
status: canonical
lastReviewed: 2026-05-02
appliesTo: main
docType: runbook
---

# Runbook de despliegue de Cerebro

Usa este runbook para desplegar u operar Cerebro como servicio MCP de memoria en producción. Cubre secretos requeridos, configuración, topología, probes, verificaciones de despliegue, respuesta a incidentes, rotación de tokens, diagnóstico de readiness y recuperación de almacenamiento.

Referencias relacionadas:

- [Configuración de Cerebro](configuration.md)
- [Ejecución de Cerebro](running.md)
- [Operaciones de Cerebro](operations.md)

## Postura de producción

La postura durable soportada para producción en esta versión es **nodo único y local-first**:

- ejecuta una sola instancia escritora de Cerebro por ruta de almacenamiento durable;
- usa SurrealDB embebido o almacenamiento `disk` local al nodo para datos durables;
- monta la ruta de almacenamiento en disco persistente o volumen persistente usado por una sola instancia activa;
- enruta tráfico de aplicación solo después de que `/readyz` responda correctamente;
- mantén `/healthz`, `/readyz` y `/metrics` privados para el orquestador y la red de monitoreo;
- expón `POST /mcp` solo detrás de ingress, gateway, red privada o service mesh confiables.

`remote_surreal`, persistencia remota compartida, HA activo-activo y múltiples escritores sobre la misma ruta no están soportados en esta versión.

## Secretos requeridos

| Secreto | Requerido | Cómo proveerlo | Notas de rotación |
|---------|-----------|----------------|-------------------|
| `CEREBRO_AUTH_TOKEN` | Requerido para producción y cualquier bind no-loopback. | Secret manager o variable de entorno secreta del orquestador. Prefiere env vars sobre archivos de configuración. | Rota con una ventana corta de compatibilidad dual en ingress o clientes. Cerebro acepta un token configurado a la vez. |
| `surreal.password` | Requerido con `storage_mode = "embedded_surreal"`. | Plantilla desde secret manager, archivo secreto montado o configuración protegida. | Rota en ventana de mantenimiento. Reinicia Cerebro y verifica `/readyz`. |
| `CEREBRO_AUDIT_TOKEN` | Opcional. | Secret manager o variable de entorno secreta. | Rota como otras credenciales servicio-a-servicio si integraciones de auditoría dependen de él. |

Nunca subas tokens de producción, configuraciones generadas con secretos reales, snapshots de almacenamiento o respaldos al repositorio.

## Configuración requerida

| Setting | Guía de producción |
|---------|--------------------|
| `host` | Usa `127.0.0.1` para sidecar o gateway local. Usa `0.0.0.0` solo dentro de red privada de pod/contenedor o detrás de ingress confiable. Cerebro rechaza bind no-loopback sin `CEREBRO_AUTH_TOKEN`. |
| `port` | Default `4040`; mantenlo alineado con Service, ingress y probes. |
| `scheme` | Normalmente se infiere. Configúralo solo cuando URLs generadas deban reflejar TLS terminado en gateway. |
| `request_timeout_secs` | Empieza con `30`. Súbelo solo por almacenamiento medidamente lento o herramientas de larga duración. |
| `max_concurrent_mcp_requests` | Empieza con `32`. Bájalo para nodos pequeños o discos lentos; súbelo solo con margen de CPU, memoria y almacenamiento. |
| `storage_mode` | `embedded_surreal` para producción durable por defecto. `disk` es una alternativa durable local al nodo. No uses `in_memory` para producción normal. |
| `surreal.storage_path` o `storage_path` | Usa una ruta durable explícita como `/var/lib/cerebro/data`. Evita depender del directorio de trabajo. |
| `storage_fallback` | Prefiere `none` para que el arranque falle claramente si el almacenamiento durable no está disponible. Usa `in_memory` solo como emergencia degradada en durabilidad. |
| `RUST_LOG` | Empieza con `info`. Usa temporalmente `cerebro=debug,surrealdb=warn` para investigaciones. |

## Configuración mínima de producción

Este ejemplo asume que Cerebro corre en una red privada de contenedor o pod detrás de ingress que provee TLS, rate limiting y control de acceso. Los secretos los inyecta el orquestador.

```toml
# /etc/cerebro/config.toml
host = "0.0.0.0"
port = 4040
scheme = "https"
request_timeout_secs = 30
max_concurrent_mcp_requests = 32

storage_mode = "embedded_surreal"
storage_fallback = "none"

[surreal]
namespace = "cerebro"
database = "cerebro"
storage_path = "/var/lib/cerebro/data"
username = "cerebro"
# Renderiza desde secret manager o plantilla protegida.
password = "${CEREBRO_SURREAL_PASSWORD}"

[tui]
enabled = false
```

Variables de entorno:

```bash
CEREBRO_AUTH_TOKEN=<token-de-servicio-generado>
CEREBRO_SURREAL_PASSWORD=<password-de-almacenamiento-generado>
RUST_LOG=info
```

Si tu renderizador no expande placeholders `${...}`, genera el archivo final antes del arranque o usa el mecanismo de proyección de secretos de la plataforma.

## Topología de despliegue

Una topología mínima de producción tiene estas capas:

1. **Ingress o gateway confiable** termina TLS, aplica rate limiting, elimina o valida headers de forwarding y permite solo callers aprobados.
2. **Instancia Cerebro** escucha en la red privada y protege `POST /mcp` con bearer auth, límites de tamaño, timeouts y concurrencia.
3. **Almacenamiento durable local al nodo** montado en la ruta configurada y adjunto a un solo proceso activo.
4. **Stack de monitoreo** scrapea `/metrics` y sondea `/healthz` y `/readyz` desde red privada.
5. **Pipeline de logs** recolecta logs estructurados con metadatos de despliegue, instancia y ruta de almacenamiento.

No expongas Cerebro directamente a Internet. Rate limits por IP son aceptables solo si ingress es el único hop confiable y elimina o valida `X-Forwarded-For`; si no, usa identidades no falsificables como mTLS o principal autenticado de gateway.

## Probes

| Probe | Endpoint | Propósito | Decisión de enrutamiento |
|-------|----------|-----------|--------------------------|
| Liveness | `GET /healthz` | Confirma que el proceso HTTP está vivo. | Reinicia solo tras fallos repetidos. No lo uses como único gate de tráfico. |
| Readiness | `GET /readyz` | Confirma readiness respaldada por almacenamiento. | Envía MCP traffic solo mientras responda correctamente. |
| Metrics | `GET /metrics` | Métricas Prometheus. | Scrapea en privado; no lo expongas públicamente. |
| Smoke de aplicación | `POST /mcp` autenticado con `mem_stats` o `mem_search` conocido. | Confirma MCP autenticado y herramienta con almacenamiento. | Ejecuta después de despliegue, restore o mitigación. |

`/healthz`, `/readyz` y `/metrics` no requieren autenticación por diseño. Restringe acceso con network policy, security groups, service mesh o topología privada.

## Checklist de despliegue

Antes del rollout:

- [ ] Generar `CEREBRO_AUTH_TOKEN` fuerte y guardarlo en secret manager.
- [ ] Generar credenciales no-placeholder para SurrealDB embebido.
- [ ] Confirmar que la configuración no usa valores demo como `local-dev-only`, `CHANGE_ME_BEFORE_PRODUCTION`, passwords `root` o bearer tokens placeholder.
- [ ] Configurar ruta durable explícita y ownership para el usuario runtime de Cerebro.
- [ ] Confirmar que solo una instancia activa puede escribir en la ruta configurada.
- [ ] Configurar ingress/gateway confiable con TLS, límite de cuerpo y rate limiting.
- [ ] Configurar probes `/healthz` y `/readyz`.
- [ ] Configurar scraping privado de `/metrics`.
- [ ] Configurar alertas por readiness, auth, storage, error rate y p95 de latencia.
- [ ] Probar backup y restore contra la ruta configurada.

Durante el rollout:

1. Desplegar la nueva instancia con tráfico deshabilitado o gated por readiness.
2. Confirmar en logs el bind, storage mode y storage path esperados, sin warnings de fallback.
3. Verificar liveness: `curl -f http://<private-cerebro-host>:4040/healthz`.
4. Verificar readiness: `curl -f http://<private-cerebro-host>:4040/readyz`.
5. Ejecutar smoke MCP autenticado:

   ```bash
   curl -fsS -X POST http://<private-cerebro-host>:4040/mcp \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${CEREBRO_AUTH_TOKEN}" \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{}}}' \
     | jq .
   ```

6. Confirmar que `/metrics` se scrapea.
7. Habilitar tráfico cuando readiness y smoke checks pasen.
8. Observar `cerebro_requests_total`, `cerebro_tool_latency_seconds`, `cerebro_storage_errors_total` y logs durante una ventana normal de tráfico.

Triggers de rollback:

- `/readyz` permanece fallido después de verificar mount y permisos de storage;
- errores de inicialización de storage o warnings de fallback aparecen inesperadamente;
- el ratio de error MCP server-side supera el umbral de page del despliegue;
- la latencia p95 de herramientas exitosas permanece sobre el umbral de page después de reducción de tráfico segura para rollback;
- los fallos de auth se disparan porque los callers no fueron actualizados con el token esperado.

## Rotación de token

Cerebro acepta un `CEREBRO_AUTH_TOKEN` a la vez. Usa compatibilidad en clientes o ingress para evitar downtime.

Rotación planificada:

1. Generar token nuevo y guardarlo en secret manager.
2. Actualizar clientes MCP, gateway o ingress para enviar o aceptar el token nuevo. Si el gateway soporta doble validación, permite token viejo y nuevo temporalmente mientras Cerebro usa el viejo.
3. Desplegar el nuevo `CEREBRO_AUTH_TOKEN` en Cerebro y reiniciar o hacer rollout.
4. Verificar `/readyz` y un `mem_stats` autenticado con el token nuevo.
5. Confirmar que llamadas con el token viejo fallen con `401 Unauthorized` al cerrar la ventana de compatibilidad.
6. Eliminar el token viejo de clientes, gateway, secretos y notas de incidente.
7. Vigilar `cerebro_auth_failures_total` e ingress logs por clientes obsoletos.

Síntomas esperados post-rotación:

- Tokens faltantes u obsoletos retornan `401 Unauthorized`.
- Los fallos de auth pueden subir temporalmente mientras los clientes refrescan secretos.
- Readiness debe permanecer saludable; la rotación de token no debe afectar readiness de storage.

Rotación de emergencia por posible filtración:

1. Bloquear fuentes sospechosas en ingress si es posible.
2. Generar y desplegar token nuevo inmediatamente.
3. Reiniciar o hacer rollout de Cerebro.
4. Actualizar clientes confiables o mappings del gateway.
5. Revocar el token viejo en todas partes.
6. Revisar `cerebro_auth_failures_total`, `cerebro_requests_total{status="unauthorized"}` e ingress logs.

## Diagnóstico de readiness

Usa este flujo cuando `/readyz` falle:

1. Confirmar alcance: ¿`/healthz` responde `200`? ¿Falla una instancia o todo el despliegue? ¿Empezó tras deploy, reinicio, cambio de storage, token o mantenimiento de nodo?
2. Sacar la instancia fallida del tráfico MCP. No uses `/healthz` como gate de tráfico.
3. Revisar logs por errores de inicialización de storage, readiness, fallback o permisos.
4. Revisar la ruta de storage:
   - existe en `surreal.storage_path` o `storage_path`;
   - volumen/disco montado read-write;
   - usuario runtime puede leer y escribir;
   - hay capacidad e inodos disponibles;
   - no hay otro proceso Cerebro usando la misma ruta.
5. Revisar métricas: `cerebro_readiness_failures_total`, `cerebro_storage_errors_total` y error ratio server-side.
6. Corregir mount, permisos o capacidad.
7. Reiniciar solo después de preservar evidencia útil y confirmar la ruta configurada.
8. Verificar `/readyz`.
9. Ejecutar `mem_stats` o `mem_search` conocido.
10. Devolver tráfico solo después de readiness y smoke checks correctos.

Si también falla `/healthz`, trátalo como incidente de proceso/runtime: revisa crash logs, exit code, eventos del scheduler, presión de memoria y compatibilidad de binario/configuración antes de enfocarte en storage.

## Recuperación de almacenamiento

Usa este flujo para corrupción, borrado accidental, migración problemática o fallos persistentes de readiness ligados a la ruta durable.

Contención inmediata:

1. Dejar de enrutar tráfico MCP a la instancia.
2. Detener Cerebro limpiamente si es posible.
3. Preservar el directorio actual de storage antes de borrarlo o sobrescribirlo.
4. Registrar ruta configurada, versión del binario, hash de config, última readiness correcta y backup candidato.

Restore desde backup:

1. Mantener Cerebro detenido. SurrealDB embebido usa RocksDB; no hagas backups/restores de archivos mientras Cerebro corre.
2. Mover la ruta dañada en vez de borrarla:

   ```bash
   sudo mv /var/lib/cerebro/data /var/lib/cerebro/data.failed.$(date +%Y%m%d-%H%M%S)
   ```

3. Restaurar el último backup conocido como bueno:

   ```bash
   sudo cp -a /backup/cerebro-20260501-120000 /var/lib/cerebro/data
   sudo chown -R cerebro:cerebro /var/lib/cerebro/data
   ```

4. Arrancar Cerebro con la misma configuración durable.
5. Confirmar `/readyz`.
6. Ejecutar `mem_stats` y comparar conteos esperados.
7. Ejecutar un `mem_search` conocido.
8. Devolver tráfico solo cuando las verificaciones pasen.
9. Conservar la copia fallida hasta cerrar la revisión del incidente.

Fallback de emergencia:

- `storage_fallback = "in_memory"` puede mantener disponibilidad si el almacenamiento durable no inicializa, pero las escrituras nuevas no sobreviven reinicios.
- Declara la instancia degradada en durabilidad, notifica a owners y prioriza restaurar almacenamiento durable.
- No trates fallback in-memory como recuperación completada.

## Respuesta a incidentes rápida

| Síntoma | Primera acción | Área probable | Verificación |
|---------|----------------|---------------|--------------|
| `/healthz` falla | Inspeccionar proceso/contenedor y logs recientes. | Crash, config runtime, nodo/scheduler. | `/healthz` responde `200` y el proceso queda estable. |
| `/readyz` falla pero `/healthz` pasa | Sacar de tráfico y revisar ruta, permisos, capacidad y logs de storage. | Conectividad o inicialización de storage. | `/readyz` responde `200`; `mem_stats` funciona. |
| Pico de `401` | Revisar rollout de token, clientes viejos, ingress logs y posible scanning. | Auth o endpoint filtrado. | Token nuevo funciona; token viejo falla; auth vuelve a baseline. |
| Error ratio MCP server-side sube | Revisar errores de storage, logs internos y deploys recientes. | Storage o regresión de servicio. | Ratio vuelve bajo umbral de warning. |
| p95 de herramienta sube | Revisar saturación de storage, CPU, memoria, concurrencia y tool labels. | Capacidad o performance backend. | p95 vuelve a baseline. |
| Pico de errores de storage por operación | Identificar operación afectada e inspeccionar ruta durable. | Fallo parcial de storage o problema en data path. | `cerebro_storage_errors_total` deja de incrementar y smoke checks pasan. |

## Alertas iniciales

Ajusta umbrales al baseline del despliegue. Buenos defaults:

- readiness: `increase(cerebro_readiness_failures_total[5m]) >= 3` o éxito de `/readyz` bajo `95%` por 5 minutos;
- auth: `increase(cerebro_auth_failures_total[10m]) > 20` o más de `5x` el baseline de 24 horas;
- error server-side: warning sobre `2%` y page sobre `5%` para `cerebro_requests_total{status=~"storage_error|internal_error"}` dividido por todas las requests MCP;
- storage: `increase(cerebro_storage_errors_total[5m]) >= 5` para una operación;
- latencia: warning cuando p95 de herramientas exitosas supere `1s`, page cuando supere `2s` por 10 minutos.

Mantén alertas de validación y autenticación separadas de alertas server-side para que clientes rotos no oculten fallos de storage o internos.
