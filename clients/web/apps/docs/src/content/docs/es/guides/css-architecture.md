---
title: Arquitectura CSS
description: Capas CSS compartidas, reglas de ownership y convenciones de import para el workspace web de Corvus.
owner: team-platform
status: canonical
lastReviewed: 2026-03-28
appliesTo: main
docType: guide
---

Esta guía define cómo se organiza el CSS en el monorepo web de Corvus para que las preocupaciones
compartidas vivan en un solo sitio y cada aplicación conserve solo los estilos específicos de su
producto.

## Capas

### `@corvus/shared/theme.css`

Contiene design tokens, aliases semánticos y propiedades personalizadas de marca.

Usa esta capa para:

- Colores y gradientes de marca
- Tokens tipográficos
- Tokens de radios, sombras y espaciado
- Aliases semánticos compartidos entre varias apps

### `@corvus/shared/base.css`

Contiene la base compartida: resets, comportamiento elemental y defaults de baja especificidad.

Usa esta capa para:

- `box-sizing`
- comportamiento por defecto de enlaces e imágenes
- primitivas tipográficas base
- manejo de `prefers-reduced-motion`

No pongas aquí layout específico de una app ni selectores de componentes de producto.

### `@corvus/shared/app-shell.css`

Contiene el shell compartido para las apps Vue, incluyendo el canvas oscuro y el tratamiento común
del scrollbar.

Usa esta capa para:

- comportamiento compartido de `html`, `body` y `#app` en apps Vue
- fondo y superficie base de aplicaciones
- chrome de shell idéntico entre experiencias tipo app

### CSS de cada app

Cada app conserva únicamente los estilos que pertenecen a su propio layout, contenido y
comportamiento visual.

Ejemplos:

- secciones de landing en Marketing
- overrides de tema Starlight en Docs
- layout de pantallas y rutas en Chat o Dashboard

## Reglas de ownership

- Pon tokens, aliases y variables de tema en `packages/shared/theme.css`.
- Pon resets y reglas elementales base en `packages/shared/base.css`.
- Pon preocupaciones compartidas del frame de apps Vue en `packages/shared/app-shell.css`.
- Mantén la presentación de Astro/Starlight o landing pages dentro de la app propietaria.
- Mantén estilos locales dentro del componente cuando sean realmente específicos del componente.

## Reglas de import

- Las apps Vue deben importar `tailwindcss` más `@corvus/shared/app-shell.css` en su stylesheet principal.
- Marketing y Docs deben importar `@corvus/shared/base.css` y `@corvus/shared/theme.css`, y después superponer su CSS específico.
- Evita importar el stylesheet de una app dentro de otra app.

## Guía de especificidad

- Prefiere custom properties frente a copiar valores crudos entre apps.
- Prefiere selectores de baja especificidad en las capas compartidas.
- Mantén selectores interactivos o de estado cerca del componente o la app que los posee.
- Evita poner selectores de layout específicos de producto dentro del CSS compartido.

## Checklist de decisión

Cuando añadas una regla nueva, pregúntate:

1. ¿La aprovecharían al menos dos apps sin cambios?
   Si sí, probablemente pertenece a `packages/shared`.
2. ¿Expresa un token de marca o un alias semántico?
   Si sí, pertenece a `theme.css`.
3. ¿Da estilo a una pantalla, sección o ruta concreta?
   Si sí, pertenece a la app propietaria.

## Nota de compatibilidad

`@corvus/shared/tokens.css` sigue disponible como alias de compatibilidad hacia `theme.css`
durante el rename, pero los imports nuevos deben preferir `@corvus/shared/theme.css`.
