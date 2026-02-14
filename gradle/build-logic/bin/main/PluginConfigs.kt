@file:Suppress("UnstableApiUsage", "PackageDirectoryMismatch")

package com.profiletailors.plugin

import org.gradle.api.initialization.Settings

@Suppress("ConstPropertyName")
object DefaultProjects {
  const val buildLogic = "gradle/build-logic"
  const val versionsPath = ":versions"
  const val versions = "gradle/versions"
  const val aggregationPath = ":aggregation"
  const val aggregation = "gradle/aggregation"
  const val docs = "docs"
}

/**
 * Create a version catalog from a file.
 *
 * @param map The map of version catalog names and paths.
 */
fun Settings.dyCreateVersionCatalogs(map: Map<String, String>) {
  dependencyResolutionManagement {
    versionCatalogs {
      map.forEach { (name, path) -> create(name) { from(layout.settingsDirectory.files(path)) } }
    }
  }
}

/**
 * Include a project with the given name and path.
 *
 * @param map The map of project names and paths.
 */
fun Settings.dyIncludeProjects(map: Map<String, String>) {
  map.forEach { (name, path) ->
    val projectDir = rootDir.resolve(path)
    if (projectDir.exists()) {
      include(name)
      project(name).projectDir = projectDir
    }
  }
}
