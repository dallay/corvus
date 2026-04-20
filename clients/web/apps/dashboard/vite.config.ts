import path from "node:path";
import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
const isTestMode = process.env.NODE_ENV === "test";

export default defineConfig({
  plugins: [vue()],
  server: {
    fs: {
      // Only allow full repo access in test mode; otherwise use minimal paths
      allow: isTestMode
        ? [repoRoot]
        : [
            path.join(repoRoot, "openspec"),
            path.join(repoRoot, "tmp"),
            path.join(repoRoot, "clients/composeApp"),
          ],
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
        // Tests need broader access; use repoRoot for test mode
        allow: isTestMode
          ? [repoRoot]
          : [
              path.join(repoRoot, "openspec"),
              path.join(repoRoot, "tmp"),
              path.join(repoRoot, "clients/composeApp"),
            ],
      },
    },
  },
});
