# Dashboard

Panel de administración para configurar Corvus Gateway sin tocar manualmente `config.toml`.

## Objetivo

- Dashboard web opcional (no embebido en el binario Rust)
- Seguridad por defecto: requiere pairing + bearer token para endpoints admin
- Edición de configuración via API `GET/PUT /web/admin/config`

## Desarrollo

```bash
# Desde clients/web
pnpm install
pnpm dev:dashboard
```

Dashboard corre en <http://localhost:4324>.

## Docker (local-first)

```bash
# Desde clients/agent-runtime
docker compose --profile dashboard up -d
```

Luego abre <http://localhost:4324>, conecta al gateway en <http://127.0.0.1:3000> y completa el
pairing en `/pair` para obtener bearer token.
