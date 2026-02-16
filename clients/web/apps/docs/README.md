[Astro](https://docs.astro.build/) + [Starlight](https://starlight.astro.build/) docs site.

```bash
npm config get registry
npm config set registry https://registry.npmjs.org/

# Install pnpm
npm i -g pnpm
pnpm config set registry https://registry.npmjs.org/

# Install dependencies
pnpm install

# Start dev server
pnpm run dev

# Build static site
pnpm run build

# Lint/format with Biome
pnpm run check
pnpm run lint
pnpm run format
```
