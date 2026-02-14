---
title: Procedimientos de Desarrollo
---

Esta guía describe los procedimientos estándar para desarrollar y mantener este proyecto.

## Uso del Makefile

El `Makefile` proporciona una forma estandarizada de interactuar con el proyecto en diferentes sistemas operativos.

### Comandos esenciales

| Comando | Descripción |
|---------|-------------|
| `make setup` | Configuración inicial (permisos, comprobación de herramientas) |
| `make run` | Ejecuta la aplicación principal |
| `make build` | Construcción completa del proyecto, incluidas las pruebas |
| `make test` | Ejecuta todas las pruebas unitarias |
| `make check` | Ejecuta todas las comprobaciones de calidad (formato + lint + pruebas) |
| `make format` | Corrige automáticamente el formateo del código |
| `make clean` | Elimina los artefactos de construcción |

### Comandos de documentación

- `make docs`: Genera la documentación de la API con Dokka.
- `make docs-web-build`: Construye el sitio web de Starlight.
- `make docs-web-dev`: Inicia el servidor de desarrollo para el sitio web.

## Flujo de trabajo

1. **Configuración**: Ejecuta `make setup` una vez después de clonar.
2. **Desarrollo**: Escribe código en `app/` o `examples/`.
3. **Formateo**: Ejecuta `make format` con frecuencia para mantener el código limpio.
4. **Verificación**: Ejecuta `make check` antes de confirmar los cambios para asegurar que todo pase.
5. **Pruebas**: Añade pruebas en `src/test` y ejecútalas con `make test`.

## Añadir dependencias

Para añadir una nueva dependencia:
1. Defínela en `gradle/libs.versions.toml`.
2. Referénciala en el `build.gradle.kts` del módulo deseado usando `libs.<nombre>`.
3. Ejecuta `make build` para verificar.
