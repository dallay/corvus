# Corvus Web Monorepo

Monorepo para apps web de Corvus, incluyendo docs, marketing y chat.

## 📁 Estructura

```text
clients/web/
├── apps/
│   ├── docs/           # Documentación (Astro + Starlight)
│   ├── marketing/      # Landing y páginas de marketing (Astro)
│   └── chat/           # Chat conversacional estilo ChatGPT (Vue 3 + Vite)
├── packages/
│   └── shared/         # Utilidades compartidas
├── biome.json          # Configuración única de Biome para todo el monorepo
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

### chat

- Framework: Vue 3 + Vite + Tailwind + shadcn-vue style components
- Puerto por defecto: 4323
- Interfaz de chat conversacional estilo ChatGPT

## 🛠️ Comandos

Requisitos mínimos:

- Node.js 20.19+ (recomendado 22+)
- pnpm 10.30+

```bash
# Instalar dependencias workspace
pnpm install

# Build de todas las apps
pnpm build

# Build individual
pnpm build:docs
pnpm build:marketing
pnpm build:chat

# Compatibilidad (alias antiguo)
pnpm build:landing

# Development
pnpm dev
pnpm dev:marketing
pnpm dev:chat

# Compatibilidad (alias antiguo)
pnpm dev:landing

# Quality
pnpm format
pnpm check
pnpm test
pnpm test:chat
```

## 🧹 Biome (Linter & Formatter)

La configuración de Biome es **única** y vive en `clients/web/biome.json`.
Todas las apps y packages heredan esta configuración automáticamente.
No se necesitan archivos `biome.json` locales en cada app.

## 📦 Añadir más proyectos web

1. Crear `apps/<nombre>/` con su `package.json`
2. Ejecutar `pnpm install` en `clients/web`
3. Ajustar scripts en `clients/web/package.json` si aplica
4. Confirmar que `clients/web/build.gradle.kts` tenga el puerto/config correspondiente
