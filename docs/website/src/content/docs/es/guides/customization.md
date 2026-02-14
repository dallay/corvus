---
title: Personalización de la Plantilla
---

Cuando utilizas la plantilla **Starter Gradle**, es probable que desees adaptarla a tu propio proyecto y organización. Esta guía cubre los pasos esenciales para hacerla tuya.

## Identidad del proyecto

El primer paso es renombrar el proyecto y actualizar su grupo y versión.

### Renombrar el proyecto

Actualiza `rootProject.name` en `settings.gradle.kts`:

```kotlin
rootProject.name = "nombre-de-tu-proyecto"
```

### Actualizar grupo y versión

Modifica las siguientes propiedades en `gradle.properties`:

```properties
GROUP=com.tuempresa
VERSION=1.0.0-SNAPSHOT
```

## Metadatos de Maven/POM

Si planeas publicar tu proyecto en un repositorio Maven, debes actualizar los metadatos en `gradle.properties`:

```properties
POM_DEVELOPER_NAME=Tu Nombre
POM_URL=https://github.com/tuusuario/tu-proyecto
POM_SCM_CONNECTION=scm:git:https://github.com/tuusuario/tu-proyecto.git
```

## Renombrado de paquetes

La plantilla utiliza `com.profiletailors` como paquete base. Debes renombrarlo para que coincida con el grupo de tu proyecto.

1. Mueve los archivos en `app/src/main/kotlin/com/profiletailors/app` a la estructura de paquetes deseada.
2. Actualiza la declaración de `package` en todos los archivos fuente.
3. Repite el proceso para los módulos en el directorio `examples/` si decides conservarlos.

## Sitio de documentación

El sitio de documentación está construido con Starlight. Personalízalo en `docs/website/astro.config.mjs`:

- `site`: Tu URL de producción.
- `base`: La subruta si no se aloja en la raíz (por ejemplo, `/tu-proyecto`).
- `starlight.title`: El título de tu documentación.
- `social`: Enlaces a tu repositorio o redes sociales.

## Licencia

La plantilla está bajo la Licencia MIT.

1. Actualiza el archivo `LICENSE` con tu nombre/organización y el año.
2. Actualiza `POM_LICENSE_URL` en `gradle.properties` si cambias la licencia.

## CI/CD e integración con GitHub

Si utilizas GitHub Actions:

1. Revisa `.github/workflows/` y actualiza cualquier referencia a `dallay/common-actions` si deseas utilizar tus propios workflows compartidos.
2. Actualiza `.github/CODEOWNERS` para reflejar los mantenedores de tu proyecto.
3. Actualiza los badges y enlaces en `README.md`.
