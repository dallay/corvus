# Marketing (Astro)

Sitio de marketing para Corvus.

## Objetivo

Este proyecto en `clients/web/apps/marketing` centraliza:

- Landing principal
- Páginas de campañas
- Script público de instalación (`/install`)

## Desarrollo

```bash
# Desde clients/web
pnpm dev:marketing

# Build estático
pnpm build:marketing

# Validaciones
pnpm --filter @corvus/marketing run check
```

Dev URL (portless): <http://marketing.localhost:1355>

Sin portless: `PORTLESS=0 pnpm dev:marketing` (usa `http://localhost:9988`).

## Configuración de dominio y puerto

- Dev default: `http://marketing.localhost:1355` (portless)
- Prod default: `https://profiletailors.com`
- El puerto de `dev/preview` se toma de `PORT` (portless) o de `MARKETING_URL` si incluye puerto; si no, usa `9988`

Variable soportada:

```bash
# Forzar URL para cualquier entorno
MARKETING_URL=https://staging.profiletailors.com pnpm build:marketing
```

Puedes cargarla por entorno con archivos `.env`:

```bash
# .env.development
MARKETING_URL=http://localhost:9988

# .env.production
MARKETING_URL=https://profiletailors.com
```

## Instalador

Este proyecto publica el script wizard en:

```bash
curl -fsSL https://profiletailors.com/install | bash
```

Archivo fuente: `scripts/install.sh`

El build copia el script a `public/install` vía `pnpm run copy-install`.
