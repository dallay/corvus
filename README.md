# Gradle Template

[![maven-central-version](https://img.shields.io/maven-central/v/com.profiletailors/profiletailors-build-logic?strategy=latestProperty)](https://central.sonatype.com/artifact/com.profiletailors/profiletailors-build-logic)
[![maven-metadata-url](https://img.shields.io/maven-metadata/v?metadataUrl=https%3A%2F%2Frepo1.maven.org%2Fmaven2%2Fcom%2Fprofiletailors%2Fprofiletailors-build-logic%2Fmaven-metadata.xml)](https://central.sonatype.com/artifact/com.profiletailors/profiletailors-build-logic)
[![git-hub-release](https://img.shields.io/github/v/release/dallay/starter-gradle)](https://github.com/dallay/starter-gradle/releases)

[![codacy-grade](https://img.shields.io/codacy/grade/64109c17cc5c4ea090db54cb773621fe)](https://app.codacy.com/gh/dallay/starter-gradle/dashboard)
[![codecov](https://img.shields.io/codecov/c/github/dallay/starter-gradle)](https://app.codecov.io/gh/dallay/starter-gradle)
[![git-hub-actions-workflow-status](https://img.shields.io/github/actions/workflow/status/dallay/starter-gradle/publish-release.yml)](https://github.com/dallay/starter-gradle/actions/workflows/publish-release.yml)

[![JDK](https://img.shields.io/badge/dynamic/toml?logo=openjdk&label=JDK&color=brightgreen&url=https%3A%2F%2Fraw.githubusercontent.com%2Fdallay%2Fstarter-gradle%2Fmain%2Fgradle%2Flibs.versions.toml&query=%24.versions.jdk&suffix=%2B)](https://jdk.java.net)
[![GRADLE](https://img.shields.io/badge/dynamic/toml?logo=gradle&label=Gradle&color=209BC4&url=https%3A%2F%2Fraw.githubusercontent.com%2Fdallay%2Fstarter-gradle%2Fmain%2Fgradle%2Flibs.versions.toml&query=%24.versions.gradle)](https://gradle.org)
[![KOTLIN](https://img.shields.io/badge/dynamic/toml?logo=kotlin&label=Kotlin&color=7f52ff&url=https%3A%2F%2Fraw.githubusercontent.com%2Fdallay%2Fstarter-gradle%2Fmain%2Fgradle%2Flibs.versions.toml&query=%24.versions.kotlin)](https://kotlinlang.org/docs/getting-started.html)
[![NODE](https://img.shields.io/badge/dynamic/toml?logo=nodedotjs&label=Node&color=5FA04E&url=https%3A%2F%2Fraw.githubusercontent.com%2Fdallay%2Fstarter-gradle%2Fmain%2Fgradle%2Flibs.versions.toml&query=%24.versions.node)](https://nodejs.org/en/download)

[![dallay](https://img.shields.io/badge/author-🤖_dallay-E07A28?logo=github)](https://github.com/dallay)
[![Markdown](https://img.shields.io/badge/md-GFM-0070C0?logo=Markdown)](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax)
[![git-hub-license](https://img.shields.io/github/license/dallay/starter-gradle)](https://github.com/dallay/starter-gradle)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/dallay/starter-gradle)

## 🏕️ Project Template

This repo contains a Gradle project structure with:

- **Centralized and maintainable** build configuration and custom build logic
- **No dependency hell** through smart dependency management with dependency rules and analysis

The starter project contains everything for a traditional JVM project.
The structure though, is good for any kind of project you may build with Gradle
(**Kotlin**, **Groovy**, **Scala**, ...).

## 🧱 Project Overview

You can find a detailed explanation of the project structure
in [Gradle Basics](https://docs.gradle.org/current/userguide/gradle_basics.html).

### Core Concepts

![Gradle Project Structure](https://docs.gradle.org/current/userguide/img/gradle-basic-1.png)

### Project Structure

```
project
├── gradle
│   ├── libs.versions.toml
│   └── wrapper
│       ├── gradle-wrapper.jar
│       └── gradle-wrapper.properties
├── gradlew
├── gradlew.bat
├── settings.gradle(.kts)
├── subproject-a
│   ├── build.gradle(.kts)
│   └── src
└── subproject-b
    ├── build.gradle(.kts)
    └── src
```

### Build Lifecycle

![Build Lifecycle](https://docs.gradle.org/current/userguide/img/build-lifecycle-example.png)

### Configuration Cache

![Configuration Cache](https://docs.gradle.org/nightly/userguide/img/configuration-cache-4.png)

The [Configuration Cache](https://docs.gradle.org/nightly/userguide/configuration_cache.html) improves build performance
by caching the result of the configuration phase and reusing it for subsequent builds.

### Dependency Scopes

![https://docs.gradle.org/nightly/userguide/declaring_configurations.html#sec:resolvable-consumable-configs](https://docs.gradle.org/nightly/userguide/img/dependency-management-java-configurations.png)

Mapping between Java module directives and Gradle configurations to declare
dependencies: [declaring_module_dependencies](https://docs.gradle.org/nightly/userguide/java_library_plugin.html#declaring_module_dependencies)

## 🍰 Project Usage

### Using Make (Recommended)

This project includes a `Makefile` with standardized commands that work across all operating systems:

```shell
make help              # Show all available commands
make run               # Run the main application
make build             # Build the entire project
make test              # Run all tests
make check             # Run all checks (format, lint, tests)
```

#### Quick Reference

| Command              | Description                            |
| -------------------- | -------------------------------------- |
| `make run`           | Run the main application (app module)  |
| `make build`         | Build the entire project               |
| `make build-fast`    | Build without tests (faster)           |
| `make test`          | Run all tests                          |
| `make test-coverage` | Run tests with coverage report         |
| `make format`        | Format all code (Spotless)             |
| `make check`         | Run all checks (format + lint + tests) |
| `make clean`         | Clean build artifacts                  |
| `make deps`          | Show project dependencies              |

#### Examples

```shell
# Quick development cycle
make format build-fast

# Run specific example
make run-java      # Run Java example
make run-kotlin    # Run Kotlin example
make run-spring    # Run Spring example

# Full CI pipeline
make all          # clean + build + test + check
```

### Using Gradle Directly

You can also run `./gradlew help` to get
Gradle [command-line usage help](https://docs.gradle.org/current/userguide/command_line_interface.html).

```shell
./gradlew help                    # Show Gradle help
./gradlew build                   # Build project
./gradlew app:run                 # Run app module
./gradlew test                    # Run tests
./gradlew check                   # Run all checks
```

### Initial Setup

Make the Gradle wrapper executable:

```shell
git update-index --chmod=+x gradlew
# Or use make:
make setup
```

## 🏝️ Thanks

This project is heavily inspired by the following awesome projects.

- [https://github.com/jjohannes/gradle-project-setup-howto](https://github.com/jjohannes/gradle-project-setup-howto)
- [https://github.com/hiero-ledger/hiero-gradle-conventions](https://github.com/hiero-ledger/hiero-gradle-conventions)
- [https://github.com/android/nowinandroid](https://github.com/android/nowinandroid)
