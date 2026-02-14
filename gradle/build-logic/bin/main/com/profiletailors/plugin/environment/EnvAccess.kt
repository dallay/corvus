@file:Suppress("UnstableApiUsage")

package com.profiletailors.plugin.environment

import com.profiletailors.plugin.gradle.eagerSharedCache
import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.projectKey
import java.io.File
import java.util.*
import org.gradle.api.Project
import org.gradle.api.initialization.Settings
import org.gradle.api.plugins.PluginAware
import org.gradle.api.provider.ProviderFactory

object EnvAccess {

  /**
   * Returns true if the current build is running in a CI environment.
   *
   * @param providers The Gradle [ProviderFactory] instance.
   * @return True if the current build is running in a CI environment, false otherwise.
   */
  fun isCi(providers: ProviderFactory): Boolean {
    val ci = LocalConfig.Props.CI
    val key = ci.key
    val defaultValue = ci.defaultValue
    val isCI =
      providers
        .environmentVariable(key)
        .orElse(providers.systemProperty(key))
        .orElse(providers.gradleProperty(key))
        .getOrNull() ?: defaultValue
    return isCI.toBoolean()
  }
}

/**
 * Returns the project build properties.
 *
 * @return The project build properties.
 */
fun PluginAware.buildProperties(): Properties {
  val key = this.projectKey("projectBuildProperties")
  return eagerSharedCache<Properties>(key) {
    val properties = Properties()
    when (this) {
      is Project -> {
        val buildPropertiesFile = isolated.projectDirectory.file("build.properties")
        providers.fileContents(buildPropertiesFile).asText.orNull?.also {
          properties.load(it.reader())
        }
      }
      is Settings -> {
        val buildPropertiesFile = File(this.settingsDir, "build.properties")
        if (buildPropertiesFile.exists()) {
          properties.load(buildPropertiesFile.reader())
        }
      }
      else -> error("Unknown PluginAware type ${this.javaClass.name}")
    }
    properties
  }
}
