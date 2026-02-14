---
title: Template Customization
---

When you use the **Starter Gradle** template, you likely want to adapt it to your own project and organization. This guide covers the essential steps to make it yours.

## Project Identity

The first step is to rename the project and update its group and version.

### Rename the project

Update the `rootProject.name` in `settings.gradle.kts`:

```kotlin
rootProject.name = "your-project-name"
```

### Update group and version

Modify the following properties in `gradle.properties`:

```properties
GROUP=com.yourcompany
VERSION=1.0.0-SNAPSHOT
```

## Maven/POM Metadata

If you plan to publish your project to a Maven repository, you should update the metadata in `gradle.properties`:

```properties
POM_DEVELOPER_NAME=Your Name
POM_URL=https://github.com/youruser/your-project
POM_SCM_CONNECTION=scm:git:https://github.com/youruser/your-project.git
```

## Package Renaming

The template uses `com.profiletailors` as the base package. You should rename this to match your project's group.

1. Move files in `app/src/main/kotlin/com/profiletailors/app` to your desired package structure.
2. Update the `package` declaration in all source files.
3. Repeat for modules in the `examples/` directory if you keep them.

## Documentation Site

The documentation site is built with Starlight. Customize it in `docs/website/astro.config.mjs`:

- `site`: Your production URL.
- `base`: The subpath if not hosted at the root (e.g., `/your-project`).
- `starlight.title`: The title of your documentation.
- `social`: Links to your repository or social media.

## License

The template is licensed under the MIT License.

1. Update the `LICENSE` file with your name/organization and year.
2. Update `POM_LICENSE_URL` in `gradle.properties` if you change the license.

## CI/CD and GitHub Integration

If you are using GitHub Actions:

1. Review `.github/workflows/` and update any references to `dallay/common-actions` if you wish to use your own shared workflows.
2. Update `.github/CODEOWNERS` to reflect the maintainers of your project.
3. Update the `README.md` badges and links.
