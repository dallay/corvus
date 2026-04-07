---
title: Proceso de Release
description: Runbook canónico de releases estables y snapshots de Corvus con release-please, workflows de publicación y GitHub Releases.
owner: team-platform
status: canonical
lastReviewed: 2026-04-07
appliesTo: main
docType: runbook
---

Este runbook define el contrato canónico de release de Corvus.

- `release-please.yml` es dueño del PR repo-wide de release y del tag canónico `vX.Y.Z`.
- `publish-release.yml` y `_publish.yml` son dueños de la publicación de artefactos y del GitHub Release final.
- GitHub Releases es la fuente canónica de notas de release estables.
- `publish-snapshot.yml` es una ruta solo de snapshots para Gradle/Maven y no es dueña de notas de release estables.

## Requisitos Previos

Antes de publicar, confirma:

1. **Acceso al repositorio**
   - Eres maintainer de `dallay/corvus`.
   - `RELEASE_PLEASE_TOKEN` está configurado para que release-please pueda abrir PRs y crear tags canónicos.
2. **Credenciales de release para Gradle/Maven**
   - `SIGNING_IN_MEMORY_KEY`
   - `SIGNING_IN_MEMORY_KEY_PASSWORD`
   - `MAVEN_CENTRAL_USERNAME`
   - `MAVEN_CENTRAL_PASSWORD`
3. **Credenciales de canales estables**
   - `CARGO_REGISTRY_TOKEN`
   - `NPM_TOKEN`
   - `DOCKERHUB_USERNAME`
   - `DOCKERHUB_TOKEN`

## Contrato de Release Estable

### Qué sale en un release estable `vX.Y.Z`

La automatización estable valida y publica solo artefactos enviados:

- artefactos Gradle/KMP, incluida la publicación de build-logic
- crate `clients/agent-runtime`
- assets de release de `modules/cerebro`
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
3. Revisa el diff del PR. Solo los artefactos estables enviados deben recibir bump de versión.
4. Mergea el PR de release.
5. release-please crea el tag canónico `vX.Y.Z`.
6. `publish-release.yml` corre desde ese tag y llama a `_publish.yml`.
7. `_publish.yml` valida versiones de artefactos enviados, publica artefactos y crea o actualiza el GitHub Release.

El GitHub Release es el registro público canónico del release. El `CHANGELOG.md` raíz es solo un apuntador.

## Flujo de Snapshot

`publish-snapshot.yml` es manual o programado y cubre solo el canal snapshot de Gradle/Maven.

- No crea el tag estable canónico.
- No crea GitHub Release.
- No publica notas de release estables.

## Diagnósticos que debes revisar

### `release-please.yml`

Revisa el summary del workflow para ver:

- versión base del manifiesto desde `.release-please-manifest.json`
- salida de versión candidata
- si se creó un PR de release en esa ejecución
- si hubo salidas de tag/release
- salidas crudas de release-please para diagnosticar drift

### `_publish.yml`

Revisa el summary del workflow para ver:

- tag recibido
- tabla de validación de versiones de artefactos enviados
- advertencias de credenciales opcionales
- resultado por superficie de publicación
- notas de política npm que confirman que `corvus-cli` es interno/privado y que Windows ARM64 no tiene soporte
- resultado de publicación del GitHub Release

## Recuperación Manual de Baseline

La recuperación de baseline es una acción del operador. Los workflows de este cambio **no** crean ni reescriben tags o releases vivos de manera automática. Trátalo como recuperación manual, no como reparación automática del workflow.

Usa este procedimiento cuando haya drift entre el manifiesto, los tags o GitHub Releases:

1. Verifica que `.release-please-manifest.json`, `version.txt`, propiedades de Gradle, manifests de Cargo y versiones de paquetes npm enviados coincidan con la versión estable esperada.
2. Verifica el SHA del commit de release previsto.
3. Verifica si el tag canónico `vX.Y.Z` ya existe.
4. Verifica si el GitHub Release ya existe.
5. Si el commit de release y los archivos de versión están correctos pero falta el tag o el release, repón ese tag faltante y vuelve a correr la publicación estable como una recuperación manual.
6. Si la evidencia entra en conflicto, detente y elige una nueva baseline hacia adelante en vez de reescribir historia.

## Solución de Problemas

### No aparece PR de release

- Confirma que `RELEASE_PLEASE_TOKEN` existe.
- Confirma que los commits siguen Conventional Commits.
- Revisa el summary de `release-please.yml` antes de tocar la configuración.

### Se mergeó el PR pero no arrancó la publicación estable

- Confirma que existe el tag canónico `vX.Y.Z`.
- Confirma que release-please creó el tag con los permisos esperados.

### Falló la publicación estable

- Revisa la tabla de validación de versiones en `_publish.yml`.
- Revisa advertencias de credenciales para Maven, Cargo, npm y Docker.
- Repara hacia adelante desde la etapa que falló. No cortes un tag competidor con la misma versión.

### Faltó el GitHub Release después de publicar artefactos

- Reejecuta o repara la parte de GitHub Release en `_publish.yml`.
- No trates `CHANGELOG.md` como fuente de verdad.

## Referencias Canónicas

- [GitHub Releases de dallay/corvus](https://github.com/dallay/corvus/releases)
- [Guía de workflows de GitHub Actions](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [Guía de configuración GPG](./gpg-setup/)
