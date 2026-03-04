---
title: Opciones de Configuración
---

El proyecto es altamente configurable a través de propiedades de Gradle y catálogos de versiones.

## Catálogo de versiones (`libs.versions.toml`)

Este archivo contiene las versiones de todas las herramientas y dependencias utilizadas en el
proyecto.

### Versiones clave

- **JDK**: Versión de Java de destino (por defecto 21).
- **Gradle**: Versión del sistema de construcción.
- **Kotlin**: Versión del compilador y la biblioteca estándar de Kotlin.
- **Node**: Requerido para la construcción de la documentación y otras herramientas JS.

### Gestión de dependencias

Las dependencias se agrupan en:

- `versions`: Fuente única de verdad para los números de versión.
- `libraries`: Definiciones de dependencias individuales.
- `bundles`: Grupos de dependencias que a menudo se usan juntas.
- `plugins`: Plugins de Gradle utilizados en el proyecto.

## Propiedades de Gradle

La configuración global de la construcción se encuentra en `gradle.properties`. Esto incluye la
configuración del demonio de Gradle, la ejecución en paralelo y el almacenamiento en caché.

## Variables de entorno

Algunas funcionalidades pueden requerir variables de entorno, especialmente para CI/CD o tareas
especializadas (por ejemplo, llaves GPG para la firma, credenciales de repositorio para la
publicación).

## Configuración MCP del Agent Runtime

El `agent-runtime` soporta servidores Model Context Protocol (MCP) detrás de un control de despliegue explícito.

```toml
[mcp]
enabled = false

[[mcp.servers]]
name = "docs"
enabled = true
command = "mcp-docs"
args = ["serve"]
startup_timeout_ms = 5000
call_timeout_ms = 30000
output_limit_bytes = 65536
```

- `mcp.enabled = false` es el valor seguro por defecto y desactiva descubrimiento/ejecución MCP.
- Las herramientas MCP usan namespace `mcp.<server>.<tool>`.
- Las llamadas MCP son deny-by-default en flujos supervisados y devuelven payload estructurado
  `approval_required` hasta recibir aprobación explícita.
- Si un servidor MCP falla al iniciar, los servidores sanos siguen registrándose; los errores se
  registran con diagnósticos redactados.

### Rollback

Para revertir MCP de inmediato, establece `mcp.enabled = false` y reinicia el proceso runtime.
