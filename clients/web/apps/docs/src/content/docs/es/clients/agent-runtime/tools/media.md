---
title: Herramientas Multimedia
description: Referencia para herramientas de visión e imágenes en Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-05-15
appliesTo: main
docType: reference
---

Las herramientas multimedia proporcionan al agente capacidades visuales, permitiéndole "ver" el entorno del host y procesar archivos de imagen.

## `screenshot`

Captura una imagen de la pantalla actual o de una región específica.

- **Nivel de Seguridad:** De Acción (Con riesgo) / Sensible.
- **Devuelve:** La ruta del archivo PNG guardado y una versión de la imagen codificada en base64 (si el tamaño lo permite).
- **Soporte de Plataformas:**
  - **macOS:** Utiliza `screencapture` nativo.
  - **Linux:** Requiere `gnome-screenshot`, `scrot` o `import` (ImageMagick).

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `filename` | `string` | Nombre de archivo opcional. Guardado en el workspace. |
| `region` | `string` | (Solo macOS) `selection` para recorte interactivo, `window` para la ventana frontal. |

---

## `image_info`

Extrae metadatos de un archivo de imagen y, opcionalmente, lo devuelve como base64 para su procesamiento por modelos multimodales.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Formatos Soportados:** PNG, JPEG, GIF, WEBP, BMP.
- **Metadatos Extraídos:** Formato, dimensiones (ancho/alto) y tamaño del archivo.
- **Restricciones:** Sandboxing de ruta al workspace; tamaño máximo de archivo 5 MB.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `path` | `string` | **Requerido.** Ruta al archivo de imagen. |
| `include_base64` | `boolean` | Incluir los datos completos de la imagen en la salida. Por defecto: `false`. |

---

## `pdf_inspect`

Inspecciona, clasifica y extrae texto de un archivo PDF. Detecta si el PDF está basado en texto, escaneado, basado en imágenes o es mixto.

- **Nivel de Seguridad:** Solo Lectura (Segura).
- **Funcionalidad:** Convierte el texto extraíble a Markdown. Devuelve el tipo de PDF, número de páginas, páginas que necesitan OCR, título y contenido en Markdown.
- **Restricciones:**
  - Tamaño máximo de archivo: 50 MB.
  - Tiempo de espera: 60 segundos.
  - Sandboxing de ruta al workspace.

### Parámetros

| Parámetro | Tipo | Descripción |
| :--- | :--- | :--- |
| `path` | `string` | **Requerido.** Ruta relativa al archivo PDF dentro del workspace. |
| `extract_text` | `boolean` | Indica si se debe extraer y convertir el texto a Markdown (por defecto: `true`). Establécelo en `false` para una clasificación solo de metadatos. |
