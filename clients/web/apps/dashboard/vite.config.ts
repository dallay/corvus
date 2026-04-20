import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));

export default defineConfig({
  plugins: [vue()],
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ["vue", "vue-i18n"],
          ui: ["@corvus/ui", "@corvus/locales"],
        },
      },
    },
  },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.spec.ts"],
    exclude: ["e2e/**"],
    server: {
      fs: {
        allow: [repoRoot],
      },
    },
  },
});
