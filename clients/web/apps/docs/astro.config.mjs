import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import { viewTransitions } from "astro-vtbot/starlight-view-transitions";

export default defineConfig({
  site: "https://dallay.github.io",
  base: "/corvus",
  integrations: [
    starlight({
      title: "Corvus",
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
              label: "CLI Reference",
              slug: "guides/cli-reference",
              translations: {
                es: "Referencia de la CLI",
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
          ],
        },
      ],
    }),
  ],
});
