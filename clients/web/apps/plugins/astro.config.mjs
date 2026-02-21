import { getPortFromUrl, PORTS, resolveSiteUrl } from "@corvus/shared/env";
import { defineConfig } from "astro/config";
import { loadEnv } from "vite";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.PLUGINS}`;
const DEFAULT_PROD_URL = "https://corvus.profiletailors.com";

const mode = process.env.NODE_ENV || "production";
const env = loadEnv(mode, process.cwd(), "");
const pluginsUrl = resolveSiteUrl({
  env,
  primaryKey: "PLUGINS_URL",
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
const resolvedPort = getPortFromUrl(pluginsUrl, PORTS.PLUGINS);

export default defineConfig({
  site: pluginsUrl,
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
});
