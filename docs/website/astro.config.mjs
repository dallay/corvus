import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import { viewTransitions } from 'astro-vtbot/starlight-view-transitions'

export default defineConfig({
  site: 'https://dallay.github.io',
  base: '/starter-gradle',
  integrations: [
    starlight({
      title: 'Starter Gradle',
      defaultLocale: 'en',
      locales: {
        en: { label: 'English', lang: 'en' },
        es: { label: 'Español', lang: 'es' },
      },
      plugins: [viewTransitions()],
      customCss: ['./src/styles/custom.css'],
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/dallay/starter-gradle',
        },
      ],
      head: [
        {
          tag: 'meta',
          attrs: {
            name: 'theme-color',
            content: '#0a0f1e',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:type',
            content: 'website',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:title',
            content: 'Starter Gradle — Modern Gradle Template',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:description',
            content:
              'A centralized, maintainable, and robust Gradle starter template for Kotlin, Java, and Spring Boot projects.',
          },
        },
      ],
      sidebar: [
        {
          label: 'Guides',
          translations: {
            es: 'Guías',
          },
          items: [
            {
              label: 'Getting Started',
              slug: 'guides/getting-started',
              translations: {
                es: 'Primeros Pasos',
              },
            },
            {
              label: 'Project Structure',
              slug: 'guides/structure',
              translations: {
                es: 'Estructura del Proyecto',
              },
            },
            {
              label: 'Features',
              slug: 'guides/features',
              translations: {
                es: 'Funcionalidades',
              },
            },
            {
              label: 'Architecture',
              slug: 'guides/architecture',
              translations: {
                es: 'Arquitectura',
              },
            },
            {
              label: 'Development',
              slug: 'guides/development',
              translations: {
                es: 'Desarrollo',
              },
            },
            {
              label: 'Configuration',
              slug: 'guides/configuration',
              translations: {
                es: 'Configuración',
              },
            },
            {
              label: 'Template Customization',
              slug: 'guides/customization',
              translations: {
                es: 'Personalización de la Plantilla',
              },
            },
            {
              label: 'Release Process',
              slug: 'guides/release',
              translations: {
                es: 'Proceso de Release',
              },
            },
          ],
        },
      ],
    }),
  ],
})
