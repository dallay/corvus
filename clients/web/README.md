# Corvus Web Monorepo

Estructura multi-app para proyectos web de Corvus.

## 📁 Estructura

```
apps/web/
├── apps/
│   ├── docs/           # Documentación Starlight (actual)
│   ├── landing/        # Landing page (futuro)
│   └── dashboard/      # Frontend web (futuro)
├── packages/
│   └── shared/         # Componentes/utilidades compartidas
├── package.json        # Workspace root
└── pnpm-workspace.yaml # Configuración pnpm workspace
```

## 🚀 Apps

### docs
- **Framework**: Astro + Starlight
- **Puerto**: 4321
- **Uso**: Documentación del proyecto

### landing (futuro)
- **Framework**: Astro/Vue/React (por definir)
- **Puerto**: 4322
- **Uso**: Landing page marketing

### dashboard (futuro)
- **Framework**: Vue/React (por definir)
- **Puerto**: 4323
- **Uso**: Panel de administración

## 🛠️ Comandos

```bash
# Instalar dependencias en todas las apps
pnpm install

# Build de todas las apps
pnpm build

# Build individual
pnpm build:docs
pnpm build:landing
pnpm build:dashboard

# Development
pnpm dev          # docs por defecto
pnpm dev:landing
pnpm dev:dashboard

# Lint/Format
pnpm format
pnpm check
```

## 📦 Packages Compartidos

Los paquetes en `packages/` pueden ser importados por cualquier app:

```typescript
import { Button } from '@corvus/shared';
```

## 🏗️ Agregar una nueva app

1. Crear directorio en `apps/<nombre>/`
2. Agregar `package.json` con nombre `@corvus/<nombre>`
3. Ejecutar `pnpm install` desde root
4. Agregar scripts en `package.json` root si es necesario
5. Actualizar `build.gradle.kts` para incluir la nueva app

## 📝 Notas

- Cada app tiene su propio `node_modules` (a través de pnpm)
- Las dependencias compartidas se instalan en root y se symlinkan
- El build de Gradle construye todas las apps automáticamente
