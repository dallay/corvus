import { defineConfig } from "astro/config";
import { loadEnv } from "vite";
import { PORTS, getPortFromUrl, resolveSiteUrl } from "@corvus/shared/env";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.MARKETING}`;
const DEFAULT_PROD_URL = "https://profiletailors.com";

export default defineConfig(({ command, mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const isProdLike = mode === "production" || command === "build";
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
    isProdLike,
  });
  const resolvedPort = getPortFromUrl(marketingUrl, PORTS.MARKETING);

  return {
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
  };
});
