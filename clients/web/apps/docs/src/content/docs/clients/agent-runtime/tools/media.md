---
title: Media Tools
description: Reference for vision and image-related tools in Corvus.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

Media tools provide the agent with visual capabilities, allowing it to "see" the host environment and process image files.

## `screenshot`

Captures a screenshot of the current screen or a specific region.

- **Security Tier:** Action-Bearing (Risk-bearing) / Sensitive.
- **Returns:** The file path of the saved PNG and a base64-encoded version of the image (if size permits).
- **Platform Support:**
  - **macOS:** Uses native `screencapture`.
  - **Linux:** Requires `gnome-screenshot`, `scrot`, or `import` (ImageMagick).

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `filename` | `string` | Optional filename. Saved in the workspace. |
| `region` | `string` | (macOS only) `selection` for interactive crop, `window` for front window. |

---

## `image_info`

Extracts metadata from an image file and optionally returns it as base64 for processing by multimodal models.

- **Security Tier:** Read-Only (Safe).
- **Supported Formats:** PNG, JPEG, GIF, WEBP, BMP.
- **Metadata Extracted:** Format, dimensions (width/height), and file size.
- **Constraints:** Path-sandboxed to the workspace; maximum file size 5 MB.

### Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `path` | `string` | **Required.** Path to the image file. |
| `include_base64` | `boolean` | Include the full image data in the output. Default: `false`. |
