import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import { viewTransitions } from "astro-vtbot/starlight-view-transitions";
import { loadEnv } from "vite";
import { getPortFromUrl, PORTS, resolveSiteUrl } from "../../packages/shared/env.mjs";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.DOCS}`;
const DEFAULT_PROD_URL = "https://docs.profiletailors.com";

const mode = process.env.NODE_ENV || "production";
const env = loadEnv(mode, process.cwd(), "");
const docsUrl = resolveSiteUrl({
  env,
  primaryKey: "DOCS_URL",
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
const resolvedPort = getPortFromUrl(docsUrl, PORTS.DOCS);

export default defineConfig({
  site: docsUrl,
  base: "/",
  server: {
    host: true,
    port: resolvedPort,
  },
  preview: {
    host: true,
    port: resolvedPort,
  },
  integrations: [
    starlight({
      title: "Corvus",
      logo: {
        light: "./public/favicon.svg",
        dark: "./public/favicon-light.svg",
      },
      defaultLocale: "en",
      locales: {
        en: { label: "English", lang: "en" },
        es: { label: "Español", lang: "es" },
      },
      plugins: [viewTransitions()],
      customCss: ["./src/styles/custom.css"],
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/dallay/corvus",
        },
      ],
      head: [
        {
          tag: "meta",
          attrs: {
            name: "theme-color",
            content: "#0a0f1e",
          },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:type",
            content: "website",
          },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:title",
            content: "Corvus — Reactive Agent Platform",
          },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:description",
            content:
              "Reactive agent platform on Kotlin, Spring Boot, Neo4j, Rust sidecars, and Astro/Vue.",
          },
        },
      ],
      sidebar: [
        {
          label: "Getting Started",
          translations: {
            es: "Empezando",
          },
          items: [
            {
              label: "Introduction",
              slug: "intro/introduction",
              translations: {
                es: "Introducción",
              },
            },
          ],
        },
        {
          label: "Agent Runtime",
          translations: {
            es: "Agent Runtime",
          },
          items: [
            {
              label: "Overview",
              slug: "clients/agent-runtime",
              translations: {
                es: "Visión General",
              },
            },
            {
              label: "Architecture",
              slug: "clients/agent-runtime/architecture",
              translations: {
                es: "Arquitectura",
              },
            },
            {
              label: "Providers",
              slug: "clients/agent-runtime/providers",
              translations: {
                es: "Proveedores",
              },
            },
            {
              label: "PR Workflow",
              slug: "clients/agent-runtime/pr-workflow",
              translations: {
                es: "Flujo de trabajo de PR",
              },
            },
            {
              label: "CI Map",
              slug: "clients/agent-runtime/ci-map",
              translations: {
                es: "Mapa de CI",
              },
            },
          ],
        },
      ],
    }),
  ],
});
