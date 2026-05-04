import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

const embeddedAssetsRoot = fileURLToPath(new URL("../../../rook/assets", import.meta.url));

export default defineConfig({
  plugins: [vue()],
  server: {
    headers: {
      "X-Content-Type-Options": "nosniff",
      "X-Frame-Options": "DENY",
      "Referrer-Policy": "strict-origin-when-cross-origin",
      "Permissions-Policy": "geolocation=(), microphone=(), camera=()",
    },
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    outDir: embeddedAssetsRoot,
    emptyOutDir: false,
  },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.spec.ts"],
    exclude: ["e2e/**"],
  },
});
