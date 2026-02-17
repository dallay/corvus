# Corvus Web Monorepo

Monorepo para apps web de Corvus, incluyendo docs, marketing y futuros frontends.

## 📁 Estructura

```text
clients/web/
├── apps/
│   ├── docs/           # Documentación (Astro + Starlight)
│   ├── marketing/      # Landing y páginas de marketing (Astro)
│   └── dashboard/      # Dashboard web (Vue 3 + Vite)
├── packages/
│   └── shared/         # Utilidades compartidas
├── package.json
└── pnpm-workspace.yaml
```

## 🚀 Apps

### docs

- Framework: Astro + Starlight
- Puerto por defecto: 4321

### marketing

- Framework: Astro
- URL configurable con `MARKETING_URL` (dev default: `http://localhost:9988`)
- Incluye script público de instalación en `/install`

### dashboard

- Framework: Vue 3 + Vite + Tailwind + shadcn-vue style components
- Puerto por defecto: 4323

## 🛠️ Comandos

```bash
# Instalar dependencias workspace
pnpm install

# Build de todas las apps
pnpm build

# Build individual
pnpm build:docs
pnpm build:marketing
pnpm build:dashboard

# Compatibilidad (alias antiguo)
pnpm build:landing

# Development
pnpm dev
pnpm dev:marketing
pnpm dev:dashboard

# Compatibilidad (alias antiguo)
pnpm dev:landing

# Quality
pnpm format
pnpm check
```

## 📦 Añadir más proyectos web

1. Crear `apps/<nombre>/` con su `package.json`
2. Ejecutar `pnpm install` en `clients/web`
3. Ajustar scripts en `clients/web/package.json` si aplica
4. Confirmar que `clients/web/build.gradle.kts` tenga el puerto/config correspondiente
