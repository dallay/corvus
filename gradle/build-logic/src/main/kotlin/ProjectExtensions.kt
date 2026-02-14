@file:Suppress("PackageDirectoryMismatch", "UnstableApiUsage")

package com.profiletailors.plugin

import com.profiletailors.plugin.gradle.ExecValueSource
import java.io.File
import javax.inject.Inject
import kotlin.jvm.optionals.getOrNull
import org.gradle.api.JavaVersion
import org.gradle.api.Project
import org.gradle.api.artifacts.VersionCatalog
import org.gradle.api.artifacts.VersionCatalogsExtension
import org.gradle.api.file.ArchiveOperations
import org.gradle.api.file.ConfigurableFileTree
import org.gradle.api.file.ProjectLayout
import org.gradle.api.initialization.Settings
import org.gradle.api.internal.file.FileOperations
import org.gradle.api.invocation.Gradle
import org.gradle.api.model.ObjectFactory
import org.gradle.api.plugins.ExtraPropertiesExtension
import org.gradle.api.plugins.JavaPluginExtension
import org.gradle.api.plugins.PluginAware
import org.gradle.api.provider.ProviderFactory
import org.gradle.internal.service.ServiceRegistry
import org.gradle.invocation.DefaultGradle
import org.gradle.kotlin.dsl.getByType
import org.gradle.kotlin.dsl.newInstance
import org.gradle.kotlin.dsl.of
import org.gradle.process.ExecOperations

/**
 * Returns the plugin key for the current project.
 *
 * @return The plugin key for the current project.
 */
fun PluginAware.projectKey(prefix: String): String {
  return when (this) {
    is Project -> "${prefix}${this.path.replace(":", ".")}"
    is Settings -> "${prefix}.settings"
    else -> error("Unknown PluginAware type ${this.javaClass.name}")
  }
}

val PluginAware.sharedGradle: Gradle
  get() =
    when (this) {
      is Project -> this.gradle
      is Settings -> this.gradle
      is Gradle -> this.gradle
      else -> error("Unknown PluginAware type ${this.javaClass.name}")
    }

/**
 * see:
 * [use_project_during_execution](https://docs.gradle.org/nightly/userguide/configuration_cache_requirements.html#config_cache:requirements:use_project_during_execution)
 * [service_injection](https://docs.gradle.org/nightly/userguide/service_injection.html)
 */
interface Injected {
  @get:Inject val providers: ProviderFactory
  @get:Inject val objects: ObjectFactory
  @get:Inject val layout: ProjectLayout
  @get:Inject val archives: ArchiveOperations
  @get:Inject val files: FileOperations
  @get:Inject val exec: ExecOperations
}

val Project.injected
  get() = project.objects.newInstance<Injected>()

/** @see [VersionCatalog] */
internal val Project.libs
  get(): VersionCatalog = extensions.getByType<VersionCatalogsExtension>().named("libs")

/** @see [VersionCatalog.findVersion] */
internal fun Project.versionFromCatalog(alias: String): String {
  return libs.findVersion(alias).get().displayName
}

/** @see [JavaVersion.current] */
internal fun Project.javaVersion(): JavaVersion {
  try {
    val extension = extensions.findByType(JavaPluginExtension::class.java)
    if (extension != null) {
      val toolchain = extension.toolchain
      val version = toolchain.languageVersion.get().asInt()
      return JavaVersion.toVersion(version)
    }
  } catch (_: Throwable) {
    // ignore failures and fallback to java version in which Gradle is running
  }
  return JavaVersion.current()
}

/**
 * Returns a [ConfigurableFileTree] for the given source directory (default: "src").
 *
 * @param src Relative source directory path (defaults to "src").
 */
fun Project.sourceFolder(src: String = "src"): ConfigurableFileTree =
  fileTree(isolated.projectDirectory.dir(src))

fun Project.isolatedProjectEnable() =
  findProperty("org.gradle.unsafe.isolated-projects")?.toString()?.toBoolean() ?: false

/**
 * Reset the task group of all tasks that match the given regex.
 *
 * @param taskName The [String] or [Regex] to match task names.
 * @param distGroup The destination group.
 */
fun Project.resetTaskGroup(taskName: Any, distGroup: String) {
  gradle.projectsEvaluated {
    runCatching {
      tasks
        .named {
          when (taskName) {
            is String -> it == taskName
            is Regex -> it.matches(taskName)
            else -> false
          }
        }
        .configureEach {
          group = distGroup
          description = "$description [group = $distGroup]"
        }
    }
  }
}

fun Gradle.serviceRegistry(): ServiceRegistry = (this as DefaultGradle).services

/**
 * command line args > system properties > gradle.properties > environment variables
 *
 * [see doc](https://docs.gradle.org/current/userguide/build_environment.html)
 *
 * usage:
 * ```
 * // settings.gradle.kts
 * settings.getOrDefault(LocalConfig.Props.XXX)
 *
 * // build.gradle.kts
 * project.getOrDefault(LocalConfig.Props.XXX)
 * ```
 *
 * providers.systemProperty(key)
 *
 * via:
 * ```
 * // command line args(-D)
 * ./gradlew build -DXXX='YYY'
 * ```
 *
 * providers.gradleProperty(key)
 *
 * via:
 * ```
 * // command line args(-D > -P)
 * ./gradlew build -Dorg.gradle.project.XXX='YYY'
 * ./gradlew build -PXXX='YYY'
 *
 * // environment variables
 * export ORG_GRADLE_PROJECT_XXX='YYY'
 * ```
 *
 * @param key The property key to get.
 * @param defaultValue The default value to return if the property is not set.
 * @param fromProvider Whether to get the property from the provider.
 * @see [Project.findProperty] and search `Dynamic Project Properties` for more detail.
 */
fun PluginAware.propOrDefault(
  key: String,
  defaultValue: String,
  fromProvider: Boolean = true,
): String {
  // TODO: https://github.com/gradle/gradle/issues/29700
  val waitingFixThisBug = true
  val value =
    when (this) {
      is Project -> {
        if (waitingFixThisBug) {
          val property = findProperty(key)
          property as? String ?: defaultValue
        } else if (!fromProvider) {
          /*
           * https://github.com/gradle/gradle/issues/29600
           *
           * project.findProperty() does not search through this project's ancestor projects.
           * Instead, it skips right to the root if there is no property local to the current project
           *
           * project.findProperty():
           * - ✅ get value from module's gradle.properties file
           * - ❌ get value from ancestor's gradle.properties file
           * - ✅ get value from setting's gradle.properties file
           */
          val property = findProperty(key)
          property as? String ?: defaultValue
        } else {
          /*
           * https://github.com/gradle/gradle/issues/23572
           * https://github.com/gradle/gradle/issues/24491
           *
           * providers.gradleProperty() doesn't read value from module's gradle.properties file:
           *
           *
           * providers.gradleProperty():
           * - ❌ get value from module's gradle.properties file
           * - ❌ get value from ancestor's gradle.properties file
           * - ✅ get value from setting's gradle.properties file
           */
          providers
            .gradleProperty(key)
            //          .orElse(providers.systemProperty(key))
            //          .orElse(providers.environmentVariable(key))
            .getOrNull() ?: defaultValue
        }
      }
      is Settings -> {
        if (!fromProvider) {
          extensions.extraProperties.getOrDefault(key, defaultValue)
        } else {
          providers
            .gradleProperty(key)
            //          .orElse(providers.systemProperty(key))
            //          .orElse(providers.environmentVariable(key))
            .getOrNull() ?: defaultValue
        }
      }
      else -> error("Unknown PluginAware type ${javaClass.name}")
    }
  return value
}

internal fun ExtraPropertiesExtension.getOrDefault(key: String, defaultValue: String): String {
  val value =
    try {
      get(key).toString()
    } catch (_: ExtraPropertiesExtension.UnknownPropertyException) {
      ""
    }
  return value.ifBlank { defaultValue }
}

object GradleExtTool {
  /** Default git ignore patterns. */
  val defaultGitIgnore = listOf("**/__*", "**/__*/**")

  /**
   * Returns true if the project is running in build logic test environment.
   *
   * @param project The project to check.
   */
  fun isTestEnv(project: Project) = project.libs.findVersion("latest").getOrNull() == null

  fun isSnapshot(version: String): Boolean {
    return version.endsWith("-SNAPSHOT")
  }

  /**
   * Finds the path to the gradlew script.
   *
   * @param searchPath The path to search for the gradlew script.
   */
  @Suppress("detekt:ReturnCount")
  fun findGradlew(searchPath: String = ""): java.nio.file.Path? {
    val dir = File(searchPath)
    val scriptName = org.gradle.internal.os.OperatingSystem.current().getScriptName("gradlew")
    val gradlew = dir.resolve(scriptName)
    if (gradlew.exists()) {
      return gradlew.toPath()
    } else {
      val parent = dir.parent ?: return null
      return findGradlew(parent)
    }
  }

  fun openBrowser(providers: ProviderFactory, url: String) {
    try {
      //    // Use Java AWT cross-platform solution
      //    if (Desktop.isDesktopSupported() &&
      // Desktop.getDesktop().isSupported(Desktop.Action.BROWSE)) {
      //      Desktop.getDesktop().browse(URI(url))
      //    }
      val os = System.getProperty("os.name").lowercase()
      val command =
        when {
          os.contains("win") -> listOf("cmd", "/c", "start", url)
          os.contains("mac") -> listOf("open", url)
          else -> listOf("xdg-open", url) // Linux/Unix
        }
      val execValueSource =
        providers.of(ExecValueSource::class) {
          parameters {
            async.set(true)
            commands.set(command)
          }
        }
      execValueSource.get()
    } catch (_: Exception) {}
  }
}
