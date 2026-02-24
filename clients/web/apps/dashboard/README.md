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
