---
title: Proceso de Release
---

Esta guía explica cómo publicar releases y snapshots en Maven Central usando GitHub Actions.

## Requisitos Previos

Antes de poder publicar, asegúrate de tener:

1. **Clave GPG configurada**: Sigue la [Guía de Configuración GPG](./gpg-setup/) para crear y configurar tu clave de firma
2. **Acceso a Maven Central**: Secrets del repositorio configurados:
   - `SIGNING_IN_MEMORY_KEY`: Tu clave privada GPG
   - `SIGNING_IN_MEMORY_KEY_PASSWORD`: Contraseña de la clave GPG
   - `MAVEN_CENTRAL_USERNAME`: Usuario de Maven Central
   - `MAVEN_CENTRAL_PASSWORD`: Contraseña de Maven Central
3. **Permisos de escritura**: Debes ser mantenedor del repositorio

## Entendiendo el Modelo de Branches

Este proyecto usa un modelo de dos branches para los releases:

- **`main`**: Releases estables. Los bug fixes y cambios no-breaking van aquí
- **`minor`**: Desarrollo de la siguiente versión minor. Las features van aquí

Consulta [MAINTENANCE.md](https://github.com/dallay/starter-gradle/blob/main/.github/MAINTENANCE.md) para el flujo de trabajo completo.

## Publicar un Release

### Paso 1: Asegurar que todos los cambios estén mergeados

Asegúrate de que todos los cambios que quieres publicar estén en el branch correcto:
- **Patch release**: Los cambios deben estar en `main`
- **Minor release**: Los cambios deben estar en `minor`

### Paso 2: Actualizar la versión

Actualiza la versión en los lugares apropiados:

```kotlin
// build.gradle.kts o gradle.properties
version = "1.2.3"  // Para releases de patch
version = "1.3.0"  // Para releases minor
```

### Sincronizar la versión desde el tag Git automáticamente

Puedes mantener la versión del proyecto en sincronía con el tag Git automáticamente con el script y el target Make incluidos en este repositorio.

- `make sync-version` — ejecuta `./sync-version-with-tag.sh` y sincroniza la última versión semántica del tag Git (`vX.Y.Z`) en:
  - `gradle.properties` (`VERSION=`)
  - `gradle/build-logic/gradle.properties` (`VERSION=`)
  - `docs/website/package.json` (`"version"`)
- `./sync-version-with-tag.sh` — script shell que selecciona el tag semántico más reciente global usando `git tag --sort=-v:refname | grep -Em1 '^v[0-9]+\.[0-9]+\.[0-9]+$'` (no el tag más cercano desde `HEAD`), extrae la versión numérica (quita la `v` inicial) y actualiza todos los targets de versión listados arriba.

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

2) Si creaste el tag primero (causa del fallo en CI), sincroniza el código con el tag localmente y commitea el cambio

```bash
# Asegúrate de tener el tag localmente (o fetch)
git fetch --tags

# Sincronizar los archivos de versión con el último tag
make sync-version
# Revisar y commitear el cambio
git add gradle.properties gradle/build-logic/gradle.properties docs/website/package.json
git commit -m "chore: sync version to $(awk -F= '/^VERSION=/{print $2; exit}' gradle.properties)"
# Pushear el commit (no es necesario recrear el tag)
git push origin main
```

Notas y advertencias:
- El CI de release exige que el tag Git (ej. `v0.1.1`) coincida con la versión en el código (ej. `0.1.1`). Si no coinciden, el build falla con un error como: "CI Release: GitHub tag (v0.1.1) must match Code version (0.1.1)".
- Es preferible crear el commit que actualiza la versión antes de crear el tag para evitar desajustes.
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
- Build del proyecto
- Generación de changelog desde commits convencionales
- Publicación en Maven Central
- Creación de un GitHub release con el changelog

## Publicar un Snapshot

Los snapshots se publican automáticamente cada día, pero también puedes dispararlos manualmente:

### Automático (Diario)

El workflow `publish-snapshot.yml` corre diariamente a las 02:12 UTC.

### Manual

1. Ve a la pestaña **Actions** → **Publish Snapshot**
2. Haz clic en **Run workflow**
3. Selecciona el branch (usualmente `main` o `minor`)
4. Haz clic en **Run workflow**

Los snapshots usan la versión definida en tus archivos de build con un sufijo `-SNAPSHOT`.

## Solución de Problemas

### El workflow de release falló

1. Revisa los logs del workflow en GitHub Actions
2. Problemas comunes:
   - **Firma fallida**: Verifica que los secrets GPG estén correctamente configurados
   - **Autenticación Maven Central fallida**: Verifica que las credenciales no hayan expirado
   - **Build fallido**: Asegúrate de que todos los tests pasen localmente con `./gradlew check`

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
- [ ] La versión está actualizada en los archivos de build
- [ ] CHANGELOG.md está actualizado (si se mantiene manualmente)
- [ ] La clave GPG es válida y no ha expirado
- [ ] Las credenciales de Maven Central son actuales
- [ ] El tag sigue el formato `vX.Y.Z`
- [ ] Se está trabajando en el branch correcto (`main` para patches, `minor` para features)

## Ver También

- [Guía de Configuración GPG](./gpg-setup/)
- [GitHub Workflows](https://github.com/dallay/starter-gradle/blob/main/.github/workflows/README.md)
- [Guía de Contribución](https://github.com/dallay/starter-gradle/blob/main/.github/CONTRIBUTING.md)
