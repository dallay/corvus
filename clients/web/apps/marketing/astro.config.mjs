import partytown from "@astrojs/partytown";
import { getPortFromUrl, PORTS, resolveSiteUrl } from "@corvus/shared/env";
import { defineConfig, envField } from "astro/config";
import { loadEnv } from "vite";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.MARKETING}`;
const DEFAULT_PROD_URL = "https://profiletailors.com";

const mode = process.env.NODE_ENV || "production";
const env = loadEnv(mode, process.cwd(), "");
const marketingUrl = resolveSiteUrl({
  env,
  primaryKey: "MARKETING_URL",
  localDefault: DEFAULT_DEV_URL,
  productionDefault: DEFAULT_PROD_URL,
  genericKeys: ["SITE_URL"],
  providerKeys: {
    cloudflare: "CF_PAGES_URL",
    vercel: "VERCEL_URL",
    netlify: "URL",
  },
  isProdLike: mode === "production",
});
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
  integrations: [partytown()],
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
