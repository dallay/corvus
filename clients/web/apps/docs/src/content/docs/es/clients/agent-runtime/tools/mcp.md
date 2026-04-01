---
title: Model Context Protocol (MCP)
description: Guía para integrar y usar herramientas MCP en el Corvus Agent Runtime.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: reference
---

# Model Context Protocol (MCP)

El Model Context Protocol (MCP) es un estándar abierto que permite a los agentes conectarse con herramientas externas, fuentes de datos y servicios. Corvus proporciona un runtime de MCP de primer nivel que integra estas capacidades externas directamente en el cinturón de herramientas del agente.

## Configuración

Los servidores MCP se configuran en el `config.toml` bajo la sección `[mcp]`. Cada servidor requiere un nombre único y un comando para iniciarlo.

```toml
[mcp]
enabled = true

[[mcp.servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_PERSONAL_ACCESS_TOKEN = "tu_token_aqui" }

[[mcp.servers]]
name = "postgres"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
```

## Namespacing

Para evitar colisiones de nombres con las herramientas integradas, todas las herramientas MCP reciben automáticamente un prefijo con el nombre del servidor:

`mcp.<nombre_servidor>.<nombre_herramienta>`

*Ejemplo:* Si el servidor `github` proporciona una herramienta `create_issue`, el agente la verá como `mcp.github.create_issue`.

## Seguridad y Aprobación

Las herramientas MCP se tratan como **De Acción (Con riesgo)** por defecto.

- **Aprobación:** En modo Supervisado, cualquier llamada a una herramienta MCP activará una solicitud de aprobación.
- **Tiempos de espera:** Cada llamada MCP tiene un tiempo de espera por defecto (30s) para evitar que el bucle del agente se bloquee.
- **Límites de salida:** Las respuestas están limitadas (por defecto 64 KB) para proteger el espacio de la ventana de contexto.

## Descubrimiento

Corvus descubre las herramientas MCP al iniciar. Si un servidor no logra iniciarse, el runtime registrará el error pero continuará operando con otros servidores sanos. Puedes verificar las herramientas descubiertas usando:

```bash
corvus doctor
```

## Tipos de Capacidades Soportadas

La implementación de Corvus MCP soporta actualmente:
- **Herramientas (Tools):** Funciones ejecutables (ej. consultar base de datos, enviar email).
- **Recursos (Resources):** (Planificado) Fuentes de datos de solo lectura.
- **Prompts:** (Planificado) Plantillas de prompts predefinidas.
