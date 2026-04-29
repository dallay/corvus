---
title: Proceso de Release
description: Runbook canónico de releases estables, beta y snapshots de Corvus con release-please, workflows de publicación y GitHub Releases.
owner: team-platform
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: runbook
---

Este runbook define el contrato canónico de release de Corvus.

- `release-please.yml` es dueño del PR repo-wide de release, del tag canónico `vX.Y.Z`, del GitHub Release canónico y de las notas canónicas del release estable.
- `release-please-beta.yml` es dueño del PR repo-wide beta, del tag canónico `vX.Y.Z-beta.N`, del GitHub Release beta canónico y de las notas canónicas del release beta desde la rama `beta`.
- `publish-release.yml` y `_publish.yml` solo son dueños de la publicación de artefactos después de que `release-please` publique el GitHub Release.
- GitHub Releases es la fuente canónica de notas de release estables.
- `publish-snapshot.yml` es una ruta solo de snapshots para Gradle/Maven y no es dueña de notas de release estables.

## Requisitos Previos

Antes de publicar, confirma:

1. **Acceso al repositorio**
   - Eres maintainer de `dallay/corvus`.
   - `APP_ID` y `APP_PRIVATE_KEY` están configurados para que release-please pueda emitir un token de GitHub App con permiso para abrir PRs y crear tags canónicos más el GitHub Release canónico.
2. **Credenciales de release para Gradle/Maven**
   - `SIGNING_IN_MEMORY_KEY`
   - `SIGNING_IN_MEMORY_KEY_PASSWORD`
   - `MAVEN_CENTRAL_USERNAME`
   - `MAVEN_CENTRAL_PASSWORD`
3. **Credenciales de canales de release**
   - `CARGO_REGISTRY_TOKEN`
   - `NPM_TOKEN`
   - `DOCKERHUB_USERNAME`
   - `DOCKERHUB_TOKEN`

## Contrato de Release Estable

### Graph canónico de release scope y resolvers

- `config/release-components.json` es el graph canónico de componentes gestionados.
- `scripts/release-components.mjs` carga y valida ese graph antes de que lo consuman los workflows.
- `scripts/resolve-release-components.mjs` resuelve el scope por archivos cambiados para `release-please.yml` y `release-please-beta.yml`.
- `scripts/resolve-release-from-tag.mjs` resuelve el scope del publish estable desde el tag del release y el override opcional `affected_components:` en el body del release.
- `scripts/sync-internal-release-deps.mjs` valida y normaliza dependencias internas versionadas del release antes de regenerar lockfiles y ejecutar validaciones pesadas de Rust.

### Qué sale en un release estable `vX.Y.Z`

La automatización estable valida y publica solo artefactos enviados:

- artefactos Gradle/KMP, incluida la publicación de build-logic
- crate de Rust: `cerebro`
- crate de Rust + paquete npm + imagen Docker: `corvus-runtime`
- crate de Rust + paquete npm + imagen Docker: `rook`
- crate `clients/agent-runtime`
- assets de release de `clients/cerebro`
- paquetes npm del runtime:
  - `@dallay/corvus`
  - `@dallay/corvus-darwin-x64`
  - `@dallay/corvus-darwin-arm64`
  - `@dallay/corvus-linux-x64`
  - `@dallay/corvus-linux-arm64`
  - `@dallay/corvus-windows-x64`
- imágenes Docker
- archivos nativos y checksums adjuntos al GitHub Release

### Exclusiones intencionales

- Los paquetes web privados están excluidos del versionado repo-wide para releases estables.
- `clients/web/**/package.json` no forma parte del fan-out estable de release-please.
- `clients/agent-runtime/npm/corvus-cli/package.json` es interno/privado y está excluido del fan-out estable y de la publicación npm estable.
- Windows ARM64 está intencionalmente sin soporte para publicación npm estable por ahora. `@dallay/corvus-windows-arm64` no se publica y no aparece en las `optionalDependencies` de `@dallay/corvus`.

## Flujo de Release Estable

1. Mergea en `main` el trabajo listo para salir.
2. `release-please.yml` abre o actualiza un único PR repo-wide de release.
3. El mantenimiento del PR de release ejecuta `node scripts/sync-internal-release-deps.mjs --write` para mantener alineadas las dependencias internas con `path` y versión antes de regenerar lockfiles.
4. Revisa el diff del PR de release. Solo deben cambiar de versión los artefactos estables publicados y sus pins declarados de dependencias internas de release.
5. Mergea el PR de release.
5. release-please crea el tag canónico `vX.Y.Z` y el GitHub Release canónico.
6. `publish-release.yml` corre desde `release.published` y pasa el contexto explícito `release_tag` / `release_id` hacia `_publish.yml`.
7. `_publish.yml` valida versiones de artefactos enviados, publica artefactos y adjunta assets al GitHub Release existente.

El GitHub Release creado por `release-please` es el registro público canónico del release. El `CHANGELOG.md` raíz es solo un apuntador.

## Contrato de Release Beta

### Qué sale en un release beta `vX.Y.Z-beta.N`

La automatización beta publica las mismas superficies de artefactos que el canal estable:

- artefactos Gradle/KMP, incluida la publicación de build-logic
- crate `clients/agent-runtime`
- assets de release de `clients/cerebro`
- paquetes npm de runtime enviados
- imágenes Docker
- archivos nativos y checksums adjuntos al GitHub Release

### Reglas del canal beta

- `release-please-beta.yml` corre desde la rama `beta` y es dueño de los PRs, tags, GitHub Releases y notas del canal beta.
- Los releases beta usan tags con formato `vX.Y.Z-beta.N`.
- El GitHub Release debe mantenerse marcado como prerelease.
- Los releases beta usan el dist-tag `beta` de npm y no deben sobrescribir `latest`.
- La publicación beta de Docker usa la versión exacta del prerelease más el tag móvil `beta` y no debe sobrescribir alias estables como `latest`, `X` o `X.Y`.

## Flujo de Release Beta

Solo `release-please` y `release-please-beta` son dueños de las notas canónicas del GitHub Release.
`publish-release.yml` y `_publish.yml` nunca deben reemplazar, editar o reinterpretar las notas canónicas del GitHub Release.
El workflow beta sigue respetando la misma superficie de artefactos: `cerebro`, `corvus-runtime` y `rook`.


1. Crea o refresca la rama `beta` desde `minor` cuando un candidato prerelease esté listo para validación más amplia.
2. Mergea fixes listos para salir en `beta`.
3. `release-please-beta.yml` abre o actualiza un único PR repo-wide beta.
4. Revisa el diff del PR prerelease. Solo los artefactos beta enviados deben recibir bump de versión.
5. Mergea el PR beta.
6. `release-please-beta.yml` crea el tag canónico `vX.Y.Z-beta.N` y el GitHub Release beta canónico.
7. `_publish.yml` publica artefactos beta en modo prerelease, usando el dist-tag `beta` de npm y tags beta-safe en Docker.

### Regla de gobernanza

- Los cambios a la automatización de releases deben entrar por pull request, no por push directo a `main`.
- Trata los pushes directos a `main` como recuperación de emergencia solamente y documenta la razón cuando ocurra.
- Si la infraestructura de release se rompe, arréglala en una rama corta, abre un PR y deja que la protección de rama y los checks validen el fix.

## Flujo de Snapshot

`publish-snapshot.yml` es manual o programado y cubre solo el canal snapshot de Gradle/Maven.

- No crea el tag estable canónico.
- No crea GitHub Release.
- No publica notas de release estables.

## Migración a releases por componente

La transición desde el release repo-wide actual hacia releases por componente se hace por fases.

1. **Descubrimiento**
   - mantener actualizado el inventario canónico de componentes releaseables
   - mantener actualizado el mapa de impacto por paths y módulos compartidos
2. **Transición controlada**
   - definir rollout, señales de éxito y rollback antes de tocar automatización
3. **Piloto de versionado**
   - empezar con un componente piloto de superficie pequeña
4. **Piloto de publish selectivo**
   - validar y publicar solo componentes afectados
5. **Expansión gradual**
   - retirar supuestos repo-wide solo cuando el patrón ya esté probado

### Regla de rollback

Si una fase introduce drift de tags, changelog confuso o publicación fuera de alcance:

- revierte juntos los archivos de configuración y manifiesto de `release-please` de esa fase
- revierte juntos los workflows llamadores y `_publish.yml` si el problema está en gating/publicación
- no crees tags competidores para “arreglar rápido” el estado
- vuelve al último estado sano y documenta la causa antes de avanzar

## Diagnósticos a revisar durante un Release

### `release-please.yml`

Revisa el summary del workflow para ver:

- versión base del manifiesto desde `.release-please-manifest.json`
- salida de versión candidata
- si se creó un PR de release en esa ejecución
- si hubo salidas de tag/release
- salidas crudas de release-please para diagnosticar drift

### `release-please-beta.yml`

Revisa el summary del workflow para ver:

- versión base del manifiesto desde `.release-please-beta-manifest.json`
- salida de versión beta candidata
- si se creó un PR beta en esa ejecución
- si hubo salidas de tag/release beta
- salidas crudas de release-please para diagnosticar drift

### `_publish.yml`

Revisa el summary del workflow para ver:

- tag recibido
- id del GitHub Release existente / destino de upload de assets
- tabla de validación de versiones de artefactos enviados
- advertencias de credenciales opcionales
- resultado por superficie de publicación
- notas de política npm que confirman que `corvus-cli` es interno/privado y que Windows ARM64 no tiene soporte
- resultado del upload de assets al GitHub Release
- confirmación de que release-please es dueño de las notas canónicas del release estable
- confirmación de que release-please-beta.yml es dueño de las notas canónicas del release beta

## Recuperación Manual de Baseline

La recuperación de baseline es una acción del operador. Los workflows de este cambio **no** crean ni reescriben tags o releases vivos de manera automática. Trátalo como recuperación manual, no como reparación automática del workflow.

Usa este procedimiento cuando haya drift entre el manifiesto, los tags o GitHub Releases:

1. Verifica que `.release-please-manifest.json` o `.release-please-beta-manifest.json`, `version.txt`, propiedades de Gradle, manifests de Cargo y versiones de paquetes npm enviados coincidan con la versión esperada para el canal que estás reparando.
2. Verifica el SHA del commit de release previsto.
3. Verifica si el tag canónico `vX.Y.Z` ya existe.
4. Verifica si el GitHub Release ya existe.
5. Si el commit de release y los archivos de versión están correctos pero falta el tag o el release, restablece primero la autoridad del GitHub Release canónico y vuelve a correr la publicación estable desde `release.published` como recuperación manual.
6. Si la evidencia entra en conflicto, detente y elige una nueva baseline hacia adelante en vez de reescribir historia.

## Solución de Problemas

### No aparece PR de release

- Confirma que `APP_ID` y `APP_PRIVATE_KEY` existen y que el GitHub App está instalado en `dallay/corvus`.
- Confirma que los commits siguen Conventional Commits.
- Revisa el summary de `release-please.yml` antes de tocar la configuración.

### Se mergeó el PR pero no arrancó la publicación estable

- Confirma que existe el GitHub Release canónico y que fue publicado.
- Confirma que release-please creó el release con el token y permisos esperados.
- Confirma que el trigger de `publish-release.yml` vio `release.published` para el mismo tag `vX.Y.Z`.

### Se mergeó el PR pero no arrancó la publicación beta

- Confirma que existe el GitHub Release beta canónico y que está marcado como prerelease.
- Confirma que `release-please-beta.yml` creó el release con el token y permisos esperados.
- Confirma que `_publish.yml` fue llamado desde `release-please-beta.yml` con `prerelease: true`.

### `release-please` falla con `Resource not accessible by integration`

- Verifica primero que el token del workflow puede llamar la API de GitHub Releases antes de cambiar permisos del GitHub App.
- Revisa el PR de release ya mergeado para detectar labels stale de `release-please`.
- Si el PR mergeado todavía tiene `autorelease: pending`, elimina ese label y vuelve a correr `release-please`.
- Trata un label stale `autorelease: pending` como drift del estado de release, no como prueba de un problema real de permisos del GitHub App.

### Drift de dependencias internas de release

- Ejecuta `node scripts/sync-internal-release-deps.mjs --check` para validar los pins internos administrados por el release.
- Si el check reporta drift, ejecuta `node scripts/sync-internal-release-deps.mjs --write` y vuelve a regenerar los lockfiles.
- Trata mismatches como `corvus-runtime -> cerebro` como fallas del contrato de release y no como fallas genéricas de `Cargo.lock`.

### Falló la publicación estable

- Revisa la tabla de validación de versiones en `_publish.yml`.
- Revisa el contexto de release id / tag que `publish-release.yml` pasó hacia `_publish.yml`.
- Revisa advertencias de credenciales para Maven, Cargo, npm y Docker.
- Repara hacia adelante desde la etapa que falló. No cortes un tag competidor con la misma versión.

### Faltó el GitHub Release después de publicar artefactos

- Repara primero el GitHub Release de `release-please`, porque `_publish.yml` solo adjunta assets al release existente.
- Vuelve a correr el handoff estable desde `release.published` después de que el release canónico exista otra vez.
- No trates `CHANGELOG.md` como fuente de verdad.

### Hay drift entre notas del release y assets publicados

- Trata a `release-please` como la única autoridad canónica de notas de release.
- Trata a `release-please-beta.yml` como la única autoridad canónica de notas de release beta.
- `_publish.yml` puede adjuntar assets al GitHub Release existente, pero no debe reemplazar el cuerpo canónico de notas.

## Referencias Canónicas

- [GitHub Releases de dallay/corvus](https://github.com/dallay/corvus/releases)
- [Guía de workflows de GitHub Actions](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [Guía de configuración GPG](./gpg-setup)
