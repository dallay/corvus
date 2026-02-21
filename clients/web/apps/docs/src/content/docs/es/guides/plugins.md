---
title: Plugins del Runtime
---

Esta guía es el playbook del equipo para agregar un nuevo plugin oficial del runtime en Corvus.

Cubre:

- Contrato del plugin y estructura del repositorio
- Flujo de build y publicación seguro
- Integración de catálogo/revocación
- Integración con wizard y runtime
- Checklist de validación y rollout

## 1. Modelo de Plugins (Estado Actual)

Los plugins de Corvus se distribuyen como artefactos WASM firmados más metadatos.

- Artefacto: `<plugin-id>.wasm`
- Metadatos: `plugin-manifest.json`, `catalog.json`, `revocations.json`
- Distribución: OCI (opcional en el workflow), más artefactos subidos como bundle
- Política del runtime: publishers en allowlist, verificación de digest, checks de revocación,
  pinning en lockfile

Código clave del runtime:

- `src/plugins/mod.rs`
- `src/config/schema.rs`

## 2. Contrato y Estructura

### 2.1 Contrato WIT

Usa el contrato WIT compartido:

- `plugins/wit/corvus-plugin.wit`

Este define interfaces exportadas para memoria, salud y capacidades del plugin.

### 2.2 Estructura del Código Fuente del Plugin

Crea un nuevo directorio bajo:

- `plugins/<tu-carpeta-plugin>/`

Archivos mínimos esperados:

- `Cargo.toml` con `crate-type = ["cdylib"]`
- `src/lib.rs`

Plugin de referencia:

- `plugins/memory-surreal-graphs/`

## 3. Requisitos de Seguridad (Innegociables)

Todos los plugins oficiales nuevos deben satisfacer:

1. Fuentes de catálogo/revocación por HTTPS en la configuración.
2. Allowlist de publishers confiables (`corvus-official` por defecto).
3. Pinning de digest en lockfile después de la instalación.
4. Soporte de revocación y checks de revocación enforceados.
5. El runtime no debe fallar completamente al inicio por problemas de instalación/carga de plugins;
   el fallback core debe permanecer disponible cuando esté diseñado.

Configuración por defecto relevante:

- Catálogo: `https://corvus.profiletailors.com/catalog.json`
- Revocaciones: `https://corvus.profiletailors.com/revocations.json`

Ver:

- `src/config/schema.rs`

## 4. Agregar un Nuevo Plugin (Pasos de Implementación)

### 4.1 Crear el crate del plugin

1. Agregar `plugins/<nuevo-plugin>/Cargo.toml`.
2. Configurar:
  - `edition = "2021"`
  - `crate-type = ["cdylib"]`
3. Agregar entrypoint(s) exportados mínimos alineados con el uso de tu contrato WIT.

### 4.2 Build local

Desde la raíz del repo:

```bash
cargo build \
  --manifest-path clients/agent-runtime/plugins/<nuevo-plugin>/Cargo.toml \
  --target wasm32-wasip1 \
  --release
```

### 4.3 Registro/Integración en el Runtime

Si el plugin es instalable vía flujo dedicado (como Surreal Graphs), agregar/actualizar:

- Constante del plugin y helpers de instalación en `src/plugins/mod.rs`
- Cualquier hook de onboarding/wizard en `src/onboard/wizard.rs`

Si el onboarding debe exponerlo:

1. Agregar texto de opción en la selección de memoria/backend del wizard.
2. Asegurar que el check de instalación del plugin corra antes de recolectar opciones
   backend-específicas que dependan de él.
3. Asegurar que el path de falla sea explícito y seguro.

## 5. Workflow de Publicación

Archivo de workflow:

- `.github/workflows/publish-plugins.yml`

Comportamiento actual del workflow:

1. Build del artefacto WASM del plugin.
2. Ensamblar metadatos del bundle:
  - `plugin-manifest.json`
  - `catalog.json`
  - `revocations.json`
3. Opcionalmente firmar con clave cosign (si el secret está presente).
4. Opcionalmente push a OCI (si el input `oci_repository` es proporcionado).
5. Upload de artefactos del bundle.

:::important
Este workflow está actualmente configurado para `memory.surreal.graphs` en los defaults de env del
job. Para un nuevo plugin, ya sea:

1. Adaptar los valores de env para el nuevo plugin, o
2. Generalizar los inputs/matrix del workflow para que plugin ID/folder/nombre de artefacto sean
   parámetros.
   :::

## 6. Comandos de Operador (Runtime)

Comandos de ciclo de vida de plugins:

```bash
corvus plugins list
corvus plugins install <plugin-id> [--version <semver>] [--source <nombre-fuente>]
corvus plugins verify [--id <plugin-id>]
corvus plugins pin <plugin-id> [--version <semver>]
corvus plugins remove <plugin-id>
corvus plugins revocations sync
```

## 7. Checklist de Validación para Nuevos Plugins

Antes de merge:

- [ ] Build del plugin para `wasm32-wasip1`.
- [ ] Validar campos de manifest/catalog/revocations e integridad de digest.
- [ ] Verificar que comandos install + verify funcionen con catálogo local o de test.
- [ ] Confirmar comportamiento de revocación:
  - plugin revocado es bloqueado como se espera
- [ ] Confirmar comportamiento de onboarding si está integrado al wizard:
  - path que requiere plugin es explícito
  - path de falla es legible para el usuario y seguro
- [ ] Confirmar reproducibilidad del lockfile:
  - `~/.corvus/plugins.lock` contiene ID/versión/digest/fuente esperados

## 8. Estrategia de Rollout

Rollout recomendado para producción:

1. Publicar artefacto y metadatos del plugin.
2. Habilitar primero para usuarios canary internos.
3. Monitorear errores de install/verify y comportamiento de startup.
4. Expandir rollout después de telemetría limpia.
5. Mantener lista de revocación operacional y testeada.

## 9. Solución de Problemas

### Instalación falla con errores de trust/publisher

- Revisa `[plugins].allow_publishers` en la config.
- Revisa el valor de publisher en el manifest.

### Instalación falla con digest mismatch

- Rebuild/republish del artefacto y regenerar digest del manifest.
- Verifica que el catálogo fuente apunte al digest correcto del artefacto.

### Problemas de sync de revocación

- Ejecuta `corvus plugins revocations sync`.
- Revisa los `[plugins.revocation].source_urls` configurados.
- Si enforcement está habilitado, fuentes de revocación rotas pueden bloquear operaciones de plugin
  por diseño.

### Migración del host de plugins antiguo

Al cargar config, Corvus migra referencias antiguas del host `plugins.corvus.ai` a
`corvus.profiletailors.com` para ambas URLs de fuente de catálogo y revocación.
