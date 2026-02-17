# Marketing (Astro)

Sitio de marketing y funnels de venta para Corvus.

## Objetivo

Este proyecto en `clients/web/apps/marketing` centraliza:

- Landing principal
- Funnels de adquisición y activación
- Experimentos de campañas
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

## Instalador

Este proyecto publica el script wizard en:

```bash
curl -fsSL https://profiletailors.com/install | bash
```

Archivo fuente: `public/install`
