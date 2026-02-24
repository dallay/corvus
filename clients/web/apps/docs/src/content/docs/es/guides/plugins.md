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

- Catálogo: `https://plugins.corvus.profiletailors.com/catalog.json`
- Revocaciones: `https://plugins.corvus.profiletailors.com/revocations.json`

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

1. Se dispara automáticamente con tags de release: `plugin/<plugin-id>/v<semver>`.
2. Resuelve dinámicamente la carpeta del plugin usando `package.metadata.corvus.plugin_id` en
   cada `Cargo.toml`.
3. Hace build del artefacto WASM para `wasm32-wasip1`.
4. Ensambla bundle inmutable con artefactos/metadatos:
   - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm`
   - `artifacts/<plugin-id>/<version>/plugin-manifest.json`
   - `catalog.json` raíz (upsert del plugin, preservando los demás)
   - `revocations.json` raíz (preserva la lista, actualiza `updated_at`)
5. Firma el artefacto con identidad OIDC keyless de cosign.
6. Verifica la firma en CI.
7. Push opcional del bundle a OCI (`oci_repository`).
8. Sube artefactos inmutables y metadatos mutables a Cloudflare R2 en orden atómico
   (artefactos primero, `catalog.json`/`revocations.json` al final).

9. Verifica integridad del catálogo en R2 y smoke-check de endpoints públicos del Worker.

10. Opcionalmente hace build/deploy del catálogo legacy en Cloudflare Pages (`deploy_cloudflare_pages=true`).

11. Sube artefactos del build y del bundle para trazabilidad.

:::important
Para integrar un nuevo plugin al release automático:

1. Créalo en `clients/agent-runtime/plugins/<carpeta-plugin>/`.
2. Agrega `package.metadata.corvus.plugin_id` en su `Cargo.toml`.
3. Define límites/capacidades en `package.metadata.corvus`.
4. Crea un tag de release: `plugin/<plugin-id>/v<version>`.

Con metadata correcta, no hace falta cambiar el workflow para plugins nuevos.
:::

Ejemplo de release:

```bash
git tag plugin/memory.surreal.graphs/v0.1.0
git push origin plugin/memory.surreal.graphs/v0.1.0
```

Configuración de Cloudflare esperada por el workflow:

- Secret: `CLOUDFLARE_API_TOKEN`
- Secret: `CLOUDFLARE_ACCOUNT_ID`
- Variable de repositorio: `CLOUDFLARE_R2_BUCKET_NAME`
- Variable opcional de repositorio (solo deploy legacy de Pages): `CLOUDFLARE_PAGES_PROJECT_NAME`

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
`plugins.corvus.profiletailors.com` para ambas URLs de fuente de catálogo y revocación.
