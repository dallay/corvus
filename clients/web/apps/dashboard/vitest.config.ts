import path from "node:path";
import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

import { isPlaywrightFsAllowMode } from "./src/utils/playwrightEnv";

const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
const isTestMode = isPlaywrightFsAllowMode();

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.spec.ts"],
    exclude: ["e2e/**"],
    server: {
      fs: {
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
