---
title: Google Gemini
---

# Proveedor Google Gemini

Gemini es el modelo de IA multimodal de Google.

## Configuración

El proveedor Gemini admite varios métodos de autenticación:

1. **Clave API Directa**:
   ```bash
   export GEMINI_API_KEY=...
   # O
   export GOOGLE_API_KEY=...
   ```

2. **Google Cloud ADC**: Utiliza `GOOGLE_APPLICATION_CREDENTIALS`.
3. **Gemini CLI**: Reutiliza la autenticación existente en `~/.gemini/`.

## Modelos Compatibles

- gemini-1.5-pro
- gemini-1.5-flash
