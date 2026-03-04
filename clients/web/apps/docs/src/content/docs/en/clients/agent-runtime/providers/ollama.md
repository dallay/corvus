---
title: Ollama
---

# Ollama (Local) Provider

Ollama allows you to run open-source models locally.

## Configuration

Set your API key (if required):

```bash
export OLLAMA_API_KEY=...
```

To configure a custom endpoint (e.g., for a remote Ollama instance), set the `api_url` field in your Corvus configuration file. The `OLLAMA_HOST` environment variable is not used for endpoint overrides in the Agent Runtime.

## Supported Models

- llama3
- mistral
- phi3
