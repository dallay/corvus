/// <reference types="vitest" />
import { fileURLToPath, URL } from "node:url";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";
const config = {
    plugins: [vue()],
    resolve: {
        alias: {
            "@": fileURLToPath(new URL("./src", import.meta.url)),
        },
    },
    test: {
        environment: "happy-dom",
        coverage: {
            provider: "v8",
            reporter: ["text", "json", "html", "lcov"],
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
};
export default defineConfig(config);
