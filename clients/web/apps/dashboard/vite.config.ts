import path from "node:path";
import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

import { isPlaywrightFsAllowMode } from "./src/utils/playwrightEnv";

const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
const isTestMode = isPlaywrightFsAllowMode();

export default defineConfig({
  plugins: [vue()],
  server: {
    headers: {
      "X-Content-Type-Options": "nosniff",
      "X-Frame-Options": "DENY",
      "Referrer-Policy": "strict-origin-when-cross-origin",
      "Permissions-Policy": "geolocation=(), microphone=(), camera=()",
    },
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
});
