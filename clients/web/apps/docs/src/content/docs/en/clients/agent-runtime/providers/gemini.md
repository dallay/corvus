---
title: Google Gemini
---

# Google Gemini Provider

Gemini is Google's multimodal AI model.

## Configuration

The Gemini provider supports multiple authentication methods:

1. **Direct API Key**:
   ```bash
   export GEMINI_API_KEY=...
   # OR
   export GOOGLE_API_KEY=...
   ```

2. **Google Cloud ADC**: Uses `GOOGLE_APPLICATION_CREDENTIALS`.
3. **Gemini CLI**: Reuses existing `~/.gemini/` authentication.

## Supported Models

- gemini-1.5-pro
- gemini-1.5-flash
