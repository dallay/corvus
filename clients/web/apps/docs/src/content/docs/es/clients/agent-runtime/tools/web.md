---
title: Herramientas Web
description: Referencia para herramientas de navegación web, búsqueda y peticiones HTTP en Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

# Herramientas Web

Las herramientas web permiten a los agentes recuperar información de Internet e interactuar con APIs externas. Todas las herramientas web imponen una política estricta de **Lista de Dominios Permitidos**.

## `web_search_tool`

Realiza una búsqueda web para encontrar información actual, noticias o temas de investigación.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Proveedores:**
  - `duckduckgo` (Por defecto): Gratuito, no requiere clave de API.
  - `brave`: Requiere `web_search.brave_api_key`.
- **Resultados:** Devuelve títulos, URLs y fragmentos (snippets) clasificados.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `query` | `string` | **Requerido.** La consulta de búsqueda. Sé específico para obtener mejores resultados. |

---

## `browser`

Automatización completa del navegador para interactuar con aplicaciones web complejas. Soporta múltiples backends, incluyendo `agent-browser` basado en Playwright y `computer_use` a nivel de sistema operativo.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Backends:**
  - `agent_browser`: Utiliza la CLI de `agent-browser`.
  - `rust_native`: Driver de Rust integrado (requiere la feature `browser-native`).
  - `computer_use`: Control de ratón/teclado a nivel de SO vía sidecar.
- **Restricciones:** Aplica `browser.allowed_domains`.

### Acciones Comunes

| Acción | Descripción |
| :--- | :--- |
| `open` | Navegar a una URL (solo HTTPS). |
| `snapshot` | Obtener una captura del árbol de accesibilidad con referencias de elementos (`@e1`, `@e2`). |
| `click` | Hacer clic en un elemento por referencia (ej. `@e5`) o selector. |
| `fill` | Escribir texto en un campo de entrada. |
| `screenshot` | Capturar una imagen visual de la página actual. |

---

## `browser_open`

Una alternativa ligera a `browser` que simplemente abre una URL HTTPS aprobada en el navegador Brave del host.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Nota:** Esta herramienta **no** permite al agente extraer datos o ver el contenido de la página; es para abrir páginas en beneficio del usuario.

---

## `http_request`

Realiza peticiones HTTP estructuradas (REST/JSON) a APIs externas.

- **Nivel de Seguridad:** De Acción (Con riesgo).
- **Restricciones:**
  - Solo se permiten esquemas `http://` y `https://`.
  - Los hosts locales/privados (protección SSRF) están estrictamente bloqueados.
  - Los encabezados sensibles (Authorization, API-Key) se redactan en los registros.
  - Las redirecciones están desactivadas por defecto por seguridad.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `url` | `string` | **Requerido.** La URL completa de la petición. |
| `method` | `string` | Método HTTP (GET, POST, PUT, DELETE, etc.). Por defecto: `GET`. |
| `headers` | `object` | Pares clave-valor opcionales para los encabezados. |
| `body` | `string` | Payload opcional para peticiones POST/PUT. |
