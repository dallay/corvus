---
title: Proceso de Release
description: Procedimiento canónico de release para publicar artefactos de Corvus mediante GitHub Actions, firma y flujos de Maven Central.
owner: team-platform
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: runbook
---

Esta guía explica cómo publicar un release completo de Corvus (KMP + Rust + artefactos web)
usando GitHub Actions.

## Requisitos Previos

Antes de poder publicar, asegúrate de tener:

1. **Clave GPG configurada**: Sigue la [Guía de Configuración GPG](./gpg-setup/) para crear y
   configurar tu clave de firma
2. **Acceso a Maven Central**: Secrets del repositorio configurados:

- `SIGNING_IN_MEMORY_KEY`: Tu clave privada GPG
- `SIGNING_IN_MEMORY_KEY_PASSWORD`: Contraseña de la clave GPG
- `MAVEN_CENTRAL_USERNAME`: Usuario de Maven Central
- `MAVEN_CENTRAL_PASSWORD`: Contraseña de Maven Central

3. **Secrets de canales de release** para artefactos no-Gradle:

- `CARGO_REGISTRY_TOKEN`: token de publicación en crates.io para `clients/agent-runtime`
- `NPM_TOKEN`: token de npm para `@dallay/corvus`
- `DOCKERHUB_USERNAME`: usuario de Docker Hub
- `DOCKERHUB_TOKEN`: token de acceso de Docker Hub

1. **Token de Release Please**: `RELEASE_PLEASE_TOKEN` debe ser un **PAT fine-grained** limitado
   a este repositorio (sin tokens de alcance global de organización).
   Permisos mínimos: **Contents (Read/Write)**, **Pull requests (Read/Write)**, **Issues (Read/Write)**.
   Usa expiración corta (≤ 90 días), rota cada 90 días y documenta ownership/seguimiento de la rotación.
   El secret debe guardarse en GitHub Secrets con el nombre exacto `RELEASE_PLEASE_TOKEN`.
2. **Permisos de escritura**: Debes ser mantenedor del repositorio

### Qué publica un release

Cuando `publish-release.yml` corre por un tag `vX.Y.Z`, publica:

- **Artefactos KMP/Gradle** a Maven Central (`publishToMavenCentral`)
- **Artefactos del plugin de build logic** a Maven Central en tags estables
- **Crate de Rust** (`clients/agent-runtime`) a crates.io
- **Paquetes npm del runtime** (`clients/agent-runtime/npm/*`) a npm, incluyendo
  `@dallay/corvus` y los paquetes específicos por plataforma
- **Imágenes de contenedor** a Docker Hub y GHCR
- **Binarios nativos + checksums** adjuntos al GitHub Release

Las apps web (`clients/web/apps/docs`, `clients/web/apps/marketing`,
`clients/web/apps/dashboard`) se construyen en workflows separados y no se publican a Maven,
crates.io o npm desde `publish-release.yml`.

## Entendiendo el Modelo de Branches

Los releases salen de `main`. Todos los cambios listos para publicar deben estar mergeados en
`main` antes de mergear el PR de release.

## Publicar un Release

### Paso 1: Mergea cambios en `main`

Asegúrate de que todos los cambios que quieres publicar estén mergeados en `main`.

### Paso 2: Release Please abre el PR de release

En cada push a `main`, Release Please crea o actualiza un PR de release que:

- Bumpea versiones en Gradle, Cargo, npm y paquetes web
- Actualiza `optionalDependencies` en `clients/agent-runtime/npm/corvus/package.json`
- Genera notas de release con Conventional Commits

Para controlar el bump, usa Conventional Commits:

- `fix:` -> patch
- `feat:` -> minor
- `feat!:` o `BREAKING CHANGE:` -> major

### Paso 3: Revisa y mergea el PR de release

Revisa el PR, valida las versiones y mergea cuando esté listo.

### Paso 4: Tag y publicación

Al mergear el PR, Release Please crea el tag `vX.Y.Z`. Ese tag dispara `publish-release.yml`,
que ejecuta `_publish.yml` para publicar todos los artefactos.

### Paso 5: Monitorear el workflow

1. Ve a la pestaña **Actions** en GitHub
2. Haz clic en el workflow **Publish Release**
3. Espera a que termine (usualmente 5-10 minutos)

El workflow hará:

- Build y publicación de artefactos Gradle/KMP en Maven Central
- Publicación del crate de Rust en crates.io
- Publicación del paquete npm CLI
- Build y publicación de imágenes Docker (Docker Hub + GHCR)
- Build de binarios nativos para Linux, macOS y Windows, generación de checksums SHA256 y adjunto al
  GitHub Release
- Generación de changelog y creación/actualización del GitHub Release

Después de publicar el GitHub Release, `deploy-docs.yml` también puede desplegar docs en
GitHub Pages.

## Publicar un Snapshot

Los snapshots se publican automáticamente cada día, pero esto aplica solo al canal
Gradle/Maven.

### Automático (Diario)

El workflow `publish-snapshot.yml` corre diariamente a las 02:12 UTC.

### Manual

1. Ve a la pestaña **Actions** → **Publish Snapshot**
2. Haz clic en **Run workflow**
3. Selecciona el branch (usualmente `main`)
4. Haz clic en **Run workflow**

Los snapshots usan la versión definida en los archivos de build de Gradle con sufijo
`-SNAPSHOT`.
Crates de Rust, paquete npm, imágenes Docker y assets de GitHub Release solo se publican en
releases estables `vX.Y.Z`.

## Solución de Problemas

### El workflow de release falló

1. Revisa los logs del workflow en GitHub Actions
2. Problemas comunes:

- **Firma fallida**: Verifica que los secrets GPG estén correctamente configurados
- **Autenticación Maven Central fallida**: Verifica que las credenciales no hayan expirado
- **Build fallido**: Asegúrate de que todos los tests pasen localmente con `./gradlew check`
- **Versiones desalineadas**: Release Please mantiene versiones alineadas. Si falla, revisa
  `release-please-config.json` y el diff del PR de release
- **PR de release no creado**: Falta `RELEASE_PLEASE_TOKEN` o los commits no son Conventional
- **Secret faltante de release**: `CARGO_REGISTRY_TOKEN`, `NPM_TOKEN`,
  `DOCKERHUB_USERNAME` o `DOCKERHUB_TOKEN`

### La versión ya existe

Maven Central no permite sobrescribir releases. Si necesitas corregir algo:

1. Usa una nueva versión de patch (ej., `v1.2.4` en lugar de `v1.2.3`)
2. Nunca borres y recrees tags con la misma versión

### Snapshot no se actualiza

Los snapshots pueden ser cacheados por Maven/Gradle. Fuerza una actualización:

```bash
./gradlew build --refresh-dependencies
```

## Checklist de Release

Usa este checklist antes de publicar:

- [ ] Todos los tests pasan localmente (`./gradlew check`)
- [ ] PR de release actualizado y mergeado
- [ ] Versiones alineadas en el diff del PR de release
- [ ] La clave GPG es válida y no ha expirado
- [ ] Las credenciales de Maven Central son actuales
- [ ] Los secrets de crates.io, npm y Docker Hub están configurados
- [ ] Tag `vX.Y.Z` creado por Release Please

## Ver También

- [Guía de Configuración GPG](./gpg-setup/)
- [GitHub Workflows](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [Guía de Contribución](https://github.com/dallay/corvus/blob/main/.github/CONTRIBUTING.md)
