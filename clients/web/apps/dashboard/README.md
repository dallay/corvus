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

Dashboard corre en <http://dashboard.localhost:1355> vía portless.

Usa `PORTLESS=0 pnpm dev:dashboard` para correr en <http://localhost:4324>.

## Docker (local-first)

```bash
# Desde la raíz del repo
make dev-up
./dev/cli.sh up-dashboard
```

Luego abre <http://corvus.localhost>, deja `Base URL` en `/api` y completa el pairing para obtener
bearer token. El dashboard y el gateway se comunican a través de Caddy con mismo origen, igual que
en un despliegue productivo detrás de reverse proxy.
