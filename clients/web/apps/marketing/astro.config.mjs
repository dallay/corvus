import { resolvePublicUrl } from "@corvus/shared";
import { getPortFromUrl, PORTS } from "@corvus/shared/env";
import { defineConfig, envField } from "astro/config";
import { loadEnv } from "vite";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.MARKETING}`;
const DEFAULT_PROD_URL = "https://profiletailors.com";

const mode = process.env.NODE_ENV || "production";
const env = loadEnv(mode, process.cwd(), "");

const marketingUrl = resolvePublicUrl(
  env.MARKETING_URL,
  mode === "production" ? DEFAULT_PROD_URL : DEFAULT_DEV_URL
);
const resolvedPort = getPortFromUrl(marketingUrl, PORTS.MARKETING);

export default defineConfig({
  site: marketingUrl,
  output: "static",
  compressHTML: true,
  server: {
    host: true,
    port: resolvedPort,
  },
  preview: {
    host: true,
    port: resolvedPort,
  },
  env: {
    schema: {
      AHREFS_KEY: envField.string({
        context: "client",
        access: "public",
        optional: true,
      }),
    },
  },
});
