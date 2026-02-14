package com.profiletailors.plugin.gradle

import com.profiletailors.plugin.sharedGradle
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.charset.Charset
import java.util.*
import javax.inject.Inject
import org.gradle.api.plugins.PluginAware
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.provider.ValueSource
import org.gradle.api.provider.ValueSourceParameters
import org.gradle.process.ExecOperations

/**
 * Computes the extensions for the given name with reified type.
 *
 * @param name The name of the extension.
 * @param loader The loader to create the extension if not exists.
 */
inline fun <reified T : Any> PluginAware.computedExtension(
  name: String,
  noinline loader: () -> T,
): T {
  val ext = sharedGradle.extensions
  if (name.isBlank()) error("name cannot be blank")
  val existing = ext.findByName(name)
  return if (existing != null) {
    require(existing is T) {
      "Extension '$name' is not of expected type ${T::class.java}, but was ${existing::class.java}"
    }
    existing
  } else {
    val value = loader()
    ext.add(name, value)
    value
  }
}

/**
 * Loads properties from the given file with the given name. to `extensions.extraProperties`
 *
 * @param name The name of the properties.
 * @param file The file to load the properties from.
 * @return The loaded properties.
 */
fun PluginAware.computedProperties(name: String, file: File): Properties {
  val extra = sharedGradle.extensions.extraProperties
  if (name.isBlank()) error("name cannot be blank")
  runCatching { extra.get(name) }
    .onSuccess {
      if (it is Properties) {
        return it
      }
    }
  val properties = Properties()
  if (file.exists()) {
    file.reader().use { properties.load(it) }
  }
  extra.set(name, properties)
  return properties
}

/**
 * A value source that returns the environment variables with the given substring. Usage:
 * ```
 * val jdkLocationsProvider = providers.of(EnvVarsWithSubstringValueSource::class) {
 *   parameters {
 *     substring = "JDK"
 *   }
 * }
 * val jdkLocations = jdkLocationsProvider.get()
 * ```
 *
 * see:
 * [config_cache:requirements](https://docs.gradle.org/nightly/userguide/configuration_cache_requirements.html#config_cache:requirements:external_processes)
 */
abstract class EnvVarsWithSubstringValueSource :
  ValueSource<Map<String, String>, EnvVarsWithSubstringValueSource.Parameters> {
  interface Parameters : ValueSourceParameters {
    val substring: Property<String>
  }

  override fun obtain(): Map<String, String> {
    return System.getenv().filterKeys { key -> key.contains(parameters.substring.get()) }
  }
}

/**
 * A value source that executes the given command line and returns the output. Usage:
 * ```
 * val gitVersionProvider = providers.of(ExecValueSource::class) {
 *   parameters {
 *     async.set(true)
 *     commands.addAll("git", "--version")
 *   }
 * }
 * val gitVersion = gitVersionProvider.get()
 * ```
 *
 * see:
 * [config_cache:requirements](https://docs.gradle.org/nightly/userguide/configuration_cache_requirements.html#config_cache:requirements:external_processes)
 */
abstract class ExecValueSource : ValueSource<String, ExecValueSource.Parameters> {
  @get:Inject abstract val execOperations: ExecOperations

  interface Parameters : ValueSourceParameters {
    val async: Property<Boolean>
    val commands: ListProperty<String>
  }

  @Suppress("detekt:SpreadOperator")
  override fun obtain(): String {
    val isAsync = parameters.async.orNull ?: false
    val execCommandLine = parameters.commands.get()
    if (isAsync) {
      Thread.startVirtualThread {
        execOperations.exec { commandLine(*execCommandLine.toTypedArray()) }
      }
      return ""
    } else {
      val output = ByteArrayOutputStream()
      execOperations.exec {
        commandLine(*execCommandLine.toTypedArray())
        standardOutput = output
      }
      return String(output.toByteArray(), Charset.defaultCharset())
    }
  }
}
