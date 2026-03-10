---
title: Proceso de Release
id: es-guides-release
---

Esta guía explica cómo publicar un release completo de Corvus (KMP + Rust + artefactos web)
usando GitHub Actions.

## Requisitos Previos

Antes de poder publicar, asegúrate de tener:

1. **Clave GPG configurada**: Sigue la [Guía de Configuración GPG](/guides/gpg-setup/) para crear y
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

4. **Permisos de escritura**: Debes ser mantenedor del repositorio

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

Este proyecto usa un modelo de dos branches para los releases:

- **`main`**: Releases estables. Los bug fixes y cambios no-breaking van aquí
- **`minor`**: Desarrollo de la siguiente versión minor. Las features van aquí

Consulta [MAINTENANCE.md](https://github.com/dallay/corvus/blob/main/.github/MAINTENANCE.md) para el
flujo de trabajo completo.

## Publicar un Release

### Paso 1: Asegurar que todos los cambios estén mergeados

Asegúrate de que todos los cambios que quieres publicar estén en el branch correcto:

- **Patch release**: Los cambios deben estar en `main`
- **Minor release**: Los cambios deben estar en `minor`

### Paso 2: Actualizar la versión

Actualiza la versión en todos los targets de versión del release:

```text
gradle.properties
gradle/build-logic/gradle.properties
clients/web/package.json
clients/web/apps/*/package.json
clients/web/packages/*/package.json
clients/agent-runtime/Cargo.toml
clients/agent-runtime/npm/corvus-cli/package.json
clients/agent-runtime/npm/corvus/package.json
clients/agent-runtime/npm/corvus-darwin-arm64/package.json
clients/agent-runtime/npm/corvus-darwin-x64/package.json
clients/agent-runtime/npm/corvus-linux-arm64/package.json
clients/agent-runtime/npm/corvus-linux-x64/package.json
clients/agent-runtime/npm/corvus-windows-arm64/package.json
clients/agent-runtime/npm/corvus-windows-x64/package.json
clients/agent-runtime/src/main.rs (actualiza #[command(version = "...")])
```

### Sincronizar la versión desde el tag Git automáticamente

Puedes mantener la versión del proyecto en sincronía con el tag Git automáticamente con el script y
el target Make incluidos en este repositorio.

- `make sync-version` — ejecuta `./scripts/sync-version-with-tag.sh` y sincroniza la última versión
  semántica del tag Git (`vX.Y.Z`) en:
  - `gradle.properties` (`VERSION=`)
  - `gradle/build-logic/gradle.properties` (`VERSION=`)
  - `clients/web/package.json` (`"version"`)
  - cada app web en `clients/web/apps/*/package.json` (`"version"`)
  - cada package compartido en `clients/web/packages/*/package.json` (`"version"`)
  - `clients/agent-runtime/Cargo.toml` (`version = "..."`)
  - `clients/agent-runtime/npm/corvus-cli/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-darwin-arm64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-darwin-x64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-linux-arm64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-linux-x64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-windows-arm64/package.json` (`"version"`)
  - `clients/agent-runtime/npm/corvus-windows-x64/package.json` (`"version"`)
  - `clients/agent-runtime/src/main.rs` (`#[command(version = "...")]`)
- `./scripts/sync-version-with-tag.sh` — script shell que selecciona el tag semántico más reciente global
  usando `git tag --sort=-v:refname | grep -Em1 '^v[0-9]+\.[0-9]+\.[0-9]+$'` (no el tag más cercano
  desde `HEAD`), extrae la versión numérica (quita la `v` inicial) y actualiza todos los targets de
  versión listados arriba.

Flujos de uso (elige uno):

1) Recomendado (actualiza el código primero, luego el tag)

```bash
# Actualiza los archivos de build y commitea
# incrementar version en gradle.properties o build.gradle.kts a 0.1.1
git add gradle.properties
git commit -m "chore: bump version to 0.1.1"

# Crear un tag anotado que coincida con la version
git tag -a v0.1.1 -m "Release v0.1.1"
# Pushear commit y tag
git push origin main
git push origin v0.1.1
```

2) Si creaste el tag primero (causa del fallo en CI), sincroniza el código con el tag localmente y
   commitea el cambio

```bash
# Asegúrate de tener el tag localmente (o fetch)
git fetch --tags

# Sincronizar los archivos de versión con el último tag
make sync-version
# Revisar y commitear el cambio
git add gradle.properties gradle/build-logic/gradle.properties clients/web/package.json clients/web/apps/*/package.json clients/web/packages/*/package.json clients/agent-runtime/Cargo.toml clients/agent-runtime/npm/corvus-cli/package.json clients/agent-runtime/npm/corvus/package.json clients/agent-runtime/npm/corvus-*/package.json
git commit -m "chore: sync version to $(awk -F= '/^VERSION=/{print $2; exit}' gradle.properties)"
# Pushear el commit (no es necesario recrear el tag)
git push origin main
```

Notas y advertencias:

- El CI de release exige que el tag Git (ej. `v0.1.1`) coincida con todos los archivos de
  versión controlados (Gradle + monorepo web + Cargo + matriz de paquetes npm del runtime). Si no
  coinciden, el build falla.
- En `clients/agent-runtime/npm/corvus/package.json`, mantén las versiones de
  `optionalDependencies` alineadas con la misma versión del release.
- Es preferible crear el commit que actualiza la versión antes de crear el tag para evitar
  desajustes.
- El script solo reconoce tags que cumplen la expresión `^v[0-9]+\.[0-9]+\.[0-9]+$`.

### Paso 3: Crear y pushear un tag

```bash
# Checkout del branch apropiado
git checkout main  # o git checkout minor

# Pull de los últimos cambios
git pull origin main

# Crear un tag anotado
git tag -a v1.2.3 -m "Release version 1.2.3"

# Pushear el tag (esto dispara el workflow de release)
git push origin v1.2.3
```

**Importante**: El tag debe coincidir con el patrón `v[0-9]+.[0-9]+.[0-9]+` (ej., `v1.2.3`)

### Paso 4: Monitorear el workflow

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
3. Selecciona el branch (usualmente `main` o `minor`)
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
- **Versiones desalineadas**: La versión del tag debe coincidir con Gradle + monorepo web + Cargo
  + versiones de paquetes npm del runtime (`clients/agent-runtime/npm/*`)
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
- [ ] La versión está sincronizada en todos los targets (Gradle, monorepo web, Cargo, matriz de
  paquetes npm del runtime)
- [ ] CHANGELOG.md está actualizado (si se mantiene manualmente)
- [ ] La clave GPG es válida y no ha expirado
- [ ] Las credenciales de Maven Central son actuales
- [ ] Los secrets de crates.io, npm y Docker Hub están configurados
- [ ] El tag sigue el formato `vX.Y.Z`
- [ ] Se está trabajando en el branch correcto (`main` para patches, `minor` para features)

## Ver También

- [Guía de Configuración GPG](/guides/gpg-setup/)
- [GitHub Workflows](https://github.com/dallay/corvus/blob/main/.github/workflows/README.md)
- [Guía de Contribución](https://github.com/dallay/corvus/blob/main/.github/CONTRIBUTING.md)
