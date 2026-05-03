# Corvus Rook

**Rook** is a standalone, local-first AI provider gateway for the Corvus platform. It exposes an OpenAI-compatible HTTP API, persists operator-managed provider configuration in SQLite, and routes requests across configured provider accounts and pools.

Rook is designed to run close to the operator: on a workstation, on a private host, or in a container behind infrastructure you control. It does not require a Corvus cloud service to run.

## Implemented runtime capabilities

Rook currently provides:

- **OpenAI-compatible gateway routes**
  - `GET /v1/models`
  - `POST /v1/chat/completions`
- **Provider/account routing** through persisted accounts, pools, model routes, routing policies, and health-aware fallback behavior.
- **Embedded dashboard and admin API** for accounts, pools, routes, settings, usage, health, metrics, and audit events.
- **SQLite persistence** for gateway configuration, accounts, pools, routes, settings, usage summaries, idempotency records, and admin audit events.
- **Inbound bearer authentication** for protected `/api/*` and `/v1/*` routes when enabled.
- **Loopback-first default bind posture** for local deployments.
- **Container images** that intentionally bind to `0.0.0.0` inside the container so an operator can publish the service explicitly.
- **Operational health endpoints** for liveness, readiness, compatibility health checks, and Prometheus/OpenMetrics scraping.
- **Rate limiting, request ID propagation, trusted proxy controls, and idempotency replay protection** for gateway traffic.
- **Operator TUI** for status/inspection workflows; dashboard-backed mutations remain the primary supported setup workflow.

Rook does **not** currently claim durable health history, billing/cost accounting, or a production multi-node clustered deployment mode. Health is runtime-current, usage accounting is request/outcome focused, and SQLite is the supported persistence backend.

## Supported production postures

### Officially supported

1. **Local-first single instance**
   - Rook runs directly on a workstation or private host.
   - Default bind is `127.0.0.1:4141`.
   - Recommended for personal automation, local gateway use, and operator-controlled private hosts.

2. **Single containerized instance**
   - Rook runs in one container with a persistent volume mounted at `/rook-data`.
   - Container defaults bind to `0.0.0.0:4141` inside the container so Docker/Kubernetes port publishing works.
   - Recommended only behind a local firewall, reverse proxy, VPN, or private network boundary.

3. **Single instance behind a trusted reverse proxy**
   - Rook remains a single process with one SQLite database.
   - Enable inbound auth for all externally reachable deployments.
   - Configure trusted proxy settings only for known proxy CIDRs.

### Not officially supported

- Multi-writer or active-active Rook nodes sharing one SQLite database.
- Exposing Rook directly to the public internet without a reverse proxy and inbound bearer auth.
- Treating runtime account health as a persisted historical health ledger.
- Using `rook.db` as a replicated database without operator-managed SQLite backup/restore discipline.

## Quick start

From the repository root:

```bash
# Validate the crate
cargo check --manifest-path clients/rook/Cargo.toml

# Run with local-first defaults: 127.0.0.1:4141 and ./rook.db
cargo run --manifest-path clients/rook/Cargo.toml -- serve

# Run diagnostics against the effective config and database path
cargo run --manifest-path clients/rook/Cargo.toml -- doctor

# Export the validated effective config with secret values redacted
cargo run --manifest-path clients/rook/Cargo.toml -- config export
```

Once running, point an OpenAI-compatible client at:

```text
http://127.0.0.1:4141/v1
```

The embedded dashboard is served from the same HTTP server. Operational routes live under `/api/*` and gateway routes live under `/v1/*`.

## Deployment

### Local-first host deployment

Use this mode when Rook should be reachable only from the local machine or from a private host-level tunnel.

1. Build or run the gateway:

   ```bash
   cargo run --manifest-path clients/rook/Cargo.toml -- serve \
     --host 127.0.0.1 \
     --port 4141 \
     --db-path /var/lib/rook/rook.db
   ```

2. Keep the database path on durable local storage.

3. If any process outside the same host can reach Rook, enable inbound auth:

   ```bash
   export ROOK_INBOUND_AUTH_ENABLED=true
   export ROOK_INBOUND_AUTH_TOKEN='replace-with-a-long-random-token'

   cargo run --manifest-path clients/rook/Cargo.toml -- serve \
     --host 127.0.0.1 \
     --port 4141 \
     --db-path /var/lib/rook/rook.db
   ```

4. Verify readiness:

   ```bash
   curl -fsS http://127.0.0.1:4141/api/health/ready
   ```

5. Verify gateway access:

   ```bash
   curl -fsS http://127.0.0.1:4141/v1/models \
     -H 'Authorization: Bearer replace-with-a-long-random-token'
   ```

   Omit the `Authorization` header only when inbound auth is disabled and the listener is not externally reachable.

### Containerized deployment

Build from the repository root so Rook path dependencies resolve:

```bash
docker build -f clients/rook/Dockerfile -t corvus-rook:local .
```

Run with a named volume and inbound auth:

```bash
docker volume create rook-data

docker run --rm \
  --name rook \
  -p 127.0.0.1:4141:4141 \
  -v rook-data:/rook-data \
  -e ROOK_INBOUND_AUTH_ENABLED=true \
  -e ROOK_INBOUND_AUTH_TOKEN='replace-with-a-long-random-token' \
  corvus-rook:local
```

The container image command uses:

```text
rook serve --host 0.0.0.0 --port 4141 --db-path /rook-data/rook.db
```

That bind is intentional for container networking. Keep Docker/Kubernetes publishing restrictive unless the service is behind a proxy, VPN, or private network.

### Reverse proxy deployment

When Rook is behind a reverse proxy:

1. Keep Rook as a single backend instance.
2. Enable inbound auth with `ROOK_INBOUND_AUTH_ENABLED=true` and `ROOK_INBOUND_AUTH_TOKEN`.
3. Terminate TLS at the proxy or at infrastructure in front of Rook.
4. Configure rate limits appropriate for the exposed surface.
5. Enable trusted proxy handling only for proxy source CIDRs you control.

Example trusted proxy environment:

```bash
export ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED=true
export ROOK_TRANSPORT_TRUSTED_PROXY_TRUSTED_CIDRS='10.0.0.0/8,192.168.0.0/16'
export ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_FOR=true
export ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PROTO=true
export ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_HOST=true
```

Do not enable forwarded headers for arbitrary client networks. Rook validates that trusted proxy mode has CIDRs configured.

## Configuration

Rook resolves one effective runtime configuration for both `rook serve` and `rook config export`.

### Default config discovery

Rook looks for `config.toml` in this order:

1. `$XDG_CONFIG_HOME/rook/config.toml`
2. `$HOME/.config/rook/config.toml`

If no config file exists, Rook uses built-in defaults.

### Built-in defaults

| Setting | Default |
|---|---|
| `host` | `127.0.0.1` |
| `port` | `4141` |
| `enable_tui` | `false` |
| `db_path` | `./rook.db` |
| inbound auth | disabled |
| request ID header | `x-request-id` |
| request ID max length | `128` |
| trusted proxy | disabled |
| `/api/*` rate limit | `60` requests / `60` seconds |
| `/v1/models` rate limit | `120` requests / `60` seconds |
| `/v1/chat/completions` rate limit | `30` requests / `60` seconds |
| chat idempotency | enabled |
| chat idempotency replay window | `86400` seconds |

### Precedence

When the same setting appears in multiple places, Rook applies:

1. built-in defaults
2. config file values
3. `ROOK_*` environment overrides
4. CLI flags

CLI flags always win. Environment values beat file values. File values beat defaults.

### Example `config.toml`

```toml
host = "127.0.0.1"
port = 4141
enable_tui = false
db_path = "/var/lib/rook/rook.db"

[inbound_auth]
enabled = true
bearer_token = "replace-with-a-long-random-token"

[transport.request_id]
inbound_header_name = "x-request-id"
response_header_name = "x-request-id"
max_length = 128

[transport.trusted_proxy]
enabled = false
trusted_cidrs = []

[transport.trusted_proxy.allowed_headers]
forwarded = false
x_forwarded_for = false
x_forwarded_host = false
x_forwarded_proto = false
x_forwarded_port = false
x_real_ip = false

[rate_limits.api]
max_requests = 60
window_seconds = 60

[rate_limits.v1_models]
max_requests = 120
window_seconds = 60

[rate_limits.v1_chat_completions]
max_requests = 30
window_seconds = 60

[idempotency.chat_completions]
enabled = true
replay_window_seconds = 86400
```

### Supported environment overrides

Top-level:

- `ROOK_HOST`
- `ROOK_PORT`
- `ROOK_ENABLE_TUI`
- `ROOK_DB_PATH`

Inbound auth:

- `ROOK_INBOUND_AUTH_ENABLED`
- `ROOK_INBOUND_AUTH_TOKEN`

Transport:

- `ROOK_TRANSPORT_REQUEST_ID_INBOUND_HEADER_NAME`
- `ROOK_TRANSPORT_REQUEST_ID_RESPONSE_HEADER_NAME`
- `ROOK_TRANSPORT_REQUEST_ID_MAX_LENGTH`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED`
- `ROOK_TRANSPORT_TRUSTED_PROXY_TRUSTED_CIDRS`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_FORWARDED`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_FOR`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_HOST`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PROTO`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PORT`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_REAL_IP`

Rate limits:

- `ROOK_API_RATE_LIMIT_MAX_REQUESTS`
- `ROOK_API_RATE_LIMIT_WINDOW_SECONDS`
- `ROOK_V1_MODELS_RATE_LIMIT_MAX_REQUESTS`
- `ROOK_V1_MODELS_RATE_LIMIT_WINDOW_SECONDS`
- `ROOK_V1_CHAT_RATE_LIMIT_MAX_REQUESTS`
- `ROOK_V1_CHAT_RATE_LIMIT_WINDOW_SECONDS`

Idempotency:

- `ROOK_CHAT_IDEMPOTENCY_ENABLED`
- `ROOK_CHAT_IDEMPOTENCY_REPLAY_WINDOW_SECONDS`

Boolean environment values accept `true`, `false`, `1`, `0`, `yes`, `no`, `on`, and `off`.

### Validation and safe export

Rook validates the final effective config before startup and before export. Invalid config fails closed with operator-facing errors. Examples include:

- blank host or database path
- inbound auth enabled without a non-blank token
- invalid request ID header names
- trusted proxy enabled without CIDRs
- zero-valued rate limits or idempotency replay windows

Export the effective config with secrets redacted:

```bash
rook config export
```

For source-tree runs:

```bash
cargo run --manifest-path clients/rook/Cargo.toml -- config export
```

## Security posture

- Rook defaults to `127.0.0.1` outside containers.
- Container images bind to `0.0.0.0` only inside the container; the operator controls actual reachability through port publishing and network policy.
- Enable inbound bearer auth for every deployment reachable beyond a trusted local process.
- Inbound bearer auth protects `/api/*` and `/v1/*` routes; it is separate from outbound provider API keys.
- Provider API keys are persisted in SQLite and redacted in operator-facing account views, config export, debug output, and structured request logs.
- Do not reuse inbound auth tokens as provider API keys.
- Prefer TLS at a reverse proxy for non-local traffic.
- Treat `rook.db` and its backups as secret-bearing artifacts.

## Storage, backups, and restore

Rook uses SQLite at `db_path`. The default is `./rook.db`, which is convenient for development but not ideal for production. Production deployments should set an explicit durable path, such as:

- local host: `/var/lib/rook/rook.db`
- container: `/rook-data/rook.db`

Back up the database regularly. Use SQLite's online backup command while Rook is running:

```bash
sqlite3 /var/lib/rook/rook.db ".backup '/var/backups/rook/rook-$(date +%Y%m%d-%H%M%S).db'"
```

For containers, the distroless runtime image is intentionally minimal and should not be assumed to include `sqlite3`. Use one of these operator-controlled approaches:

- run a temporary SQLite-capable helper container against the same persistent volume and execute `.backup` there
- stop Rook briefly and copy the database file from the persistent volume
- use platform-native volume snapshotting when your container platform supports consistent snapshots

Restore procedure:

1. Stop Rook.
2. Preserve the current database before replacing it.
3. Copy the selected backup to the configured `db_path`.
4. Ensure file ownership and permissions match the Rook process user.
5. Start Rook.
6. Run `rook doctor` and check `/api/health/ready`.

Never restore a backup into a running Rook process.

## Health checks and observability

Operational endpoints:

| Endpoint | Purpose |
|---|---|
| `GET /api/health` | Compatibility check; returns `ok` when the HTTP server responds. |
| `GET /api/health/live` | Liveness; returns JSON status for process liveness. |
| `GET /api/health/ready` | Readiness; reports config, database, router, and embedded asset startup readiness. |
| `GET /api/metrics` | OpenMetrics/Prometheus scrape output. |
| `GET /api/health/accounts` | Current account health view. |
| `GET /api/health/summary` | Current health summary. |
| `GET /api/usage` | Persisted request/outcome usage summaries. |
| `GET /api/audit/events` | Recent persisted admin audit events. |

Recommended probes:

```bash
curl -fsS http://127.0.0.1:4141/api/health/live
curl -fsS http://127.0.0.1:4141/api/health/ready
curl -fsS http://127.0.0.1:4141/api/metrics
```

When inbound auth is enabled, include:

```bash
-H 'Authorization: Bearer replace-with-a-long-random-token'
```

Readiness can be `fail` when config, database, or routing startup checks fail. It can be `degraded` when embedded dashboard assets are missing while critical gateway dependencies are ready.

## Operational workflows

### Start and verify

```bash
rook serve --host 127.0.0.1 --port 4141 --db-path /var/lib/rook/rook.db
rook doctor
curl -fsS http://127.0.0.1:4141/api/health/ready
```

### Inspect effective configuration

```bash
rook config export
```

Use this before and after environment or config-file changes. Secret-bearing values are redacted.

### Configure accounts, pools, and routes

Use the embedded dashboard or admin API to manage provider accounts, pools, routes, and settings. The dashboard-backed workflow is the supported mutation path. The TUI is intended for operator inspection/status workflows and dashboard bridge flows.

### Rotate inbound auth token

1. Generate a new long random token.
2. Update `ROOK_INBOUND_AUTH_TOKEN` or `[inbound_auth].bearer_token`.
3. Restart Rook.
4. Update clients and probes.
5. Confirm old-token requests receive `401 Unauthorized` and new-token requests succeed.

### Move the database

1. Stop Rook.
2. Copy `rook.db` to the new durable location.
3. Update `db_path` or `ROOK_DB_PATH`.
4. Start Rook.
5. Run `rook doctor`.
6. Confirm `/api/health/ready` reports ready.

## Troubleshooting

### `rook serve` fails with a config error

Run:

```bash
rook config export
```

Check for blank values, invalid numeric values, enabled inbound auth without a token, trusted proxy enabled without CIDRs, and zero-valued rate-limit windows.

### `401 Unauthorized` on `/api/*` or `/v1/*`

Inbound auth is enabled and the request is missing the expected header:

```text
Authorization: Bearer <ROOK_INBOUND_AUTH_TOKEN>
```

Confirm the effective auth state with `rook config export`. The token itself will be redacted.

### Cannot connect to Rook

- For local mode, confirm Rook is bound to `127.0.0.1:4141` unless you intentionally changed `host`.
- For containers, confirm port publishing, for example `-p 127.0.0.1:4141:4141`.
- Check whether a firewall, proxy, or network policy blocks the port.
- Confirm the process logs show `Rook listening on http://...`.

### Readiness fails

Query:

```bash
curl -fsS http://127.0.0.1:4141/api/health/ready
```

Inspect the `checks` object:

- `config.ready = false`: fix invalid config and restart.
- `database.ready = false`: verify `db_path`, parent directory, permissions, and disk availability.
- `router.ready = false`: inspect route/pool/account configuration and startup logs.
- `assets.ready = false`: rebuild or use an image/binary that includes embedded dashboard assets; gateway operation may be degraded rather than fully failed.

### SQLite database cannot be opened

- Ensure the parent directory exists.
- Ensure the Rook process user can read and write the database and directory.
- Check available disk space.
- Avoid sharing one SQLite file between multiple active Rook instances.
- If corruption is suspected, stop Rook and restore from a known-good backup.

### No route for a chat completion model

- Confirm the requested model matches a persisted model route.
- Confirm the target pool has enabled account members.
- Confirm provider account capabilities include chat support.
- Check `/api/health/accounts` and `/api/health/summary` for current runtime health.

### Upstream provider requests fail

- Verify the provider account has a valid outbound API key.
- Verify any `api_base_override` is correct.
- Check provider-side rate limits and account status.
- Inspect `/api/metrics` for gateway and upstream outcome counters.
- Remember that inbound bearer auth is not reused as outbound provider auth.

### Rate limited requests

Rook has default global rate limits for `/api/*`, `/v1/models`, and `/v1/chat/completions`. Tune the `rate_limits` config or corresponding `ROOK_*_RATE_LIMIT_*` environment variables if legitimate traffic is rejected.

### Idempotency conflicts on chat completions

`POST /v1/chat/completions` has idempotency replay protection enabled by default. Reusing an idempotency key with a different request body can produce a conflict. Use unique keys per logical request or disable/tune idempotency only when the operational risk is understood.

## Module overview

```text
src/
├── main.rs          CLI entrypoint and subcommands
├── lib.rs           library root and module declarations
├── domain/          provider accounts, pools, routes, policies, errors
├── registry/        SQLite-backed persistence facade
├── db/              SQLite storage implementations and migrations
├── routing/         account selection and fallback routing
├── gateway/         OpenAI-compatible HTTP handlers and upstream proxying
├── admin/           management, health, metrics, usage, and audit API
├── dashboard/       embedded admin UI assets
├── auth/            inbound bearer auth boundary
├── transport/       request IDs, trusted proxy handling, rate limits
├── idempotency/     chat completion idempotency middleware/services
├── services/        account, pool, route, health, usage, audit services
├── tui/             operator terminal interface
└── config/          TOML/env/CLI config assembly and validation
```

## Shared crates

Rook reuses contracts from `clients/agent-runtime/crates/` as path dependencies. It does **not** depend on the `corvus` binary or its internals.

| Crate | Role |
|---|---|
| `corvus-traits` | Shared async provider/tool/memory trait contracts |

## Verification commands

Useful confidence checks:

```bash
cargo test --manifest-path clients/rook/Cargo.toml
cargo clippy --manifest-path clients/rook/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path clients/rook/Cargo.toml --check
```

Repository-wide confidence check:

```bash
make build
```

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
