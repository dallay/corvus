@file:Suppress("UnstableApiUsage")

rootProject.name = "profiletailors-build-logic"

run {
  enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")
  enableFeaturePreview("STABLE_CONFIGURATION_CACHE")
}

pluginManagement {
  repositories {
    gradlePluginPortal()
    mavenCentral()
    maven {
      setUrl("https://central.sonatype.com/repository/maven-snapshots/")
      mavenContent { snapshotsOnly() }
      content { includeVersionByRegex(".*", ".*", ".*-SNAPSHOT(?:\\+.*)?") }
    }
    google {
      content {
        includeGroupAndSubgroups("androidx")
        includeGroupAndSubgroups("com.android")
        includeGroupAndSubgroups("com.google")
      }
    }
  }
}

dependencyResolutionManagement {
  versionCatalogs { create("libs", Action { from(files("../libs.versions.toml")) }) }
  repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
  repositories {
    mavenLocal()
    mavenCentral()
    maven {
      setUrl("https://central.sonatype.com/repository/maven-snapshots/")
      mavenContent { snapshotsOnly() }
      content { includeVersionByRegex(".*", ".*", ".*-SNAPSHOT(?:\\+.*)?") }
    }
    maven {
      setUrl("https://jitpack.io")
      content { includeGroupByRegex("com\\.github.*") }
    }
    google {
      content {
        includeGroupAndSubgroups("androidx")
        includeGroupAndSubgroups("com.android")
        includeGroupAndSubgroups("com.google")
      }
    }
    gradlePluginPortal()
  }
}

plugins { id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0" }
