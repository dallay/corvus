import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import { viewTransitions } from "astro-vtbot/starlight-view-transitions";
import { loadEnv } from "vite";
import { getPortFromUrl, PORTS, resolveSiteUrl } from "../../packages/shared/env.mjs";

const DEFAULT_DEV_URL = `http://localhost:${PORTS.DOCS}`;
const DEFAULT_PROD_URL = "https://docs.profiletailors.com";

const mode = process.env.NODE_ENV || "production";
const env = loadEnv(mode, process.cwd(), "");
const portlessUrl =
  mode === "production" ? undefined : (process.env.PORTLESS_URL ?? env.PORTLESS_URL);
const runtimeEnv = portlessUrl ? { ...env, DOCS_URL: portlessUrl } : env;
const docsUrl = resolveSiteUrl({
  env: runtimeEnv,
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
const portCandidate = process.env.PORT ?? env.PORT;
const parsedPort = Number.parseInt(portCandidate ?? "", 10);
const resolvedPort = Number.isFinite(parsedPort) ? parsedPort : getPortFromUrl(docsUrl, PORTS.DOCS);

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
      disable404Route: true,
      title: "Corvus",
      logo: {
        light: "./public/favicon.svg",
        dark: "./public/favicon-light.svg",
      },
      defaultLocale: "root",
      locales: {
        root: { label: "English", lang: "en" },
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
            content: "#000000",
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
          label: "Guides",
          translations: {
            es: "Guías",
          },
          items: [
            {
              label: "Getting Started",
              slug: "guides/getting-started",
              translations: {
                es: "Primeros Pasos",
              },
            },
            {
              label: "Project Structure",
              slug: "guides/structure",
              translations: {
                es: "Estructura del Proyecto",
              },
            },
            {
              label: "Features",
              slug: "guides/features",
              translations: {
                es: "Funcionalidades",
              },
            },
            {
              label: "Development",
              slug: "guides/development",
              translations: {
                es: "Desarrollo",
              },
            },
            {
              label: "Configuration",
              slug: "guides/configuration",
              translations: {
                es: "Configuración",
              },
            },
            {
              label: "Template Customization",
              slug: "guides/customization",
              translations: {
                es: "Personalización de la Plantilla",
              },
            },
            {
              label: "Release Process",
              slug: "guides/release",
              translations: {
                es: "Proceso de Release",
              },
            },
            {
              label: "SurrealDB Operations",
              slug: "guides/surrealdb",
              translations: {
                es: "Operación de SurrealDB",
              },
            },
            {
              label: "CLI Reference",
              slug: "guides/cli-reference",
              translations: {
                es: "Referencia de la CLI",
              },
            },
            {
              label: "Runtime Sandbox Isolation",
              slug: "guides/runtime-sandbox-isolation",
              translations: {
                es: "Aislamiento del Sandbox del Runtime",
              },
            },
            {
              label: "Model Routing & Query Classification",
              slug: "guides/model-routing",
              translations: {
                es: "Enrutamiento de Modelos y Clasificación de Consultas",
              },
            },
          ],
        },
        {
          label: "Architecture",
          translations: {
            es: "Arquitectura",
          },
          items: [
            {
              label: "Architecture Overview",
              slug: "guides/architecture",
              translations: {
                es: "Visión General",
              },
            },
            {
              label: "Architecture Diagrams",
              slug: "guides/architecture/overview",
              translations: {
                es: "Diagramas de Arquitectura",
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
              label: "AI Providers",
              slug: "clients/agent-runtime/providers",
              translations: {
                es: "Providers de IA",
              },
            },
            {
              label: "Architecture",
              slug: "clients/agent-runtime/architecture",
              translations: {
                es: "Arquitectura",
              },
            },
          ],
        },
        {
          label: "Cerebro",
          translations: {
            es: "Cerebro",
          },
          items: [
            {
              label: "Overview",
              slug: "cerebro",
              translations: {
                es: "Descripción General",
              },
            },
            {
              label: "Configuration",
              slug: "cerebro/configuration",
              translations: {
                es: "Configuración",
              },
            },
            {
              label: "Running",
              slug: "cerebro/running",
              translations: {
                es: "Ejecución",
              },
            },
            {
              label: "CLI Reference",
              slug: "cerebro/cli-reference",
              translations: {
                es: "Referencia CLI",
              },
            },
            {
              label: "MCP Tools Reference",
              slug: "cerebro/mcp-tools",
              translations: {
                es: "Referencia de Herramientas MCP",
              },
            },
            {
              label: "Integration",
              slug: "cerebro/integration",
              translations: {
                es: "Integración",
              },
            },
            {
              label: "Migration",
              slug: "cerebro/migration",
              translations: {
                es: "Migración",
              },
            },
            {
              label: "Operations",
              slug: "cerebro/operations",
              translations: {
                es: "Operaciones",
              },
            },
          ],
        },
      ],
    }),
  ],
});
