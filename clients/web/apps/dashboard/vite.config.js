import path from "node:path";
import { fileURLToPath, URL } from "node:url";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";
const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
export default defineConfig({
    plugins: [vue()],
    server: {
        fs: {
            // Narrow to only paths used by dashboard contract tests
            allow: [
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
});
