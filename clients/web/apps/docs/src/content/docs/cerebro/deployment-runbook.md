---
title: Cerebro Deployment Runbook
description: >-
  Operational deployment runbook for Cerebro production rollouts,
  incidents, token rotation, readiness troubleshooting, and storage recovery.
owner: team-platform
status: canonical
lastReviewed: 2026-05-02
appliesTo: main
docType: runbook
---

# Cerebro Deployment Runbook

Use this runbook when deploying or operating Cerebro as a production MCP memory service. It is intentionally operational: it covers the required secrets, configuration, topology, probes, rollout checks, incident response, token rotation, readiness troubleshooting, and storage recovery steps an on-call operator needs.

For deeper background, keep these references nearby:

- [Cerebro configuration](configuration.md)
- [Running Cerebro](running.md)
- [Cerebro operations](operations.md)

## Production posture

Cerebro's supported durable production posture in this build is **single-node and local-first**:

- run one writable Cerebro instance per durable storage path;
- use embedded SurrealDB or disk-backed node-local storage for durable data;
- attach the storage path to persistent node-local disk or a persistent volume that is mounted by only one active Cerebro instance;
- route application traffic only after `/readyz` succeeds;
- keep `/healthz`, `/readyz`, and `/metrics` private to the orchestrator and monitoring network;
- expose `POST /mcp` only behind trusted ingress, gateway, private network, or service mesh controls.

`remote_surreal`, shared remote persistence, active-active HA, and multiple writers on the same storage path are unsupported in this build.

## Required secrets

| Secret | Required | How to provide | Rotation notes |
|--------|----------|----------------|----------------|
| `CEREBRO_AUTH_TOKEN` | Required for production and for any non-loopback bind. | Secret manager or orchestrator secret environment variable. Prefer env injection over config files. | Rotate with a short dual-token compatibility window at ingress or caller config level. Cerebro accepts one configured token at a time. |
| `surreal.password` | Required when `storage_mode = "embedded_surreal"`. | Secret manager template, mounted secret file rendered into config, or protected deployment config. | Rotate during a maintenance window. Restart Cerebro after updating the secret and verify `/readyz`. |
| `CEREBRO_AUDIT_TOKEN` | Optional. | Secret manager or orchestrator secret environment variable. | Rotate like other service-to-service credentials if audit integrations rely on it. |

Never commit production tokens, generated config with live secrets, storage snapshots, or backup archives to source control.

## Required configuration

Production deployments must set or verify these values explicitly:

| Setting | Production guidance |
|---------|---------------------|
| `host` | Use `127.0.0.1` for sidecar or local gateway topologies. Use `0.0.0.0` only inside a private pod/container network or behind trusted ingress. Cerebro refuses non-loopback bind without `CEREBRO_AUTH_TOKEN`. |
| `port` | Default `4040`; keep consistent with Service, ingress, and probe definitions. |
| `scheme` | Usually inferred. Set only when generated endpoint URLs must reflect gateway TLS termination. |
| `request_timeout_secs` | Start with `30`. Increase only for measured slow storage or known long-running tools. |
| `max_concurrent_mcp_requests` | Start with `32`. Lower for small nodes or slow disks; raise only with CPU, memory, and storage headroom. |
| `storage_mode` | `embedded_surreal` for the default durable production mode. `disk` is a simpler node-local durable alternative. Do not use `in_memory` for normal production. |
| `surreal.storage_path` or `storage_path` | Use an explicit durable path such as `/var/lib/cerebro/data`. Avoid relying on the process working directory. |
| `storage_fallback` | Prefer `none` for normal production so startup fails clearly if durable storage is unavailable. Use `in_memory` only as an emergency availability mode with durability-degraded incident tracking. |
| `RUST_LOG` | Start with `info`. Temporarily use `cerebro=debug,surrealdb=warn` for investigations. |

## Minimal production config

This example assumes Cerebro runs in a private container or pod network behind ingress that provides TLS, rate limiting, and network access control. Secrets are injected by the orchestrator.

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
# Render from a secret manager or protected config template.
password = "${CEREBRO_SURREAL_PASSWORD}"

[tui]
enabled = false
```

Runtime environment:

```bash
CEREBRO_AUTH_TOKEN=<generated-service-token>
CEREBRO_SURREAL_PASSWORD=<generated-storage-password>
RUST_LOG=info
```

If your config renderer does not expand `${...}` placeholders, render the final file before startup or use the deployment platform's supported secret projection mechanism.

## Deployment topology

A minimal production topology has these layers:

1. **Trusted ingress or gateway** terminates TLS, enforces request-frequency rate limits, strips or validates forwarding headers, and forwards only approved callers to Cerebro.
2. **Cerebro service instance** listens on the private network and protects `POST /mcp` with bearer authentication, body limits, timeout limits, and concurrency limits.
3. **Durable node-local storage** is mounted at the configured storage path and attached to only one active Cerebro process.
4. **Monitoring stack** scrapes `/metrics` and probes `/healthz` and `/readyz` from a private monitoring network.
5. **Log pipeline** collects structured logs with deployment, instance, and storage-path metadata.

Do not expose Cerebro directly to the public internet. Source-IP rate limits are acceptable only when ingress is the sole trusted hop and strips or validates `X-Forwarded-For`; otherwise use non-spoofable identities such as mTLS identity or authenticated gateway principal.

## Probe configuration

| Probe | Endpoint | Purpose | Routing decision |
|-------|----------|---------|------------------|
| Liveness | `GET /healthz` | Confirms the HTTP process is alive. | Restart only after repeated failures. Do not use as the sole traffic gate. |
| Readiness | `GET /readyz` | Confirms storage-backed readiness checks pass. | Send MCP traffic only while this succeeds. |
| Metrics | `GET /metrics` | Prometheus-compatible operational metrics. | Scrape privately; do not expose publicly. |
| Application smoke | Authenticated `POST /mcp` with `mem_stats` or known `mem_search`. | Confirms authenticated MCP and storage-backed tool execution. | Run after deploy, restore, or incident mitigation. |

Example Kubernetes-style probe targets:

```yaml
livenessProbe:
  httpGet:
    path: /healthz
    port: 4040
  periodSeconds: 10
  failureThreshold: 3
readinessProbe:
  httpGet:
    path: /readyz
    port: 4040
  periodSeconds: 10
  failureThreshold: 2
```

`/healthz`, `/readyz`, and `/metrics` are intentionally unauthenticated. Restrict them with network policy, security groups, service mesh policy, or private service topology.

## Deployment checklist

Before rollout:

- [ ] Generate a strong `CEREBRO_AUTH_TOKEN` and store it in the deployment secret manager.
- [ ] Generate non-placeholder embedded SurrealDB credentials.
- [ ] Confirm config does not use demo values such as `local-dev-only`, `CHANGE_ME_BEFORE_PRODUCTION`, `root` as a password, or placeholder bearer tokens.
- [ ] Configure explicit durable storage path and mount ownership for the Cerebro runtime user.
- [ ] Confirm only one active Cerebro instance can write to the configured storage path.
- [ ] Configure trusted ingress or gateway TLS, authentication boundary, body limits, and rate limiting.
- [ ] Configure liveness and readiness probes using `/healthz` and `/readyz`.
- [ ] Configure private metrics scraping for `/metrics`.
- [ ] Configure alerts for readiness failures, auth failures, storage errors, server-side error rate, and p95 tool latency.
- [ ] Confirm backup and restore procedures have been tested against the configured storage path.

During rollout:

1. Deploy the new instance with traffic disabled or readiness-gated.
2. Confirm startup logs show the expected bind address, storage mode, and storage path without fallback warnings.
3. Check liveness:

   ```bash
   curl -f http://<private-cerebro-host>:4040/healthz
   ```

4. Check readiness:

   ```bash
   curl -f http://<private-cerebro-host>:4040/readyz
   ```

5. Run an authenticated MCP smoke check:

   ```bash
   curl -fsS -X POST http://<private-cerebro-host>:4040/mcp \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer ${CEREBRO_AUTH_TOKEN}" \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{}}}' \
     | jq .
   ```

6. Confirm `/metrics` is being scraped and new time series are visible.
7. Enable traffic after readiness and smoke checks pass.
8. Watch `cerebro_requests_total`, `cerebro_tool_latency_seconds`, `cerebro_storage_errors_total`, and structured logs for at least one normal traffic window.

Rollback triggers:

- `/readyz` remains failed after storage mount and permission checks;
- storage initialization errors or fallback warnings appear unexpectedly;
- server-side MCP error ratio exceeds the deployment's page threshold;
- p95 successful tool latency remains above the page threshold after rollback-safe traffic reduction;
- auth failures spike because callers were not updated with the expected token.

## Token rotation

Cerebro accepts one `CEREBRO_AUTH_TOKEN` at a time. Use caller or ingress compatibility to avoid downtime.

Planned rotation:

1. Generate a new token and store it in the secret manager.
2. Update MCP clients, gateway secret mappings, or ingress auth policy so callers can send the new token. If your gateway supports dual validation, allow both old and new tokens temporarily at the gateway while Cerebro still uses the old token.
3. Deploy the new `CEREBRO_AUTH_TOKEN` to Cerebro and restart or roll the service according to your orchestrator.
4. Run `/readyz` and an authenticated `mem_stats` smoke check with the new token.
5. Confirm old-token calls fail with `401 Unauthorized` after the compatibility window closes.
6. Remove the old token from clients, gateway policy, secret stores, runbooks, and incident notes.
7. Watch `cerebro_auth_failures_total` and ingress logs for stale callers.

Emergency rotation for suspected leak:

1. Block suspicious sources at ingress if possible.
2. Generate and deploy a new token immediately.
3. Restart/roll Cerebro so the new token is active.
4. Update trusted clients or gateway mappings.
5. Revoke the old token everywhere.
6. Review `cerebro_auth_failures_total`, `cerebro_requests_total{status="unauthorized"}`, and ingress logs for continued use of the old token.

Expected post-rotation symptoms:

- Missing or stale tokens return `401 Unauthorized`.
- Auth failures may temporarily rise while clients refresh secrets.
- Readiness should remain healthy; token rotation should not affect storage readiness.

## Readiness troubleshooting

Use this flow when `/readyz` fails.

1. Confirm scope:
   - Does `/healthz` still return `200`?
   - Is only one instance failing, or all instances for this deployment?
   - Did failure start after deploy, restart, storage move, token change, or node maintenance?
2. Remove the failing instance from MCP traffic. Do not rely on `/healthz` for routing.
3. Check logs for storage initialization errors, readiness failures, storage fallback warnings, or permission errors.
4. Check storage path:
   - path exists at the configured `surreal.storage_path` or `storage_path`;
   - persistent volume or disk is mounted read-write;
   - runtime user owns or can read/write the directory;
   - disk capacity and inodes are available;
   - no second Cerebro process is using the same storage path.
5. Check metrics:
   - `increase(cerebro_readiness_failures_total[5m])`;
   - `increase(cerebro_storage_errors_total[5m])` by `operation`;
   - server-side error ratio from `cerebro_requests_total{status=~"storage_error|internal_error"}`.
6. Fix the underlying mount, permission, or capacity issue.
7. Restart Cerebro only after preserving useful failure evidence and confirming the configured storage path is correct.
8. Verify `/readyz` succeeds.
9. Run authenticated `mem_stats` or a known `mem_search` smoke check.
10. Return traffic only after readiness and smoke checks pass.

If `/healthz` fails too, treat it as a process or runtime incident: inspect crash logs, container exit code, scheduler events, memory pressure, and binary/config compatibility before focusing on storage.

## Storage recovery

Use this flow for storage corruption, accidental deletion, bad migration, or persistent readiness failures tied to the durable path.

Immediate containment:

1. Stop routing MCP traffic to the instance.
2. Stop Cerebro gracefully if possible.
3. Preserve the current storage directory for analysis before deleting or overwriting it.
4. Record the configured storage path, binary version, config hash, last successful readiness time, and backup candidate.

Restore from backup:

1. Keep Cerebro stopped. Embedded SurrealDB uses RocksDB; do not take or restore file-level backups while Cerebro is running.
2. Move the damaged storage directory aside instead of deleting it:

   ```bash
   sudo mv /var/lib/cerebro/data /var/lib/cerebro/data.failed.$(date +%Y%m%d-%H%M%S)
   ```

3. Restore the latest known-good backup to the configured path:

   ```bash
   sudo cp -a /backup/cerebro-20260501-120000 /var/lib/cerebro/data
   sudo chown -R cerebro:cerebro /var/lib/cerebro/data
   ```

4. Start Cerebro with the same durable backend configuration.
5. Confirm `/readyz` succeeds.
6. Run `mem_stats` and compare counts with pre-incident or pre-backup expectations.
7. Run a known `mem_search` query for representative data.
8. Return traffic only after data checks pass.
9. Keep the failed storage copy until the incident review is complete.

Emergency fallback:

- `storage_fallback = "in_memory"` may keep the service available if durable storage cannot initialize, but new writes are not durable across restart.
- Declare the instance durability-degraded, notify service owners, and prioritize restoration to embedded or disk-backed durable storage.
- Do not treat in-memory fallback as a completed recovery.

## Incident response quick reference

| Symptom | First action | Likely area | Verification |
|---------|--------------|-------------|--------------|
| `/healthz` fails | Inspect process/container state and recent logs. | Process crash, runtime config, scheduler/node failure. | `/healthz` returns `200`; process remains stable. |
| `/readyz` fails but `/healthz` passes | Remove from traffic and inspect storage path, permissions, capacity, and storage logs. | Storage connectivity or storage initialization. | `/readyz` returns `200`; `mem_stats` succeeds. |
| Spike in `401` responses | Check token rollout, stale clients, ingress logs, and possible scanning. | Authentication or leaked endpoint. | New token succeeds; old token fails; auth failures return to baseline. |
| Server-side MCP error ratio rises | Check storage errors, internal error logs, and recent deploys. | Storage or service regression. | Error ratio returns below warning threshold. |
| Tool p95 latency rises | Check storage saturation, CPU, memory, request concurrency, and slow tool labels. | Capacity or backend performance. | p95 latency returns to baseline. |
| Storage errors spike by operation | Identify affected operation and inspect durable path. | Partial storage failure or data path issue. | `cerebro_storage_errors_total` stops increasing and smoke checks pass. |

## Alert starting points

Tune thresholds to your deployment baseline. Good production defaults are:

- readiness degradation: `increase(cerebro_readiness_failures_total[5m]) >= 3` or `/readyz` success rate below `95%` for 5 minutes;
- auth anomaly: `increase(cerebro_auth_failures_total[10m]) > 20` or more than `5x` the 24-hour baseline;
- server-side error rate: warn above `2%` and page above `5%` for `cerebro_requests_total{status=~"storage_error|internal_error"}` divided by all MCP requests;
- storage operation spike: `increase(cerebro_storage_errors_total[5m]) >= 5` for one operation;
- latency spike: warn when p95 successful tool latency exceeds `1s`, and page when it exceeds `2s` for 10 minutes.

Keep validation and authentication alerts separate from server-side error alerts so broken clients do not mask storage or internal failures.
